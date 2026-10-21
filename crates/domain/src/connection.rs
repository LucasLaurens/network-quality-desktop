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

/// Whether the system says data costs money on this connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metering {
    /// The system does not say (no API, or no answer).
    Unknown,
    Unmetered,
    /// Billed by volume: a mobile plan, a phone hotspot.
    Metered,
    /// The user asked to save data on this network (macOS Low Data Mode).
    Constrained,
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

impl ConnectionKind {
    /// `Other` without a default route.
    #[must_use]
    pub fn from_context(context: Option<ConnectionContext>) -> Self {
        context.map_or(Self::Other, |context| context.kind)
    }
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
    /// Tells two networks apart when their gateway's MAC address cannot be read.
    pub gateway_address: Option<Ipv4Addr>,
    pub metering: Metering,
    /// What carries the physical link: a phone hotspot may come over Wi-Fi or a USB cable.
    pub medium: Medium,
}

impl ConnectionContext {
    /// `measured` carries the traffic to the measurement targets; `others` are the other
    /// interfaces that are up, where the physical link under a VPN is looked for; `metering` is
    /// what the system says of the connection in use. O(i) time and O(1) space for i interfaces.
    #[must_use]
    pub fn classify(
        measured: &InterfaceSnapshot,
        others: &[InterfaceSnapshot],
        metering: Metering,
    ) -> Self {
        if !measured.is_tunnel_like() {
            return Self::describe_physical(measured, false, metering);
        }
        match (find_physical_link(others), measured.medium) {
            (Some(physical), _) => Self::describe_physical(physical, true, metering),
            (None, Medium::PointToPoint) => Self::describe_physical(measured, false, metering),
            (None, _) => Self {
                kind: ConnectionKind::Other,
                is_vpn: true,
                gateway: None,
                gateway_address: None,
                metering,
                medium: Medium::Tunnel,
            },
        }
    }

    /// Data counts on this connection. A recognised phone hotspot always counts (macOS calls an
    /// Android hotspot free); without an answer from the system, so does a cellular modem.
    #[must_use]
    pub fn is_limited(&self) -> bool {
        match self.metering {
            Metering::Metered | Metering::Constrained => true,
            Metering::Unmetered => self.kind == ConnectionKind::PhoneHotspot,
            Metering::Unknown => matches!(
                self.kind,
                ConnectionKind::Cellular | ConnectionKind::PhoneHotspot
            ),
        }
    }

    /// This reading, with the gateway known from an `earlier` reading of the same network where
    /// this one could not read it: a gateway that later turns out different then still shows. O(1).
    #[must_use]
    pub fn complete_with(&self, earlier: &Self) -> Self {
        Self {
            gateway: self.gateway.or(earlier.gateway),
            gateway_address: self.gateway_address.or(earlier.gateway_address),
            ..*self
        }
    }

    /// The Wi-Fi radio matters only when it carries the traffic: the computer may stay
    /// associated with a Wi-Fi while a cable serves.
    #[must_use]
    pub fn is_carried_by_wifi(&self) -> bool {
        self.medium == Medium::WiFi
    }

    /// Whether `other` is still this network. What is learnt later about the same link (a
    /// gateway MAC resolved, a hotspot hint, the metering) never makes it a new one. O(1).
    #[must_use]
    pub fn is_same_network(&self, other: &Self) -> bool {
        let is_same_kind = self.kind == other.kind
            || [self.kind, other.kind].contains(&ConnectionKind::PhoneHotspot);
        is_same_kind
            && self.is_vpn == other.is_vpn
            && agree(self.gateway, other.gateway)
            && agree(self.gateway_address, other.gateway_address)
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
    fn describe_physical(snapshot: &InterfaceSnapshot, is_vpn: bool, metering: Metering) -> Self {
        let kind = if count_hotspot_signals(snapshot, metering) >= HOTSPOT_SIGNALS_NEEDED {
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
            gateway_address: snapshot.gateway.and_then(|gateway| gateway.ipv4),
            metering,
            medium: snapshot.medium,
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
fn count_hotspot_signals(snapshot: &InterfaceSnapshot, metering: Metering) -> usize {
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
        metering == Metering::Metered,
    ]
    .into_iter()
    .filter(|signal| *signal)
    .count()
}

/// Two readings disagree only when both know the value.
fn agree<T: PartialEq>(first: Option<T>, second: Option<T>) -> bool {
    first
        .zip(second)
        .is_none_or(|(first, second)| first == second)
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
            ConnectionContext::classify(&wifi, &[], Metering::Unknown).kind,
            ConnectionKind::WiFi
        );
        assert_eq!(
            ConnectionContext::classify(&ethernet, &[], Metering::Unknown).kind,
            ConnectionKind::Ethernet
        );
    }

    #[test]
    fn recognises_an_iphone_hotspot_from_its_addressing_and_random_gateway() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::WiFi, [172, 20, 10, 1], RANDOM_MAC);
        snapshot.ipv4 = Some((Ipv4Addr::new(172, 20, 10, 3), IPHONE_PREFIX));

        // Act
        let context = ConnectionContext::classify(&snapshot, &[], Metering::Unknown);

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
            ConnectionContext::classify(&iphone_like, &[], Metering::Unknown).kind,
            ConnectionKind::WiFi
        );
        assert_eq!(
            ConnectionContext::classify(&android_like, &[], Metering::Unknown).kind,
            ConnectionKind::WiFi
        );
    }

    #[test]
    fn recognises_a_phone_plugged_over_usb() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::Ethernet, [192, 168, 42, 129], ROUTER_MAC);
        snapshot.has_phone_name = true;

        // Act
        let context = ConnectionContext::classify(&snapshot, &[], Metering::Unknown);

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
        let context = ConnectionContext::classify(&snapshot, &[], Metering::Unknown);

        // Assert
        assert_eq!(context.gateway, MacAddress::from_octets(ROUTER_MAC));
    }

    #[test]
    fn shows_the_physical_link_under_a_vpn() {
        // Arrange
        let wifi = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let context =
            ConnectionContext::classify(&create_tunnel(Medium::Tunnel), &[wifi], Metering::Unknown);

        // Assert
        assert_eq!(
            context,
            ConnectionContext {
                kind: ConnectionKind::WiFi,
                is_vpn: true,
                gateway: MacAddress::from_octets(ROUTER_MAC),
                gateway_address: Some(Ipv4Addr::new(192, 168, 1, 1)),
                metering: Metering::Unknown,
                medium: Medium::WiFi,
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
        let context = ConnectionContext::classify(
            &create_tunnel(Medium::Tunnel),
            &[unresolved, ethernet],
            Metering::Unknown,
        );

        // Assert
        assert_eq!(context.kind, ConnectionKind::Ethernet);
    }

    #[test]
    fn keeps_measuring_through_a_vpn_whose_physical_link_is_hidden() {
        // Act
        let context =
            ConnectionContext::classify(&create_tunnel(Medium::Tunnel), &[], Metering::Unknown);

        // Assert
        assert_eq!(context.kind, ConnectionKind::Other);
        assert!(context.is_vpn);
        assert_eq!(context.identify(), None);
    }

    #[test]
    fn treats_a_lone_point_to_point_link_as_the_connection_itself() {
        // Act
        let context = ConnectionContext::classify(
            &create_tunnel(Medium::PointToPoint),
            &[],
            Metering::Unknown,
        );

        // Assert
        assert!(!context.is_vpn);
        assert_eq!(context.kind, ConnectionKind::Other);
    }

    #[test]
    fn treats_a_point_to_point_link_over_another_link_as_a_vpn() {
        // Arrange
        let wifi = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let context = ConnectionContext::classify(
            &create_tunnel(Medium::PointToPoint),
            &[wifi],
            Metering::Unknown,
        );

        // Assert
        assert!(context.is_vpn);
        assert_eq!(context.kind, ConnectionKind::WiFi);
    }

    #[test]
    fn identifies_a_network_by_its_gateway_and_kind() {
        // Arrange
        let snapshot = create_snapshot(Medium::Ethernet, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let identity = ConnectionContext::classify(&snapshot, &[], Metering::Unknown).identify();

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
    fn recognises_an_iphone_hotspot_the_system_calls_metered() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::WiFi, [172, 20, 10, 1], ROUTER_MAC);
        snapshot.ipv4 = Some((Ipv4Addr::new(172, 20, 10, 3), IPHONE_PREFIX));

        // Act
        let context = ConnectionContext::classify(&snapshot, &[], Metering::Metered);

        // Assert
        assert_eq!(context.kind, ConnectionKind::PhoneHotspot);
    }

    #[test]
    fn never_calls_a_hotspot_from_metering_alone() {
        // Arrange
        let snapshot = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let context = ConnectionContext::classify(&snapshot, &[], Metering::Metered);

        // Assert
        assert_eq!(context.kind, ConnectionKind::WiFi);
        assert!(context.is_limited());
    }

    #[test]
    fn assumes_a_cellular_link_limited_when_the_system_does_not_say() {
        // Arrange
        let cellular = create_snapshot(Medium::Cellular, [10, 64, 0, 1], ROUTER_MAC);
        let wifi = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let cellular = ConnectionContext::classify(&cellular, &[], Metering::Unknown);
        let wifi = ConnectionContext::classify(&wifi, &[], Metering::Unknown);

        // Assert
        assert!(cellular.is_limited());
        assert!(!wifi.is_limited());
    }

    #[test]
    fn counts_the_low_data_mode_as_limited() {
        // Arrange
        let snapshot = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let context = ConnectionContext::classify(&snapshot, &[], Metering::Constrained);

        // Assert
        assert!(context.is_limited());
    }

    #[test]
    fn tells_networks_apart_by_their_gateway_even_without_its_mac() {
        // Arrange
        let mut home = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);
        let mut office = create_snapshot(Medium::WiFi, [10, 0, 0, 1], ROUTER_MAC);
        for snapshot in [&mut home, &mut office] {
            if let Some(gateway) = snapshot.gateway.as_mut() {
                gateway.mac = None;
            }
        }
        let home_first = ConnectionContext::classify(&home, &[], Metering::Unknown);
        let home_metered = ConnectionContext::classify(&home, &[], Metering::Metered);
        let office = ConnectionContext::classify(&office, &[], Metering::Unknown);

        // Act & Assert
        assert!(home_first.is_same_network(&home_metered));
        assert!(!home_first.is_same_network(&office));
    }

    #[test]
    fn keeps_the_network_when_its_gateway_mac_becomes_readable() {
        // Arrange
        let resolved = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);
        let mut unresolved = resolved;
        if let Some(gateway) = unresolved.gateway.as_mut() {
            gateway.mac = None;
        }
        let before = ConnectionContext::classify(&unresolved, &[], Metering::Unknown);
        let after = ConnectionContext::classify(&resolved, &[], Metering::Unknown);

        // Act & Assert
        assert!(before.is_same_network(&after));
    }

    #[test]
    fn keeps_the_network_when_a_late_hint_reveals_a_hotspot() {
        // Arrange
        let snapshot = create_snapshot(Medium::WiFi, [192, 168, 152, 167], RANDOM_MAC);
        let before = ConnectionContext::classify(&snapshot, &[], Metering::Unknown);
        let after = ConnectionContext::classify(&snapshot, &[], Metering::Metered);

        // Act & Assert
        assert_eq!(after.kind, ConnectionKind::PhoneHotspot);
        assert!(before.is_same_network(&after));
    }

    #[test]
    fn counts_a_recognised_hotspot_as_limited_even_when_the_system_says_otherwise() {
        // Arrange
        let mut snapshot = create_snapshot(Medium::Ethernet, [192, 168, 42, 129], ROUTER_MAC);
        snapshot.has_phone_name = true;

        // Act
        let context = ConnectionContext::classify(&snapshot, &[], Metering::Unmetered);

        // Assert
        assert_eq!(context.kind, ConnectionKind::PhoneHotspot);
        assert!(context.is_limited());
    }

    #[test]
    fn keeps_what_an_earlier_reading_knew_of_the_same_network() {
        // Arrange
        let known = ConnectionContext::classify(
            &create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC),
            &[],
            Metering::Unknown,
        );
        let mut unresolved = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);
        if let Some(gateway) = unresolved.gateway.as_mut() {
            gateway.mac = None;
        }
        let later = ConnectionContext::classify(&unresolved, &[], Metering::Metered);

        // Act
        let merged = later.complete_with(&known);

        // Assert
        assert_eq!(merged.gateway, known.gateway);
        assert_eq!(merged.metering, Metering::Metered);
    }

    #[test]
    fn knows_a_phone_hotspot_over_usb_is_not_carried_by_wifi() {
        // Arrange
        let mut tether = create_snapshot(Medium::Ethernet, [192, 168, 42, 129], ROUTER_MAC);
        tether.has_phone_name = true;
        let wifi = create_snapshot(Medium::WiFi, [192, 168, 1, 1], ROUTER_MAC);

        // Act
        let tether = ConnectionContext::classify(&tether, &[], Metering::Unknown);
        let wifi = ConnectionContext::classify(&wifi, &[], Metering::Unknown);

        // Assert
        assert_eq!(tether.kind, ConnectionKind::PhoneHotspot);
        assert!(!tether.is_carried_by_wifi());
        assert!(wifi.is_carried_by_wifi());
    }
}
