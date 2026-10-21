//! Concrete in-memory implementations of the ports, used by the engine tests and the app tests.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use domain::{ConnectionContext, ProbeOutcome, Target, Timestamp};

use crate::{Clock, ConnectionContextReader, LatencyProbe, VerdictPresenter, VerdictView};

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

/// A connection the test switches at will; no default route until told otherwise.
#[derive(Debug, Clone, Default)]
pub struct InMemoryConnection {
    context: Arc<Mutex<Option<ConnectionContext>>>,
}

impl InMemoryConnection {
    pub fn set_context(&self, context: Option<ConnectionContext>) {
        *self.context.lock().unwrap_or_else(PoisonError::into_inner) = context;
    }
}

impl ConnectionContextReader for InMemoryConnection {
    fn read_context(&self) -> impl Future<Output = Option<ConnectionContext>> + Send {
        std::future::ready(*self.context.lock().unwrap_or_else(PoisonError::into_inner))
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
