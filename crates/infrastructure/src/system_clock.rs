use std::time::{Instant, SystemTime, UNIX_EPOCH};

use application::Clock;
use domain::Timestamp;

/// Monotonic clock started when the measurement starts.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    started_at: Instant,
}

impl SystemClock {
    #[must_use]
    pub fn start() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn read_now(&self) -> Timestamp {
        Timestamp::from_elapsed(self.started_at.elapsed())
    }

    /// A clock set before 1970 reads as 1970: the history stays ordered.
    fn read_utc_seconds(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
            })
    }
}
