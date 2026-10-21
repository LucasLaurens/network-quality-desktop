//! Whether the system says data costs money on the connection in use, one adapter per system.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use domain::Metering;

/// Blocking: called from the context reader's own thread. O(1).
#[cfg(target_os = "linux")]
pub fn read_metering() -> Metering {
    linux::read_metering()
}

#[cfg(target_os = "macos")]
pub fn read_metering() -> Metering {
    macos::read_metering()
}

#[cfg(target_os = "windows")]
pub fn read_metering() -> Metering {
    windows::read_metering()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn read_metering() -> Metering {
    Metering::Unknown
}
