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

    /// A launch by the user shows that it worked; a login stays in the tray, unless there is none.
    #[must_use]
    pub fn should_open_dashboard_at_launch(self, origin: LaunchOrigin) -> bool {
        self == Self::Window || origin == LaunchOrigin::User
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

/// Passed by the login item, so the app can tell a login from a launch by the user.
pub const LOGIN_LAUNCH_FLAG: &str = "--from-login";

/// Who started the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchOrigin {
    User,
    Login,
}

impl LaunchOrigin {
    /// O(n) time, O(1) space, on the n command-line arguments.
    #[must_use]
    pub fn detect(args: impl IntoIterator<Item = String>) -> Self {
        if args.into_iter().any(|arg| arg == LOGIN_LAUNCH_FLAG) {
            Self::Login
        } else {
            Self::User
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
            message: catalog.render_text("tray-hint-message"),
            link_label: catalog.render_text("tray-hint-link"),
            dismiss_label: catalog.render_text("tray-hint-dismiss"),
            quit_label: catalog.render_text("menu-quit"),
        })
    }
}

#[cfg(test)]
mod tests {
    use fake::Fake;
    use fake::faker::filesystem::en::FilePath;

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

    #[test]
    fn knows_a_launch_from_the_login_item_by_the_flag_it_passes() {
        // Arrange
        let program: String = FilePath().fake();

        // Act
        let origin = LaunchOrigin::detect([program, LOGIN_LAUNCH_FLAG.to_owned()]);

        // Assert
        assert_eq!(origin, LaunchOrigin::Login);
    }

    #[test]
    fn takes_any_other_launch_for_the_users() {
        // Arrange
        let program: String = FilePath().fake();

        // Act
        let origin = LaunchOrigin::detect([program]);

        // Assert
        assert_eq!(origin, LaunchOrigin::User);
    }

    #[test]
    fn opens_the_dashboard_at_launch_in_tray_mode_only_when_the_user_launched_it() {
        // Act
        let user_launch = ShellMode::Tray.should_open_dashboard_at_launch(LaunchOrigin::User);
        let login_launch = ShellMode::Tray.should_open_dashboard_at_launch(LaunchOrigin::Login);

        // Assert
        assert!(user_launch);
        assert!(!login_launch);
    }

    #[test]
    fn always_opens_the_dashboard_at_launch_in_window_mode() {
        // Act
        let user_launch = ShellMode::Window.should_open_dashboard_at_launch(LaunchOrigin::User);
        let login_launch = ShellMode::Window.should_open_dashboard_at_launch(LaunchOrigin::Login);

        // Assert
        assert!(user_launch);
        assert!(login_launch);
    }
}
