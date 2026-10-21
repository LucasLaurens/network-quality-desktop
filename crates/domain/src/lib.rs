//! Pure measurement rules: verdict, thresholds, hysteresis. No I/O, no OS access.
#![forbid(unsafe_code)]

mod grade;
mod hysteresis;
mod latency_window;
mod target;
mod timestamp;
mod verdict;

pub use grade::{Grade, GradeBounds, Thresholds};
pub use hysteresis::Hysteresis;
pub use latency_window::{LatencyWindow, compute_median};
pub use target::Target;
pub use timestamp::Timestamp;
pub use verdict::Verdict;
