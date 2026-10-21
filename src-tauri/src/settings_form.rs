//! The settings as the Réglages page reads and sends them, and the words of its answers.

use domain::{Appearance, MeasurementServers, Settings, Target};
use fluent_bundle::FluentArgs;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::dashboard_display::format_time_of_day;
use crate::text_catalog::TextCatalog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AppearanceChoice {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SettingsForm {
    pub launch_at_login: bool,
    pub appearance: AppearanceChoice,
    /// The address typed for the user's own server; `None` for the public servers.
    pub own_server: Option<String>,
}

impl SettingsForm {
    /// O(1).
    #[must_use]
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            launch_at_login: settings.launch_at_login,
            appearance: match settings.appearance {
                Appearance::System => AppearanceChoice::System,
                Appearance::Light => AppearanceChoice::Light,
                Appearance::Dark => AppearanceChoice::Dark,
            },
            own_server: match settings.servers {
                MeasurementServers::Default => None,
                MeasurementServers::Custom(target) => Some(target.address().to_string()),
            },
        }
    }

    /// The settings the form describes, or why the server cannot be one. O(length of the address).
    ///
    /// # Errors
    ///
    /// The own server is not an IP address: the message says so in words.
    pub fn to_settings(&self, catalog: &TextCatalog) -> Result<Settings, String> {
        let servers = match self.own_server.as_deref() {
            None => MeasurementServers::Default,
            Some(text) => MeasurementServers::Custom(
                Target::try_parse(text).ok_or_else(|| catalog.render("server-invalid", None))?,
            ),
        };
        Ok(Settings {
            launch_at_login: self.launch_at_login,
            appearance: match self.appearance {
                AppearanceChoice::System => Appearance::System,
                AppearanceChoice::Light => Appearance::Light,
                AppearanceChoice::Dark => Appearance::Dark,
            },
            servers,
        })
    }
}

/// « Enregistré à 14 h 05. » in the user's time zone. O(1).
#[must_use]
pub fn confirm_saved(now: i64, offset: i64, catalog: &TextCatalog) -> String {
    let mut args = FluentArgs::new();
    args.set("time", format_time_of_day(now, offset, catalog));
    catalog.render("settings-saved", Some(&args))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NBSP: char = '\u{a0}';

    fn create_form(own_server: Option<&str>) -> SettingsForm {
        SettingsForm {
            launch_at_login: false,
            appearance: AppearanceChoice::Dark,
            own_server: own_server.map(str::to_owned),
        }
    }

    #[test]
    fn turns_the_form_into_settings_and_back() {
        // Arrange
        let catalog = TextCatalog::load();
        let form = create_form(Some("192.168.1.10"));

        // Act
        let settings = form.to_settings(&catalog).expect("valid settings");

        // Assert
        assert_eq!(
            settings.servers,
            MeasurementServers::Custom(Target::parse("192.168.1.10"))
        );
        assert_eq!(settings.appearance, Appearance::Dark);
        assert_eq!(SettingsForm::from_settings(&settings), form);
    }

    #[test]
    fn says_why_a_server_address_is_refused() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let refused = create_form(Some("box.local")).to_settings(&catalog);

        // Assert
        assert_eq!(refused, Err("Adresse IP invalide.".to_owned()));
    }

    #[test]
    fn confirms_a_save_with_its_time() {
        // Arrange
        let catalog = TextCatalog::load();
        // 2023-11-14 13:05 UTC, one hour ahead in Paris.
        let now = 1_699_967_100;

        // Act
        let confirmation = confirm_saved(now, 3600, &catalog);

        // Assert
        assert_eq!(confirmation, format!("Enregistré à 14{NBSP}h{NBSP}05."));
    }
}
