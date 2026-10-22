//! The throughput tests in words: the dashboard's panel, each test's row, and the warning before
//! a test on a connection where data counts.

use application::ThroughputRecord;
use domain::{Bitrate, ConnectionContext, ConnectionKind, TestProfile, ThroughputResult};
use fluent_bundle::FluentArgs;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::dashboard_display::{format_date_time, format_or_unknown};
use crate::text_catalog::TextCatalog;

const BITS_PER_MEGABIT: f64 = 1_000_000.0;
const BYTES_PER_MEGABYTE: f64 = 1_000_000.0;
/// Below this rate, one decimal says more than the integer.
const DECIMAL_BELOW_MEGABITS: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ProfileChoice {
    Quick,
    Full,
}

impl From<ProfileChoice> for TestProfile {
    fn from(choice: ProfileChoice) -> Self {
        match choice {
            ProfileChoice::Quick => Self::Quick,
            ProfileChoice::Full => Self::Full,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThroughputRow {
    pub when: String,
    pub profile: String,
    pub download: String,
    pub upload: String,
    pub loaded_latency: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThroughputPanel {
    /// The setting « Autoriser les tests de débit ».
    pub is_allowed: bool,
    /// Under the buttons: when tests leave, and how much data each may use.
    pub note: String,
    /// Newest first.
    pub tests: Vec<ThroughputRow>,
}

/// What a click on a test gives: its row, or the warning to accept first where data counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ThroughputOutcome {
    Done { row: ThroughputRow },
    NeedsConfirmation { warning: String },
}

impl ThroughputPanel {
    /// O(t) for t tests.
    #[must_use]
    pub fn compose(
        is_allowed: bool,
        records: &[ThroughputRecord],
        offset: i64,
        catalog: &TextCatalog,
    ) -> Self {
        let mut args = FluentArgs::new();
        args.set(
            "quick",
            format_megabytes(TestProfile::Quick.compute_max_bytes(), catalog),
        );
        args.set(
            "full",
            format_megabytes(TestProfile::Full.compute_max_bytes(), catalog),
        );
        Self {
            is_allowed,
            note: catalog.render("throughput-note", Some(&args)),
            tests: records
                .iter()
                .map(|record| ThroughputRow::compose(record.utc, &record.result, offset, catalog))
                .collect(),
        }
    }
}

/// « Partage de connexion : ce test consomme jusqu'à 4,2 Mo. » where data counts, read when the
/// user clicks; `None` elsewhere. O(s) for s steps.
#[must_use]
pub fn compose_warning(
    context: Option<&ConnectionContext>,
    profile: TestProfile,
    catalog: &TextCatalog,
) -> Option<String> {
    let key = match context {
        Some(context) if context.kind == ConnectionKind::PhoneHotspot => {
            "throughput-warning-hotspot"
        }
        Some(context) if context.is_limited() => "throughput-warning-limited",
        Some(_) | None => return None,
    };
    let mut args = FluentArgs::new();
    args.set(
        "data",
        format_megabytes(profile.compute_max_bytes(), catalog),
    );
    Some(catalog.render(key, Some(&args)))
}

impl ThroughputRow {
    /// O(1).
    #[must_use]
    pub fn compose(
        utc: i64,
        result: &ThroughputResult,
        offset: i64,
        catalog: &TextCatalog,
    ) -> Self {
        let rate = |rate: Option<Bitrate>| {
            rate.map_or_else(
                || catalog.render("value-unknown", None),
                |rate| format_bitrate(rate, catalog),
            )
        };
        Self {
            when: format_date_time(utc, offset, catalog),
            profile: catalog.render(to_profile_key(result.profile), None),
            download: rate(result.download),
            upload: rate(result.upload),
            loaded_latency: format_or_unknown(result.loaded_latency, catalog),
            data: format_megabytes(result.bytes, catalog),
        }
    }
}

const fn to_profile_key(profile: TestProfile) -> &'static str {
    match profile {
        TestProfile::Quick => "throughput-quick",
        TestProfile::Full => "throughput-full",
    }
}

/// « 2,5 Mbit/s » below 10 Mbit/s, « 85 Mbit/s » above. O(1).
#[expect(
    clippy::cast_precision_loss,
    reason = "a line's rate fits a double exactly enough to show"
)]
pub fn format_bitrate(rate: Bitrate, catalog: &TextCatalog) -> String {
    let megabits = rate.to_bits_per_second() as f64 / BITS_PER_MEGABIT;
    let value = if megabits < DECIMAL_BELOW_MEGABITS {
        format_one_decimal(megabits)
    } else {
        format!("{megabits:.0}")
    };
    let mut args = FluentArgs::new();
    args.set("value", value);
    catalog.render("unit-megabits", Some(&args))
}

/// « 4,2 Mo », one decimal. O(1).
#[expect(
    clippy::cast_precision_loss,
    reason = "a test moves a few hundred megabytes at most"
)]
pub fn format_megabytes(bytes: u64, catalog: &TextCatalog) -> String {
    let mut args = FluentArgs::new();
    args.set(
        "value",
        format_one_decimal(bytes as f64 / BYTES_PER_MEGABYTE),
    );
    catalog.render("unit-megabytes", Some(&args))
}

/// « 4,2 »: one decimal, after a comma. O(1).
pub fn format_one_decimal(value: f64) -> String {
    format!("{value:.1}").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::time::Duration;

    use application::NetworkKey;
    use domain::{MacAddress, Medium, Metering};

    use super::*;

    const NBSP: char = '\u{a0}';
    /// 2023-11-14 13:05 UTC.
    const SOME_UTC: i64 = 1_699_967_100;
    const PARIS: i64 = 3600;

    /// `~` marks a no-break space.
    fn insert_no_break_spaces(text: &str) -> String {
        text.replace('~', &NBSP.to_string())
    }

    fn create_context(kind: ConnectionKind, metering: Metering) -> ConnectionContext {
        ConnectionContext {
            kind,
            is_vpn: false,
            gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, 0x31]),
            gateway_address: Some(Ipv4Addr::new(192, 168, 1, 1)),
            metering,
            medium: Medium::WiFi,
        }
    }

    fn create_record(download: Option<u64>) -> ThroughputRecord {
        ThroughputRecord {
            utc: SOME_UTC,
            network: NetworkKey {
                gateway: None,
                kind: ConnectionKind::WiFi,
                name: None,
            },
            result: ThroughputResult {
                profile: TestProfile::Quick,
                download: download.map(Bitrate::from_bits_per_second),
                upload: Some(Bitrate::from_bits_per_second(2_500_000)),
                loaded_latency: Some(Duration::from_millis(46)),
                bytes: 4_200_000,
                duration: Duration::from_secs(3),
            },
        }
    }

    #[test]
    fn writes_a_test_the_french_way() {
        // Arrange
        let catalog = TextCatalog::load();
        let record = create_record(Some(85_400_000));

        // Act
        let row = ThroughputRow::compose(record.utc, &record.result, PARIS, &catalog);

        // Assert
        assert_eq!(row.when, insert_no_break_spaces("14/11, 14~h~05"));
        assert_eq!(row.profile, "Rapide");
        assert_eq!(row.download, insert_no_break_spaces("85~Mbit/s"));
        assert_eq!(row.upload, insert_no_break_spaces("2,5~Mbit/s"));
        assert_eq!(row.loaded_latency, insert_no_break_spaces("46~ms"));
        assert_eq!(row.data, insert_no_break_spaces("4,2~Mo"));
    }

    #[test]
    fn says_unknown_for_a_direction_not_measured() {
        // Arrange
        let catalog = TextCatalog::load();
        let record = create_record(None);

        // Act
        let row = ThroughputRow::compose(record.utc, &record.result, PARIS, &catalog);

        // Assert
        assert_eq!(row.download, "inconnu");
    }

    #[test]
    fn warns_before_a_test_where_data_counts_only() {
        // Arrange
        let catalog = TextCatalog::load();
        let hotspot = create_context(ConnectionKind::PhoneHotspot, Metering::Unknown);
        let limited = create_context(ConnectionKind::WiFi, Metering::Metered);
        let free = create_context(ConnectionKind::Ethernet, Metering::Unmetered);

        // Act
        let on_hotspot = compose_warning(Some(&hotspot), TestProfile::Quick, &catalog);
        let on_limited = compose_warning(Some(&limited), TestProfile::Full, &catalog);
        let on_free = compose_warning(Some(&free), TestProfile::Full, &catalog);

        // Assert
        assert!(on_hotspot.is_some_and(|warning| warning.starts_with("Partage de connexion")));
        assert!(on_limited.is_some_and(|warning| warning.starts_with("Connexion limitée")));
        assert_eq!(on_free, None);
    }

    #[test]
    fn says_how_much_data_each_test_may_use() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let panel = ThroughputPanel::compose(true, &[], PARIS, &catalog);

        // Assert
        assert!(panel.note.contains(&format_megabytes(
            TestProfile::Quick.compute_max_bytes(),
            &catalog
        )));
    }
}
