//! Use cases and the ports (traits) that infrastructure adapters implement.
#![forbid(unsafe_code)]

mod engine;
mod history;
mod ports;
#[cfg(any(test, feature = "test-support"))]
pub mod repository_contract;
mod targets;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use engine::{
    CONTEXT_REFRESH_SECONDS, EngineMessage, EnginePorts, MeasurementEngine, MetricReading,
    VerdictView, WARM_UP_SAMPLES,
};
pub use history::{
    EventKind, HistoryCommand, HistoryEvent, HistoryTracker, HistoryWriter, MINUTE_RETENTION,
    MeasuredSecond, NetworkKey, RepositoryError,
};
pub use ports::{
    Clock, ConnectionContextReader, GatewayProbe, HistoryRecorder, LatencyProbe,
    MeasurementRepository, NetworkChangeWatcher, TrayHost, VerdictPresenter, WifiReader,
};
pub use targets::list_default_targets;
