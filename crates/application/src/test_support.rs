//! Concrete in-memory implementations of the ports, used by the engine tests and the app tests.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use std::net::Ipv4Addr;

use domain::{
    Aggregate, ConnectionContext, GatewayEcho, Granularity, ProbeOutcome, Target, Timestamp,
    WifiLink,
};

use crate::{
    Clock, ConnectionContextReader, GatewayProbe, HistoryCommand, HistoryEvent, HistoryRecorder,
    LatencyProbe, MeasurementRepository, NetworkKey, RepositoryError, VerdictPresenter,
    VerdictView, WifiReader,
};

/// The manual clock's wall-clock start: an arbitrary UTC second, on an hour boundary.
pub const MANUAL_CLOCK_EPOCH: i64 = 1_700_002_800;

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

/// A Wi-Fi link the test sets; none until told otherwise.
#[derive(Debug, Clone, Default)]
pub struct InMemoryWifi {
    link: Arc<Mutex<Option<WifiLink>>>,
}

impl InMemoryWifi {
    pub fn set_link(&self, link: Option<WifiLink>) {
        *self.link.lock().unwrap_or_else(PoisonError::into_inner) = link;
    }
}

impl WifiReader for InMemoryWifi {
    fn read_wifi(&self) -> impl Future<Output = Option<WifiLink>> + Send {
        std::future::ready(
            self.link
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone(),
        )
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

    fn read_utc_seconds(&self) -> i64 {
        let elapsed = self
            .now
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_secs();
        MANUAL_CLOCK_EPOCH + i64::try_from(elapsed).unwrap_or(i64::MAX)
    }
}

/// Keeps every history command, in order; answers flushes at once.
#[derive(Debug, Clone, Default)]
pub struct InMemoryHistoryRecorder {
    commands: Arc<Mutex<Vec<HistoryCommand>>>,
}

impl InMemoryHistoryRecorder {
    #[must_use]
    pub fn list_saved_minutes(&self) -> Vec<(NetworkKey, i64, Aggregate)> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter_map(|command| match command {
                HistoryCommand::SaveMinute {
                    network,
                    start,
                    aggregate,
                } => Some((network.clone(), *start, *aggregate)),
                HistoryCommand::RecordEvent(_) | HistoryCommand::Flush(_) => None,
            })
            .collect()
    }

    #[must_use]
    pub fn list_events(&self) -> Vec<HistoryEvent> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter_map(|command| match command {
                HistoryCommand::RecordEvent(event) => Some(event.clone()),
                HistoryCommand::SaveMinute { .. } | HistoryCommand::Flush(_) => None,
            })
            .collect()
    }
}

impl HistoryRecorder for InMemoryHistoryRecorder {
    fn record(&self, command: HistoryCommand) {
        if let HistoryCommand::Flush(answer) = &command {
            let _ = answer.send(());
        }
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(command);
    }
}

/// Minutes, hours and events in plain vectors. Linear scans: a test keeps a few rows.
#[derive(Debug, Default)]
pub struct InMemoryMeasurementRepository {
    rows: Vec<(NetworkKey, Granularity, i64, Aggregate)>,
    events: Vec<HistoryEvent>,
}

impl InMemoryMeasurementRepository {
    fn find_row(
        &mut self,
        network: &NetworkKey,
        granularity: Granularity,
        start: i64,
    ) -> Option<&mut (NetworkKey, Granularity, i64, Aggregate)> {
        self.rows
            .iter_mut()
            .find(|(key, row_granularity, row_start, _)| {
                is_same_identity(key, network)
                    && *row_granularity == granularity
                    && *row_start == start
            })
    }

    fn remember_name(&mut self, network: &NetworkKey) {
        if network.name.is_none() {
            return;
        }
        for (key, ..) in self
            .rows
            .iter_mut()
            .filter(|(key, ..)| is_same_identity(key, network))
        {
            key.name.clone_from(&network.name);
        }
    }
}

fn is_same_identity(first: &NetworkKey, second: &NetworkKey) -> bool {
    first.gateway == second.gateway && first.kind == second.kind
}

impl MeasurementRepository for InMemoryMeasurementRepository {
    fn add_minute(
        &mut self,
        network: &NetworkKey,
        start: i64,
        aggregate: &Aggregate,
    ) -> Result<(), RepositoryError> {
        if let Some((.., kept)) = self.find_row(network, Granularity::Minute, start) {
            kept.merge(aggregate);
        } else {
            self.rows
                .push((network.clone(), Granularity::Minute, start, *aggregate));
        }
        self.remember_name(network);
        Ok(())
    }

    fn save_hour(
        &mut self,
        network: &NetworkKey,
        start: i64,
        aggregate: &Aggregate,
    ) -> Result<(), RepositoryError> {
        if let Some((.., kept)) = self.find_row(network, Granularity::Hour, start) {
            *kept = *aggregate;
        } else {
            self.rows
                .push((network.clone(), Granularity::Hour, start, *aggregate));
        }
        self.remember_name(network);
        Ok(())
    }

    fn list_aggregates(
        &self,
        network: &NetworkKey,
        granularity: Granularity,
        from: i64,
        to: i64,
    ) -> Result<Vec<(i64, Aggregate)>, RepositoryError> {
        let mut rows: Vec<(i64, Aggregate)> = self
            .rows
            .iter()
            .filter(|(key, row_granularity, start, _)| {
                is_same_identity(key, network)
                    && *row_granularity == granularity
                    && (from..to).contains(start)
            })
            .map(|(_, _, start, aggregate)| (*start, *aggregate))
            .collect();
        rows.sort_by_key(|(start, _)| *start);
        Ok(rows)
    }

    fn list_unmerged_hours(&self, before: i64) -> Result<Vec<(NetworkKey, i64)>, RepositoryError> {
        let mut hours: Vec<(NetworkKey, i64)> = Vec::new();
        for (key, granularity, start, _) in &self.rows {
            let hour = Granularity::Hour.find_start(*start);
            let is_merged = self
                .rows
                .iter()
                .any(|(other, other_granularity, other_start, _)| {
                    is_same_identity(other, key)
                        && *other_granularity == Granularity::Hour
                        && *other_start == hour
                });
            let is_listed = hours
                .iter()
                .any(|(listed, listed_hour)| is_same_identity(listed, key) && *listed_hour == hour);
            if *granularity == Granularity::Minute && *start < before && !is_merged && !is_listed {
                hours.push((key.clone(), hour));
            }
        }
        hours.sort_by_key(|(_, hour)| *hour);
        Ok(hours)
    }

    fn purge_minutes_before(&mut self, utc: i64) -> Result<(), RepositoryError> {
        self.rows.retain(|(_, granularity, start, _)| {
            *granularity != Granularity::Minute || *start >= utc
        });
        Ok(())
    }

    fn record_event(&mut self, event: &HistoryEvent) -> Result<(), RepositoryError> {
        self.events.push(event.clone());
        Ok(())
    }

    fn list_events(&self, from: i64, to: i64) -> Result<Vec<HistoryEvent>, RepositoryError> {
        Ok(self
            .events
            .iter()
            .filter(|event| (from..to).contains(&event.utc))
            .cloned()
            .collect())
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
            .cloned()
    }
}

impl VerdictPresenter for InMemoryVerdictPresenter {
    fn present(&self, view: &VerdictView) {
        self.views
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(view.clone());
    }
}
