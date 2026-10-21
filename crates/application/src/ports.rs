use std::future::Future;

use std::net::Ipv4Addr;

use domain::{
    Aggregate, ConnectionContext, GatewayEcho, Granularity, ProbeOutcome, Target, Timestamp,
    WifiLink,
};

use crate::{HistoryCommand, HistoryEvent, NetworkKey, RepositoryError, VerdictView};

/// Sends one probe to a target and says whether it answered, was lost, or is unreachable.
pub trait LatencyProbe {
    fn send_probe(&self, target: Target) -> impl Future<Output = ProbeOutcome> + Send;
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

    /// The events in `[from, to)`, oldest first.
    ///
    /// # Errors
    ///
    /// The storage failed.
    fn list_events(&self, from: i64, to: i64) -> Result<Vec<HistoryEvent>, RepositoryError>;
}

/// Says whether the desktop can show a tray icon: some Linux sessions cannot.
pub trait TrayHost {
    fn is_tray_available(&self) -> bool;
}

/// Receives every verdict computed by the engine (menu-bar dot, later the popover).
pub trait VerdictPresenter {
    fn present(&self, view: &VerdictView);
}
