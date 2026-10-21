//! `NWPath.isExpensive` and `isConstrained` through the C API of Network.framework, without
//! Swift. A path monitor runs for the life of the app and keeps the latest answer.

use std::ffi::c_void;
use std::sync::{Mutex, Once, PoisonError};

use block2::{Block, RcBlock};
use dispatch2::{DispatchQueue, DispatchRetained};
use domain::Metering;

/// `nw_path_monitor_t` and `nw_path_t`: opaque Network.framework objects.
type NetworkObject = *mut c_void;

#[link(name = "Network", kind = "framework")]
unsafe extern "C" {
    fn nw_path_monitor_create() -> NetworkObject;
    fn nw_path_monitor_set_queue(monitor: NetworkObject, queue: *mut c_void);
    fn nw_path_monitor_set_update_handler(
        monitor: NetworkObject,
        handler: &Block<dyn Fn(NetworkObject)>,
    );
    fn nw_path_monitor_start(monitor: NetworkObject);
    fn nw_path_is_expensive(path: NetworkObject) -> bool;
    fn nw_path_is_constrained(path: NetworkObject) -> bool;
}

static LATEST: Mutex<Metering> = Mutex::new(Metering::Unknown);
static MONITOR: Once = Once::new();

/// The first call starts the monitor and still answers `Unknown`: its first update arrives on
/// its own queue a moment later. O(1).
pub fn read_metering() -> Metering {
    MONITOR.call_once(start_monitor);
    *LATEST.lock().unwrap_or_else(PoisonError::into_inner)
}

fn start_monitor() {
    let queue: DispatchRetained<DispatchQueue> = DispatchQueue::new("path-monitor", None);
    // Nothing in the handler can panic: unwinding through the C frame that calls it is undefined.
    let handler = RcBlock::new(|path: NetworkObject| {
        // SAFETY: Network.framework passes a path that stays valid for the handler call; only its
        // two booleans are copied out.
        let (is_expensive, is_constrained) =
            unsafe { (nw_path_is_expensive(path), nw_path_is_constrained(path)) };
        *LATEST.lock().unwrap_or_else(PoisonError::into_inner) =
            to_metering(is_expensive, is_constrained);
    });
    // SAFETY: the monitor is created, given a serial queue and a handler, then started, in the
    // order the API requires. Monitor, queue and handler are kept for the life of the app.
    unsafe {
        let monitor = nw_path_monitor_create();
        nw_path_monitor_set_queue(monitor, DispatchRetained::as_ptr(&queue).as_ptr().cast());
        nw_path_monitor_set_update_handler(monitor, &handler);
        nw_path_monitor_start(monitor);
    }
    std::mem::forget(queue);
    std::mem::forget(handler);
}

const fn to_metering(is_expensive: bool, is_constrained: bool) -> Metering {
    match (is_expensive, is_constrained) {
        (true, _) => Metering::Metered,
        (false, true) => Metering::Constrained,
        (false, false) => Metering::Unmetered,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn counts_an_expensive_path_as_metered_before_the_low_data_mode() {
        // Act & Assert
        assert_eq!(to_metering(true, true), Metering::Metered);
        assert_eq!(to_metering(false, true), Metering::Constrained);
        assert_eq!(to_metering(false, false), Metering::Unmetered);
    }

    // Depends on the machine running the test: run on demand with `-- --ignored`.
    #[test]
    #[ignore = "reads the network path of the machine running the test"]
    fn answers_once_the_monitor_reported_the_current_path() {
        // Arrange
        read_metering();
        std::thread::sleep(Duration::from_millis(500));

        // Act
        let metering = read_metering();

        // Assert
        println!("{metering:?}");
        assert_ne!(metering, Metering::Unknown);
    }
}
