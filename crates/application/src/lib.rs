//! Use cases and the ports (traits) that infrastructure adapters implement.
#![forbid(unsafe_code)]

mod engine;
mod ports;
mod targets;
pub mod test_support;

pub use engine::{EngineMessage, MeasurementEngine, VerdictView};
pub use ports::{Clock, LatencyProbe, VerdictPresenter};
pub use targets::list_default_targets;
