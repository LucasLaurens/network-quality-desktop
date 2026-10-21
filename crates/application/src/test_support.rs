//! Concrete in-memory implementations of the ports, used by the engine tests and the app tests.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use std::net::Ipv4Addr;

use domain::{ConnectionContext, GatewayEcho, ProbeOutcome, Target, Timestamp};

use crate::{
    Clock, ConnectionContextReader, GatewayProbe, LatencyProbe, VerdictPresenter, VerdictView,
};

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

/// A gateway the test sets to answer, stay silent or be refused; silent until told otherwise.
#[derive(Debug, Clone)]
pub struct InMemoryGateway {
    echo: Arc<Mutex<GatewayEcho>>,
}

impl Default for InMemoryGateway {
    fn default() -> Self {
        Self {
            echo: Arc::new(Mutex::new(GatewayEcho::Lost)),
        }
    }
}

impl InMemoryGateway {
    pub fn set_echo(&self, echo: GatewayEcho) {
        *self.echo.lock().unwrap_or_else(PoisonError::into_inner) = echo;
    }
}

impl GatewayProbe for InMemoryGateway {
    fn ping_gateway(&self, _gateway: Ipv4Addr) -> impl Future<Output = GatewayEcho> + Send {
        std::future::ready(*self.echo.lock().unwrap_or_else(PoisonError::into_inner))
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
