//! Use cases and the ports (traits) that infrastructure adapters implement.
#![forbid(unsafe_code)]

mod engine;
mod ports;
mod targets;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use engine::{
    CONTEXT_REFRESH_SECONDS, EngineMessage, MeasurementEngine, MetricReading, VerdictView,
    WARM_UP_SAMPLES,
};
pub use ports::{
    Clock, ConnectionContextReader, LatencyProbe, NetworkChangeWatcher, TrayHost, VerdictPresenter,
};
pub use targets::list_default_targets;
