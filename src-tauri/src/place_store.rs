//! The connection that names places, beside the writer's and the dashboard's.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use application::{NetworkKey, PlaceRepository, RepositoryError};
use domain::PlaceName;
use infrastructure::SqliteMeasurementRepository;

pub struct PlaceStore(Mutex<Box<dyn PlaceRepository + Send>>);

impl PlaceStore {
    pub fn open(path: &Path) -> Self {
        match SqliteMeasurementRepository::open_alongside(path) {
            Ok(repository) => Self(Mutex::new(Box::new(repository))),
            Err(error) => {
                eprintln!("Opening the history to name places failed: {}", error.0);
                Self::create_unsaved()
            }
        }
    }

    /// Without a history file, the dashboard lists no network: there is nothing to name.
    pub fn create_unsaved() -> Self {
        Self(Mutex::new(Box::new(UnsavedPlaces)))
    }

    /// # Errors
    ///
    /// The storage failed; nothing was written.
    pub fn save(&self, network: &NetworkKey, place: &PlaceName) -> Result<(), RepositoryError> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .save_place(network, place)
    }
}

/// Null object: refuses every name, there is no file to keep it in.
struct UnsavedPlaces;

impl PlaceRepository for UnsavedPlaces {
    fn save_place(
        &mut self,
        _network: &NetworkKey,
        _place: &PlaceName,
    ) -> Result<(), RepositoryError> {
        Err(RepositoryError(
            "no history file to name places in".to_owned(),
        ))
    }
}
