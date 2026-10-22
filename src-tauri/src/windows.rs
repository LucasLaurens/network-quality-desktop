//! The two windows: the dashboard and the popover anchored to the menu-bar dot.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder, Window, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_specta::Event;

use crate::shell_mode::{CloseAction, ShellMode};
use crate::throughput_display::ProfileChoice;

/// What the tray menu or the popover asks the dashboard to do once shown: it alone can ask for a
/// confirmation, show a test running and keep its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type, Event)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum DashboardRequested {
    Settings,
    ThroughputTest { profile: ProfileChoice },
}

pub const MAIN_WINDOW: &str = "main";
pub const POPOVER_WINDOW: &str = "popover";

/// Linux trays do not report clicks: the menu replaces the popover there.
pub const HAS_POPOVER: bool = !cfg!(target_os = "linux");

/// The tray sits at the top on macOS and at the bottom of the taskbar on Windows.
const POPOVER_POSITION: Position = if cfg!(target_os = "windows") {
    Position::TrayBottomCenter
} else {
    Position::TrayCenter
};

/// A click on the dot first blurs the open popover, then arrives as a click: within this delay
/// it is the same gesture, meant to close the popover, not to reopen it.
const SAME_GESTURE_WINDOW: Duration = Duration::from_millis(300);

/// When the popover last closed because it lost the focus; `None` until it first does.
#[derive(Default)]
pub struct PopoverDismissal(Mutex<Option<Instant>>);

impl PopoverDismissal {
    fn record(&self, at: Instant) {
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(at);
    }

    fn is_same_gesture(&self, now: Instant) -> bool {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some_and(|dismissed_at| now.duration_since(dismissed_at) < SAME_GESTURE_WINDOW)
    }
}

/// Builds the hidden popover from its `tauri.conf.json` entry (declared with `create: false`), only
/// where it can be opened: Linux would keep a webview alive for nothing. O(w) for w windows.
pub fn create_popover(app: &AppHandle) -> tauri::Result<()> {
    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == POPOVER_WINDOW)
    else {
        eprintln!("The popover window is missing from tauri.conf.json");
        return Ok(());
    };
    WebviewWindowBuilder::from_config(app, config)?.build()?;
    Ok(())
}

/// Opens the popover under (or above) the dot, or closes it when it is already open.
pub fn toggle_popover(app: &AppHandle) {
    let Some(popover) = app.get_webview_window(POPOVER_WINDOW) else {
        eprintln!("The popover window is missing");
        return;
    };
    if popover.is_visible().unwrap_or(false) {
        report_failure("Hiding the popover", popover.hide());
        return;
    }
    if app
        .state::<PopoverDismissal>()
        .is_same_gesture(Instant::now())
    {
        return;
    }
    report_failure("Opening the popover", show_next_to_tray(&popover));
}

/// The popover closes as soon as the user clicks elsewhere. Closing the dashboard never quits:
/// in tray mode it hides (the dot brings it back), in window mode it minimises to the taskbar.
pub fn apply_window_event(window: &Window, event: &WindowEvent) {
    match (window.label(), event) {
        (POPOVER_WINDOW, WindowEvent::Focused(false)) => {
            window.state::<PopoverDismissal>().record(Instant::now());
            report_failure("Hiding the popover", window.hide());
        }
        (MAIN_WINDOW, WindowEvent::CloseRequested { api, .. }) => {
            api.prevent_close();
            match window.state::<ShellMode>().choose_close_action() {
                CloseAction::Hide => report_failure("Hiding the dashboard", window.hide()),
                CloseAction::Minimise => {
                    report_failure("Minimising the dashboard", window.minimize());
                }
            }
        }
        _ => {}
    }
}

/// Brings the dashboard forward, from the popover or the tray menu.
pub fn show_dashboard(app: &AppHandle) {
    match app.get_webview_window(MAIN_WINDOW) {
        Some(main) => report_failure(
            "Showing the dashboard",
            main.unminimize()
                .and_then(|()| main.show())
                .and_then(|()| main.set_focus()),
        ),
        None => eprintln!("The dashboard window is missing"),
    }
}

/// Shows the dashboard, then hands it the request. O(1).
pub fn request_from_dashboard(app: &AppHandle, request: DashboardRequested) {
    show_dashboard(app);
    if let Err(error) = request.emit_to(app, MAIN_WINDOW) {
        eprintln!("Asking the dashboard failed: {error}");
    }
}

/// A window that fails to move is not worth crashing the app: the failure is only logged.
pub fn report_failure(action: &str, result: tauri::Result<()>) {
    if let Err(error) = result {
        eprintln!("{action} failed: {error}");
    }
}

fn show_next_to_tray(popover: &WebviewWindow) -> tauri::Result<()> {
    popover.move_window(POPOVER_POSITION)?;
    popover.show()?;
    popover.set_focus()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treats_a_click_right_after_the_dismissal_as_the_same_gesture() {
        // Arrange
        let dismissal = PopoverDismissal::default();
        let dismissed_at = Instant::now();
        dismissal.record(dismissed_at);

        // Act & Assert
        assert!(dismissal.is_same_gesture(dismissed_at + Duration::from_millis(50)));
        assert!(!dismissal.is_same_gesture(dismissed_at + SAME_GESTURE_WINDOW));
    }

    #[test]
    fn opens_on_the_first_click_when_never_dismissed() {
        // Act & Assert
        assert!(!PopoverDismissal::default().is_same_gesture(Instant::now()));
    }
}
