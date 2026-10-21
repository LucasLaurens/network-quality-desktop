//! Pure measurement rules: verdict, thresholds, hysteresis, connection context. No I/O, no OS
//! access.
#![forbid(unsafe_code)]

mod connection;
mod diagnosis;
mod grade;
mod hysteresis;
mod latency_window;
mod loss;
mod offline;
mod probe_outcome;
mod segment;
mod target;
mod timestamp;
mod verdict;
mod wifi;

pub use connection::{
    ConnectionContext, ConnectionKind, Gateway, InterfaceSnapshot, MacAddress, Medium, Metering,
    NetworkIdentity,
};
pub use diagnosis::{Diagnosis, MeasuringCause, Metric};
pub use grade::{Grade, GradeBounds, Scalable, Thresholds};
pub use hysteresis::Hysteresis;
pub use latency_window::{LatencyWindow, compute_median};
pub use loss::{LossRate, LossWindow};
pub use offline::OfflineDetector;
pub use probe_outcome::ProbeOutcome;
pub use segment::{
    Fault, GatewayEcho, GatewayWindow, LOCAL_LATENCY_LIMIT, LOCAL_LOSS_LIMIT, LocalProblem,
    LocalSegment, Unmeasurable,
};
pub use target::Target;
pub use timestamp::Timestamp;
pub use verdict::Verdict;
pub use wifi::{WEAK_SIGNAL_DBM, WifiBand, WifiLink, compute_channel, convert_quality_to_dbm};
