//! Where the history file lives, how the dashboard reads it, and what the user is told when it
//! cannot be used as is.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use application::{HistoryEvent, HistoryReader, NetworkKey, RepositoryError, ThroughputRecord};
use domain::{Aggregate, Granularity};
use infrastructure::{SqliteHistoryRecorder, SqliteMeasurementRepository, SystemClock};
use tauri::{AppHandle, Manager};

use crate::history_export::HistoryLocation;
use crate::settings_store::SettingsStore;
use crate::text_catalog::TextCatalog;

const HISTORY_FILE: &str = "history.sqlite3";

/// Shown in the dashboard when the history was set aside or cannot be kept on disk.
pub struct HistoryNotice(pub Option<String>);

impl HistoryNotice {
    /// Every sentence given, in order; none when there is nothing to say. O(s) for s sentences.
    fn from_sentences<const N: usize>(sentences: [Option<String>; N]) -> Self {
        let sentences: Vec<String> = sentences.into_iter().flatten().collect();
        Self((!sentences.is_empty()).then(|| sentences.join(" ")))
    }
}

/// The dashboard's own connection to the history file, beside the writer's.
pub struct DashboardHistory(Mutex<Box<dyn HistoryReader + Send>>);

impl DashboardHistory {
    /// The dashboard's reader of `path`, and the sentence that tells the user when it cannot
    /// open: the dashboard then looks empty while the writer keeps recording.
    fn open(path: &Path, catalog: &TextCatalog) -> (Self, Option<String>) {
        match SqliteMeasurementRepository::open_read_only(path) {
            Ok(reader) => (Self(Mutex::new(Box::new(reader))), None),
            Err(error) => {
                eprintln!("Opening the history for the dashboard failed: {}", error.0);
                (
                    Self::create_empty(),
                    Some(catalog.render_text("history-dashboard-unreadable")),
                )
            }
        }
    }

    /// A history kept in memory belongs to the writer alone: the dashboard sees nothing of it.
    fn create_empty() -> Self {
        Self(Mutex::new(Box::new(EmptyHistory)))
    }

    pub fn read<T>(&self, read: impl FnOnce(&dyn HistoryReader) -> T) -> T {
        read(
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_ref(),
        )
    }
}

/// Null object: a history with nothing in it.
struct EmptyHistory;

impl HistoryReader for EmptyHistory {
    fn list_all_aggregates(
        &self,
        _granularity: Granularity,
        _from: i64,
        _to: i64,
    ) -> Result<Vec<(NetworkKey, i64, Aggregate)>, RepositoryError> {
        Ok(Vec::new())
    }

    fn list_events(&self, _from: i64, _to: i64) -> Result<Vec<HistoryEvent>, RepositoryError> {
        Ok(Vec::new())
    }

    fn is_history_empty(&self) -> Result<bool, RepositoryError> {
        Ok(true)
    }

    fn list_throughput_tests(
        &self,
        _from: i64,
        _to: i64,
    ) -> Result<Vec<ThroughputRecord>, RepositoryError> {
        Ok(Vec::new())
    }
}

/// Everything kept in the history file, each with the connection it needs.
pub struct OpenedStores {
    pub recorder: SqliteHistoryRecorder,
    pub notice: HistoryNotice,
    pub dashboard: DashboardHistory,
    pub settings: SettingsStore,
    pub location: HistoryLocation,
}

/// Opens the history in the app's data folder, then the dashboard's and the settings'
/// connections beside it; an unreadable file is set aside and a new history starts. Without a
/// usable disk, the history and the settings live in memory for this session.
pub fn open_stores(app: &AppHandle, catalog: &TextCatalog, now_utc: i64) -> OpenedStores {
    let opened = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())
        .and_then(|directory| {
            std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
            let path = directory.join(HISTORY_FILE);
            SqliteMeasurementRepository::open(&path, now_utc)
                .map(|opened| (opened, path))
                .map_err(|error| error.0)
        });
    match opened {
        Ok((opened, path)) => {
            let set_aside = opened.set_aside.map(|path| {
                let file = path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                catalog.render_value("history-set-aside", "file", file)
            });
            // Opened after the writer: the file exists and is migrated by now.
            let (dashboard, unreadable) = DashboardHistory::open(&path, catalog);
            OpenedStores {
                recorder: SqliteHistoryRecorder::start(opened.repository, SystemClock::start()),
                notice: HistoryNotice::from_sentences([set_aside, unreadable]),
                dashboard,
                settings: SettingsStore::open(&path),
                location: HistoryLocation::OnDisk(path),
            }
        }
        Err(error) => {
            eprintln!("Opening the history failed, it is kept in memory: {error}");
            let notice = HistoryNotice(Some(catalog.render_text("history-not-saved")));
            match SqliteMeasurementRepository::open_in_memory() {
                Ok(repository) => OpenedStores {
                    recorder: SqliteHistoryRecorder::start(repository, SystemClock::start()),
                    notice,
                    dashboard: DashboardHistory::create_empty(),
                    settings: SettingsStore::create_in_memory(),
                    location: HistoryLocation::InMemory,
                },
                Err(error) => panic!("an in-memory history cannot fail to open: {}", error.0),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_so_when_the_dashboard_cannot_open_the_history() {
        // Arrange
        let catalog = TextCatalog::load();
        let missing = std::env::temp_dir()
            .join(format!("no-history-{}", std::process::id()))
            .join(HISTORY_FILE);

        // Act
        let (dashboard, notice) = DashboardHistory::open(&missing, &catalog);

        // Assert
        assert_eq!(
            notice,
            Some(catalog.render_text("history-dashboard-unreadable"))
        );
        assert_eq!(dashboard.read(|reader| reader.is_history_empty()), Ok(true));
    }

    #[test]
    fn joins_every_history_problem_in_one_notice() {
        // Arrange
        let set_aside = Some("Mis de côté.".to_owned());
        let unreadable = Some("Illisible.".to_owned());

        // Act
        let both = HistoryNotice::from_sentences([set_aside.clone(), unreadable]);
        let one = HistoryNotice::from_sentences([set_aside, None]);
        let none = HistoryNotice::from_sentences([None, None]);

        // Assert
        assert_eq!(both.0.as_deref(), Some("Mis de côté. Illisible."));
        assert_eq!(one.0.as_deref(), Some("Mis de côté."));
        assert_eq!(none.0, None);
    }
}
