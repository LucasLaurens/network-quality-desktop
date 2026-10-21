//! Sleep and wake from the system: `IOKit` on macOS, the power broadcast callback on Windows,
//! logind on Linux. Each listens on a thread or queue of its own and only hands the event on.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use application::{PowerEvent, PowerEventSource};

type PowerHandler = Box<dyn Fn(PowerEvent) + Send + Sync>;

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemPowerEventSource;

impl PowerEventSource for SystemPowerEventSource {
    fn watch(self, on_event: impl Fn(PowerEvent) + Send + Sync + 'static) {
        let handler: PowerHandler = Box::new(on_event);
        #[cfg(target_os = "macos")]
        macos::watch(handler);
        #[cfg(target_os = "windows")]
        windows::watch(handler);
        #[cfg(target_os = "linux")]
        linux::watch(handler);
        // Elsewhere, the gap between two ticks stands in for the notices.
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        drop(handler);
    }
}
