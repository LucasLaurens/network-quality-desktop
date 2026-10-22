//! Concrete in-memory implementations of the ports, used by the engine tests and the app tests.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use std::net::Ipv4Addr;

use fake::faker::internet::en::IPv4;
use fake::faker::lorem::en::Word;
use fake::{Fake, Faker};

use domain::{
    Aggregate, BoxLine, ConnectionContext, Direction, GatewayEcho, Granularity, ProbeOutcome,
    Settings, Target, Timestamp, Transfer, WifiLink,
};

use crate::{
    AccessTechnologyDetector, Clock, ConnectionContextReader, GatewayProbe, HistoryCommand,
    HistoryEvent, HistoryReader, HistoryRecorder, HistoryStream, LatencyProbe,
    MeasurementRepository, NetworkChangeWatcher, NetworkKey, Notifier, PowerEvent,
    PowerEventSource, RepositoryError, SettingsRepository, ThroughputFailure, ThroughputProbe,
    ThroughputRecord, VerdictPresenter, VerdictView, WifiReader,
};

/// The manual clock's wall-clock start: an arbitrary UTC second, on an hour boundary.
pub const MANUAL_CLOCK_EPOCH: i64 = 1_700_002_800;

/// A simulated network: each target gives the outcome set for it; an unknown target is unreachable.
#[derive(Debug, Clone, Default)]
pub struct InMemoryNetwork {
    outcomes: Arc<Mutex<HashMap<Target, ProbeOutcome>>>,
    forgotten_fallbacks: Arc<AtomicU32>,
}

impl InMemoryNetwork {
    pub fn set_outcome(&self, target: Target, outcome: ProbeOutcome) {
        self.outcomes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(target, outcome);
    }

    /// How many times the engine asked to try ICMP again.
    #[must_use]
    pub fn count_forgotten_fallbacks(&self) -> u32 {
        self.forgotten_fallbacks.load(Ordering::SeqCst)
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

    fn forget_fallbacks(&self) {
        self.forgotten_fallbacks.fetch_add(1, Ordering::SeqCst);
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

    /// Moves the wall clock back, as a time server can. The monotonic time cannot go back, so
    /// both are moved by the same amount: tests only rely on the wall clock here.
    pub fn set_back(&self, by: Duration) {
        let mut now = self.now.lock().unwrap_or_else(PoisonError::into_inner);
        *now = now.saturating_sub(by);
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
                HistoryCommand::RecordEvent(_)
                | HistoryCommand::RecordThroughput(_)
                | HistoryCommand::EraseAll(_)
                | HistoryCommand::Flush(_) => None,
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
                HistoryCommand::SaveMinute { .. }
                | HistoryCommand::RecordThroughput(_)
                | HistoryCommand::EraseAll(_)
                | HistoryCommand::Flush(_) => None,
            })
            .collect()
    }

    #[must_use]
    pub fn has_erase(&self) -> bool {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .any(|command| matches!(command, HistoryCommand::EraseAll(_)))
    }

    #[must_use]
    pub fn list_throughput_tests(&self) -> Vec<ThroughputRecord> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter_map(|command| match command {
                HistoryCommand::RecordThroughput(record) => Some(record.clone()),
                HistoryCommand::SaveMinute { .. }
                | HistoryCommand::RecordEvent(_)
                | HistoryCommand::EraseAll(_)
                | HistoryCommand::Flush(_) => None,
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

/// Minutes, hours and events in plain vectors. Quadratic scans at worst: a test keeps few rows.
#[derive(Debug, Default)]
pub struct InMemoryMeasurementRepository {
    rows: Vec<(NetworkKey, Granularity, i64, Aggregate)>,
    events: Vec<HistoryEvent>,
    throughput_tests: Vec<ThroughputRecord>,
    is_erase_failing: bool,
}

impl InMemoryMeasurementRepository {
    /// Every erase fails from now on, as a full disk would make it.
    pub const fn fail_erases(&mut self) {
        self.is_erase_failing = true;
    }

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
        for event in self
            .events
            .iter_mut()
            .filter(|event| is_same_identity(&event.network, network))
        {
            event.network.name.clone_from(&network.name);
        }
        for record in self
            .throughput_tests
            .iter_mut()
            .filter(|record| is_same_identity(&record.network, network))
        {
            record.network.name.clone_from(&network.name);
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
        let kept_hours: Vec<(NetworkKey, i64)> = self
            .rows
            .iter()
            .filter(|(_, granularity, ..)| *granularity == Granularity::Hour)
            .map(|(key, _, start, _)| (key.clone(), *start))
            .collect();
        self.rows.retain(|(key, granularity, start, _)| {
            let is_hour_kept = kept_hours.iter().any(|(hour_key, hour)| {
                is_same_identity(hour_key, key) && *hour == Granularity::Hour.find_start(*start)
            });
            *granularity != Granularity::Minute || *start >= utc || !is_hour_kept
        });
        Ok(())
    }

    fn record_event(&mut self, event: &HistoryEvent) -> Result<(), RepositoryError> {
        self.events.push(event.clone());
        self.remember_name(&event.network);
        Ok(())
    }

    fn record_throughput_test(&mut self, record: &ThroughputRecord) -> Result<(), RepositoryError> {
        self.throughput_tests.push(record.clone());
        self.remember_name(&record.network);
        Ok(())
    }

    fn erase_history(&mut self) -> Result<(), RepositoryError> {
        if self.is_erase_failing {
            return Err(RepositoryError("the erase failed".to_owned()));
        }
        *self = Self::default();
        Ok(())
    }
}

/// Oldest first, each with the network's current name, as the database answers.
impl HistoryReader for InMemoryMeasurementRepository {
    fn list_all_aggregates(
        &self,
        granularity: Granularity,
        from: i64,
        to: i64,
    ) -> Result<Vec<(NetworkKey, i64, Aggregate)>, RepositoryError> {
        let mut rows: Vec<(NetworkKey, i64, Aggregate)> = self
            .rows
            .iter()
            .filter(|(_, row_granularity, start, _)| {
                *row_granularity == granularity && (from..to).contains(start)
            })
            .map(|(key, _, start, aggregate)| (key.clone(), *start, *aggregate))
            .collect();
        rows.sort_by_key(|(_, start, _)| *start);
        Ok(rows)
    }

    fn list_throughput_tests(
        &self,
        from: i64,
        to: i64,
    ) -> Result<Vec<ThroughputRecord>, RepositoryError> {
        let mut tests: Vec<ThroughputRecord> = self
            .throughput_tests
            .iter()
            .filter(|record| (from..to).contains(&record.utc))
            .cloned()
            .collect();
        tests.sort_by_key(|record| std::cmp::Reverse(record.utc));
        Ok(tests)
    }

    fn is_history_empty(&self) -> Result<bool, RepositoryError> {
        Ok(self.rows.is_empty() && self.events.is_empty() && self.throughput_tests.is_empty())
    }

    fn list_events(&self, from: i64, to: i64) -> Result<Vec<HistoryEvent>, RepositoryError> {
        let mut events: Vec<HistoryEvent> = self
            .events
            .iter()
            .filter(|event| (from..to).contains(&event.utc))
            .cloned()
            .collect();
        events.sort_by_key(|event| event.utc);
        Ok(events)
    }
}

/// Streams what the lists answer: a test keeps few rows.
impl HistoryStream for InMemoryMeasurementRepository {
    fn stream_all_aggregates<E: From<RepositoryError>>(
        &self,
        granularity: Granularity,
        from: i64,
        to: i64,
        mut visit: impl FnMut(&NetworkKey, i64, &Aggregate) -> Result<(), E>,
    ) -> Result<(), E> {
        self.list_all_aggregates(granularity, from, to)?
            .iter()
            .try_for_each(|(network, start, aggregate)| visit(network, *start, aggregate))
    }

    fn stream_events<E: From<RepositoryError>>(
        &self,
        from: i64,
        to: i64,
        visit: impl FnMut(&HistoryEvent) -> Result<(), E>,
    ) -> Result<(), E> {
        self.list_events(from, to)?.iter().try_for_each(visit)
    }

    fn stream_throughput_tests<E: From<RepositoryError>>(
        &self,
        from: i64,
        to: i64,
        visit: impl FnMut(&ThroughputRecord) -> Result<(), E>,
    ) -> Result<(), E> {
        self.list_throughput_tests(from, to)?
            .iter()
            .rev()
            .try_for_each(visit)
    }
}

/// A box that answers with the line it was given, or not at all.
#[derive(Debug, Clone, Copy, Default)]
pub struct InMemoryAccessDetector(Option<BoxLine>);

impl InMemoryAccessDetector {
    #[must_use]
    pub const fn from_line(line: BoxLine) -> Self {
        Self(Some(line))
    }
}

impl AccessTechnologyDetector for InMemoryAccessDetector {
    fn read_box_line(&self) -> impl Future<Output = Option<BoxLine>> + Send {
        std::future::ready(self.0)
    }
}

/// Settings in memory: the defaults until saved.
#[derive(Debug, Clone, Copy, Default)]
pub struct InMemorySettingsRepository(Settings);

impl SettingsRepository for InMemorySettingsRepository {
    fn read_settings(&self) -> Result<Settings, RepositoryError> {
        Ok(self.0)
    }

    fn save_settings(&mut self, settings: &Settings) -> Result<(), RepositoryError> {
        self.0 = *settings;
        Ok(())
    }
}

/// A test server with a steady rate each way, timing transfers without waiting; or one that never
/// answers. Remembers the sizes asked for.
#[derive(Debug, Clone, Default)]
pub struct InMemoryThroughputServer {
    rates: Option<(u64, u64)>,
    transfers: Arc<Mutex<Vec<(Direction, u64)>>>,
}

impl InMemoryThroughputServer {
    /// The round trip measured during every transfer.
    pub const LOADED_ROUND_TRIP: Duration = Duration::from_millis(48);

    /// Rates in bits per second.
    #[must_use]
    pub fn with_rates(download: u64, upload: u64) -> Self {
        Self {
            rates: Some((download, upload)),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn create_unreachable() -> Self {
        Self::default()
    }

    /// The sizes asked for in `direction`, in order.
    #[must_use]
    pub fn list_transfers(&self, direction: Direction) -> Vec<u64> {
        self.transfers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|(asked, _)| *asked == direction)
            .map(|(_, bytes)| *bytes)
            .collect()
    }
}

impl ThroughputProbe for InMemoryThroughputServer {
    fn transfer(
        &self,
        direction: Direction,
        bytes: u64,
    ) -> impl Future<Output = Result<Transfer, ThroughputFailure>> + Send {
        self.transfers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((direction, bytes));
        let transfer =
            self.rates
                .ok_or(ThroughputFailure::Unreachable)
                .map(|(download, upload)| {
                    let rate = match direction {
                        Direction::Download => download,
                        Direction::Upload => upload,
                    };
                    Transfer {
                        bytes,
                        elapsed: Duration::from_micros(bytes * 8 * 1_000_000 / rate),
                        loaded_round_trips: vec![Self::LOADED_ROUND_TRIP],
                    }
                });
        std::future::ready(transfer)
    }
}

/// Keeps every degradation announced, in order.
#[derive(Debug, Clone, Default)]
pub struct InMemoryNotifier {
    notified: Arc<Mutex<Vec<VerdictView>>>,
}

impl InMemoryNotifier {
    #[must_use]
    pub fn list_notified(&self) -> Vec<VerdictView> {
        self.notified
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Notifier for InMemoryNotifier {
    fn notify_degradation(&self, view: &VerdictView) {
        self.notified
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(view.clone());
    }
}

/// Keeps every view it was given, in order.
#[derive(Debug, Clone, Default)]
pub struct InMemoryVerdictPresenter {
    views: Arc<Mutex<Vec<VerdictView>>>,
}

impl InMemoryVerdictPresenter {
    #[must_use]
    pub fn count_views(&self) -> usize {
        self.views
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

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

type ChangeListener = Box<dyn FnMut() + Send>;
type PowerListener = Box<dyn Fn(PowerEvent) + Send + Sync>;

/// A watcher the test fires by hand; clones share the listeners, so the test keeps one.
#[derive(Clone, Default)]
pub struct InMemoryNetworkChangeWatcher {
    listeners: Arc<Mutex<Vec<ChangeListener>>>,
}

impl InMemoryNetworkChangeWatcher {
    /// Tells every listener, as the system does on an interface or address change.
    pub fn report_change(&self) {
        for listener in self
            .listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter_mut()
        {
            listener();
        }
    }
}

impl NetworkChangeWatcher for InMemoryNetworkChangeWatcher {
    fn watch(self, on_change: impl FnMut() + Send + 'static) {
        self.listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Box::new(on_change));
    }
}

/// A power source the test drives by hand; clones share the listeners, so the test keeps one.
#[derive(Clone, Default)]
pub struct InMemoryPowerEventSource {
    listeners: Arc<Mutex<Vec<PowerListener>>>,
}

impl InMemoryPowerEventSource {
    /// Tells every listener and returns once they handled it, as the system waits on a sleep.
    pub fn report(&self, event: PowerEvent) {
        for listener in self
            .listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
        {
            listener(event);
        }
    }
}

impl PowerEventSource for InMemoryPowerEventSource {
    fn watch(self, on_event: impl Fn(PowerEvent) + Send + Sync + 'static) {
        self.listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(Box::new(on_event));
    }
}

/// A network name, as a box or a phone gives one: data a test needs but does not test.
#[must_use]
pub fn generate_network_name() -> String {
    format!("{}-{:04X}", Word().fake::<String>(), Faker.fake::<u16>())
}

/// `count` different addresses, for measurement targets or an own server. O(count²): a test asks
/// for a handful.
#[must_use]
pub fn generate_targets(count: usize) -> Vec<Target> {
    let mut targets: Vec<Target> = Vec::with_capacity(count);
    while targets.len() < count {
        let target = Target::parse(&IPv4().fake::<String>());
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets
}
