//! The Wi-Fi link in use, one adapter per system, read off the async threads.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use application::WifiReader;
use domain::WifiLink;

/// What asking for the location permission did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationRequest {
    /// The system shows its own prompt.
    Prompted,
    /// The answer was already given, or the system never prompts: only its settings can change it.
    NeedsSettings,
    /// No permission guards the network name here.
    NotNeeded,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemWifiReader;

impl WifiReader for SystemWifiReader {
    async fn read_wifi(&self) -> Option<WifiLink> {
        match tokio::task::spawn_blocking(read_wifi_now).await {
            Ok(link) => link,
            Err(error) => {
                eprintln!("Reading the Wi-Fi link failed: {error}");
                None
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn read_wifi_now() -> Option<WifiLink> {
    linux::read_wifi()
}

#[cfg(target_os = "macos")]
fn read_wifi_now() -> Option<WifiLink> {
    macos::read_wifi()
}

#[cfg(target_os = "windows")]
fn read_wifi_now() -> Option<WifiLink> {
    windows::read_wifi()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn read_wifi_now() -> Option<WifiLink> {
    None
}

/// Asks for the permission that reveals the Wi-Fi name. On macOS, call it on the main thread:
/// `CoreLocation` answers through the main run loop.
#[must_use]
pub fn request_location_permission() -> LocationRequest {
    #[cfg(target_os = "macos")]
    {
        macos::request_location_permission()
    }
    #[cfg(target_os = "windows")]
    {
        // Windows prompts by itself at the first blocked call; afterwards only its settings can.
        LocationRequest::NeedsSettings
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        LocationRequest::NotNeeded
    }
}
