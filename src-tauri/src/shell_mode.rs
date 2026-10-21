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

impl ShellMode {
    #[must_use]
    pub fn detect(host: &impl TrayHost) -> Self {
        if host.is_tray_available() {
            Self::Tray
        } else {
            Self::Window
        }
    }
}

/// The discreet note shown in window mode, pointing to the extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrayHint {
    pub message: String,
    pub link_label: String,
    pub dismiss_label: String,
}

impl TrayHint {
    /// `None` in tray mode: there is nothing to explain.
    #[must_use]
    pub fn compose(mode: ShellMode, catalog: &TextCatalog) -> Option<Self> {
        (mode == ShellMode::Window).then(|| Self {
            message: catalog.render("tray-hint-message", None),
            link_label: catalog.render("tray-hint-link", None),
            dismiss_label: catalog.render("tray-hint-dismiss", None),
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
    fn explains_the_missing_dot_only_in_window_mode() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let window_hint = TrayHint::compose(ShellMode::Window, &catalog);
        let tray_hint = TrayHint::compose(ShellMode::Tray, &catalog);

        // Assert
        let window_hint = window_hint.expect("a hint in window mode");
        assert!(window_hint.message.contains("AppIndicator"));
        assert_eq!(tray_hint, None);
    }
}
