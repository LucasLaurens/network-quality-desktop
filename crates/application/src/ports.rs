use std::future::Future;

use domain::{ConnectionContext, ProbeOutcome, Target, Timestamp};

use crate::VerdictView;

/// Sends one probe to a target and says whether it answered, was lost, or is unreachable.
pub trait LatencyProbe {
    fn send_probe(&self, target: Target) -> impl Future<Output = ProbeOutcome> + Send;
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
}

/// Says whether the desktop can show a tray icon: some Linux sessions cannot.
pub trait TrayHost {
    fn is_tray_available(&self) -> bool;
}

/// Receives every verdict computed by the engine (menu-bar dot, later the popover).
pub trait VerdictPresenter {
    fn present(&self, view: &VerdictView);
}
