//! Whether the desktop shows tray icons. Linux trays follow the `StatusNotifierItem` protocol, whose
//! watcher is missing from GNOME without the `AppIndicator` extension; macOS and Windows always have one.

use application::TrayHost;

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTrayHost;

impl TrayHost for SystemTrayHost {
    #[cfg(target_os = "linux")]
    fn is_tray_available(&self) -> bool {
        // Without a session bus there is no tray either: the window mode keeps the app usable.
        linux::has_status_notifier_watcher().unwrap_or(false)
    }

    #[cfg(not(target_os = "linux"))]
    fn is_tray_available(&self) -> bool {
        true
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::time::Duration;

    use dbus::blocking::Connection;

    const BUS_NAME: &str = "org.freedesktop.DBus";
    const BUS_PATH: &str = "/org/freedesktop/DBus";
    const WATCHER_NAME: &str = "org.kde.StatusNotifierWatcher";
    /// Asked once at startup: a slow bus must not hold the app back.
    const BUS_TIMEOUT: Duration = Duration::from_millis(500);

    /// One round trip to the session bus. O(1).
    pub fn has_status_notifier_watcher() -> Result<bool, dbus::Error> {
        let connection = Connection::new_session()?;
        let bus = connection.with_proxy(BUS_NAME, BUS_PATH, BUS_TIMEOUT);
        let (has_owner,): (bool,) = bus.method_call(BUS_NAME, "NameHasOwner", (WATCHER_NAME,))?;
        Ok(has_owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn always_has_a_tray_outside_linux() {
        // Act & Assert
        assert!(SystemTrayHost.is_tray_available());
    }

    // Depends on the desktop running the test: run on demand with `-- --ignored`.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "reads the session bus of the machine running the test"]
    fn asks_the_session_bus_without_failing() {
        // Act & Assert
        let _ = SystemTrayHost.is_tray_available();
    }
}
