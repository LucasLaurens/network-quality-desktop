//! Typed commands exposed to the front end. Tauri cannot export `pub` commands from the crate root.

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, State};

use crate::display::ConnectionDisplay;
use infrastructure::{LocationRequest, request_location_permission};

use crate::history_store::HistoryNotice;
use crate::shell_mode::{APPINDICATOR_EXTENSION_URL, WindowModeTexts};
use crate::shell_presenter::DisplayStore;
use crate::windows::{POPOVER_WINDOW, report_failure, show_dashboard};

/// Identity of the running build, shown in the window footer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[tauri::command]
#[specta::specta]
#[must_use]
pub fn read_app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

/// The latest display, for a window that opens between two events.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_connection_display(store: State<'_, DisplayStore>) -> ConnectionDisplay {
    store.read()
}

/// Brings the dashboard window forward and closes the popover.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn open_dashboard(app: AppHandle) {
    close_popover(app.clone());
    show_dashboard(&app);
}

/// Closes the popover from the keyboard (Escape).
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn close_popover(app: AppHandle) {
    if let Some(popover) = app.get_webview_window(POPOVER_WINDOW) {
        report_failure("Hiding the popover", popover.hide());
    }
}

/// Where the location permission is given once the system stopped asking.
const LOCATION_SETTINGS_URL: Option<&str> = if cfg!(target_os = "macos") {
    Some("x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices")
} else if cfg!(target_os = "windows") {
    Some("ms-settings:privacy-location")
} else {
    None
};

/// The note shown in window mode; `None` when the tray works.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_window_mode_texts(hint: State<'_, Option<WindowModeTexts>>) -> Option<WindowModeTexts> {
    hint.inner().clone()
}

/// Opens the extension page in the default browser.
#[tauri::command]
#[specta::specta]
pub fn open_tray_extension_page() {
    if let Err(error) = tauri_plugin_opener::open_url(APPINDICATOR_EXTENSION_URL, None::<&str>) {
        eprintln!("Opening the extension page failed: {error}");
    }
}

/// Quits from the dashboard, the only way out in window mode.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Asks for the location permission that reveals the Wi-Fi name, or opens the system settings
/// where it can be given once the question was answered.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn reveal_network_name(app: AppHandle) {
    // CoreLocation answers through the main run loop.
    let asked = app.run_on_main_thread(|| {
        if request_location_permission() == LocationRequest::NeedsSettings
            && let Some(url) = LOCATION_SETTINGS_URL
            && let Err(error) = tauri_plugin_opener::open_url(url, None::<&str>)
        {
            eprintln!("Opening the location settings failed: {error}");
        }
    });
    report_failure("Asking for the location permission", asked);
}

/// Why the history is not what it was, if something happened to it at launch.
#[tauri::command]
#[specta::specta]
#[must_use]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri injects command arguments by value"
)]
pub fn read_history_notice(notice: State<'_, HistoryNotice>) -> Option<String> {
    notice.0.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_app_info_reports_the_package_name_and_version() {
        // Act
        let info = read_app_info();

        // Assert
        assert_eq!(info.name, "app");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }
}
