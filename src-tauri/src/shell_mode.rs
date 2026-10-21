//! Tray mode (dot, popover, menu) or window mode, for a desktop that cannot show a tray icon.

use application::TrayHost;
use serde::Serialize;
use specta::Type;

use crate::text_catalog::TextCatalog;

/// Page of the GNOME extension that brings tray icons back.
pub const APPINDICATOR_EXTENSION_URL: &str =
    "https://extensions.gnome.org/extension/615/appindicator-support/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellMode {
    Tray,
    /// The dashboard opens at launch, stays in the taskbar and minimises instead of closing.
    Window,
}

/// What closing the dashboard does: it never quits, since the app keeps measuring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// The dot brings it back.
    Hide,
    /// It stays in the taskbar, the only way back without a tray.
    Minimise,
}

impl ShellMode {
    #[must_use]
    pub const fn choose_close_action(self) -> CloseAction {
        match self {
            Self::Tray => CloseAction::Hide,
            Self::Window => CloseAction::Minimise,
        }
    }

    #[must_use]
    pub fn detect(host: &impl TrayHost) -> Self {
        if host.is_tray_available() {
            Self::Tray
        } else {
            Self::Window
        }
    }
}

/// What window mode adds to the dashboard: a discreet note pointing to the extension, and a way
/// to quit, since there is no tray menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowModeTexts {
    pub message: String,
    pub link_label: String,
    pub dismiss_label: String,
    pub quit_label: String,
}

impl WindowModeTexts {
    /// `None` in tray mode: there is nothing to explain.
    #[must_use]
    pub fn compose(mode: ShellMode, catalog: &TextCatalog) -> Option<Self> {
        (mode == ShellMode::Window).then(|| Self {
            message: catalog.render("tray-hint-message", None),
            link_label: catalog.render("tray-hint-link", None),
            dismiss_label: catalog.render("tray-hint-dismiss", None),
            quit_label: catalog.render("menu-quit", None),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InMemoryTrayHost {
        is_available: bool,
    }

    impl TrayHost for InMemoryTrayHost {
        fn is_tray_available(&self) -> bool {
            self.is_available
        }
    }

    #[test]
    fn uses_the_tray_when_the_desktop_has_one() {
        // Act & Assert
        assert_eq!(
            ShellMode::detect(&InMemoryTrayHost { is_available: true }),
            ShellMode::Tray
        );
    }

    #[test]
    fn falls_back_to_a_window_without_a_tray() {
        // Act & Assert
        assert_eq!(
            ShellMode::detect(&InMemoryTrayHost {
                is_available: false
            }),
            ShellMode::Window
        );
    }

    #[test]
    fn hides_the_dashboard_in_tray_mode_and_minimises_it_in_window_mode() {
        // Act & Assert
        assert_eq!(ShellMode::Tray.choose_close_action(), CloseAction::Hide);
        assert_eq!(
            ShellMode::Window.choose_close_action(),
            CloseAction::Minimise
        );
    }

    #[test]
    fn explains_the_missing_dot_only_in_window_mode() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let window_hint = WindowModeTexts::compose(ShellMode::Window, &catalog);
        let tray_hint = WindowModeTexts::compose(ShellMode::Tray, &catalog);

        // Assert
        let window_hint = window_hint.expect("a hint in window mode");
        assert!(window_hint.message.contains("AppIndicator"));
        assert_eq!(tray_hint, None);
    }
}
