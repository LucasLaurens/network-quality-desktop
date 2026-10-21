//! The dashboard's collapsed technical details in words: the connection, the Wi-Fi, the traffic
//! seen on the interface, then the box's line, read apart because the box may take seconds.

use application::LineReading;
use domain::{AccessEstimate, Bitrate, TrafficRate};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use specta::Type;

use crate::network_display::{NetworkDisplay, select_band_key};
use crate::shell_presenter::ConnectionReading;
use crate::text_catalog::TextCatalog;
use crate::throughput_display::{format_bitrate, format_megabytes};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LineDetails {
    pub section: DetailSection,
    /// Why some values are missing or how far to trust them.
    pub note: String,
}

impl TechnicalDetails {
    /// Without a route there is no connection to describe, only the traffic. O(1).
    #[must_use]
    pub fn compose(
        connection: &ConnectionReading,
        current: Option<TrafficRate>,
        peak: Option<TrafficRate>,
        catalog: &TextCatalog,
    ) -> Self {
        let row = |key: &str, value: String| DetailRow {
            label: catalog.render(key, None),
            value,
        };
        let mut sections = Vec::new();
        if let Some(context) = &connection.context {
            let network = NetworkDisplay::from_context(context, connection.wifi.as_ref(), catalog);
            sections.push(DetailSection {
                title: catalog.render("details-connection", None),
                rows: vec![
                    row("details-network", network.label),
                    row("details-connection-kind", network.connection),
                    row(
                        "details-gateway",
                        context.gateway_address.map_or_else(
                            || catalog.render("value-unknown", None),
                            |address| address.to_string(),
                        ),
                    ),
                ],
            });
            if let Some(wifi) = connection
                .wifi
                .as_ref()
                .filter(|_| context.is_carried_by_wifi())
            {
                let unknown = || catalog.render("value-unknown", None);
                let render = |key: &str, value: String| {
                    let mut args = FluentArgs::new();
                    args.set("value", value);
                    catalog.render(key, Some(&args))
                };
                sections.push(DetailSection {
                    title: catalog.render("details-wifi", None),
                    rows: vec![
                        row(
                            "details-wifi-signal",
                            wifi.signal_dbm.map_or_else(unknown, |signal| {
                                render("unit-dbm", signal.unsigned_abs().to_string())
                            }),
                        ),
                        row(
                            "details-wifi-band",
                            select_band_key(wifi.band)
                                .map_or_else(unknown, |key| catalog.render(key, None)),
                        ),
                        row(
                            "details-wifi-channel",
                            wifi.channel
                                .map_or_else(unknown, |channel| channel.to_string()),
                        ),
                        row(
                            "details-wifi-rate",
                            wifi.rate_mbps.map_or_else(unknown, |rate| {
                                render("unit-megabits", rate.to_string())
                            }),
                        ),
                    ],
                });
            }
        }
        let waiting = || catalog.render("details-traffic-waiting", None);
        let pair = |rate: TrafficRate| format_pair(rate.received, rate.sent, catalog);
        sections.push(DetailSection {
            title: catalog.render("details-traffic", None),
            rows: vec![
                row("details-traffic-now", current.map_or_else(waiting, pair)),
                row("details-traffic-peak", peak.map_or_else(waiting, pair)),
            ],
        });
        Self { sections }
    }
}

impl LineDetails {
    /// O(1).
    #[must_use]
    pub fn compose(reading: &LineReading, catalog: &TextCatalog) -> Self {
        let row = |key: &str, value: String| DetailRow {
            label: catalog.render(key, None),
            value,
        };
        let mut rows = vec![row(
            "details-access",
            catalog.render(to_access_key(reading.access), None),
        )];
        if let Some(line) = &reading.line {
            if let (Some(down), Some(up)) = (line.downstream_max, line.upstream_max) {
                rows.push(row("details-line-rate", format_pair(down, up, catalog)));
            }
            if let (Some(received), Some(sent)) = (line.total_received, line.total_sent) {
                let mut args = FluentArgs::new();
                args.set("received", format_bytes(received, catalog));
                args.set("sent", format_bytes(sent, catalog));
                rows.push(row(
                    "details-box-traffic",
                    catalog.render("details-pair", Some(&args)),
                ));
            }
        }
        let note_key = if reading.line.is_some() {
            "details-line-note"
        } else {
            "details-line-silent"
        };
        Self {
            section: DetailSection {
                title: catalog.render("details-line", None),
                rows,
            },
            note: catalog.render(note_key, None),
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

/// « ↓ 85 Mbit/s, ↑ 2,5 Mbit/s ». O(1).
fn format_pair(received: Bitrate, sent: Bitrate, catalog: &TextCatalog) -> String {
    let mut args = FluentArgs::new();
    args.set("received", format_bitrate(received, catalog));
    args.set("sent", format_bitrate(sent, catalog));
    catalog.render("details-pair", Some(&args))
}

/// « 850,0 Mo » below a gigabyte, « 9,0 Go » above. O(1).
#[expect(
    clippy::cast_precision_loss,
    reason = "a box counts a few terabytes at most"
)]
fn format_bytes(bytes: u64, catalog: &TextCatalog) -> String {
    if bytes < BYTES_PER_GIGABYTE {
        return format_megabytes(bytes, catalog);
    }
    let mut args = FluentArgs::new();
    args.set(
        "value",
        format!("{:.1}", bytes as f64 / BYTES_PER_GIGABYTE as f64).replace('.', ","),
    );
    catalog.render("unit-gigabytes", Some(&args))
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use domain::{
        BoxLine, ConnectionContext, ConnectionKind, MacAddress, Medium, Metering, WanAccessType,
        WifiBand, WifiLink,
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
                gateway_address: Some(Ipv4Addr::new(192, 168, 1, 254)),
                metering: Metering::Unmetered,
                medium,
            }),
            wifi: Some(WifiLink {
                name: Some("Maison".to_owned()),
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

        // Act
        let details =
            TechnicalDetails::compose(&create_reading(Medium::WiFi), Some(rate), None, &catalog);

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
            catalog.render("details-traffic-waiting", None)
        );
        assert_eq!(details.sections[0].rows[2].value, "192.168.1.254");
    }

    #[test]
    fn leaves_out_a_wifi_that_does_not_carry_the_traffic() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let details =
            TechnicalDetails::compose(&create_reading(Medium::Ethernet), None, None, &catalog);

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
                "ADSL (annoncé par la box)".to_owned(),
                insert_no_break_spaces("↓ 17~Mbit/s, ↑ 1,0~Mbit/s"),
                insert_no_break_spaces("↓ 9,0~Go, ↑ 700,0~Mo"),
            ]
        );
        assert_eq!(details.note, catalog.render("details-line-note", None));
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
        assert_eq!(details.note, catalog.render("details-line-silent", None));
    }
}
