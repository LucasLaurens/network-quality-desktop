use std::time::Instant;

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
}
