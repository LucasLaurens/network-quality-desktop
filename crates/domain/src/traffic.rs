//! The traffic seen on the interface, from the system's byte counters: what the computer moves,
//! not what the line can carry. Shown beside the verdict, never part of it.

use std::collections::VecDeque;
use std::time::Duration;

use crate::Bitrate;

/// Averaged over this long: one second is too jumpy to read.
pub const TRAFFIC_WINDOW: Duration = Duration::from_secs(5);
/// The peak is the busiest window of the last hour.
const PEAK_WINDOWS: usize = 720;
/// A longer gap between two readings (a sleep, a stalled thread) tells nothing about a rate.
const MAX_READING_GAP: Duration = Duration::from_secs(3);
const BITS_PER_BYTE: u128 = 8;
const MICROS_PER_SECOND: u128 = 1_000_000;

/// The cumulative counters of one interface, as the system reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteCounters {
    /// Another interface starts the counting over.
    pub interface: u32,
    pub received: u64,
    pub sent: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrafficRate {
    pub received: Bitrate,
    pub sent: Bitrate,
}

/// Rates over `TRAFFIC_WINDOW`, the last hour of them.
#[derive(Debug, Clone, Default)]
pub struct TrafficMeter {
    previous: Option<(Duration, ByteCounters)>,
    /// Bytes and time gathered towards the window in progress.
    received: u64,
    sent: u64,
    elapsed: Duration,
    windows: VecDeque<TrafficRate>,
}

impl TrafficMeter {
    /// `at` is a monotonic time. A counter that went back (a reset, a 32-bit wrap on macOS), another
    /// interface or a long gap only starts a new interval. O(1) amortised.
    pub fn record(&mut self, at: Duration, counters: ByteCounters) {
        let previous = self.previous.replace((at, counters));
        let Some((previous_at, previous_counters)) = previous else {
            return;
        };
        let gap = at.saturating_sub(previous_at);
        let (Some(received), Some(sent)) = (
            counters.received.checked_sub(previous_counters.received),
            counters.sent.checked_sub(previous_counters.sent),
        ) else {
            return;
        };
        if counters.interface != previous_counters.interface || gap > MAX_READING_GAP {
            return;
        }
        self.received = self.received.saturating_add(received);
        self.sent = self.sent.saturating_add(sent);
        self.elapsed += gap;
        if self.elapsed >= TRAFFIC_WINDOW {
            self.close_window();
        }
    }

    /// The last window. O(1).
    #[must_use]
    pub fn compute_current(&self) -> Option<TrafficRate> {
        self.windows.back().copied()
    }

    /// The busiest window of the last hour, each way on its own. O(w) for w ≤ 720 windows.
    #[must_use]
    pub fn compute_peak(&self) -> Option<TrafficRate> {
        let received = self.windows.iter().map(|rate| rate.received).max()?;
        let sent = self.windows.iter().map(|rate| rate.sent).max()?;
        Some(TrafficRate { received, sent })
    }

    fn close_window(&mut self) {
        if self.windows.len() == PEAK_WINDOWS {
            self.windows.pop_front();
        }
        self.windows.push_back(TrafficRate {
            received: compute_rate(self.received, self.elapsed),
            sent: compute_rate(self.sent, self.elapsed),
        });
        self.received = 0;
        self.sent = 0;
        self.elapsed = Duration::ZERO;
    }
}

/// O(1).
fn compute_rate(bytes: u64, elapsed: Duration) -> Bitrate {
    let bits_per_second =
        u128::from(bytes) * BITS_PER_BYTE * MICROS_PER_SECOND / elapsed.as_micros().max(1);
    Bitrate::from_bits_per_second(u64::try_from(bits_per_second).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MEGABYTE: u64 = 1_000_000;

    fn create_counters(received: u64, sent: u64) -> ByteCounters {
        ByteCounters {
            interface: 4,
            received,
            sent,
        }
    }

    /// One reading a second from `start`, the counters growing by `received` and `sent` bytes.
    fn record_seconds(
        meter: &mut TrafficMeter,
        start: u64,
        seconds: u64,
        received: u64,
        sent: u64,
    ) {
        for second in start..=start + seconds {
            let elapsed = second - start;
            meter.record(
                Duration::from_secs(second),
                create_counters(elapsed * received, elapsed * sent),
            );
        }
    }

    #[test]
    fn averages_the_traffic_over_five_seconds() {
        // Arrange
        let mut meter = TrafficMeter::default();

        // Act
        record_seconds(&mut meter, 0, 5, MEGABYTE, MEGABYTE / 10);

        // Assert
        assert_eq!(
            meter.compute_current(),
            Some(TrafficRate {
                received: Bitrate::from_bits_per_second(8_000_000),
                sent: Bitrate::from_bits_per_second(800_000),
            })
        );
    }

    #[test]
    fn says_nothing_before_a_whole_window() {
        // Arrange
        let mut meter = TrafficMeter::default();

        // Act
        record_seconds(&mut meter, 0, 4, MEGABYTE, MEGABYTE);

        // Assert
        assert_eq!(meter.compute_current(), None);
        assert_eq!(meter.compute_peak(), None);
    }

    #[test]
    fn keeps_the_busiest_window_of_each_way() {
        // Arrange
        let mut meter = TrafficMeter::default();
        record_seconds(&mut meter, 0, 5, 10 * MEGABYTE, 0);
        meter = TrafficMeter {
            previous: None,
            ..meter
        };

        // Act
        record_seconds(&mut meter, 100, 5, 0, MEGABYTE);

        // Assert
        assert_eq!(
            meter.compute_peak(),
            Some(TrafficRate {
                received: Bitrate::from_bits_per_second(80_000_000),
                sent: Bitrate::from_bits_per_second(8_000_000),
            })
        );
    }

    #[test]
    fn counts_nothing_across_a_reset_another_interface_or_a_gap() {
        // Arrange
        let mut meter = TrafficMeter::default();
        meter.record(Duration::ZERO, create_counters(5 * MEGABYTE, 0));

        // Act
        meter.record(Duration::from_secs(1), create_counters(0, 0));
        meter.record(
            Duration::from_secs(2),
            ByteCounters {
                interface: 9,
                received: 900 * MEGABYTE,
                sent: 0,
            },
        );
        meter.record(
            Duration::from_secs(60),
            ByteCounters {
                interface: 9,
                received: 1_800 * MEGABYTE,
                sent: 0,
            },
        );

        // Assert
        assert_eq!(meter.compute_current(), None);
        assert_eq!(meter.received, 0);
    }

    #[test]
    fn forgets_a_peak_older_than_an_hour() {
        // Arrange
        let mut meter = TrafficMeter::default();
        record_seconds(&mut meter, 0, 5, 10 * MEGABYTE, 0);
        let quiet = create_counters(50 * MEGABYTE, 0);

        // Act
        for second in 6..=3_605 {
            meter.record(Duration::from_secs(second), quiet);
        }

        // Assert
        let peak = meter.compute_peak().expect("a peak");
        assert_eq!(peak.received, Bitrate::from_bits_per_second(0));
    }
}
