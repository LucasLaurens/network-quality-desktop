//! What a setting changes outside the measurement: the look of every window and the login item.

use std::sync::Arc;

use domain::{Appearance, Settings};
use tauri::{AppHandle, Manager, Theme};
use tauri_plugin_autostart::ManagerExt;

use crate::shell_presenter::DisplayStore;
use crate::text_catalog::TextCatalog;
use crate::tray_menu::{TrayMenu, compose_menu_entries};

/// A development build never registers itself to start at login.
const CAN_REGISTER_AT_LOGIN: bool = !cfg!(debug_assertions);

/// Applies the look at once, shows or hides the tests in the tray menu, and lines the login item
/// up with the setting. O(1).
pub fn apply_settings(app: &AppHandle, settings: &Settings) {
    app.set_theme(match settings.appearance {
        Appearance::System => None,
        Appearance::Light => Some(Theme::Light),
        Appearance::Dark => Some(Theme::Dark),
    });
    // Absent at launch: the menu is then built with these settings.
    if let Some(menu) = app.try_state::<Arc<TrayMenu>>() {
        menu.update(&compose_menu_entries(
            &app.state::<DisplayStore>().read(),
            settings.allows_throughput_tests,
            &app.state::<TextCatalog>(),
        ));
    }
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
