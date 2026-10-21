//! Use cases and the ports (traits) that infrastructure adapters implement.
#![forbid(unsafe_code)]

mod dashboard;
mod engine;
mod history;
mod ports;
#[cfg(any(test, feature = "test-support"))]
pub mod repository_contract;
mod targets;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use dashboard::{
    DayOverview, Evolution, NetworkSummary, Outage, OutageEnd, Period, SUMMARY_DAYS,
    list_network_summaries, read_day_overview, read_evolution,
};
pub use engine::{
    CONTEXT_REFRESH_SECONDS, EngineMessage, EnginePorts, MeasurementEngine, MetricReading,
    VerdictView, WARM_UP_SAMPLES,
};
pub use history::{
    EventKind, HistoryCommand, HistoryEvent, HistoryTracker, HistoryWriter, LiveHistory,
    MINUTE_RETENTION, MeasuredSecond, NetworkKey, RepositoryError,
};
pub use ports::{
    Clock, ConnectionContextReader, GatewayProbe, HistoryReader, HistoryRecorder, LatencyProbe,
    MeasurementRepository, NetworkChangeWatcher, TrayHost, VerdictPresenter, WifiReader,
};
pub use targets::list_default_targets;
