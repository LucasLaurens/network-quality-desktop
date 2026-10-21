//! Use cases and the ports (traits) that infrastructure adapters implement.
#![forbid(unsafe_code)]

mod engine;
mod ports;
mod targets;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use engine::{EngineMessage, MeasurementEngine, MetricReading, VerdictView};
pub use ports::{Clock, LatencyProbe, VerdictPresenter};
pub use targets::list_default_targets;
