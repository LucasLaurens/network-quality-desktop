//! Use cases and the ports (traits) that infrastructure adapters implement.
#![forbid(unsafe_code)]

mod access;
mod dashboard;
mod engine;
mod export;
mod history;
mod ports;
#[cfg(any(test, feature = "test-support"))]
pub mod repository_contract;
mod settings;
mod targets;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
mod throughput;

pub use access::{LineReading, detect_access};
pub use dashboard::{
    DayOverview, Evolution, NetworkStay, NetworkSummary, Outage, OutageEnd, Period, SUMMARY_DAYS,
    list_network_summaries, read_day_overview, read_evolution,
};
pub use engine::{
    CONTEXT_REFRESH_SECONDS, EngineMessage, EnginePorts, MeasurementEngine, MetricReading,
    VerdictView, WARM_UP_SAMPLES,
};
pub use export::{
    ExportError, ExportPeriod, encode_csv_line, export_events, export_measures,
    export_throughput_tests, format_utc_iso8601,
};
pub use history::{
    EventKind, HistoryCommand, HistoryEvent, HistoryTracker, HistoryWriter, LiveHistory,
    MINUTE_RETENTION, MeasuredSecond, NetworkKey, RepositoryError, ThroughputRecord,
};
pub use ports::{
    AccessTechnologyDetector, Clock, ConnectionContextReader, GatewayProbe, HistoryReader,
    HistoryRecorder, HistoryStream, LatencyProbe, MeasurementRepository, NetworkChangeWatcher,
    Notifier, PowerEvent, PowerEventSource, SettingsRepository, ThroughputProbe,
    TrafficCounterReader, TrayHost, VerdictPresenter, WifiReader,
};
pub use settings::can_measure_against;
pub use targets::{list_default_targets, list_targets};
pub use throughput::{ThroughputFailure, run_throughput_test};
