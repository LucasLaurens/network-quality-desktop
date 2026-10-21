//! Which network the computer is on, and through what: the connection context.

use std::fmt;
use std::net::Ipv4Addr;

/// Gateways handed out by phones sharing their connection. A hint only: a home router may use
/// the same address, so it never decides alone.
const PHONE_GATEWAYS: [Ipv4Addr; 4] = [
    Ipv4Addr::new(172, 20, 10, 1),
    Ipv4Addr::new(192, 168, 43, 1),
    Ipv4Addr::new(192, 168, 42, 1),
    Ipv4Addr::new(192, 168, 42, 129),
];
/// An iPhone shares its connection on 172.20.10.0/28.
const IPHONE_NETWORK: Ipv4Addr = Ipv4Addr::new(172, 20, 10, 0);
const IPHONE_PREFIX: u8 = 28;
/// Words found in the interface names of a phone plugged over USB.
const PHONE_NAME_HINTS: [&str; 5] = [
    "iphone",
    "android",
    "rndis",
    "remote ndis",
    "apple mobile device",
];
/// How many independent signals make a phone hotspot.
const HOTSPOT_SIGNALS_NEEDED: usize = 2;

/// Link-layer address of the gateway. The all-zero address means "not resolved" and builds none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    #[must_use]
    pub fn from_octets(octets: [u8; 6]) -> Option<Self> {
        (octets != [0; 6]).then_some(Self(octets))
    }

    #[must_use]
    pub const fn to_octets(self) -> [u8; 6] {
        self.0
    }

    /// Phones and some mesh systems use random addresses; routers keep their maker's.
    #[must_use]
    pub const fn is_locally_administered(self) -> bool {
        self.0[0] & 0b10 != 0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hexadecimal: Vec<String> = self.0.iter().map(|octet| format!("{octet:02x}")).collect();
        formatter.write_str(&hexadecimal.join(":"))
    }
}

/// What carries the traffic of an interface, as the system reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Medium {
    Ethernet,
    WiFi,
    /// A cellular modem built into the computer.
    Cellular,
    /// A VPN tunnel.
    Tunnel,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gateway {
    pub ipv4: Ipv4Addr,
    pub mac: Option<MacAddress>,
}

/// What the system says about one interface: the input of the classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceSnapshot {
    pub medium: Medium,
    /// System name, display name and description, as available.
    pub names: Vec<String>,
    /// First IPv4 address and its prefix length.
    pub ipv4: Option<(Ipv4Addr, u8)>,
    pub gateway: Option<Gateway>,
}

/// The kind of connection the user recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectionKind {
    Ethernet,
    WiFi,
    Cellular,
    PhoneHotspot,
    /// Unrecognised, or hidden under a VPN.
    Other,
}

/// A network seen again is the same gateway reached the same way; the Wi-Fi name is only a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkIdentity {
    pub gateway: MacAddress,
    pub kind: ConnectionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionContext {
    /// The physical connection, under the VPN when there is one.
    pub kind: ConnectionKind,
    /// Measurements go through the VPN: they describe what the user lives.
    pub is_vpn: bool,
    pub gateway: Option<MacAddress>,
}

impl ConnectionContext {
    /// `default` carries the default route; `others` are the other interfaces that are up, where
    /// the physical link under a VPN is looked for. O(i·n) time for i interfaces of n names.
    #[must_use]
    pub fn classify(default: &InterfaceSnapshot, others: &[InterfaceSnapshot]) -> Self {
        if default.medium != Medium::Tunnel {
            return Self::describe_physical(default, false);
        }
        others
            .iter()
            .find(|snapshot| snapshot.medium != Medium::Tunnel && snapshot.gateway.is_some())
            .map_or(
                Self {
                    kind: ConnectionKind::Other,
                    is_vpn: true,
                    gateway: None,
                },
                |physical| Self::describe_physical(physical, true),
            )
    }

    /// `None` while the gateway address is unknown: such a network cannot be recognised later.
    #[must_use]
    pub fn identify(&self) -> Option<NetworkIdentity> {
        self.gateway.map(|gateway| NetworkIdentity {
            gateway,
            kind: self.kind,
        })
    }

    fn describe_physical(snapshot: &InterfaceSnapshot, is_vpn: bool) -> Self {
        let kind = if count_hotspot_signals(snapshot) >= HOTSPOT_SIGNALS_NEEDED {
            ConnectionKind::PhoneHotspot
        } else {
            match snapshot.medium {
                Medium::Ethernet => ConnectionKind::Ethernet,
                Medium::WiFi => ConnectionKind::WiFi,
                Medium::Cellular => ConnectionKind::Cellular,
                Medium::Tunnel | Medium::Other => ConnectionKind::Other,
            }
        };
        Self {
            kind,
            is_vpn,
            gateway: snapshot.gateway.and_then(|gateway| gateway.mac),
        }
    }
}

/// Independent hints that a phone shares its connection. No single one is trusted. O(n) for n
/// names.
fn count_hotspot_signals(snapshot: &InterfaceSnapshot) -> usize {
    let has_phone_gateway = snapshot
        .gateway
        .is_some_and(|gateway| PHONE_GATEWAYS.contains(&gateway.ipv4));
    let is_on_iphone_network = snapshot.ipv4.is_some_and(|(address, prefix)| {
        prefix == IPHONE_PREFIX && mask(address, IPHONE_PREFIX) == IPHONE_NETWORK
    });
    let has_phone_name = snapshot.names.iter().any(|name| {
        let name = name.to_lowercase();
        PHONE_NAME_HINTS.iter().any(|hint| name.contains(hint))
    });
    let has_random_gateway = snapshot
        .gateway
        .and_then(|gateway| gateway.mac)
        .is_some_and(MacAddress::is_locally_administered);
    [
        has_phone_gateway,
        is_on_iphone_network,
        has_phone_name,
        has_random_gateway,
    ]
    .into_iter()
    .filter(|signal| *signal)
    .count()
}

fn mask(address: Ipv4Addr, prefix: u8) -> Ipv4Addr {
    let bits = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
    Ipv4Addr::from(u32::from(address) & bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A maker-assigned address, as a home router has.
    const ROUTER_MAC: [u8; 6] = [0x54, 0x9b, 0x49, 0x87, 0xe3, 0x48];
    /// A random address, as a phone hotspot has.
    const RANDOM_MAC: [u8; 6] = [0x06, 0xa1, 0xee, 0x39, 0x03, 0x1d];

    fn create_snapshot(medium: Medium, gateway: [u8; 4], mac: [u8; 6]) -> InterfaceSnapshot {
        InterfaceSnapshot {
            medium,
            names: vec!["en0".to_owned()],
            ipv4: Some((Ipv4Addr::new(gateway[0], gateway[1], gateway[2], 42), 24)),
            gateway: Some(Gateway {
                ipv4: Ipv4Addr::from(gateway),
                mac: MacAddress::from_octets(mac),
            }),
        }
    }

    #[test]
    fn names_the_medium_of_an_ordinary_connection() {
        // Arrange
        let wifi = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);
        let ethernet = create_snapshot(Medium::Ethernet, [192, 168, 1, 1], ROUTER_MAC);

        // Act & Assert
        assert_eq!(
            ConnectionContext::classify(&wifi, &[]).kind,
            ConnectionKind::WiFi
        );
        assert_eq!(
            ConnectionContext::classify(&ethernet, &[]).kind,
            ConnectionKind::Ethernet
        );
    }

    #[test]
    fn recognises_an_iphone_hotspot_from_its_gateway_and_network() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::WiFi, [172, 20, 10, 1], ROUTER_MAC);
        snapshot.ipv4 = Some((Ipv4Addr::new(172, 20, 10, 3), IPHONE_PREFIX));

        // Act
        let context = ConnectionContext::classify(&snapshot, &[]);

        // Assert
        assert_eq!(context.kind, ConnectionKind::PhoneHotspot);
    }

    #[test]
    fn never_calls_a_hotspot_from_the_gateway_address_alone() {
        // Arrange
        let snapshot = create_snapshot(Medium::WiFi, [192, 168, 43, 1], ROUTER_MAC);

        // Act
        let context = ConnectionContext::classify(&snapshot, &[]);

        // Assert
        assert_eq!(context.kind, ConnectionKind::WiFi);
    }

    #[test]
    fn recognises_a_phone_plugged_over_usb() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::Ethernet, [192, 168, 42, 129], ROUTER_MAC);
        snapshot
            .names
            .push("Remote NDIS based Internet Sharing Device".to_owned());

        // Act
        let context = ConnectionContext::classify(&snapshot, &[]);

        // Assert
        assert_eq!(context.kind, ConnectionKind::PhoneHotspot);
    }

    #[test]
    fn counts_a_random_gateway_address_as_one_hint_among_others() {
        // Arrange
        let random_only = create_snapshot(Medium::WiFi, [192, 168, 152, 167], RANDOM_MAC);
        let random_and_known_gateway = create_snapshot(Medium::WiFi, [192, 168, 43, 1], RANDOM_MAC);

        // Act & Assert
        assert_eq!(
            ConnectionContext::classify(&random_only, &[]).kind,
            ConnectionKind::WiFi
        );
        assert_eq!(
            ConnectionContext::classify(&random_and_known_gateway, &[]).kind,
            ConnectionKind::PhoneHotspot
        );
    }

    #[test]
    fn shows_the_physical_link_under_a_vpn() {
        // Arrange
        let tunnel = InterfaceSnapshot {
            medium: Medium::Tunnel,
            names: vec!["utun4".to_owned()],
            ipv4: Some((Ipv4Addr::new(10, 8, 0, 2), 32)),
            gateway: None,
        };
        let wifi = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let context = ConnectionContext::classify(&tunnel, &[wifi]);

        // Assert
        assert_eq!(
            context,
            ConnectionContext {
                kind: ConnectionKind::WiFi,
                is_vpn: true,
                gateway: MacAddress::from_octets(ROUTER_MAC),
            }
        );
    }

    #[test]
    fn keeps_measuring_through_a_vpn_whose_physical_link_is_hidden() {
        // Arrange
        let tunnel = InterfaceSnapshot {
            medium: Medium::Tunnel,
            names: vec!["wg0".to_owned()],
            ipv4: None,
            gateway: None,
        };

        // Act
        let context = ConnectionContext::classify(&tunnel, &[]);

        // Assert
        assert_eq!(context.kind, ConnectionKind::Other);
        assert!(context.is_vpn);
        assert_eq!(context.identify(), None);
    }

    #[test]
    fn identifies_a_network_by_its_gateway_and_kind() {
        // Arrange
        let snapshot = create_snapshot(Medium::Ethernet, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let identity = ConnectionContext::classify(&snapshot, &[]).identify();

        // Assert
        assert_eq!(
            identity,
            MacAddress::from_octets(ROUTER_MAC).map(|gateway| NetworkIdentity {
                gateway,
                kind: ConnectionKind::Ethernet,
            })
        );
    }

    #[test]
    fn treats_an_all_zero_address_as_unresolved() {
        // Act & Assert
        assert_eq!(MacAddress::from_octets([0; 6]), None);
    }

    #[test]
    fn writes_a_mac_address_in_lowercase_hexadecimal() {
        // Arrange
        let mac = MacAddress::from_octets(RANDOM_MAC).expect("a resolved address");

        // Act & Assert
        assert_eq!(mac.to_string(), "06:a1:ee:39:03:1d");
    }
}
