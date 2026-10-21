//! The history in SQLite: one file, one writer thread, every write in a transaction.

mod sqlite_repository;
mod writer_thread;

pub use sqlite_repository::{OpenedHistory, SqliteMeasurementRepository};
pub use writer_thread::SqliteHistoryRecorder;
