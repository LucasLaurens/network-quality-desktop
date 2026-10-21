//! The history written as three CSV files into a new folder of Downloads, read through a
//! connection of its own so the dashboard and the writer go on meanwhile.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use application::{
    ExportError, ExportPeriod, HistoryStream, export_events, export_measures,
    export_throughput_tests,
};
use domain::{CivilDate, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE};
use infrastructure::SqliteMeasurementRepository;
use serde::Deserialize;
use specta::Type;

const MEASURES_FILE: &str = "mesures.csv";
const EVENTS_FILE: &str = "evenements.csv";
const TESTS_FILE: &str = "tests-de-debit.csv";

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

/// Writes the three files into a new folder of `downloads` named after the local time, and
/// returns it; a failed export leaves no folder behind. Blocking: run it off the async runtime.
/// O(r) time for r rows, O(1) space.
///
/// # Errors
///
/// The history could not be read, or the folder written.
pub fn write_export(
    history: &Path,
    downloads: &Path,
    period: ExportPeriod,
    now_utc: i64,
    offset: i64,
) -> Result<PathBuf, ExportError> {
    let stream = SqliteMeasurementRepository::open_read_only(history)?;
    let folder = downloads.join(format_folder_name(now_utc + offset));
    std::fs::create_dir(&folder)?;
    let written = write_files(&stream, &folder, period, now_utc);
    if written.is_err() {
        let _ = std::fs::remove_dir_all(&folder);
    }
    written.map(|()| folder)
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

/// "qualite-reseau-2026-10-21-140512", to the second: two exports never share a folder. O(1).
fn format_folder_name(local: i64) -> String {
    let date = CivilDate::from_days(local.div_euclid(SECONDS_PER_DAY));
    let second_of_day = local.rem_euclid(SECONDS_PER_DAY);
    format!(
        "qualite-reseau-{:04}-{:02}-{:02}-{:02}{:02}{:02}",
        date.year,
        date.month,
        date.day,
        second_of_day / SECONDS_PER_HOUR,
        second_of_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        second_of_day % SECONDS_PER_MINUTE
    )
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    /// 2023-11-14 22:13:20 UTC.
    const SOME_UTC: i64 = 1_700_000_000;
    const PARIS: i64 = 3600;

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
    fn writes_three_files_into_a_folder_named_after_the_local_time() {
        // Arrange
        let directory = create_temporary_directory("files");
        let history = directory.join("history.sqlite3");
        SqliteMeasurementRepository::open(&history, SOME_UTC).expect("a history");

        // Act
        let folder = write_export(&history, &directory, ExportPeriod::All, SOME_UTC, PARIS)
            .expect("an export");

        // Assert
        assert!(folder.ends_with("qualite-reseau-2023-11-14-231320"));
        for (file, header) in [
            (MEASURES_FILE, "debut_utc,"),
            (EVENTS_FILE, "horodatage_utc,"),
            (TESTS_FILE, "horodatage_utc,"),
        ] {
            let text = std::fs::read_to_string(folder.join(file)).expect("a file");
            assert!(text.starts_with(header), "{file}");
        }
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn leaves_no_folder_when_the_history_cannot_be_read() {
        // Arrange
        let directory = create_temporary_directory("unreadable");

        // Act
        let exported = write_export(
            &directory.join("missing.sqlite3"),
            &directory,
            ExportPeriod::All,
            SOME_UTC,
            PARIS,
        );

        // Assert
        assert!(exported.is_err());
        let entries = std::fs::read_dir(&directory).expect("a folder").count();
        assert_eq!(entries, 0);
        let _ = std::fs::remove_dir_all(directory);
    }
}
