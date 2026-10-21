use std::sync::mpsc::{self, Sender};
use std::time::{SystemTime, UNIX_EPOCH};

use application::{HistoryCommand, HistoryRecorder, HistoryWriter, MeasurementRepository};

/// Sends history commands to the single thread that owns the database.
#[derive(Debug, Clone)]
pub struct SqliteHistoryRecorder {
    commands: Sender<HistoryCommand>,
}

impl SqliteHistoryRecorder {
    /// Starts the writer thread: it first closes the hours a stop left open, then applies the
    /// commands in order until every recorder is dropped.
    #[must_use]
    pub fn start(repository: impl MeasurementRepository + Send + 'static) -> Self {
        let (commands, inbox) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("history-writer".to_owned())
            .spawn(move || {
                let mut writer = HistoryWriter::new(repository);
                if let Err(error) = writer.finish_pending_hours(read_utc_seconds()) {
                    eprintln!("Closing the open hours failed: {}", error.0);
                }
                for command in inbox {
                    if let Err(error) = writer.apply(command, read_utc_seconds()) {
                        eprintln!("Writing the history failed: {}", error.0);
                    }
                }
            });
        if let Err(error) = spawned {
            eprintln!("Starting the history writer failed: {error}");
        }
        Self { commands }
    }
}

impl HistoryRecorder for SqliteHistoryRecorder {
    fn record(&self, command: HistoryCommand) {
        // Fails only once the writer thread is gone: the app is closing, nothing to keep.
        let _ = self.commands.send(command);
    }
}

fn read_utc_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}
