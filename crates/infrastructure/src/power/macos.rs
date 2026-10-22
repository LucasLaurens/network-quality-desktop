//! `IOKit` system power notices: the only macOS notice that can hold the sleep back (up to 30 s)
//! until the minute in progress is written.

use std::ffi::c_void;
use std::sync::OnceLock;

use application::PowerEvent;
use dispatch2::DispatchQueue;
use objc2_io_kit::{
    IOAllowPowerChange, IONotificationPort, IONotificationPortRef, IORegisterForSystemPower,
    io_connect_t, io_object_t, io_service_t, kIOMessageCanSystemSleep,
    kIOMessageSystemHasPoweredOn, kIOMessageSystemWillSleep,
};

use super::PowerHandler;

/// What the `IOKit` callback needs, kept for the life of the app.
struct Registration {
    /// Known only once the registration returns, before the first notice can arrive.
    root_port: OnceLock<io_connect_t>,
    on_event: PowerHandler,
}

/// Registers with `IOKit` and returns at once; the notices come on the power queue. O(1).
pub fn watch(on_event: PowerHandler) {
    let registration: &'static Registration = Box::leak(Box::new(Registration {
        root_port: OnceLock::new(),
        on_event,
    }));
    let mut port: IONotificationPortRef = std::ptr::null_mut();
    let mut notifier: io_object_t = 0;
    // SAFETY: the context points to a registration leaked for the life of the app, and the two
    // out-pointers are valid locals.
    let root_port = unsafe {
        IORegisterForSystemPower(
            std::ptr::from_ref(registration).cast_mut().cast(),
            &raw mut port,
            Some(relay_power_change),
            &raw mut notifier,
        )
    };
    if root_port == 0 || port.is_null() {
        eprintln!("Registering for sleep notices failed: the gap between two ticks stands in");
        return;
    }
    let _ = registration.root_port.set(root_port);
    // A serial queue of its own: a notice that waits for the engine blocks nothing else.
    let queue = DispatchQueue::new("power-events", None);
    // SAFETY: the port was just created by IOKit and is never destroyed.
    unsafe { IONotificationPort::set_dispatch_queue(port, Some(&queue)) };
    // The notices come for the life of the app: the queue must outlive them.
    std::mem::forget(queue);
}

/// Called by `IOKit` on the power queue. Idle sleep is never vetoed; a sleep is acknowledged once
/// the minute is written. O(1) plus the engine's answer.
unsafe extern "C-unwind" fn relay_power_change(
    context: *mut c_void,
    _service: io_service_t,
    message_type: u32,
    argument: *mut c_void,
) {
    // SAFETY: the context is the registration leaked in `watch`.
    let registration = unsafe { &*context.cast::<Registration>() };
    let Some(&root_port) = registration.root_port.get() else {
        return;
    };
    let notification = argument.addr().cast_signed();
    let hand_on = |event: PowerEvent| {
        // A panic must not unwind into IOKit: the sleep would wait for an answer that never comes.
        let handled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (registration.on_event)(event);
        }));
        if handled.is_err() {
            eprintln!("Handling a sleep notice failed");
        }
    };
    if message_type == kIOMessageCanSystemSleep {
        IOAllowPowerChange(root_port, notification);
    } else if message_type == kIOMessageSystemWillSleep {
        hand_on(PowerEvent::WillSleep);
        IOAllowPowerChange(root_port, notification);
    } else if message_type == kIOMessageSystemHasPoweredOn {
        hand_on(PowerEvent::DidWake);
    }
}
