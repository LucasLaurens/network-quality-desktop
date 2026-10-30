//! The dashboard's collapsed technical details in words: the connection, the Wi-Fi, the traffic
//! seen on the interface, then the box's line, read apart because the box may take seconds.

use application::LineReading;
use domain::{
    AccessEstimate, ConnectionContext, TRAFFIC_WINDOW, TrafficRate, TrafficReading, WifiLink,
};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use specta::Type;

use crate::network_display::{NetworkDisplay, select_band_key};
use crate::shell_presenter::ConnectionReading;
use crate::text_catalog::TextCatalog;
use crate::throughput_display::{format_bitrate, format_megabytes, format_one_decimal};
use crate::traffic_store::SAMPLE_EVERY;

const BYTES_PER_GIGABYTE: u64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DetailRow {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DetailSection {
    pub title: String,
    pub rows: Vec<DetailRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TechnicalDetails {
    /// The connection, the Wi-Fi when it carries the traffic, the traffic seen.
    pub sections: Vec<DetailSection>,
    /// The volume moves with every sample: reading faster shows nothing new.
    pub refresh_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LineDetails {
    pub section: DetailSection,
    /// Why some values are missing or how far to trust them.
    pub note: String,
}

impl DetailRow {
    fn compose(key: &str, value: String, catalog: &TextCatalog) -> Self {
        Self {
            label: catalog.render_text(key),
            value,
        }
    }
}

impl TechnicalDetails {
    /// Without a route there is no connection to describe, only the traffic. O(1).
    #[must_use]
    pub fn compose(
        connection: &ConnectionReading,
        traffic: TrafficReading,
        catalog: &TextCatalog,
    ) -> Self {
        let mut sections = Vec::new();
        if let Some(context) = &connection.context {
            sections.push(compose_connection_section(
                context,
                connection.wifi.as_ref(),
                catalog,
            ));
            if let Some(wifi) = connection
                .wifi
                .as_ref()
                .filter(|_| context.is_carried_by_wifi())
            {
                sections.push(compose_wifi_section(wifi, catalog));
            }
        }
        sections.push(compose_traffic_section(traffic, catalog));
        Self {
            sections,
            refresh_seconds: u32::try_from(SAMPLE_EVERY.as_secs()).unwrap_or(u32::MAX),
        }
    }
}

/// O(1).
fn compose_connection_section(
    context: &ConnectionContext,
    wifi: Option<&WifiLink>,
    catalog: &TextCatalog,
) -> DetailSection {
    let network = NetworkDisplay::from_context(context, wifi, catalog);
    let gateway = context.gateway_address.map_or_else(
        || catalog.render_text("value-unknown"),
        |address| address.to_string(),
    );
    DetailSection {
        title: catalog.render_text("details-connection"),
        rows: vec![
            DetailRow::compose("details-network", network.label, catalog),
            DetailRow::compose("details-connection-kind", network.connection, catalog),
            DetailRow::compose("details-gateway", gateway, catalog),
        ],
    }
}

/// O(1).
fn compose_wifi_section(wifi: &WifiLink, catalog: &TextCatalog) -> DetailSection {
    let unknown = || catalog.render_text("value-unknown");
    let signal = wifi.signal_dbm.map_or_else(unknown, |signal| {
        catalog.render_value("unit-dbm", "value", signal.unsigned_abs().to_string())
    });
    let band = select_band_key(wifi.band).map_or_else(unknown, |key| catalog.render_text(key));
    let channel = wifi
        .channel
        .map_or_else(unknown, |channel| channel.to_string());
    let rate = wifi.rate_mbps.map_or_else(unknown, |rate| {
        catalog.render_value("unit-megabits", "value", rate.to_string())
    });
    DetailSection {
        title: catalog.render_text("details-wifi"),
        rows: vec![
            DetailRow::compose("details-wifi-signal", signal, catalog),
            DetailRow::compose("details-wifi-band", band, catalog),
            DetailRow::compose("details-wifi-channel", channel, catalog),
            DetailRow::compose("details-wifi-rate", rate, catalog),
        ],
    }
}

/// O(1).
fn compose_traffic_section(traffic: TrafficReading, catalog: &TextCatalog) -> DetailSection {
    let format = |rate: Option<TrafficRate>| {
        rate.map_or_else(
            || catalog.render_text("details-traffic-waiting"),
            |rate| {
                format_pair(
                    format_bitrate(rate.received, catalog),
                    format_bitrate(rate.sent, catalog),
                    catalog,
                )
            },
        )
    };
    let volume = format_pair(
        format_bytes(traffic.volume.received, catalog),
        format_bytes(traffic.volume.sent, catalog),
        catalog,
    );
    DetailSection {
        title: catalog.render_text("details-traffic"),
        rows: vec![
            DetailRow {
                label: catalog.render_value(
                    "details-traffic-now",
                    "seconds",
                    TRAFFIC_WINDOW.as_secs().to_string(),
                ),
                value: format(traffic.current),
            },
            DetailRow::compose("details-traffic-peak", format(traffic.peak), catalog),
            DetailRow::compose("details-traffic-volume", volume, catalog),
        ],
    }
}

impl LineDetails {
    /// O(1).
    #[must_use]
    pub fn compose(reading: &LineReading, catalog: &TextCatalog) -> Self {
        let row = |key: &str, value: String| DetailRow::compose(key, value, catalog);
        let mut rows = vec![row(
            "details-access",
            catalog.render_text(to_access_key(reading.access)),
        )];
        if let Some(line) = &reading.line {
            // Each way on its own: many boxes give one and not the other.
            let rates = [line.downstream_max, line.upstream_max]
                .map(|rate| rate.map(|rate| format_bitrate(rate, catalog)));
            if let Some(value) = format_known_pair(rates, catalog) {
                rows.push(row("details-line-rate", value));
            }
            let totals = [line.total_received, line.total_sent]
                .map(|bytes| bytes.map(|bytes| format_bytes(bytes, catalog)));
            if let Some(value) = format_known_pair(totals, catalog) {
                rows.push(row("details-box-traffic", value));
            }
        }
        let note_key = match (reading.line, reading.access) {
            (Some(_), _) => "details-line-note",
            (None, AccessEstimate::Unknown) => "details-line-silent-unknown",
            (None, _) => "details-line-silent",
        };
        Self {
            section: DetailSection {
                title: catalog.render_text("details-line"),
                rows,
            },
            note: catalog.render_text(note_key),
        }
    }
}

const fn to_access_key(access: AccessEstimate) -> &'static str {
    match access {
        AccessEstimate::ReportedDsl => "access-reported-dsl",
        AccessEstimate::EstimatedFibre => "access-estimated-fibre",
        AccessEstimate::EstimatedAdsl => "access-estimated-adsl",
        AccessEstimate::Unknown => "access-unknown",
    }
}

/// « ↓ 17 Mbit/s, ↑ valeur inconnue » when one way is known; `None` when neither is. O(1).
fn format_known_pair(
    [received, sent]: [Option<String>; 2],
    catalog: &TextCatalog,
) -> Option<String> {
    if received.is_none() && sent.is_none() {
        return None;
    }
    let unknown = || catalog.render_text("value-unknown");
    Some(format_pair(
        received.unwrap_or_else(unknown),
        sent.unwrap_or_else(unknown),
        catalog,
    ))
}

/// « ↓ 85 Mbit/s, ↑ 2,5 Mbit/s ». O(1).
fn format_pair(received: String, sent: String, catalog: &TextCatalog) -> String {
    let mut args = FluentArgs::new();
    args.set("received", received);
    args.set("sent", sent);
    catalog.render("details-pair", Some(&args))
}

/// « 850,0 Mo » below a gigabyte, « 9,0 Go » above. O(1).
#[expect(
    clippy::cast_precision_loss,
    reason = "a byte counter reaches a few terabytes at most"
)]
fn format_bytes(bytes: u64, catalog: &TextCatalog) -> String {
    if bytes < BYTES_PER_GIGABYTE {
        return format_megabytes(bytes, catalog);
    }
    catalog.render_value(
        "unit-gigabytes",
        "value",
        format_one_decimal(bytes as f64 / BYTES_PER_GIGABYTE as f64),
    )
}

#[cfg(test)]
mod tests {
    use application::test_support::{generate_address, generate_network_name};
    use domain::{
        Bitrate, BoxLine, ConnectionKind, MacAddress, Medium, Metering, TrafficVolume,
        WanAccessType, WifiBand,
    };

    use super::*;

    /// `~` marks a no-break space.
    fn insert_no_break_spaces(text: &str) -> String {
        text.replace('~', "\u{a0}")
    }

    fn create_reading(medium: Medium) -> ConnectionReading {
        ConnectionReading {
            context: Some(ConnectionContext {
                kind: ConnectionKind::WiFi,
                is_vpn: false,
                gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, 0x31]),
                gateway_address: Some(generate_address()),
                metering: Metering::Unmetered,
                medium,
            }),
            wifi: Some(WifiLink {
                name: Some(generate_network_name()),
                signal_dbm: Some(-58),
                rate_mbps: Some(866),
                channel: Some(36),
                band: WifiBand::FiveGhz,
            }),
            latency: None,
        }
    }

    fn list_titles(details: &TechnicalDetails) -> Vec<&str> {
        details
            .sections
            .iter()
            .map(|section| section.title.as_str())
            .collect()
    }

    #[test]
    fn describes_the_connection_its_wifi_and_the_traffic() {
        // Arrange
        let catalog = TextCatalog::load();
        let rate = TrafficRate {
            received: Bitrate::from_bits_per_second(85_000_000),
            sent: Bitrate::from_bits_per_second(2_500_000),
        };
        let reading = create_reading(Medium::WiFi);
        let gateway = reading
            .context
            .and_then(|context| context.gateway_address)
            .expect("a gateway address");

        // Act
        let details = TechnicalDetails::compose(
            &reading,
            TrafficReading {
                current: Some(rate),
                peak: None,
                volume: TrafficVolume {
                    received: 1_500_000_000,
                    sent: 42_000_000,
                },
            },
            &catalog,
        );

        // Assert
        assert_eq!(
            list_titles(&details),
            ["Connexion", "Wi-Fi", "Trafic observé"]
        );
        let traffic = &details.sections[2].rows;
        assert_eq!(
            traffic[0].value,
            insert_no_break_spaces("↓ 85~Mbit/s, ↑ 2,5~Mbit/s")
        );
        assert_eq!(
            traffic[1].value,
            catalog.render_text("details-traffic-waiting")
        );
        assert_eq!(
            traffic[2].value,
            insert_no_break_spaces("↓ 1,5~Go, ↑ 42,0~Mo")
        );
        assert_eq!(details.sections[0].rows[2].value, gateway.to_string());
    }

    #[test]
    fn leaves_out_a_wifi_that_does_not_carry_the_traffic() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let details = TechnicalDetails::compose(
            &create_reading(Medium::Ethernet),
            TrafficReading::default(),
            &catalog,
        );

        // Assert
        assert_eq!(list_titles(&details), ["Connexion", "Trafic observé"]);
    }

    #[test]
    fn shows_what_the_box_announces_of_its_line() {
        // Arrange
        let catalog = TextCatalog::load();
        let reading = LineReading {
            line: Some(BoxLine {
                access_type: WanAccessType::Dsl,
                downstream_max: Some(Bitrate::from_bits_per_second(17_400_000)),
                upstream_max: Some(Bitrate::from_bits_per_second(1_000_000)),
                total_received: Some(9_000_000_000),
                total_sent: Some(700_000_000),
            }),
            access: AccessEstimate::ReportedDsl,
        };

        // Act
        let details = LineDetails::compose(&reading, &catalog);

        // Assert
        let values: Vec<&str> = details
            .section
            .rows
            .iter()
            .map(|row| row.value.as_str())
            .collect();
        assert_eq!(
            values,
            [
                "DSL (annoncé par la box)".to_owned(),
                insert_no_break_spaces("↓ 17~Mbit/s, ↑ 1,0~Mbit/s"),
                insert_no_break_spaces("↓ 9,0~Go, ↑ 700,0~Mo"),
            ]
        );
        assert_eq!(details.note, catalog.render_text("details-line-note"));
    }

    #[test]
    fn says_the_box_is_silent_and_keeps_the_estimate() {
        // Arrange
        let catalog = TextCatalog::load();
        let reading = LineReading {
            line: None,
            access: AccessEstimate::EstimatedFibre,
        };

        // Act
        let details = LineDetails::compose(&reading, &catalog);

        // Assert
        assert_eq!(details.section.rows.len(), 1);
        assert_eq!(details.section.rows[0].value, "Fibre (estimé)");
        assert_eq!(details.note, catalog.render_text("details-line-silent"));
    }

    #[test]
    fn shows_the_one_way_a_box_announces() {
        // Arrange
        let catalog = TextCatalog::load();
        let reading = LineReading {
            line: Some(BoxLine {
                access_type: WanAccessType::Other,
                downstream_max: Some(Bitrate::from_bits_per_second(17_400_000)),
                upstream_max: None,
                total_received: None,
                total_sent: None,
            }),
            access: AccessEstimate::Unknown,
        };

        // Act
        let details = LineDetails::compose(&reading, &catalog);

        // Assert
        let rows = &details.section.rows;
        assert_eq!(rows.len(), 2, "the access and the one rate known");
        assert_eq!(
            rows[1].value,
            insert_no_break_spaces("↓ 17~Mbit/s, ↑ valeur inconnue")
        );
    }

    #[test]
    fn does_not_call_estimated_what_stays_unknown() {
        // Arrange
        let catalog = TextCatalog::load();
        let reading = LineReading {
            line: None,
            access: AccessEstimate::Unknown,
        };

        // Act
        let details = LineDetails::compose(&reading, &catalog);

        // Assert
        assert_eq!(
            details.note,
            catalog.render_text("details-line-silent-unknown")
        );
    }
}
