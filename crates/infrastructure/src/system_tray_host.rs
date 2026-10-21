//! Whether the desktop shows tray icons. Linux trays follow the `StatusNotifierItem` protocol, whose
//! watcher is missing from GNOME without the `AppIndicator` extension; macOS and Windows always have one.

use application::TrayHost;

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTrayHost;

impl TrayHost for SystemTrayHost {
    #[cfg(target_os = "linux")]
    fn is_tray_available(&self) -> bool {
        // No answer in time, or no session bus: the window mode keeps the app usable either way.
        linux::has_status_notifier_watcher_within_timeout()
    }

    #[cfg(not(target_os = "linux"))]
    fn is_tray_available(&self) -> bool {
        true
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use dbus::blocking::Connection;

    const BUS_NAME: &str = "org.freedesktop.DBus";
    const BUS_PATH: &str = "/org/freedesktop/DBus";
    const WATCHER_NAME: &str = "org.kde.StatusNotifierWatcher";
    /// Asked once at startup: a slow bus must not hold the app back.
    const BUS_TIMEOUT: Duration = Duration::from_millis(500);

    /// Connecting uses libdbus's own timeout (about 25 s): the check runs on a thread so a wedged
    /// bus delays the launch by `BUS_TIMEOUT` at most. O(1).
    pub fn has_status_notifier_watcher_within_timeout() -> bool {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            // The receiver is gone once the timeout passed: nobody needs the answer then.
            let _ = sender.send(has_status_notifier_watcher());
        });
        receiver
            .recv_timeout(BUS_TIMEOUT)
            .ok()
            .and_then(Result::ok)
            .unwrap_or(false)
    }

    fn has_status_notifier_watcher() -> Result<bool, dbus::Error> {
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

    // Depends on the desktop running the test: run on demand with `-- --ignored`. A smoke test:
    // the answer varies with the desktop, only a panic or a hang would fail it.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "reads the session bus of the machine running the test"]
    fn asks_the_session_bus_without_failing() {
        // Act
        let _ = SystemTrayHost.is_tray_available();
    }
}
