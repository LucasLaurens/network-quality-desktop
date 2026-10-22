use std::future::Future;

use std::net::Ipv4Addr;

use domain::{
    Aggregate, BoxLine, ByteCounters, ColorScheme, ConnectionContext, Direction, GatewayEcho,
    Granularity, ProbeOutcome, Settings, Target, Timestamp, Transfer, WifiLink,
};

use crate::{
    HistoryCommand, HistoryEvent, NetworkKey, RepositoryError, ThroughputFailure, ThroughputRecord,
    VerdictView,
};

/// Sends one probe to a target and says whether it answered, was lost, or is unreachable.
pub trait LatencyProbe {
    fn send_probe(&self, target: Target) -> impl Future<Output = ProbeOutcome> + Send;
}

/// Moves `bytes` one way to the test server and times it, measuring round trips meanwhile: the
/// latency under load.
pub trait ThroughputProbe {
    fn transfer(
        &self,
        direction: Direction,
        bytes: u64,
    ) -> impl Future<Output = Result<Transfer, ThroughputFailure>> + Send;
}

/// Pings the gateway over ICMP only: a TCP fallback would say nothing reliable about the link.
pub trait GatewayProbe {
    fn ping_gateway(&self, gateway: Ipv4Addr) -> impl Future<Output = GatewayEcho> + Send;
}

/// Reads the Wi-Fi link in use; `None` without Wi-Fi or when the system says nothing.
pub trait WifiReader {
    fn read_wifi(&self) -> impl Future<Output = Option<WifiLink>> + Send;
}

/// Describes the connection of the default route; `None` when there is none.
pub trait ConnectionContextReader {
    fn read_context(&self) -> impl Future<Output = Option<ConnectionContext>> + Send;
}

/// Reports changes of network interfaces or addresses, for the life of the app. Noise included
/// (a virtual interface coming and going): the engine checks whether the network really changed.
pub trait NetworkChangeWatcher {
    fn watch(self, on_change: impl FnMut() + Send + 'static);
}

/// What the system says about sleep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerEvent {
    WillSleep,
    DidWake,
}

/// Reports sleep and wake for the life of the app. The system may sleep as soon as the call for
/// `WillSleep` returns: `on_event` returns once the event is handled, within the system's delay.
pub trait PowerEventSource {
    fn watch(self, on_event: impl Fn(PowerEvent) + Send + Sync + 'static);
}

/// Monotonic time source, replaced by a manual clock in tests.
pub trait Clock {
    fn read_now(&self) -> Timestamp;
    /// Wall-clock seconds since 1970 UTC: the history is dated with them.
    fn read_utc_seconds(&self) -> i64;
}

/// Hands history writes to their own thread: recording never blocks the measurement.
pub trait HistoryRecorder {
    fn record(&self, command: HistoryCommand);
}

/// Where minutes, hours and events are kept. Every write is atomic.
pub trait MeasurementRepository {
    /// Adds to the minute already kept for that start, if any.
    ///
    /// # Errors
    ///
    /// The storage failed; nothing was written.
    fn add_minute(
        &mut self,
        network: &NetworkKey,
        start: i64,
        aggregate: &Aggregate,
    ) -> Result<(), RepositoryError>;

    /// Replaces the hour kept for that start, if any.
    ///
    /// # Errors
    ///
    /// The storage failed; nothing was written.
    fn save_hour(
        &mut self,
        network: &NetworkKey,
        start: i64,
        aggregate: &Aggregate,
    ) -> Result<(), RepositoryError>;

    /// The rows starting in `[from, to)`, oldest first.
    ///
    /// # Errors
    ///
    /// The storage failed.
    fn list_aggregates(
        &self,
        network: &NetworkKey,
        granularity: Granularity,
        from: i64,
        to: i64,
    ) -> Result<Vec<(i64, Aggregate)>, RepositoryError>;

    /// The hours before `before` that have minutes but no hour row yet.
    ///
    /// # Errors
    ///
    /// The storage failed.
    fn list_unmerged_hours(&self, before: i64) -> Result<Vec<(NetworkKey, i64)>, RepositoryError>;

    /// # Errors
    ///
    /// The storage failed; nothing was deleted.
    fn purge_minutes_before(&mut self, utc: i64) -> Result<(), RepositoryError>;

    /// # Errors
    ///
    /// The storage failed; nothing was written.
    fn record_event(&mut self, event: &HistoryEvent) -> Result<(), RepositoryError>;

    /// # Errors
    ///
    /// The storage failed; nothing was written.
    fn record_throughput_test(&mut self, record: &ThroughputRecord) -> Result<(), RepositoryError>;

    /// Deletes every minute, hour, event, throughput test and network; the settings stay.
    ///
    /// # Errors
    ///
    /// The storage failed; nothing was deleted.
    fn erase_history(&mut self) -> Result<(), RepositoryError>;
}

/// Reads the history for the dashboard, apart from the writer.
pub trait HistoryReader {
    /// The rows of every network starting in `[from, to)`, oldest first, each with the
    /// network's current name.
    ///
    /// # Errors
    ///
    /// The storage failed.
    fn list_all_aggregates(
        &self,
        granularity: Granularity,
        from: i64,
        to: i64,
    ) -> Result<Vec<(NetworkKey, i64, Aggregate)>, RepositoryError>;

    /// Nothing measured, no event and no test kept: there is nothing to export.
    ///
    /// # Errors
    ///
    /// The storage failed.
    fn is_history_empty(&self) -> Result<bool, RepositoryError>;

    /// The events in `[from, to)`, oldest first.
    ///
    /// # Errors
    ///
    /// The storage failed.
    fn list_events(&self, from: i64, to: i64) -> Result<Vec<HistoryEvent>, RepositoryError>;

    /// The throughput tests in `[from, to)`, newest first.
    ///
    /// # Errors
    ///
    /// The storage failed.
    fn list_throughput_tests(
        &self,
        from: i64,
        to: i64,
    ) -> Result<Vec<ThroughputRecord>, RepositoryError>;
}

/// Reads whether the system shows light or dark, as its menu bar does. Called once a second:
/// cheap, never blocking long.
pub trait ColorSchemeReader {
    fn read_color_scheme(&self) -> ColorScheme;
}

/// Reads the byte counters of the interface carrying the default route; `None` without one.
pub trait TrafficCounterReader {
    fn read_counters(&self) -> Option<ByteCounters>;
}

/// Asks the box about its line, read only; `None` when it does not answer (`UPnP` off, no such
/// service, another router in front).
pub trait AccessTechnologyDetector {
    fn read_box_line(&self) -> impl Future<Output = Option<BoxLine>> + Send;
}

/// Reads the history row by row, oldest first, for an export that never holds it whole. Each
/// visit may stop the reading with its own error; a storage failure converts into it.
pub trait HistoryStream {
    /// The rows of every network starting in `[from, to)`, each with the network's current name.
    ///
    /// # Errors
    ///
    /// The storage failed, or a visit did.
    fn stream_all_aggregates<E: From<RepositoryError>>(
        &self,
        granularity: Granularity,
        from: i64,
        to: i64,
        visit: impl FnMut(&NetworkKey, i64, &Aggregate) -> Result<(), E>,
    ) -> Result<(), E>;

    /// The events in `[from, to)`.
    ///
    /// # Errors
    ///
    /// The storage failed, or a visit did.
    fn stream_events<E: From<RepositoryError>>(
        &self,
        from: i64,
        to: i64,
        visit: impl FnMut(&HistoryEvent) -> Result<(), E>,
    ) -> Result<(), E>;

    /// The throughput tests in `[from, to)`.
    ///
    /// # Errors
    ///
    /// The storage failed, or a visit did.
    fn stream_throughput_tests<E: From<RepositoryError>>(
        &self,
        from: i64,
        to: i64,
        visit: impl FnMut(&ThroughputRecord) -> Result<(), E>,
    ) -> Result<(), E>;
}

/// Where the settings are kept: one row, defaults until the first save.
pub trait SettingsRepository {
    /// The defaults when nothing was saved yet.
    ///
    /// # Errors
    ///
    /// The storage failed.
    fn read_settings(&self) -> Result<Settings, RepositoryError>;

    /// Replaces every setting at once.
    ///
    /// # Errors
    ///
    /// The storage failed; nothing was written.
    fn save_settings(&mut self, settings: &Settings) -> Result<(), RepositoryError>;
}

/// Says whether the desktop can show a tray icon: some Linux sessions cannot.
pub trait TrayHost {
    fn is_tray_available(&self) -> bool;
}

/// Shows a system notification: called only for a durable degradation the user wants to hear of.
pub trait Notifier {
    fn notify_degradation(&self, view: &VerdictView);
}

/// Receives every verdict computed by the engine (menu-bar dot, later the popover).
pub trait VerdictPresenter {
    fn present(&self, view: &VerdictView);
}
