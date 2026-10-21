use std::future::Future;

use domain::{ProbeOutcome, Target, Timestamp};

use crate::VerdictView;

/// Sends one probe to a target and says whether it answered, was lost, or is unreachable.
pub trait LatencyProbe {
    fn send_probe(&self, target: Target) -> impl Future<Output = ProbeOutcome> + Send;
}

/// Monotonic time source, replaced by a manual clock in tests.
pub trait Clock {
    fn read_now(&self) -> Timestamp;
}

/// Receives every verdict computed by the engine (menu-bar dot, later the popover).
pub trait VerdictPresenter {
    fn present(&self, view: &VerdictView);
}
