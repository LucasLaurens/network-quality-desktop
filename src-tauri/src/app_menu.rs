//! The macOS application menu. The menu bar stays hidden (no Dock icon), but its shortcuts still
//! work while a window of the app is in front: ⌘Q quits the whole app, as the dot's menu does.

use tauri::menu::{Menu, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Wry};

use crate::text_catalog::TextCatalog;

/// Quit, editing and window shortcuts.
///
/// # Errors
///
/// The system refused to build a menu item.
pub fn build_app_menu(app: &AppHandle<Wry>, catalog: &TextCatalog) -> tauri::Result<Menu<Wry>> {
    let text = |key: &str| catalog.render_text(key);
    // The first submenu is the application menu: macOS titles it with the app's name.
    let application = Submenu::with_items(
        app,
        text("menu-application"),
        true,
        &[&PredefinedMenuItem::quit(app, Some(&text("menu-quit")))?],
    )?;
    let edit = Submenu::with_items(
        app,
        text("menu-edit"),
        true,
        &[
            &PredefinedMenuItem::undo(app, Some(&text("menu-undo")))?,
            &PredefinedMenuItem::redo(app, Some(&text("menu-redo")))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some(&text("menu-cut")))?,
            &PredefinedMenuItem::copy(app, Some(&text("menu-copy")))?,
            &PredefinedMenuItem::paste(app, Some(&text("menu-paste")))?,
            &PredefinedMenuItem::select_all(app, Some(&text("menu-select-all")))?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        text("menu-window"),
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some(&text("menu-minimize")))?,
            &PredefinedMenuItem::close_window(app, Some(&text("menu-close-window")))?,
        ],
    )?;
    Menu::with_items(app, &[&application, &edit, &window])
}
