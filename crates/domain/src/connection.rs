//! Which network the computer is on, and through what: the connection context.

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

    /// Phones and some mesh systems use random addresses; routers keep their maker's.
    #[must_use]
    pub const fn is_locally_administered(self) -> bool {
        self.0[0] & 0b10 != 0
    }
}

/// What carries the traffic of an interface, as the system reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Medium {
    Ethernet,
    WiFi,
    /// A cellular modem built into the computer.
    Cellular,
    /// A VPN tunnel, known as such.
    Tunnel,
    /// A point-to-point link: a VPN when another link carries it, else a PPP connection (DSL,
    /// cellular dongle) that is itself the physical link.
    PointToPoint,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gateway {
    /// `None` on an IPv6-only network: the gateway is still there, and so may be its address.
    pub ipv4: Option<Ipv4Addr>,
    pub mac: Option<MacAddress>,
}

/// What the system says about one interface: the input of the classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterfaceSnapshot {
    pub medium: Medium,
    /// The system names it after a phone plugged over USB (read by the OS adapter).
    pub has_phone_name: bool,
    /// First IPv4 address and its prefix length.
    pub ipv4: Option<(Ipv4Addr, u8)>,
    pub gateway: Option<Gateway>,
}

impl InterfaceSnapshot {
    const fn is_tunnel_like(&self) -> bool {
        matches!(self.medium, Medium::Tunnel | Medium::PointToPoint)
    }

    const fn has_resolved_gateway(&self) -> bool {
        matches!(self.gateway, Some(Gateway { mac: Some(_), .. }))
    }
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
    /// `measured` carries the traffic to the measurement targets; `others` are the other
    /// interfaces that are up, where the physical link under a VPN is looked for. O(i) time and
    /// O(1) space for i interfaces.
    #[must_use]
    pub fn classify(measured: &InterfaceSnapshot, others: &[InterfaceSnapshot]) -> Self {
        if !measured.is_tunnel_like() {
            return Self::describe_physical(measured, false);
        }
        match (find_physical_link(others), measured.medium) {
            (Some(physical), _) => Self::describe_physical(physical, true),
            (None, Medium::PointToPoint) => Self::describe_physical(measured, false),
            (None, _) => Self {
                kind: ConnectionKind::Other,
                is_vpn: true,
                gateway: None,
            },
        }
    }

    /// `None` while the gateway address is unknown: such a network cannot be recognised later.
    #[must_use]
    pub fn identify(&self) -> Option<NetworkIdentity> {
        self.gateway.map(|gateway| NetworkIdentity {
            gateway,
            kind: self.kind,
        })
    }

    /// O(1).
    fn describe_physical(snapshot: &InterfaceSnapshot, is_vpn: bool) -> Self {
        let kind = if count_hotspot_signals(snapshot) >= HOTSPOT_SIGNALS_NEEDED {
            ConnectionKind::PhoneHotspot
        } else {
            match snapshot.medium {
                Medium::Ethernet => ConnectionKind::Ethernet,
                Medium::WiFi => ConnectionKind::WiFi,
                Medium::Cellular => ConnectionKind::Cellular,
                Medium::Tunnel | Medium::PointToPoint | Medium::Other => ConnectionKind::Other,
            }
        };
        Self {
            kind,
            is_vpn,
            gateway: snapshot.gateway.and_then(|gateway| gateway.mac),
        }
    }
}

/// The link under a VPN: a non-tunnel interface with a gateway, preferring one whose gateway
/// address is resolved. shortcut: with several such links (Wi-Fi and Ethernet), the first one
/// listed wins; asking the route to the VPN server would be exact. O(i) for i interfaces.
fn find_physical_link(others: &[InterfaceSnapshot]) -> Option<&InterfaceSnapshot> {
    others
        .iter()
        .filter(|snapshot| !snapshot.is_tunnel_like() && snapshot.gateway.is_some())
        .min_by_key(|snapshot| !snapshot.has_resolved_gateway())
}

/// Independent hints that a phone shares its connection. The addressing plan counts once: a
/// router may be configured like a phone. O(1).
fn count_hotspot_signals(snapshot: &InterfaceSnapshot) -> usize {
    let has_phone_gateway = snapshot
        .gateway
        .and_then(|gateway| gateway.ipv4)
        .is_some_and(|address| PHONE_GATEWAYS.contains(&address));
    let is_on_iphone_network = snapshot.ipv4.is_some_and(|(address, prefix)| {
        prefix == IPHONE_PREFIX && mask(address, IPHONE_PREFIX) == IPHONE_NETWORK
    });
    let has_random_gateway = snapshot
        .gateway
        .and_then(|gateway| gateway.mac)
        .is_some_and(MacAddress::is_locally_administered);
    [
        has_phone_gateway || is_on_iphone_network,
        snapshot.has_phone_name,
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
            has_phone_name: false,
            ipv4: Some((Ipv4Addr::new(gateway[0], gateway[1], gateway[2], 42), 24)),
            gateway: Some(Gateway {
                ipv4: Some(Ipv4Addr::from(gateway)),
                mac: MacAddress::from_octets(mac),
            }),
        }
    }

    fn create_tunnel(medium: Medium) -> InterfaceSnapshot {
        InterfaceSnapshot {
            medium,
            has_phone_name: false,
            ipv4: Some((Ipv4Addr::new(10, 8, 0, 2), 32)),
            gateway: None,
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
    fn recognises_an_iphone_hotspot_from_its_addressing_and_random_gateway() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::WiFi, [172, 20, 10, 1], RANDOM_MAC);
        snapshot.ipv4 = Some((Ipv4Addr::new(172, 20, 10, 3), IPHONE_PREFIX));

        // Act
        let context = ConnectionContext::classify(&snapshot, &[]);

        // Assert
        assert_eq!(context.kind, ConnectionKind::PhoneHotspot);
    }

    #[test]
    fn never_calls_a_hotspot_from_the_addressing_alone() {
        // Arrange
        let mut iphone_like = create_snapshot(Medium::WiFi, [172, 20, 10, 1], ROUTER_MAC);
        iphone_like.ipv4 = Some((Ipv4Addr::new(172, 20, 10, 3), IPHONE_PREFIX));
        let android_like = create_snapshot(Medium::WiFi, [192, 168, 43, 1], ROUTER_MAC);

        // Act & Assert
        assert_eq!(
            ConnectionContext::classify(&iphone_like, &[]).kind,
            ConnectionKind::WiFi
        );
        assert_eq!(
            ConnectionContext::classify(&android_like, &[]).kind,
            ConnectionKind::WiFi
        );
    }

    #[test]
    fn recognises_a_phone_plugged_over_usb() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::Ethernet, [192, 168, 42, 129], ROUTER_MAC);
        snapshot.has_phone_name = true;

        // Act
        let context = ConnectionContext::classify(&snapshot, &[]);

        // Assert
        assert_eq!(context.kind, ConnectionKind::PhoneHotspot);
    }

    #[test]
    fn keeps_the_gateway_of_an_ipv6_only_network() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);
        snapshot.ipv4 = None;
        snapshot.gateway = Some(Gateway {
            ipv4: None,
            mac: MacAddress::from_octets(ROUTER_MAC),
        });

        // Act
        let context = ConnectionContext::classify(&snapshot, &[]);

        // Assert
        assert_eq!(context.gateway, MacAddress::from_octets(ROUTER_MAC));
    }

    #[test]
    fn shows_the_physical_link_under_a_vpn() {
        // Arrange
        let wifi = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let context = ConnectionContext::classify(&create_tunnel(Medium::Tunnel), &[wifi]);

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
    fn prefers_a_link_whose_gateway_is_resolved_under_a_vpn() {
        // Arrange
        let mut unresolved = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);
        unresolved.gateway = Some(Gateway {
            ipv4: Some(Ipv4Addr::new(192, 168, 1, 1)),
            mac: None,
        });
        let ethernet = create_snapshot(Medium::Ethernet, [10, 0, 0, 1], ROUTER_MAC);

        // Act
        let context =
            ConnectionContext::classify(&create_tunnel(Medium::Tunnel), &[unresolved, ethernet]);

        // Assert
        assert_eq!(context.kind, ConnectionKind::Ethernet);
    }

    #[test]
    fn keeps_measuring_through_a_vpn_whose_physical_link_is_hidden() {
        // Act
        let context = ConnectionContext::classify(&create_tunnel(Medium::Tunnel), &[]);

        // Assert
        assert_eq!(context.kind, ConnectionKind::Other);
        assert!(context.is_vpn);
        assert_eq!(context.identify(), None);
    }

    #[test]
    fn treats_a_lone_point_to_point_link_as_the_connection_itself() {
        // Act
        let context = ConnectionContext::classify(&create_tunnel(Medium::PointToPoint), &[]);

        // Assert
        assert!(!context.is_vpn);
        assert_eq!(context.kind, ConnectionKind::Other);
    }

    #[test]
    fn treats_a_point_to_point_link_over_another_link_as_a_vpn() {
        // Arrange
        let wifi = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let context = ConnectionContext::classify(&create_tunnel(Medium::PointToPoint), &[wifi]);

        // Assert
        assert!(context.is_vpn);
        assert_eq!(context.kind, ConnectionKind::WiFi);
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
}
