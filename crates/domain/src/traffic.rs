//! The traffic seen on the interface, from the system's byte counters: what the computer moves,
//! not what the line can carry. Shown beside the verdict, never part of it.

use std::collections::VecDeque;
use std::time::Duration;

use crate::Bitrate;

/// Averaged over this long: one second is too jumpy to read.
pub const TRAFFIC_WINDOW: Duration = Duration::from_secs(5);
/// The peak is the busiest window of the last hour.
const PEAK_SPAN: Duration = Duration::from_hours(1);
/// A longer gap between two readings (a sleep, a stalled thread) tells nothing about a rate.
const MAX_READING_GAP: Duration = Duration::from_secs(3);

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

/// The bytes moved since the meter started, each way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrafficVolume {
    pub received: u64,
    pub sent: u64,
}

/// What the meter says: the rates are `None` until a whole window was seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrafficReading {
    pub current: Option<TrafficRate>,
    pub peak: Option<TrafficRate>,
    pub volume: TrafficVolume,
}

/// Rates over `TRAFFIC_WINDOW`, the last hour of them.
#[derive(Debug, Clone, Default)]
pub struct TrafficMeter {
    previous: Option<(Duration, ByteCounters)>,
    /// Bytes and time gathered towards the window in progress.
    received: u64,
    sent: u64,
    elapsed: Duration,
    /// Each window with the time it closed, oldest first.
    windows: VecDeque<(Duration, TrafficRate)>,
    volume: TrafficVolume,
}

impl TrafficMeter {
    /// `at` is a monotonic time. A counter that went back (a reset, a 32-bit wrap on macOS), another
    /// interface or a long gap only starts a new interval; the volume still counts a gap's bytes.
    /// O(1) amortised.
    pub fn record(&mut self, at: Duration, counters: ByteCounters) {
        let previous = self.previous.replace((at, counters));
        while self
            .windows
            .front()
            .is_some_and(|(closed, _)| *closed + PEAK_SPAN < at)
        {
            self.windows.pop_front();
        }
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
        if counters.interface != previous_counters.interface {
            return;
        }
        // A gap tells nothing about a rate, but its bytes did move.
        self.volume.received = self.volume.received.saturating_add(received);
        self.volume.sent = self.volume.sent.saturating_add(sent);
        if gap > MAX_READING_GAP {
            return;
        }
        self.received = self.received.saturating_add(received);
        self.sent = self.sent.saturating_add(sent);
        self.elapsed += gap;
        if self.elapsed >= TRAFFIC_WINDOW {
            self.close_window(at);
        }
    }

    /// The last window, unless it closed before a gap (a sleep): it no longer says what is
    /// happening. O(1).
    #[must_use]
    pub fn compute_current(&self) -> Option<TrafficRate> {
        let (latest, _) = self.previous?;
        self.windows
            .back()
            .filter(|(closed, _)| *closed + 2 * TRAFFIC_WINDOW >= latest)
            .map(|(_, rate)| *rate)
    }

    /// The busiest window of the last hour, each way on its own. O(w) for w ≤ 720 windows.
    #[must_use]
    pub fn compute_peak(&self) -> Option<TrafficRate> {
        let received = self.windows.iter().map(|(_, rate)| rate.received).max()?;
        let sent = self.windows.iter().map(|(_, rate)| rate.sent).max()?;
        Some(TrafficRate { received, sent })
    }

    /// O(w) for w ≤ 720 windows.
    #[must_use]
    pub fn compute_reading(&self) -> TrafficReading {
        TrafficReading {
            current: self.compute_current(),
            peak: self.compute_peak(),
            volume: self.volume,
        }
    }

    /// A window lasts at least `TRAFFIC_WINDOW`: never too short to time. O(1).
    fn close_window(&mut self, at: Duration) {
        if let (Some(received), Some(sent)) = (
            Bitrate::from_transfer(self.received, self.elapsed),
            Bitrate::from_transfer(self.sent, self.elapsed),
        ) {
            self.windows.push_back((at, TrafficRate { received, sent }));
        }
        self.received = 0;
        self.sent = 0;
        self.elapsed = Duration::ZERO;
    }
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

    /// One reading a second from `start`, counting on from `counters` by `received` bytes a second.
    fn record_seconds_from(
        meter: &mut TrafficMeter,
        start: u64,
        seconds: u64,
        counters: ByteCounters,
        received: u64,
    ) {
        for second in start..=start + seconds {
            meter.record(
                Duration::from_secs(second),
                ByteCounters {
                    received: counters.received + (second - start) * received,
                    ..counters
                },
            );
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
        // Another interface in between: the counting starts over.
        meter.record(
            Duration::from_secs(99),
            ByteCounters {
                interface: 5,
                ..create_counters(0, 0)
            },
        );

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

        record_seconds_from(&mut meter, 61, 5, create_counters(0, 0), MEGABYTE);

        // Assert
        assert_eq!(
            meter.compute_current().map(|rate| rate.received),
            Some(Bitrate::from_bits_per_second(8_000_000)),
            "only the five steady seconds count"
        );
    }

    #[test]
    fn forgets_a_peak_older_than_an_hour() {
        // Arrange
        let mut meter = TrafficMeter::default();
        record_seconds(&mut meter, 0, 5, 10 * MEGABYTE, 0);
        let quiet = create_counters(50 * MEGABYTE, 0);

        // Act
        for second in 6..=3_606 {
            meter.record(Duration::from_secs(second), quiet);
        }

        // Assert
        let peak = meter.compute_peak().expect("a peak");
        assert_eq!(peak.received, Bitrate::from_bits_per_second(0));
    }

    #[test]
    fn forgets_what_came_before_a_long_sleep() {
        // Arrange
        let mut meter = TrafficMeter::default();
        record_seconds(&mut meter, 0, 5, 10 * MEGABYTE, 0);

        // Act
        meter.record(Duration::from_hours(2), create_counters(0, 0));

        // Assert
        assert_eq!(meter.compute_current(), None);
        assert_eq!(meter.compute_peak(), None);
    }

    #[test]
    fn adds_up_the_bytes_moved_each_way() {
        // Arrange
        let mut meter = TrafficMeter::default();

        // Act
        record_seconds(&mut meter, 0, 3, MEGABYTE, MEGABYTE / 10);

        // Assert
        assert_eq!(
            meter.compute_reading().volume,
            TrafficVolume {
                received: 3 * MEGABYTE,
                sent: 3 * MEGABYTE / 10,
            },
            "counted before a whole window"
        );
    }

    #[test]
    fn counts_a_gap_in_the_volume_but_not_a_reset_or_another_interface() {
        // Arrange
        let mut meter = TrafficMeter::default();
        meter.record(Duration::ZERO, create_counters(5 * MEGABYTE, 0));

        // Act
        meter.record(Duration::from_mins(10), create_counters(7 * MEGABYTE, 0));
        meter.record(Duration::from_secs(601), create_counters(0, 0));
        meter.record(
            Duration::from_secs(602),
            ByteCounters {
                interface: 9,
                received: 900 * MEGABYTE,
                sent: 0,
            },
        );

        // Assert
        assert_eq!(
            meter.compute_reading().volume,
            TrafficVolume {
                received: 2 * MEGABYTE,
                sent: 0,
            }
        );
    }

    #[test]
    fn keeps_the_peak_but_not_the_current_rate_across_a_short_sleep() {
        // Arrange
        let mut meter = TrafficMeter::default();
        record_seconds(&mut meter, 0, 5, 10 * MEGABYTE, 0);

        // Act
        meter.record(Duration::from_secs(600), create_counters(0, 0));

        // Assert
        assert_eq!(meter.compute_current(), None);
        assert!(meter.compute_peak().is_some());
    }
}
