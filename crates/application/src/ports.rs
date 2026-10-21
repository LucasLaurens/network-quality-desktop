use std::future::Future;
use std::time::Duration;

use domain::{Target, Timestamp};

use crate::VerdictView;

/// Measures one round trip to a target; `None` when it did not answer in time.
pub trait LatencyProbe {
    fn measure_round_trip(&self, target: Target) -> impl Future<Output = Option<Duration>> + Send;
}

/// Monotonic time source, replaced by a manual clock in tests.
pub trait Clock {
    fn read_now(&self) -> Timestamp;
}

/// Receives every verdict computed by the engine (menu-bar dot, later the popover).
pub trait VerdictPresenter {
    fn present(&self, view: &VerdictView);
}
