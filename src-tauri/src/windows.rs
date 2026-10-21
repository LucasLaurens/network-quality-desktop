//! The two windows: the dashboard and the popover anchored to the menu-bar dot.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, WebviewWindow, Window, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

pub const MAIN_WINDOW: &str = "main";
pub const POPOVER_WINDOW: &str = "popover";

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

/// The popover closes as soon as the user clicks elsewhere; closing the dashboard only hides it,
/// so the popover can bring it back.
pub fn apply_window_event(window: &Window, event: &WindowEvent) {
    match (window.label(), event) {
        (POPOVER_WINDOW, WindowEvent::Focused(false)) => {
            window.state::<PopoverDismissal>().record(Instant::now());
            report_failure("Hiding the popover", window.hide());
        }
        (MAIN_WINDOW, WindowEvent::CloseRequested { api, .. }) => {
            api.prevent_close();
            report_failure("Hiding the dashboard", window.hide());
        }
        _ => {}
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
