//! `NetworkManager` over the system bus. No permission guards the name; the signal comes as a
//! percentage, turned back into dBm with `NetworkManager`'s own scale.

use std::time::Duration;

use dbus::Path;
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use dbus::blocking::{Connection, Proxy};
use domain::{WifiBand, WifiLink, compute_channel, convert_kilobits_to_megabits};

const NETWORK_MANAGER: &str = "org.freedesktop.NetworkManager";
const NETWORK_MANAGER_PATH: &str = "/org/freedesktop/NetworkManager";
const DEVICE: &str = "org.freedesktop.NetworkManager.Device";
const WIRELESS: &str = "org.freedesktop.NetworkManager.Device.Wireless";
const ACCESS_POINT: &str = "org.freedesktop.NetworkManager.AccessPoint";
/// `NM_DEVICE_TYPE_WIFI` and `NM_DEVICE_STATE_ACTIVATED`.
const DEVICE_TYPE_WIFI: u32 = 2;
const DEVICE_STATE_ACTIVATED: u32 = 100;
const BUS_TIMEOUT: Duration = Duration::from_millis(500);

/// O(d) for d network devices.
pub fn read_wifi() -> Option<WifiLink> {
    let connection = Connection::new_system().ok()?;
    let manager = connection.with_proxy(NETWORK_MANAGER, NETWORK_MANAGER_PATH, BUS_TIMEOUT);
    let (devices,): (Vec<Path<'static>>,) = manager
        .method_call(NETWORK_MANAGER, "GetDevices", ())
        .ok()?;
    let device = devices.into_iter().find(|path| {
        let proxy = connection.with_proxy(NETWORK_MANAGER, path.clone(), BUS_TIMEOUT);
        read_u32(&proxy, DEVICE, "DeviceType") == Some(DEVICE_TYPE_WIFI)
            && read_u32(&proxy, DEVICE, "State") == Some(DEVICE_STATE_ACTIVATED)
    })?;
    let device = connection.with_proxy(NETWORK_MANAGER, device, BUS_TIMEOUT);
    let access_point: Path<'static> = device.get(WIRELESS, "ActiveAccessPoint").ok()?;
    let access_point = connection.with_proxy(NETWORK_MANAGER, access_point, BUS_TIMEOUT);
    let frequency = read_u32(&access_point, ACCESS_POINT, "Frequency");
    let ssid: Option<Vec<u8>> = access_point.get(ACCESS_POINT, "Ssid").ok();
    let strength: Option<u8> = access_point.get(ACCESS_POINT, "Strength").ok();
    Some(WifiLink {
        name: ssid
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .filter(|name| !name.is_empty()),
        signal_dbm: strength.map(convert_strength_to_dbm),
        rate_mbps: read_u32(&device, WIRELESS, "Bitrate").map(convert_kilobits_to_megabits),
        channel: frequency.and_then(compute_channel),
        band: frequency.map_or(WifiBand::Unknown, WifiBand::from_frequency),
    })
}

/// `NetworkManager` maps −100 dBm to 0 % and −40 dBm to 100 %, linearly. O(1).
fn convert_strength_to_dbm(percent: u8) -> i32 {
    i32::from(percent.min(100)) * 3 / 5 - 100
}

fn read_u32(proxy: &Proxy<'_, &Connection>, interface: &str, property: &str) -> Option<u32> {
    proxy.get(interface, property).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_strength_on_the_network_manager_scale() {
        // Act & Assert
        assert_eq!(convert_strength_to_dbm(0), -100);
        assert_eq!(convert_strength_to_dbm(50), -70);
        assert_eq!(convert_strength_to_dbm(100), -40);
    }
}
