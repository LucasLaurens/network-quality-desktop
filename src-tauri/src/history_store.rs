//! Where the history file lives, and what the user is told when it cannot be used as is.

use fluent_bundle::FluentArgs;
use infrastructure::{SqliteHistoryRecorder, SqliteMeasurementRepository, SystemClock};
use tauri::{AppHandle, Manager};

use crate::text_catalog::TextCatalog;

const HISTORY_FILE: &str = "history.sqlite3";

/// Shown in the dashboard when the history was set aside or cannot be kept on disk.
pub struct HistoryNotice(pub Option<String>);

/// Opens the history in the app's data folder; an unreadable file is set aside and a new
/// history starts. Without a usable disk, the history lives in memory for this session.
pub fn open_history(
    app: &AppHandle,
    catalog: &TextCatalog,
    now_utc: i64,
) -> (SqliteHistoryRecorder, HistoryNotice) {
    let opened = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())
        .and_then(|directory| {
            std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
            SqliteMeasurementRepository::open(&directory.join(HISTORY_FILE), now_utc)
                .map_err(|error| error.0)
        });
    match opened {
        Ok(opened) => {
            let notice = opened.set_aside.map(|path| {
                let mut args = FluentArgs::new();
                let file = path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                args.set("file", file);
                catalog.render("history-set-aside", Some(&args))
            });
            (
                SqliteHistoryRecorder::start(opened.repository, SystemClock::start()),
                HistoryNotice(notice),
            )
        }
        Err(error) => {
            eprintln!("Opening the history failed, it is kept in memory: {error}");
            let notice = HistoryNotice(Some(catalog.render("history-not-saved", None)));
            match SqliteMeasurementRepository::open_in_memory() {
                Ok(repository) => (
                    SqliteHistoryRecorder::start(repository, SystemClock::start()),
                    notice,
                ),
                Err(error) => panic!("an in-memory history cannot fail to open: {}", error.0),
            }
        }
    }
}
