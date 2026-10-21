//! Typed commands exposed to the front end. Tauri cannot export `pub` commands from the crate root.

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, State};

use crate::display::ConnectionDisplay;
use crate::shell_presenter::DisplayStore;
use crate::windows::{MAIN_WINDOW, POPOVER_WINDOW, report_failure};

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
    match app.get_webview_window(MAIN_WINDOW) {
        Some(main) => report_failure(
            "Showing the dashboard",
            main.show().and_then(|()| main.set_focus()),
        ),
        None => eprintln!("The dashboard window is missing"),
    }
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
