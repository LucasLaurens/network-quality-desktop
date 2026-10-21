//! Keeps the 1 Hz measurement on time while no window is visible.

/// macOS puts an app with no visible window into App Nap, which stretches its timers to tens of
/// seconds: declaring a user-initiated activity for the life of the app prevents it, and still
/// lets the computer sleep when idle. Nothing to do elsewhere.
pub fn keep_measuring_in_background() {
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

        let activity = NSProcessInfo::processInfo().beginActivityWithOptions_reason(
            NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
            &NSString::from_str("Mesure de la connexion chaque seconde"),
        );
        // Ended with the process only.
        std::mem::forget(activity);
    }
}
