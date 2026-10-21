//! Where the history file lives, how the dashboard reads it, and what the user is told when it
//! cannot be used as is.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use application::{HistoryEvent, HistoryReader, NetworkKey, RepositoryError, ThroughputRecord};
use domain::{Aggregate, Granularity};
use fluent_bundle::FluentArgs;
use infrastructure::{SqliteHistoryRecorder, SqliteMeasurementRepository, SystemClock};
use tauri::{AppHandle, Manager};

use crate::settings_store::SettingsStore;
use crate::text_catalog::TextCatalog;

const HISTORY_FILE: &str = "history.sqlite3";

/// Shown in the dashboard when the history was set aside or cannot be kept on disk.
pub struct HistoryNotice(pub Option<String>);

/// The dashboard's own connection to the history file, beside the writer's.
pub struct DashboardHistory(Mutex<Box<dyn HistoryReader + Send>>);

impl DashboardHistory {
    fn open(path: &Path) -> Self {
        match SqliteMeasurementRepository::open_read_only(path) {
            Ok(reader) => Self(Mutex::new(Box::new(reader))),
            Err(error) => {
                eprintln!("Opening the history for the dashboard failed: {}", error.0);
                Self::create_empty()
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
            let notice = opened.set_aside.map(|path| {
                let mut args = FluentArgs::new();
                let file = path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                args.set("file", file);
                catalog.render("history-set-aside", Some(&args))
            });
            // Opened after the writer: the file exists and is migrated by now.
            OpenedStores {
                recorder: SqliteHistoryRecorder::start(opened.repository, SystemClock::start()),
                notice: HistoryNotice(notice),
                dashboard: DashboardHistory::open(&path),
                settings: SettingsStore::open(&path),
            }
        }
        Err(error) => {
            eprintln!("Opening the history failed, it is kept in memory: {error}");
            let notice = HistoryNotice(Some(catalog.render("history-not-saved", None)));
            match SqliteMeasurementRepository::open_in_memory() {
                Ok(repository) => OpenedStores {
                    recorder: SqliteHistoryRecorder::start(repository, SystemClock::start()),
                    notice,
                    dashboard: DashboardHistory::create_empty(),
                    settings: SettingsStore::create_in_memory(),
                },
                Err(error) => panic!("an in-memory history cannot fail to open: {}", error.0),
            }
        }
    }
}
