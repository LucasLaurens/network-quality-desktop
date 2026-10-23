use std::collections::VecDeque;

use crate::grade::Scalable;

const WINDOW_SECONDS: usize = 60;
/// Below 60 probes (30 s with two targets) a single loss would weigh several percent.
const MIN_PROBES: u32 = 60;
const PERCENT: f64 = 100.0;

/// Share of probes that got no answer, in percent.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct LossRate(f64);

impl LossRate {
    #[must_use]
    pub const fn from_percent(percent: f64) -> Self {
        Self(percent)
    }

    #[must_use]
    pub const fn to_percent(self) -> f64 {
        self.0
    }

    /// `sent` must be positive: callers wait for enough probes to judge. O(1).
    #[must_use]
    pub fn from_counts(lost: u32, sent: u32) -> Self {
        Self(f64::from(lost) * PERCENT / f64::from(sent))
    }
}

impl Scalable for LossRate {
    fn scale_by(self, factor: f64) -> Self {
        Self(self.0 * factor)
    }
}

/// Probes sent and lost over the last 60 seconds, all targets together.
#[derive(Debug, Clone, Default)]
pub struct LossWindow {
    seconds: VecDeque<(u32, u32)>,
    sent: u32,
    lost: u32,
}

impl LossWindow {
    /// O(1) time: running totals are adjusted instead of re-summed. O(1) space (60 entries at most).
    pub fn record_second(&mut self, sent: u32, lost: u32) {
        if self.seconds.len() == WINDOW_SECONDS
            && let Some((old_sent, old_lost)) = self.seconds.pop_front()
        {
            self.sent -= old_sent;
            self.lost -= old_lost;
        }
        self.seconds.push_back((sent, lost));
        self.sent += sent;
        self.lost += lost;
    }

    /// `None` until 60 probes were sent. O(1).
    #[must_use]
    pub fn compute_rate(&self) -> Option<LossRate> {
        (self.sent >= MIN_PROBES).then(|| LossRate::from_counts(self.lost, self.sent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_for_sixty_probes() {
        // Arrange
        let mut window = LossWindow::default();
        for _ in 0..29 {
            window.record_second(2, 0);
        }

        // Act
        let before = window.compute_rate();
        window.record_second(2, 1);
        let after = window.compute_rate();

        // Assert
        assert_eq!(before, None);
        assert_eq!(after.map(LossRate::to_percent), Some(100.0 / 60.0));
    }

    #[test]
    fn forgets_what_is_older_than_a_minute() {
        // Arrange
        let mut window = LossWindow::default();
        window.record_second(2, 2);
        for _ in 1..WINDOW_SECONDS {
            window.record_second(2, 0);
        }

        // Act
        window.record_second(2, 0);

        // Assert
        assert_eq!(window.compute_rate(), Some(LossRate::from_percent(0.0)));
    }

    #[test]
    fn rates_lost_probes_out_of_those_sent() {
        // Act & Assert
        assert_eq!(LossRate::from_counts(3, 60), LossRate::from_percent(5.0));
    }

    #[test]
    fn scales_a_rate() {
        // Act & Assert
        assert_eq!(
            LossRate::from_percent(5.0).scale_by(0.8),
            LossRate::from_percent(4.0)
        );
    }
}
