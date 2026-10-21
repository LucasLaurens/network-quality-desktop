//! What a setting changes outside the measurement: the look of every window and the login item.

use domain::{Appearance, Settings};
use tauri::{AppHandle, Theme};
use tauri_plugin_autostart::ManagerExt;

/// A development build never registers itself to start at login.
const CAN_REGISTER_AT_LOGIN: bool = !cfg!(debug_assertions);

/// Applies the look at once and lines the login item up with the setting. O(1).
pub fn apply_settings(app: &AppHandle, settings: &Settings) {
    app.set_theme(match settings.appearance {
        Appearance::System => None,
        Appearance::Light => Some(Theme::Light),
        Appearance::Dark => Some(Theme::Dark),
    });
    if !CAN_REGISTER_AT_LOGIN {
        return;
    }
    let login_item = app.autolaunch();
    let lined_up = if settings.launch_at_login {
        login_item.enable()
    } else {
        login_item.disable()
    };
    if let Err(error) = lined_up {
        eprintln!("Updating the login item failed: {error}");
    }
}
