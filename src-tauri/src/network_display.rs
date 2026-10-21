//! The network in words: what to call it, and how the computer reaches it.

use domain::{ConnectionContext, ConnectionKind, WifiBand, WifiLink};
use fluent_bundle::FluentArgs;
use serde::Serialize;
use specta::Type;

use crate::text_catalog::TextCatalog;

/// Only macOS and Windows hide the Wi-Fi name behind the location permission.
const CAN_REVEAL_NAME: bool = cfg!(any(target_os = "macos", target_os = "windows"));

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDisplay {
    pub label: String,
    pub connection: String,
    /// Signal, channel and rate of the Wi-Fi carrying the traffic, e.g. "-58 dBm, canal 36".
    pub wifi_details: Option<String>,
    /// Offered when the system hides the Wi-Fi name until the location permission is given.
    pub reveal_label: Option<String>,
}

impl NetworkDisplay {
    /// The Wi-Fi name when the system gives it, a generic name otherwise. O(1).
    #[must_use]
    pub fn from_context(
        context: &ConnectionContext,
        wifi: Option<&WifiLink>,
        catalog: &TextCatalog,
    ) -> Self {
        let (label_key, connection_key) = match (context.kind, context.is_vpn) {
            (ConnectionKind::Ethernet, _) => ("network-ethernet", "connection-ethernet"),
            (ConnectionKind::WiFi, _) => ("network-wifi", "connection-wifi"),
            (ConnectionKind::Cellular, _) => ("network-cellular", "connection-cellular"),
            (ConnectionKind::PhoneHotspot, _) => ("network-hotspot", "connection-hotspot"),
            (ConnectionKind::Other, true) => ("network-vpn", "connection-vpn"),
            (ConnectionKind::Other, false) => ("network-other", "connection-other"),
        };
        let wifi = wifi.filter(|_| context.is_carried_by_wifi());
        let label = match wifi {
            Some(WifiLink {
                name: Some(name), ..
            }) => name.clone(),
            Some(_) if context.kind == ConnectionKind::WiFi => {
                catalog.render("network-wifi-hidden", None)
            }
            _ => catalog.render(label_key, None),
        };
        let mut connection = catalog.render(connection_key, None);
        if let Some(band_key) = wifi.and_then(|link| select_band_key(link.band))
            && context.kind == ConnectionKind::WiFi
        {
            let mut args = FluentArgs::new();
            args.set("band", catalog.render(band_key, None));
            connection = catalog.render("connection-wifi-band", Some(&args));
        }
        if context.is_vpn && context.kind != ConnectionKind::Other {
            connection = append_to_connection(connection, "connection-through-vpn", catalog);
        }
        if context.is_limited() {
            connection = append_to_connection(connection, "connection-limited", catalog);
        }
        let is_name_hidden = wifi.is_some_and(|link| link.name.is_none());
        Self {
            label,
            connection,
            wifi_details: wifi.and_then(|link| describe_wifi(link, catalog)),
            reveal_label: (CAN_REVEAL_NAME && is_name_hidden)
                .then(|| catalog.render("action-reveal-name", None)),
        }
    }
}

/// `None` when the system gave none of the three values. O(1).
fn describe_wifi(link: &WifiLink, catalog: &TextCatalog) -> Option<String> {
    let render = |key: &str, value: String| {
        let mut args = FluentArgs::new();
        args.set("value", value);
        catalog.render(key, Some(&args))
    };
    let parts: Vec<String> = [
        link.signal_dbm
            .map(|signal| render("unit-dbm", signal.unsigned_abs().to_string())),
        link.channel
            .map(|channel| render("wifi-channel", channel.to_string())),
        link.rate_mbps
            .map(|rate| render("unit-megabits", rate.to_string())),
    ]
    .into_iter()
    .flatten()
    .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

const fn select_band_key(band: WifiBand) -> Option<&'static str> {
    match band {
        WifiBand::TwoPointFourGhz => Some("band-2-4-ghz"),
        WifiBand::FiveGhz => Some("band-5-ghz"),
        WifiBand::SixGhz => Some("band-6-ghz"),
        WifiBand::Unknown => None,
    }
}

fn append_to_connection(connection: String, key: &str, catalog: &TextCatalog) -> String {
    let mut args = FluentArgs::new();
    args.set("connection", connection);
    catalog.render(key, Some(&args))
}

#[cfg(test)]
mod tests {
    use domain::{Medium, Metering};

    use super::*;

    fn create_context(kind: ConnectionKind) -> ConnectionContext {
        ConnectionContext {
            kind,
            is_vpn: false,
            gateway: None,
            gateway_address: None,
            metering: Metering::Unmetered,
            medium: if kind == ConnectionKind::WiFi {
                Medium::WiFi
            } else {
                Medium::Ethernet
            },
        }
    }

    fn create_link(name: Option<&str>) -> WifiLink {
        WifiLink {
            name: name.map(str::to_owned),
            signal_dbm: Some(-58),
            rate_mbps: Some(866),
            channel: Some(36),
            band: WifiBand::FiveGhz,
        }
    }

    #[test]
    fn names_the_wifi_network_when_the_system_gives_its_name() {
        // Arrange
        let name = "réseau-de-test";

        // Act
        let network = NetworkDisplay::from_context(
            &create_context(ConnectionKind::WiFi),
            Some(&create_link(Some(name))),
            &TextCatalog::load(),
        );

        // Assert
        assert_eq!(network.label, name);
        assert_eq!(network.connection, "Wi-Fi 5\u{a0}GHz");
        assert_eq!(
            network.wifi_details.as_deref(),
            Some("\u{2212}58\u{a0}dBm, canal 36, 866\u{a0}Mbit/s")
        );
        assert_eq!(network.reveal_label, None);
    }

    #[test]
    fn says_the_name_is_hidden_and_offers_to_reveal_it() {
        // Act
        let network = NetworkDisplay::from_context(
            &create_context(ConnectionKind::WiFi),
            Some(&create_link(None)),
            &TextCatalog::load(),
        );

        // Assert
        assert_eq!(network.label, "Wi-Fi (nom masqué)");
        assert_eq!(network.reveal_label.is_some(), CAN_REVEAL_NAME);
    }

    #[test]
    fn ignores_a_wifi_association_while_a_cable_carries_the_traffic() {
        // Act
        let network = NetworkDisplay::from_context(
            &create_context(ConnectionKind::Ethernet),
            Some(&create_link(None)),
            &TextCatalog::load(),
        );

        // Assert
        assert_eq!(network.label, "Réseau Ethernet");
        assert_eq!(network.reveal_label, None);
    }
}
