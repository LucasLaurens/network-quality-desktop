//! Where the settings are kept: the settings row of the history file, or memory for the session
//! when the history cannot be kept on disk.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use application::{RepositoryError, SettingsRepository};
use domain::Settings;
use infrastructure::SqliteMeasurementRepository;

use crate::settings_form::Durability;

pub struct SettingsStore {
    repository: Mutex<Box<dyn SettingsRepository + Send>>,
    durability: Durability,
}

impl SettingsStore {
    pub fn open(path: &Path) -> Self {
        match SqliteMeasurementRepository::open_alongside(path) {
            Ok(repository) => Self {
                repository: Mutex::new(Box::new(repository)),
                durability: Durability::OnDisk,
            },
            Err(error) => {
                eprintln!(
                    "Opening the settings failed, they last this session: {}",
                    error.0
                );
                Self::create_in_memory()
            }
        }
    }

    pub fn create_in_memory() -> Self {
        Self {
            repository: Mutex::new(Box::new(SessionSettings::default())),
            durability: Durability::ThisSession,
        }
    }

    #[must_use]
    pub const fn read_durability(&self) -> Durability {
        self.durability
    }

    /// The defaults when the settings cannot be read: the app still starts.
    pub fn read(&self) -> Settings {
        self.repository
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .read_settings()
            .unwrap_or_else(|error| {
                eprintln!(
                    "Reading the settings failed, the defaults apply: {}",
                    error.0
                );
                Settings::default()
            })
    }

    /// # Errors
    ///
    /// The storage failed; nothing was written.
    pub fn save(&self, settings: &Settings) -> Result<(), RepositoryError> {
        self.repository
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .save_settings(settings)
    }
}

/// Settings that last until the app quits.
#[derive(Default)]
struct SessionSettings(Settings);

impl SettingsRepository for SessionSettings {
    fn read_settings(&self) -> Result<Settings, RepositoryError> {
        Ok(self.0)
    }

    fn save_settings(&mut self, settings: &Settings) -> Result<(), RepositoryError> {
        self.0 = *settings;
        Ok(())
    }
}
