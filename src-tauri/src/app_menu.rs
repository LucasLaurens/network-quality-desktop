//! The macOS application menu. The menu bar stays hidden (no Dock icon), but its shortcuts still
//! work in the dashboard: Tauri's default menu would let ⌘Q quit, while quitting belongs to the
//! dot's menu alone.

use tauri::menu::{Menu, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Wry};

use crate::text_catalog::TextCatalog;

/// Editing and window shortcuts, and no Quit item.
///
/// # Errors
///
/// The system refused to build a menu item.
pub fn build_app_menu(app: &AppHandle<Wry>) -> tauri::Result<Menu<Wry>> {
    let catalog = TextCatalog::load();
    let text = |key: &str| catalog.render(key, None);
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
    Menu::with_items(app, &[&edit, &window])
}
