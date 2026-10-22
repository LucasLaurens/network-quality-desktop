//! The native tray menu: right click on macOS and Windows, the only way in on Linux.

use std::sync::{Mutex, PoisonError};

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Wry};

use domain::Settings;

use crate::display::ConnectionDisplay;
use crate::text_catalog::TextCatalog;
use crate::throughput_display::ProfileChoice;
use crate::windows::{DashboardRequested, request_from_dashboard, show_dashboard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    OpenDashboard,
    RunQuickTest,
    RunFullTest,
    OpenSettings,
    Quit,
}

impl MenuAction {
    const ALL: [Self; 5] = [
        Self::OpenDashboard,
        Self::RunQuickTest,
        Self::RunFullTest,
        Self::OpenSettings,
        Self::Quit,
    ];

    #[must_use]
    pub const fn to_id(self) -> &'static str {
        match self {
            Self::OpenDashboard => "open-dashboard",
            Self::RunQuickTest => "run-quick-test",
            Self::RunFullTest => "run-full-test",
            Self::OpenSettings => "open-settings",
            Self::Quit => "quit",
        }
    }

    /// `None` for an id this menu did not create. O(1): five actions.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.to_id() == id)
    }
}

/// Tests and settings go through the dashboard: it alone can ask for a confirmation, show a test
/// running and keep its result. O(1).
pub fn apply_menu_action(app: &AppHandle, action: MenuAction) {
    match action {
        MenuAction::OpenDashboard => show_dashboard(app),
        MenuAction::RunQuickTest => request_from_dashboard(
            app,
            DashboardRequested::ThroughputTest {
                profile: ProfileChoice::Quick,
            },
        ),
        MenuAction::RunFullTest => request_from_dashboard(
            app,
            DashboardRequested::ThroughputTest {
                profile: ProfileChoice::Full,
            },
        ),
        MenuAction::OpenSettings => request_from_dashboard(app, DashboardRequested::Settings),
        MenuAction::Quit => app.exit(0),
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

/// The single menu definition for the three systems: the tests appear once the settings allow
/// them. O(1).
#[must_use]
pub fn compose_menu_entries(
    display: &ConnectionDisplay,
    settings: &Settings,
    catalog: &TextCatalog,
) -> Vec<MenuEntry> {
    let mut entries = vec![
        MenuEntry::Info(display.state_label.clone()),
        MenuEntry::Info(display.cause.clone()),
        MenuEntry::Separator,
        MenuEntry::Action(
            MenuAction::OpenDashboard,
            display.open_dashboard_label.clone(),
        ),
    ];
    if settings.allows_throughput_tests {
        entries.push(MenuEntry::Action(
            MenuAction::RunQuickTest,
            catalog.render("menu-quick-test", None),
        ));
        entries.push(MenuEntry::Action(
            MenuAction::RunFullTest,
            catalog.render("menu-full-test", None),
        ));
    }
    entries.extend([
        MenuEntry::Action(
            MenuAction::OpenSettings,
            catalog.render("menu-settings", None),
        ),
        MenuEntry::Separator,
        MenuEntry::Action(MenuAction::Quit, catalog.render("menu-quit", None)),
    ]);
    entries
}

/// The built menu, with handles on its read-only lines so they can be rewritten in place. It keeps
/// what it shows: the display and the settings change apart, and each change recomposes the menu
/// under one lock, so a late display can never undo newer settings.
pub struct TrayMenu {
    app: AppHandle,
    menu: Menu<Wry>,
    shown: Mutex<ShownMenu>,
}

struct ShownMenu {
    display: ConnectionDisplay,
    settings: Settings,
    built: BuiltEntries,
}

/// The native items of some entries, and the read-only ones apart to rewrite them later.
struct CreatedItems {
    items: Vec<Box<dyn IsMenuItem<Wry>>>,
    infos: Vec<MenuItem<Wry>>,
}

/// What the menu shows now: its read-only lines, their texts and its actions.
struct BuiltEntries {
    infos: Vec<MenuItem<Wry>>,
    texts: Vec<String>,
    actions: Vec<MenuAction>,
}

impl TrayMenu {
    /// O(e) for e entries.
    pub fn build(
        app: &AppHandle,
        display: &ConnectionDisplay,
        settings: &Settings,
        catalog: &TextCatalog,
    ) -> tauri::Result<Self> {
        let entries = compose_menu_entries(display, settings, catalog);
        let CreatedItems { items, infos } = create_items(app, &entries)?;
        let references: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(AsRef::as_ref).collect();
        Ok(Self {
            app: app.clone(),
            menu: Menu::with_items(app, &references)?,
            shown: Mutex::new(ShownMenu {
                display: display.clone(),
                settings: *settings,
                built: BuiltEntries {
                    infos,
                    texts: read_info_texts(&entries),
                    actions: list_actions(&entries),
                },
            }),
        })
    }

    /// O(e) for e entries.
    pub fn show_display(&self, display: &ConnectionDisplay, catalog: &TextCatalog) {
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        shown.display = display.clone();
        self.refresh(&mut shown, catalog);
    }

    /// O(e) for e entries.
    pub fn show_settings(&self, settings: &Settings, catalog: &TextCatalog) {
        let mut shown = self.shown.lock().unwrap_or_else(PoisonError::into_inner);
        shown.settings = *settings;
        self.refresh(&mut shown, catalog);
    }

    #[must_use]
    pub const fn read_menu(&self) -> &Menu<Wry> {
        &self.menu
    }

    /// Rewrites the read-only lines when their text changed, and rebuilds the menu when its
    /// actions changed (the tests allowed or not). O(e) for e entries.
    fn refresh(&self, shown: &mut ShownMenu, catalog: &TextCatalog) {
        let entries = &compose_menu_entries(&shown.display, &shown.settings, catalog);
        let built = &mut shown.built;
        let actions = list_actions(entries);
        if built.actions != actions {
            match self.replace_items(entries) {
                Ok(infos) => {
                    *built = BuiltEntries {
                        infos,
                        texts: read_info_texts(entries),
                        actions,
                    };
                }
                Err(error) => eprintln!("Rebuilding the tray menu failed: {error}"),
            }
            return;
        }
        let texts = read_info_texts(entries);
        if built.texts == texts {
            return;
        }
        for (item, text) in built.infos.iter().zip(&texts) {
            if let Err(error) = item.set_text(text) {
                eprintln!("Updating the tray menu failed: {error}");
            }
        }
        built.texts = texts;
    }

    /// O(e) for e entries.
    fn replace_items(&self, entries: &[MenuEntry]) -> tauri::Result<Vec<MenuItem<Wry>>> {
        while self.menu.remove_at(0)?.is_some() {}
        let CreatedItems { items, infos } = create_items(&self.app, entries)?;
        let references: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(AsRef::as_ref).collect();
        self.menu.append_items(&references)?;
        Ok(infos)
    }
}

/// O(e) for e entries.
fn create_items(app: &AppHandle, entries: &[MenuEntry]) -> tauri::Result<CreatedItems> {
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
    Ok(CreatedItems { items, infos })
}

/// O(e) for e entries.
fn list_actions(entries: &[MenuEntry]) -> Vec<MenuAction> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Action(action, _) => Some(*action),
            MenuEntry::Info(_) | MenuEntry::Separator => None,
        })
        .collect()
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
    fn lists_state_cause_dashboard_settings_and_quit_while_tests_are_off() {
        // Arrange
        let catalog = TextCatalog::load();
        let display = ConnectionDisplay::create_initial(&catalog);

        // Act
        let entries = compose_menu_entries(&display, &Settings::default(), &catalog);

        // Assert
        assert_eq!(
            entries,
            vec![
                MenuEntry::Info(display.state_label.clone()),
                MenuEntry::Info(display.cause.clone()),
                MenuEntry::Separator,
                MenuEntry::Action(MenuAction::OpenDashboard, display.open_dashboard_label),
                MenuEntry::Action(
                    MenuAction::OpenSettings,
                    catalog.render("menu-settings", None)
                ),
                MenuEntry::Separator,
                MenuEntry::Action(MenuAction::Quit, catalog.render("menu-quit", None)),
            ]
        );
    }

    #[test]
    fn offers_both_tests_once_they_are_allowed() {
        // Arrange
        let catalog = TextCatalog::load();
        let display = ConnectionDisplay::create_initial(&catalog);
        let settings = Settings {
            allows_throughput_tests: true,
            ..Settings::default()
        };

        // Act
        let entries = compose_menu_entries(&display, &settings, &catalog);

        // Assert
        let actions = list_actions(&entries);
        assert_eq!(
            actions,
            [
                MenuAction::OpenDashboard,
                MenuAction::RunQuickTest,
                MenuAction::RunFullTest,
                MenuAction::OpenSettings,
                MenuAction::Quit,
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
