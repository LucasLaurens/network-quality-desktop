//! Native Wifi (`WLAN` API). Since Windows 11 24H2 the current connection, with the name and the
//! rate, needs the location permission; the signal and the channel do not.

use std::ffi::c_void;
use std::ptr;

use domain::{WifiBand, WifiLink};
use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
use windows::Win32::NetworkManagement::WiFi::{
    WLAN_CONNECTION_ATTRIBUTES, WLAN_INTERFACE_INFO, WLAN_INTERFACE_INFO_LIST, WLAN_INTF_OPCODE,
    WlanCloseHandle, WlanEnumInterfaces, WlanFreeMemory, WlanOpenHandle, WlanQueryInterface,
    wlan_interface_state_connected, wlan_intf_opcode_channel_number,
    wlan_intf_opcode_current_connection, wlan_intf_opcode_rssi,
};

/// Version 2 of the client API: Windows Vista and later.
const WLAN_CLIENT_VERSION: u32 = 2;
/// The 2.4 GHz band ends at channel 14; above, 5 and 6 GHz share channel numbers.
const LAST_2_4_GHZ_CHANNEL: u32 = 14;

/// O(i) for i wireless interfaces.
pub fn read_wifi() -> Option<WifiLink> {
    let client = WlanClient::open()?;
    let interface = client.find_connected_interface()?;
    let guid = interface.InterfaceGuid;
    let signal_dbm = client.query::<i32>(&guid, wlan_intf_opcode_rssi);
    let channel = client.query::<u32>(&guid, wlan_intf_opcode_channel_number);
    let connection =
        client.query::<WLAN_CONNECTION_ATTRIBUTES>(&guid, wlan_intf_opcode_current_connection);
    Some(WifiLink {
        name: connection.as_ref().and_then(|connection| {
            let ssid = &connection.wlanAssociationAttributes.dot11Ssid;
            let length = usize::try_from(ssid.uSSIDLength)
                .ok()?
                .min(ssid.ucSSID.len());
            String::from_utf8(ssid.ucSSID[..length].to_vec())
                .ok()
                .filter(|name| !name.is_empty())
        }),
        signal_dbm,
        noise_dbm: None,
        // Kbit/s in the association attributes.
        rate_mbps: connection
            .map(|connection| connection.wlanAssociationAttributes.ulTxRate / 1000),
        channel,
        band: match channel {
            Some(number) if number <= LAST_2_4_GHZ_CHANNEL => WifiBand::TwoPointFourGhz,
            _ => WifiBand::Unknown,
        },
    })
}

/// An open WLAN client handle, closed when dropped.
struct WlanClient(HANDLE);

impl WlanClient {
    fn open() -> Option<Self> {
        let mut negotiated = 0;
        let mut handle = HANDLE::default();
        // SAFETY: both out-pointers point to live locals.
        let status = unsafe {
            WlanOpenHandle(
                WLAN_CLIENT_VERSION,
                None,
                &raw mut negotiated,
                &raw mut handle,
            )
        };
        (status == ERROR_SUCCESS.0).then_some(Self(handle))
    }

    fn find_connected_interface(&self) -> Option<WLAN_INTERFACE_INFO> {
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = ptr::null_mut();
        // SAFETY: the list is allocated by the API and freed below; entries are copied out first.
        unsafe {
            if WlanEnumInterfaces(self.0, None, &raw mut list) != ERROR_SUCCESS.0 || list.is_null()
            {
                return None;
            }
            let count = usize::try_from((*list).dwNumberOfItems).unwrap_or(0);
            let first = (&raw const (*list).InterfaceInfo).cast::<WLAN_INTERFACE_INFO>();
            let found = (0..count)
                .map(|index| *first.add(index))
                .find(|interface| interface.isState == wlan_interface_state_connected);
            WlanFreeMemory(list.cast::<c_void>());
            found
        }
    }

    /// Copies the answer out and frees the API's buffer; `None` when refused or unavailable.
    fn query<T: Copy>(&self, guid: &windows_core::GUID, opcode: WLAN_INTF_OPCODE) -> Option<T> {
        let mut size = 0_u32;
        let mut data: *mut c_void = ptr::null_mut();
        // SAFETY: the buffer is checked for null and size before it is read, then freed.
        unsafe {
            let status = WlanQueryInterface(
                self.0,
                guid,
                opcode,
                None,
                &raw mut size,
                &raw mut data,
                None,
            );
            if status != ERROR_SUCCESS.0 || data.is_null() {
                return None;
            }
            let value = (usize::try_from(size).ok()? >= size_of::<T>())
                .then(|| data.cast::<T>().read_unaligned());
            WlanFreeMemory(data);
            value
        }
    }
}

impl Drop for WlanClient {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by `WlanOpenHandle` and is closed once.
        unsafe {
            WlanCloseHandle(self.0, None);
        }
    }
}
