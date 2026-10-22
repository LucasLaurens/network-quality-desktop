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

/// Whether the system shows light or dark: the menu bar follows it, whatever the app's own
/// appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorScheme {
    #[default]
    Light,
    Dark,
}

/// Where the latency probes go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeasurementServers {
    /// Cloudflare and Google.
    #[default]
    Default,
    /// The user's own server: nothing else is pinged.
    Custom(Target),
}

impl MeasurementServers {
    /// The user's own server, if there is one. O(1).
    #[must_use]
    pub const fn to_own_target(self) -> Option<Target> {
        match self {
            Self::Default => None,
            Self::Custom(target) => Some(target),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    pub launch_at_login: bool,
    pub appearance: Appearance,
    pub servers: MeasurementServers,
    /// A system notification when the connection gets durably worse.
    pub notifies_on_degradation: bool,
    /// Throughput tests can be started; they never start by themselves.
    pub allows_throughput_tests: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            launch_at_login: true,
            appearance: Appearance::default(),
            servers: MeasurementServers::default(),
            notifies_on_degradation: true,
            allows_throughput_tests: false,
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
        assert!(settings.notifies_on_degradation);
        assert!(!settings.allows_throughput_tests);
    }
}
