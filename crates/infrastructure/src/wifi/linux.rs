//! `NetworkManager` over the system bus. No permission guards the name; the signal comes as a
//! percentage, turned into dBm with the scale Windows documents.

use std::time::Duration;

use dbus::Path;
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use dbus::blocking::{Connection, Proxy};
use domain::{WifiBand, WifiLink, compute_channel, convert_quality_to_dbm};

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
        signal_dbm: strength.map(convert_quality_to_dbm),
        noise_dbm: None,
        // Kbit/s.
        rate_mbps: read_u32(&device, WIRELESS, "Bitrate").map(|rate| rate / 1000),
        channel: frequency.and_then(compute_channel),
        band: frequency.map_or(WifiBand::Unknown, WifiBand::from_frequency),
    })
}

fn read_u32(proxy: &Proxy<'_, &Connection>, interface: &str, property: &str) -> Option<u32> {
    proxy.get(interface, property).ok()
}
