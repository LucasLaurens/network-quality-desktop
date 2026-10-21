/// Seconds without any server before « Hors ligne » shows.
pub const OFFLINE_AFTER_SECONDS: u32 = 5;

/// Counts consecutive seconds in which no target was reachable at all.
#[derive(Debug, Clone, Default)]
pub struct OfflineDetector {
    unreachable_seconds: u32,
}

impl OfflineDetector {
    /// Returns whether the connection is offline (5 consecutive unreachable seconds). O(1).
    pub fn record_second(&mut self, any_target_reachable: bool) -> bool {
        self.unreachable_seconds = if any_target_reachable {
            0
        } else {
            self.unreachable_seconds + 1
        };
        self.unreachable_seconds >= OFFLINE_AFTER_SECONDS
    }
}
