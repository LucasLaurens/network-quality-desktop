//! Pure measurement rules: verdict, thresholds, hysteresis, connection context. No I/O, no OS
//! access.
#![forbid(unsafe_code)]

mod access;
mod calendar;
mod connection;
mod degradation;
mod diagnosis;
mod grade;
mod history;
mod hysteresis;
mod latency_window;
mod loss;
mod offline;
mod probe_outcome;
mod segment;
mod settings;
mod target;
mod throughput;
mod timestamp;
mod traffic;
mod verdict;
mod wifi;

pub use access::{ACCESS_MIN_TESTS, ACCESS_TESTS, AccessEstimate, BoxLine, WanAccessType};
pub use calendar::{CivilDate, CivilDateTime};
pub use connection::{
    ConnectionContext, ConnectionKind, Gateway, InterfaceSnapshot, MacAddress, Medium, Metering,
    NetworkIdentity,
};
pub use degradation::DegradationWatch;
pub use diagnosis::{Diagnosis, MeasuringCause, Metric};
pub use grade::{Grade, GradeBounds, Scalable, Thresholds};
pub use history::{
    Aggregate, CountedState, Granularity, LATENCY_BUCKETS, LatencyHistogram, MinuteAccumulator,
    RING_SECONDS, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE, SecondPoint, SecondRing,
    SecondSample, StateSeconds,
};
pub use hysteresis::{DEGRADE_AFTER, Hysteresis};
pub use latency_window::{LatencyWindow, compute_median};
pub use loss::{LossRate, LossWindow};
pub use offline::{OFFLINE_AFTER_SECONDS, OfflineDetector};
pub use probe_outcome::ProbeOutcome;
pub use segment::{
    Fault, GatewayEcho, GatewayWindow, LOCAL_LATENCY_LIMIT, LOCAL_LOSS_LIMIT, LocalProblem,
    LocalSegment, Unmeasurable,
};
pub use settings::{Appearance, MeasurementServers, Settings};
pub use target::Target;
pub use throughput::{
    Bitrate, Direction, ENOUGH_TRANSFER_TIME, Step, THROUGHPUT_CAP_DURATION, TestProfile,
    ThroughputResult, Transfer,
};
pub use timestamp::Timestamp;
pub use traffic::{ByteCounters, TRAFFIC_WINDOW, TrafficMeter, TrafficRate, TrafficReading};
pub use verdict::Verdict;
pub use wifi::{
    WEAK_SIGNAL_DBM, WifiBand, WifiLink, compute_channel, convert_kilobits_to_megabits,
};
