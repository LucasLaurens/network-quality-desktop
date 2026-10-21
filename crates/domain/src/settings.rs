//! What the user can set, and what a first launch starts from.

use crate::Target;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    /// Follows the system's light or dark setting.
    #[default]
    System,
    Light,
    Dark,
}

/// Where the latency probes go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeasurementServers {
    /// Two public providers, so one slow or filtering server cannot decide the verdict alone.
    #[default]
    Default,
    /// The user's own server: nothing else is pinged.
    Custom(Target),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub launch_at_login: bool,
    pub appearance: Appearance,
    pub servers: MeasurementServers,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_login: true,
            appearance: Appearance::default(),
            servers: MeasurementServers::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_login_with_the_system_look_and_the_public_servers() {
        // Act
        let settings = Settings::default();

        // Assert
        assert!(settings.launch_at_login);
        assert_eq!(settings.appearance, Appearance::System);
        assert_eq!(settings.servers, MeasurementServers::Default);
    }

    #[test]
    fn reads_a_typed_address_and_refuses_anything_else() {
        // Act & Assert
        assert_eq!(
            Target::try_parse(" 192.168.1.10 "),
            Some(Target::parse("192.168.1.10"))
        );
        assert_eq!(
            Target::try_parse("2606:4700:4700::1111").map(Target::is_ipv6),
            Some(true)
        );
        assert_eq!(Target::try_parse("192.168.1"), None);
        assert_eq!(Target::try_parse("box.local"), None);
        assert_eq!(Target::try_parse(""), None);
    }
}
