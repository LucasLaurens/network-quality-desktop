//! The suspend and resume callback (Windows 8 and later), without a window. Windows waits about
//! 2 s after the suspend notice and cannot be asked for more.

use std::ffi::c_void;

use application::PowerEvent;
use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
use windows::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, PowerRegisterSuspendResumeNotification,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DEVICE_NOTIFY_CALLBACK, PBT_APMRESUMEAUTOMATIC, PBT_APMSUSPEND,
};

use super::PowerHandler;

/// Registers the callback and returns at once. O(1).
pub fn watch(on_event: PowerHandler) {
    let handler: &'static PowerHandler = Box::leak(Box::new(on_event));
    let parameters: &'static mut DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS =
        Box::leak(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(relay_power_change),
            Context: std::ptr::from_ref(handler).cast_mut().cast(),
        }));
    let mut registration: *mut c_void = std::ptr::null_mut();
    // SAFETY: the parameters and the handler they point to are leaked for the life of the app,
    // the registration is never cancelled, and `registration` is a valid local for the handle.
    let result = unsafe {
        PowerRegisterSuspendResumeNotification(
            DEVICE_NOTIFY_CALLBACK,
            HANDLE(std::ptr::from_mut(parameters).cast()),
            &raw mut registration,
        )
    };
    if result != ERROR_SUCCESS {
        eprintln!(
            "Registering for sleep notices failed ({result:?}): the gap between two ticks stands in"
        );
    }
}

/// Called by Windows on a thread of its own. The automatic resume always comes, the
/// user-initiated one only sometimes: listening to it alone gives one wake per sleep. O(1) plus
/// the engine's answer.
unsafe extern "system" fn relay_power_change(
    context: *const c_void,
    kind: u32,
    _setting: *const c_void,
) -> u32 {
    // SAFETY: the context is the handler leaked in `watch`.
    let handler = unsafe { &*context.cast::<PowerHandler>() };
    if kind == PBT_APMSUSPEND {
        handler(PowerEvent::WillSleep);
    } else if kind == PBT_APMRESUMEAUTOMATIC {
        handler(PowerEvent::DidWake);
    }
    ERROR_SUCCESS.0
}
