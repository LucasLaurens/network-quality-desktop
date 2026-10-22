//! The system's light or dark setting, which the menu bar follows even when the app forces its
//! own appearance. Read once a second by the tray: each system answers from memory.

use application::ColorSchemeReader;
use domain::ColorScheme;

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemColorSchemeReader;

impl ColorSchemeReader for SystemColorSchemeReader {
    fn read_color_scheme(&self) -> ColorScheme {
        platform::read_color_scheme()
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use domain::ColorScheme;
    use objc2_foundation::{NSString, NSUserDefaults};

    /// The global `AppleInterfaceStyle` default, "Dark" or absent; `NSApp`'s appearance would
    /// follow the app's own setting instead. A bar tinted by a dark wallpaper in light mode is not
    /// seen. O(1).
    pub fn read_color_scheme() -> ColorScheme {
        let style = NSUserDefaults::standardUserDefaults()
            .stringForKey(&NSString::from_str("AppleInterfaceStyle"));
        if style.is_some_and(|style| style.to_string() == "Dark") {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use domain::ColorScheme;
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::w;

    const DWORD_SIZE: u32 = 4;

    /// `SystemUsesLightTheme` is the taskbar's setting; `AppsUseLightTheme` is the windows'.
    /// Absent before Windows 10 1903: light then. O(1).
    pub fn read_color_scheme() -> ColorScheme {
        let mut value: u32 = 1;
        let mut size = DWORD_SIZE;
        // SAFETY: `value` and `size` outlive the call; the flags ask for a 4-byte DWORD only.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
                w!("SystemUsesLightTheme"),
                RRF_RT_REG_DWORD,
                None,
                Some((&raw mut value).cast()),
                Some(&raw mut size),
            )
        };
        if status.is_ok() && value == 0 {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use dbus::arg::Variant;
    use dbus::blocking::Connection;
    use domain::ColorScheme;

    const PORTAL: &str = "org.freedesktop.portal.Desktop";
    const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
    const SETTINGS: &str = "org.freedesktop.portal.Settings";
    const CALL_TIMEOUT: Duration = Duration::from_millis(500);
    /// A D-Bus round trip each second would be wasted: the setting rarely changes.
    const READ_EVERY: Duration = Duration::from_secs(5);
    /// `color-scheme` of `org.freedesktop.appearance`: 1 asks for dark.
    const PREFER_DARK: u32 = 1;

    static LAST_READ: Mutex<Option<(Instant, ColorScheme)>> = Mutex::new(None);

    /// The desktop portal's `color-scheme`; light without a portal. GNOME's top bar stays dark in
    /// both schemes: the light colours were checked legible on it. O(1).
    pub fn read_color_scheme() -> ColorScheme {
        let mut last = LAST_READ
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((at, scheme)) = *last
            && at.elapsed() < READ_EVERY
        {
            return scheme;
        }
        let scheme = query_portal().unwrap_or_default();
        *last = Some((Instant::now(), scheme));
        scheme
    }

    /// `ReadOne` from portal version 2 on; older portals (Ubuntu 22.04, Debian 12) only have
    /// `Read`, which wraps the value in one more variant.
    fn query_portal() -> Result<ColorScheme, dbus::Error> {
        let connection = Connection::new_session()?;
        let proxy = connection.with_proxy(PORTAL, PORTAL_PATH, CALL_TIMEOUT);
        let key = ("org.freedesktop.appearance", "color-scheme");
        let read_one: Result<(Variant<u32>,), dbus::Error> =
            proxy.method_call(SETTINGS, "ReadOne", key);
        let value = match read_one {
            Ok((value,)) => value.0,
            Err(_) => {
                let (value,): (Variant<Variant<u32>>,) =
                    proxy.method_call(SETTINGS, "Read", key)?;
                value.0.0
            }
        };
        Ok(if value == PREFER_DARK {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reads the real setting, once in each scheme:
    // `EXPECTED_SCHEME=dark cargo test -p infrastructure -- --ignored reads_the_scheme`.
    #[test]
    #[ignore = "reads the scheme of the system it runs on"]
    fn reads_the_scheme_of_the_system() {
        // Arrange
        let expected = match std::env::var("EXPECTED_SCHEME").as_deref() {
            Ok("dark") => ColorScheme::Dark,
            _ => ColorScheme::Light,
        };

        // Act
        let scheme = SystemColorSchemeReader.read_color_scheme();

        // Assert
        assert_eq!(scheme, expected);
    }
}
