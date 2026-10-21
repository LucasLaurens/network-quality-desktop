//! The native tray menu: right click on macOS and Windows, the only way in on Linux.

use std::sync::{Mutex, PoisonError};

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Wry};

use crate::display::ConnectionDisplay;
use crate::text_catalog::TextCatalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    OpenDashboard,
    Quit,
}

impl MenuAction {
    const ALL: [Self; 2] = [Self::OpenDashboard, Self::Quit];

    #[must_use]
    pub const fn to_id(self) -> &'static str {
        match self {
            Self::OpenDashboard => "open-dashboard",
            Self::Quit => "quit",
        }
    }

    /// `None` for an id this menu did not create. O(1): two actions.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.to_id() == id)
    }
}

/// One line of the menu, in display order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    /// Read-only line (state, cause), rewritten when the connection changes.
    Info(String),
    Separator,
    Action(MenuAction, String),
}

/// The single menu definition for the three systems. O(1).
#[must_use]
pub fn compose_menu_entries(display: &ConnectionDisplay, catalog: &TextCatalog) -> Vec<MenuEntry> {
    vec![
        MenuEntry::Info(display.state_label.clone()),
        MenuEntry::Info(display.cause.clone()),
        MenuEntry::Separator,
        MenuEntry::Action(
            MenuAction::OpenDashboard,
            display.open_dashboard_label.clone(),
        ),
        MenuEntry::Separator,
        MenuEntry::Action(MenuAction::Quit, catalog.render("menu-quit", None)),
    ]
}

/// The built menu, with handles on its read-only lines so they can be rewritten in place.
pub struct TrayMenu {
    menu: Menu<Wry>,
    infos: Vec<MenuItem<Wry>>,
    shown: Mutex<Vec<String>>,
}

impl TrayMenu {
    /// O(e) for e entries.
    pub fn build(app: &AppHandle, entries: &[MenuEntry]) -> tauri::Result<Self> {
        let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::with_capacity(entries.len());
        let mut infos = Vec::new();
        for entry in entries {
            match entry {
                MenuEntry::Info(text) => {
                    let item = MenuItem::new(app, text, false, None::<&str>)?;
                    infos.push(item.clone());
                    items.push(Box::new(item));
                }
                MenuEntry::Separator => items.push(Box::new(PredefinedMenuItem::separator(app)?)),
                MenuEntry::Action(action, label) => items.push(Box::new(MenuItem::with_id(
                    app,
                    action.to_id(),
                    label,
                    true,
                    None::<&str>,
                )?)),
            }
        }
        let references: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(AsRef::as_ref).collect();
        Ok(Self {
            menu: Menu::with_items(app, &references)?,
            infos,
            shown: Mutex::new(read_info_texts(entries)),
        })
    }

    #[must_use]
    pub const fn read_menu(&self) -> &Menu<Wry> {
        &self.menu
    }

    /// Rewrites the read-only lines, only when their text changed. O(e) for e entries.
    pub fn update(&self, entries: &[MenuEntry]) {
        let texts = read_info_texts(entries);
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        if *shown == texts {
            return;
        }
        for (item, text) in self.infos.iter().zip(&texts) {
            if let Err(error) = item.set_text(text) {
                eprintln!("Updating the tray menu failed: {error}");
            }
        }
        *shown = texts;
    }
}

/// O(e) for e entries.
fn read_info_texts(entries: &[MenuEntry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Info(text) => Some(text.clone()),
            MenuEntry::Separator | MenuEntry::Action(..) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_state_cause_dashboard_and_quit_in_this_order() {
        // Arrange
        let catalog = TextCatalog::load();
        let display = ConnectionDisplay::create_initial(&catalog);

        // Act
        let entries = compose_menu_entries(&display, &catalog);

        // Assert
        assert_eq!(
            entries,
            vec![
                MenuEntry::Info(display.state_label.clone()),
                MenuEntry::Info(display.cause.clone()),
                MenuEntry::Separator,
                MenuEntry::Action(MenuAction::OpenDashboard, display.open_dashboard_label),
                MenuEntry::Separator,
                MenuEntry::Action(MenuAction::Quit, catalog.render("menu-quit", None)),
            ]
        );
    }

    #[test]
    fn finds_each_action_back_from_its_id() {
        // Act & Assert
        for action in MenuAction::ALL {
            assert_eq!(MenuAction::from_id(action.to_id()), Some(action));
        }
        assert_eq!(MenuAction::from_id("unknown"), None);
    }
}
