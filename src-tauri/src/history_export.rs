//! The history written as three CSV files into the folder the user chose, read through a
//! connection of its own so the dashboard and the writer go on meanwhile.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use application::{
    ExportError, ExportPeriod, HistoryStream, export_events, export_measures,
    export_throughput_tests,
};
use infrastructure::SqliteMeasurementRepository;
use serde::{Deserialize, Serialize};
use specta::Type;

const MEASURES_FILE: &str = "mesures.csv";
const EVENTS_FILE: &str = "evenements.csv";
const TESTS_FILE: &str = "tests-debit.csv";
const FILES: [&str; 3] = [MEASURES_FILE, EVENTS_FILE, TESTS_FILE];

/// Where the history is kept: a history in memory has nothing to export.
pub enum HistoryLocation {
    OnDisk(PathBuf),
    InMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ExportPeriodChoice {
    LastDay,
    LastWeek,
    LastMonth,
    All,
}

impl From<ExportPeriodChoice> for ExportPeriod {
    fn from(choice: ExportPeriodChoice) -> Self {
        match choice {
            ExportPeriodChoice::LastDay => Self::LastDay,
            ExportPeriodChoice::LastWeek => Self::LastWeek,
            ExportPeriodChoice::LastMonth => Self::LastMonth,
            ExportPeriodChoice::All => Self::All,
        }
    }
}

/// What a click on « Exporter l'historique… » gives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ExportOutcome {
    /// The folder dialog was closed without a choice.
    Cancelled,
    Done {
        message: String,
    },
}

/// Where the last export went, for « Afficher le dossier »: the front never sends a path.
#[derive(Default)]
pub struct LastExport(Mutex<Option<PathBuf>>);

impl LastExport {
    pub fn remember(&self, folder: PathBuf) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(folder);
    }

    #[must_use]
    pub fn read(&self) -> Option<PathBuf> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Writes the three files into `folder`, replacing files of the same name; a failed export takes
/// its files back. Blocking: run it off the async runtime. O(r) time for r rows, O(1) space.
///
/// # Errors
///
/// The history could not be read, or a file written.
pub fn write_export(
    history: &Path,
    folder: &Path,
    period: ExportPeriod,
    now_utc: i64,
) -> Result<(), ExportError> {
    let stream = SqliteMeasurementRepository::open_read_only(history)?;
    let written = write_files(&stream, folder, period, now_utc);
    if written.is_err() {
        for file in FILES {
            let _ = std::fs::remove_file(folder.join(file));
        }
    }
    written
}

/// O(r) time, O(1) space.
fn write_files(
    stream: &impl HistoryStream,
    folder: &Path,
    period: ExportPeriod,
    now_utc: i64,
) -> Result<(), ExportError> {
    let mut measures = BufWriter::new(File::create(folder.join(MEASURES_FILE))?);
    export_measures(stream, period, now_utc, &mut measures)?;
    measures.flush()?;
    let mut events = BufWriter::new(File::create(folder.join(EVENTS_FILE))?);
    export_events(stream, period, now_utc, &mut events)?;
    events.flush()?;
    let mut tests = BufWriter::new(File::create(folder.join(TESTS_FILE))?);
    export_throughput_tests(stream, period, now_utc, &mut tests)?;
    tests.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    /// 2023-11-14 22:13:20 UTC.
    const SOME_UTC: i64 = 1_700_000_000;

    fn create_temporary_directory(test: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let directory =
            std::env::temp_dir().join(format!("export-{test}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        directory
    }

    #[test]
    fn writes_three_files_into_the_chosen_folder() {
        // Arrange
        let directory = create_temporary_directory("files");
        let history = directory.join("history.sqlite3");
        SqliteMeasurementRepository::open(&history, SOME_UTC).expect("a history");

        // Act
        write_export(&history, &directory, ExportPeriod::All, SOME_UTC).expect("an export");

        // Assert
        for (file, header) in [
            (MEASURES_FILE, "start_utc,"),
            (EVENTS_FILE, "time_utc,"),
            (TESTS_FILE, "time_utc,"),
        ] {
            let text = std::fs::read_to_string(directory.join(file)).expect("a file");
            assert!(text.starts_with(header), "{file}");
        }
        assert_eq!(TESTS_FILE, "tests-debit.csv");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn leaves_no_file_when_the_history_cannot_be_read() {
        // Arrange
        let directory = create_temporary_directory("unreadable");

        // Act
        let exported = write_export(
            &directory.join("missing.sqlite3"),
            &directory,
            ExportPeriod::All,
            SOME_UTC,
        );

        // Assert
        assert!(exported.is_err());
        let entries = std::fs::read_dir(&directory).expect("a folder").count();
        assert_eq!(entries, 0);
        let _ = std::fs::remove_dir_all(directory);
    }
}
