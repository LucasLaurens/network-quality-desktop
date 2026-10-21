//! Concrete in-memory implementations of the ports, used by the engine tests and the app tests.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use domain::{Target, Timestamp};

use crate::{Clock, LatencyProbe, VerdictPresenter, VerdictView};

/// A simulated network: each target answers with the round trip set for it, or not at all.
#[derive(Debug, Clone, Default)]
pub struct InMemoryNetwork {
    round_trips: Arc<Mutex<HashMap<Target, Option<Duration>>>>,
}

impl InMemoryNetwork {
    pub fn set_round_trip(&self, target: Target, round_trip: Option<Duration>) {
        self.round_trips
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(target, round_trip);
    }
}

impl LatencyProbe for InMemoryNetwork {
    fn measure_round_trip(&self, target: Target) -> impl Future<Output = Option<Duration>> + Send {
        let round_trip = self
            .round_trips
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&target)
            .copied()
            .flatten();
        std::future::ready(round_trip)
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
