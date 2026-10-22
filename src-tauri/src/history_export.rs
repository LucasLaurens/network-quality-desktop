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
/// Added while a file is written; renamed away once all three are complete.
const PARTIAL_SUFFIX: &str = ".partial";

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

/// A finished export: what to say, and how to name the button that shows its folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportDone {
    pub message: String,
    /// « Afficher dans le Finder » on macOS, the file manager's name elsewhere.
    pub reveal_label: String,
}

/// Whether « Exporter l'historique… » can be used, and why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ExportAvailability {
    Ready,
    Unavailable { reason: String },
}

/// The folder chosen for the exports, kept in the shell: the front never sends a path.
#[derive(Default)]
pub struct ExportFolder(Mutex<Option<PathBuf>>);

impl ExportFolder {
    pub fn remember(&self, folder: PathBuf) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(folder);
    }

    /// `None` until a folder was chosen.
    #[must_use]
    pub fn read(&self) -> Option<PathBuf> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Writes the three files into `folder` under temporary names, then renames them over any files
/// of the same name: a failed export removes only its own temporary files and leaves an earlier
/// export whole. Blocking: run it off the async runtime. O(r) time for r rows, O(1) space.
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
    let partials = FILES.map(|file| folder.join(format!("{file}{PARTIAL_SUFFIX}")));
    let written = write_files(&stream, &partials, period, now_utc).and_then(|()| {
        for (partial, file) in partials.iter().zip(FILES) {
            std::fs::rename(partial, folder.join(file))?;
        }
        Ok(())
    });
    if written.is_err() {
        for partial in &partials {
            if partial.is_file() {
                let _ = std::fs::remove_file(partial);
            }
        }
    }
    written
}

/// O(r) time, O(1) space.
fn write_files(
    stream: &impl HistoryStream,
    [measures, events, tests]: &[PathBuf; 3],
    period: ExportPeriod,
    now_utc: i64,
) -> Result<(), ExportError> {
    let mut writer = BufWriter::new(File::create(measures)?);
    export_measures(stream, period, now_utc, &mut writer)?;
    writer.flush()?;
    let mut writer = BufWriter::new(File::create(events)?);
    export_events(stream, period, now_utc, &mut writer)?;
    writer.flush()?;
    let mut writer = BufWriter::new(File::create(tests)?);
    export_throughput_tests(stream, period, now_utc, &mut writer)?;
    writer.flush()?;
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
            ("mesures.csv", "start_utc,"),
            ("evenements.csv", "time_utc,"),
            ("tests-debit.csv", "time_utc,"),
        ] {
            let text = std::fs::read_to_string(directory.join(file)).expect("a file");
            assert!(text.starts_with(header), "{file}");
        }
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn keeps_an_earlier_export_when_this_one_fails_halfway() {
        // Arrange
        let directory = create_temporary_directory("halfway");
        let history = directory.join("history.sqlite3");
        SqliteMeasurementRepository::open(&history, SOME_UTC).expect("a history");
        let earlier = "earlier export";
        for file in FILES {
            std::fs::write(directory.join(file), earlier).expect("an earlier file");
        }
        // The last file cannot be written: a folder stands where it goes.
        std::fs::create_dir(directory.join(format!("{TESTS_FILE}{PARTIAL_SUFFIX}")))
            .expect("a blocking folder");

        // Act
        let exported = write_export(&history, &directory, ExportPeriod::All, SOME_UTC);

        // Assert
        assert!(exported.is_err());
        for file in FILES {
            let text = std::fs::read_to_string(directory.join(file)).expect("a file");
            assert_eq!(text, earlier, "{file}");
        }
        assert!(
            !directory
                .join(format!("{MEASURES_FILE}{PARTIAL_SUFFIX}"))
                .exists()
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}
