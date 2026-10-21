//! Reads the connection of the default route with `netdev`, off the async threads: one read
//! costs milliseconds of system calls.

use application::ConnectionContextReader;
use domain::{ConnectionContext, Gateway, InterfaceSnapshot, MacAddress, Medium};
use netdev::Interface;
use netdev::interface::types::InterfaceType;

/// Name prefixes of VPN interfaces whose type the system does not report as a tunnel.
const TUNNEL_NAME_PREFIXES: [&str; 5] = ["utun", "tun", "wg", "ppp", "ipsec"];

#[derive(Debug, Clone, Copy, Default)]
pub struct NetdevContextReader;

impl ConnectionContextReader for NetdevContextReader {
    async fn read_context(&self) -> Option<ConnectionContext> {
        tokio::task::spawn_blocking(read_context_now)
            .await
            .ok()
            .flatten()
    }
}

/// O(i) for i interfaces.
fn read_context_now() -> Option<ConnectionContext> {
    let interfaces = netdev::get_interfaces();
    let default = interfaces.iter().find(|interface| interface.default)?;
    let others: Vec<InterfaceSnapshot> = interfaces
        .iter()
        .filter(|interface| !interface.default && interface.is_up() && !interface.is_loopback())
        .map(to_snapshot)
        .collect();
    Some(ConnectionContext::classify(&to_snapshot(default), &others))
}

fn to_snapshot(interface: &Interface) -> InterfaceSnapshot {
    let names = [
        Some(interface.name.clone()),
        interface.friendly_name.clone(),
        interface.description.clone(),
    ]
    .into_iter()
    .flatten()
    .collect();
    InterfaceSnapshot {
        medium: to_medium(interface),
        names,
        ipv4: interface
            .ipv4
            .first()
            .map(|network| (network.addr(), network.prefix_len())),
        gateway: interface.gateway.as_ref().and_then(|gateway| {
            let ipv4 = *gateway.ipv4.first()?;
            // All zeros when unresolved, e.g. on macOS without the Local Network permission.
            let mac = MacAddress::from_octets(gateway.mac_addr.octets());
            Some(Gateway { ipv4, mac })
        }),
    }
}

fn to_medium(interface: &Interface) -> Medium {
    let is_tunnel_name = TUNNEL_NAME_PREFIXES
        .iter()
        .any(|prefix| interface.name.starts_with(prefix));
    match interface.if_type {
        InterfaceType::Tunnel => Medium::Tunnel,
        _ if interface.is_tun() || is_tunnel_name => Medium::Tunnel,
        InterfaceType::Wireless80211 => Medium::WiFi,
        InterfaceType::Ethernet
        | InterfaceType::FastEthernetT
        | InterfaceType::FastEthernetFx
        | InterfaceType::GigabitEthernet => Medium::Ethernet,
        InterfaceType::Wwan | InterfaceType::Wwanpp | InterfaceType::Wwanpp2 => Medium::Cellular,
        _ => Medium::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_interface(name: &str, if_type: InterfaceType) -> Interface {
        let mut interface = Interface::dummy();
        interface.name = name.to_owned();
        interface.if_type = if_type;
        interface
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
            to_medium(&create_interface("wwan0", InterfaceType::Wwan)),
            Medium::Cellular
        );
        assert_eq!(
            to_medium(&create_interface("wg0", InterfaceType::Unknown)),
            Medium::Tunnel
        );
    }

    // Depends on the machine running the test: run on demand with `-- --ignored`.
    #[tokio::test]
    #[ignore = "reads the network interfaces of the machine running the test"]
    async fn reads_the_connection_of_this_machine() {
        // Act
        let context = NetdevContextReader.read_context().await;

        // Assert
        println!("{context:?}");
    }
}
