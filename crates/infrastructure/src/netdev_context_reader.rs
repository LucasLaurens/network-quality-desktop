//! Reads the connection that carries the measurements with `netdev`, off the async threads: one
//! read costs milliseconds of system calls.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

use application::{ConnectionContextReader, TrafficCounterReader};

use crate::metering;
use domain::{ByteCounters, ConnectionContext, Gateway, InterfaceSnapshot, MacAddress, Medium};
use netdev::Interface;
use netdev::interface::types::InterfaceType;

/// Name prefixes of VPN interfaces on macOS and Linux.
const TUNNEL_NAME_PREFIXES: [&str; 4] = ["utun", "tun", "wg", "ipsec"];
/// Words in the display name or description of VPN adapters, mostly on Windows where the system
/// name is a GUID.
const TUNNEL_NAME_MARKERS: [&str; 6] = [
    "wireguard",
    "wintun",
    "tap-windows",
    "openvpn",
    "tailscale",
    "vpn",
];
/// Words in the names of a phone plugged over USB.
const PHONE_NAME_MARKERS: [&str; 5] = [
    "iphone",
    "android",
    "rndis",
    "remote ndis",
    "apple mobile device",
];
/// Any port: connecting a UDP socket sends nothing, it only asks the route.
const ROUTE_QUERY_PORT: u16 = 443;

#[derive(Debug, Clone, Copy)]
pub struct NetdevContextReader {
    /// A measurement target: the interface that reaches it is the one measured, VPN included.
    route_reference: IpAddr,
}

impl NetdevContextReader {
    #[must_use]
    pub const fn new(route_reference: IpAddr) -> Self {
        Self { route_reference }
    }
}

impl ConnectionContextReader for NetdevContextReader {
    async fn read_context(&self) -> Option<ConnectionContext> {
        let route_reference = self.route_reference;
        match tokio::task::spawn_blocking(move || read_context_now(route_reference)).await {
            Ok(context) => context,
            Err(error) => {
                eprintln!("Reading the connection failed: {error}");
                None
            }
        }
    }
}

/// The counters of the interface measured, the same one as the context's. Blocking: a few system
/// calls. O(i) for i interfaces.
impl TrafficCounterReader for NetdevContextReader {
    fn read_counters(&self) -> Option<ByteCounters> {
        let source = find_source_address(self.route_reference)?;
        let mut interface = netdev::get_interfaces()
            .into_iter()
            .find(|interface| !interface.is_loopback() && owns_address(interface, source))?;
        // Windows leaves the counters out of the listing. A failure leaves `stats` empty: read
        // every second, it would flood the log.
        let _ = interface.update_stats();
        let stats = interface.stats?;
        Some(ByteCounters {
            interface: interface.index,
            received: stats.rx_bytes,
            sent: stats.tx_bytes,
        })
    }
}

/// O(i) for i interfaces.
fn read_context_now(route_reference: IpAddr) -> Option<ConnectionContext> {
    let source = find_source_address(route_reference)?;
    let interfaces = netdev::get_interfaces();
    let measured = interfaces
        .iter()
        .find(|interface| !interface.is_loopback() && owns_address(interface, source))?;
    let others: Vec<InterfaceSnapshot> = interfaces
        .iter()
        .filter(|interface| {
            interface.index != measured.index && interface.is_up() && !interface.is_loopback()
        })
        .map(to_snapshot)
        .collect();
    Some(ConnectionContext::classify(
        &to_snapshot(measured),
        &others,
        metering::read_metering(),
    ))
}

/// The local address the system would use to reach `target`; `None` without a route.
fn find_source_address(target: IpAddr) -> Option<IpAddr> {
    let unspecified: IpAddr = match target {
        IpAddr::V4(_) => Ipv4Addr::UNSPECIFIED.into(),
        IpAddr::V6(_) => Ipv6Addr::UNSPECIFIED.into(),
    };
    let socket = UdpSocket::bind(SocketAddr::new(unspecified, 0)).ok()?;
    socket
        .connect(SocketAddr::new(target, ROUTE_QUERY_PORT))
        .ok()?;
    Some(socket.local_addr().ok()?.ip())
}

/// O(a) for a addresses.
fn owns_address(interface: &Interface, address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => interface.ipv4.iter().any(|network| network.addr() == v4),
        IpAddr::V6(v6) => interface.ipv6.iter().any(|network| network.addr() == v6),
    }
}

/// O(n) for the n words of its names.
fn to_snapshot(interface: &Interface) -> InterfaceSnapshot {
    InterfaceSnapshot {
        medium: to_medium(interface),
        has_phone_name: has_name_marker(interface, &PHONE_NAME_MARKERS),
        ipv4: interface
            .ipv4
            .first()
            .map(|network| (network.addr(), network.prefix_len())),
        gateway: interface.gateway.as_ref().map(|gateway| Gateway {
            ipv4: gateway.ipv4.first().copied(),
            // All zeros when unresolved, e.g. on macOS without the Local Network permission.
            mac: MacAddress::from_octets(gateway.mac_addr.octets()),
        }),
    }
}

/// Cellular modems are point-to-point too: their type is read first.
fn to_medium(interface: &Interface) -> Medium {
    let is_tunnel_name = TUNNEL_NAME_PREFIXES
        .iter()
        .any(|prefix| interface.name.starts_with(prefix))
        || has_name_marker(interface, &TUNNEL_NAME_MARKERS);
    match interface.if_type {
        InterfaceType::Wwan | InterfaceType::Wwanpp | InterfaceType::Wwanpp2 => Medium::Cellular,
        InterfaceType::Tunnel | InterfaceType::ProprietaryVirtual => Medium::Tunnel,
        _ if is_tunnel_name => Medium::Tunnel,
        InterfaceType::Ppp => Medium::PointToPoint,
        _ if interface.is_tun() => Medium::PointToPoint,
        InterfaceType::Wireless80211 => Medium::WiFi,
        InterfaceType::Ethernet
        | InterfaceType::FastEthernetT
        | InterfaceType::FastEthernetFx
        | InterfaceType::GigabitEthernet => Medium::Ethernet,
        _ => Medium::Other,
    }
}

/// Looks in the display name and description, lower-cased. O(n) for n words.
fn has_name_marker(interface: &Interface, markers: &[&str]) -> bool {
    [&interface.friendly_name, &interface.description]
        .into_iter()
        .flatten()
        .map(|name| name.to_lowercase())
        .any(|name| markers.iter().any(|marker| name.contains(marker)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const IFF_UP: u32 = 0x1;
    const IFF_POINTOPOINT: u32 = 0x10;

    fn create_interface(name: &str, if_type: InterfaceType) -> Interface {
        let mut interface = Interface::dummy();
        interface.name = name.to_owned();
        interface.if_type = if_type;
        interface
    }

    // Needs a default route: `cargo test -p infrastructure -- --ignored`.
    #[test]
    #[ignore = "reads the real interface"]
    fn reads_growing_counters_on_the_measured_interface() {
        // Arrange
        let reader = NetdevContextReader::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)));
        let first = reader.read_counters().expect("counters");
        let _ =
            UdpSocket::bind("0.0.0.0:0").and_then(|socket| socket.send_to(&[0; 512], "1.1.1.1:53"));

        // Act
        let second = reader.read_counters().expect("counters");

        // Assert
        assert_eq!(second.interface, first.interface);
        assert!(second.sent >= first.sent);
    }

    #[test]
    fn maps_the_system_interface_types_to_media() {
        // Act & Assert
        assert_eq!(
            to_medium(&create_interface("en0", InterfaceType::Wireless80211)),
            Medium::WiFi
        );
        assert_eq!(
            to_medium(&create_interface("eth0", InterfaceType::GigabitEthernet)),
            Medium::Ethernet
        );
        assert_eq!(
            to_medium(&create_interface("wg0", InterfaceType::Unknown)),
            Medium::Tunnel
        );
    }

    #[test]
    fn keeps_a_point_to_point_cellular_modem_cellular() {
        // Arrange
        let mut modem = create_interface("wwan0", InterfaceType::Wwan);
        modem.flags = IFF_UP | IFF_POINTOPOINT;

        // Act & Assert
        assert_eq!(to_medium(&modem), Medium::Cellular);
    }

    #[test]
    fn leaves_a_ppp_link_undecided_between_vpn_and_physical_link() {
        // Arrange
        let mut pppoe = create_interface("ppp0", InterfaceType::Ppp);
        pppoe.flags = IFF_UP | IFF_POINTOPOINT;

        // Act & Assert
        assert_eq!(to_medium(&pppoe), Medium::PointToPoint);
    }

    #[test]
    fn recognises_a_windows_vpn_by_its_description() {
        // Arrange
        let mut wireguard = create_interface("{8E1F6A0B}", InterfaceType::Ethernet);
        wireguard.description = Some("WireGuard Tunnel".to_owned());

        // Act & Assert
        assert_eq!(to_medium(&wireguard), Medium::Tunnel);
    }

    #[test]
    fn notices_a_phone_plugged_over_usb_by_its_name() {
        // Arrange
        let mut tether = create_interface("{3C2D9E}", InterfaceType::Ethernet);
        tether.description = Some("Remote NDIS based Internet Sharing Device".to_owned());

        // Act
        let snapshot = to_snapshot(&tether);

        // Assert
        assert!(snapshot.has_phone_name);
    }

    // Depends on the machine running the test: run on demand with `-- --ignored`.
    #[tokio::test]
    #[ignore = "reads the network interfaces of the machine running the test"]
    async fn reads_the_connection_of_this_machine() {
        // Act
        let context = NetdevContextReader::new(Ipv4Addr::new(1, 1, 1, 1).into())
            .read_context()
            .await;

        // Assert
        println!("{context:?}");
    }
}
