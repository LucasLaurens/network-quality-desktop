//! Concrete in-memory implementations of the ports, used by the engine tests and the app tests.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use domain::{ProbeOutcome, Target, Timestamp};

use crate::{Clock, LatencyProbe, VerdictPresenter, VerdictView};

/// A simulated network: each target gives the outcome set for it; an unknown target is unreachable.
#[derive(Debug, Clone, Default)]
pub struct InMemoryNetwork {
    outcomes: Arc<Mutex<HashMap<Target, ProbeOutcome>>>,
}

impl InMemoryNetwork {
    pub fn set_outcome(&self, target: Target, outcome: ProbeOutcome) {
        self.outcomes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(target, outcome);
    }
}

impl LatencyProbe for InMemoryNetwork {
    fn send_probe(&self, target: Target) -> impl Future<Output = ProbeOutcome> + Send {
        let outcome = self
            .outcomes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&target)
            .copied()
            .unwrap_or(ProbeOutcome::Unreachable);
        std::future::ready(outcome)
    }
}

/// A clock that only moves when the test says so.
#[derive(Debug, Clone, Default)]
pub struct ManualClock {
    now: Arc<Mutex<Duration>>,
}

impl ManualClock {
    pub fn advance(&self, by: Duration) {
        *self.now.lock().unwrap_or_else(PoisonError::into_inner) += by;
    }
}

impl Clock for ManualClock {
    fn read_now(&self) -> Timestamp {
        Timestamp::from_elapsed(*self.now.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// Keeps every view it was given, in order.
#[derive(Debug, Clone, Default)]
pub struct InMemoryVerdictPresenter {
    views: Arc<Mutex<Vec<VerdictView>>>,
}

impl InMemoryVerdictPresenter {
    #[must_use]
    pub fn read_last(&self) -> Option<VerdictView> {
        self.views
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .last()
            .copied()
    }
}

impl VerdictPresenter for InMemoryVerdictPresenter {
    fn present(&self, view: &VerdictView) {
        self.views
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(*view);
    }
}
