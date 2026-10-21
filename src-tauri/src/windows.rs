//! The two windows: the dashboard and the popover anchored to the menu-bar dot.

use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_positioner::{Position, WindowExt};

pub const MAIN_WINDOW: &str = "main";
pub const POPOVER_WINDOW: &str = "popover";

/// The tray sits at the top on macOS and at the bottom of the taskbar on Windows.
const POPOVER_POSITION: Position = if cfg!(target_os = "windows") {
    Position::TrayBottomCenter
} else {
    Position::TrayCenter
};

/// Opens the popover under (or above) the dot, or closes it when it is already open.
pub fn toggle_popover(app: &AppHandle) {
    let Some(popover) = app.get_webview_window(POPOVER_WINDOW) else {
        eprintln!("The popover window is missing");
        return;
    };
    if popover.is_visible().unwrap_or(false) {
        let _ = popover.hide();
        return;
    }
    if let Err(error) = show_next_to_tray(&popover) {
        eprintln!("Opening the popover failed: {error}");
    }
}

fn show_next_to_tray(popover: &WebviewWindow) -> tauri::Result<()> {
    popover.move_window(POPOVER_POSITION)?;
    popover.show()?;
    popover.set_focus()
}
