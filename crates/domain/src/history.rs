//! What is kept of the measurements: per-minute and per-hour aggregates that add up, and the
//! last hour second by second in memory. No raw sample is ever stored.

use std::collections::VecDeque;
use std::time::Duration;

/// Latency histogram buckets: one under 1 ms, 22 log-spaced up to 2 s, one beyond.
pub const LATENCY_BUCKETS: usize = 24;
const FIRST_EDGE: Duration = Duration::from_millis(1);
const LAST_EDGE: Duration = Duration::from_secs(2);
/// Log-spaced buckets between the first and the last edge.
const INNER_BUCKETS: f64 = 22.0;
const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3600;
/// The ring keeps one point per second for the last hour.
pub const RING_SECONDS: usize = 3600;

/// Counts of latency samples per bucket. Two histograms add up, so any period's percentiles
/// come from its minutes or hours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LatencyHistogram([u32; LATENCY_BUCKETS]);

impl LatencyHistogram {
    /// O(1).
    pub fn record(&mut self, latency: Duration) {
        let bucket = find_bucket(latency);
        self.0[bucket] = self.0[bucket].saturating_add(1);
    }

    /// O(B), B = 24.
    pub fn merge(&mut self, other: &Self) {
        for (count, added) in self.0.iter_mut().zip(other.0) {
            *count = count.saturating_add(added);
        }
    }

    /// The upper edge of the bucket holding the `percentile`; `None` without samples. O(B).
    #[must_use]
    pub fn compute_percentile(&self, percentile: f64) -> Option<Duration> {
        let total: u64 = self.0.iter().map(|count| u64::from(*count)).sum();
        if total == 0 {
            return None;
        }
        // Nearest rank, as for the live p90.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let rank = ((percentile / 100.0) * total as f64).ceil().max(1.0) as u64;
        let mut seen = 0;
        self.0.iter().enumerate().find_map(|(bucket, count)| {
            seen += u64::from(*count);
            (seen >= rank).then(|| compute_upper_edge(bucket))
        })
    }

    /// Little-endian counts, 4 bytes each: the stored form. O(B).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0
            .iter()
            .flat_map(|count| count.to_le_bytes())
            .collect()
    }

    /// `None` when the bytes do not hold exactly 24 counts. O(B).
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != LATENCY_BUCKETS * 4 {
            return None;
        }
        let mut counts = [0; LATENCY_BUCKETS];
        for (count, chunk) in counts.iter_mut().zip(bytes.as_chunks::<4>().0) {
            *count = u32::from_le_bytes(*chunk);
        }
        Some(Self(counts))
    }
}

/// O(1): a logarithm and a clamp.
fn find_bucket(latency: Duration) -> usize {
    if latency < FIRST_EDGE {
        return 0;
    }
    if latency >= LAST_EDGE {
        return LATENCY_BUCKETS - 1;
    }
    let position = (latency.div_duration_f64(FIRST_EDGE)).ln()
        / LAST_EDGE.div_duration_f64(FIRST_EDGE).ln()
        * INNER_BUCKETS;
    // Between 0 and 22 by the two guards above.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let inner = position.floor() as usize;
    1 + inner.min(LATENCY_BUCKETS - 3)
}

fn compute_upper_edge(bucket: usize) -> Duration {
    if bucket + 1 >= LATENCY_BUCKETS {
        return LAST_EDGE;
    }
    #[expect(clippy::cast_precision_loss)]
    let exponent = bucket as f64 / INNER_BUCKETS;
    FIRST_EDGE.mul_f64(LAST_EDGE.div_duration_f64(FIRST_EDGE).powf(exponent))
}

/// The state shown during a second, as it is counted in the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountedState {
    Solid,
    Unstable,
    Degraded,
    Offline,
}

/// What one measured second adds to the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondSample {
    pub latency: Option<Duration>,
    /// Change of latency since the previous second.
    pub jitter: Option<Duration>,
    pub probes_sent: u32,
    pub probes_lost: u32,
    /// `None` while measuring: those seconds count in no state.
    pub state: Option<CountedState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StateSeconds {
    pub solid: u32,
    pub unstable: u32,
    pub degraded: u32,
    pub offline: u32,
}

/// Everything kept of a minute or an hour. Merging two gives the aggregate of both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Aggregate {
    pub latency: LatencyHistogram,
    pub jitter_sum_us: u64,
    pub jitter_count: u32,
    pub probes_sent: u32,
    pub probes_lost: u32,
    pub seconds: StateSeconds,
}

impl Aggregate {
    /// O(1).
    pub fn record_second(&mut self, sample: &SecondSample) {
        if let Some(latency) = sample.latency {
            self.latency.record(latency);
        }
        if let Some(jitter) = sample.jitter {
            self.jitter_sum_us = self
                .jitter_sum_us
                .saturating_add(u64::try_from(jitter.as_micros()).unwrap_or(u64::MAX));
            self.jitter_count = self.jitter_count.saturating_add(1);
        }
        self.probes_sent = self.probes_sent.saturating_add(sample.probes_sent);
        self.probes_lost = self.probes_lost.saturating_add(sample.probes_lost);
        let counter = match sample.state {
            Some(CountedState::Solid) => &mut self.seconds.solid,
            Some(CountedState::Unstable) => &mut self.seconds.unstable,
            Some(CountedState::Degraded) => &mut self.seconds.degraded,
            Some(CountedState::Offline) => &mut self.seconds.offline,
            None => return,
        };
        *counter = counter.saturating_add(1);
    }

    /// O(B).
    pub fn merge(&mut self, other: &Self) {
        self.latency.merge(&other.latency);
        self.jitter_sum_us = self.jitter_sum_us.saturating_add(other.jitter_sum_us);
        self.jitter_count = self.jitter_count.saturating_add(other.jitter_count);
        self.probes_sent = self.probes_sent.saturating_add(other.probes_sent);
        self.probes_lost = self.probes_lost.saturating_add(other.probes_lost);
        self.seconds.solid = self.seconds.solid.saturating_add(other.seconds.solid);
        self.seconds.unstable = self.seconds.unstable.saturating_add(other.seconds.unstable);
        self.seconds.degraded = self.seconds.degraded.saturating_add(other.seconds.degraded);
        self.seconds.offline = self.seconds.offline.saturating_add(other.seconds.offline);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Granularity {
    Minute,
    Hour,
}

impl Granularity {
    #[must_use]
    pub const fn to_seconds(self) -> i64 {
        match self {
            Self::Minute => SECONDS_PER_MINUTE,
            Self::Hour => SECONDS_PER_HOUR,
        }
    }

    /// The start of the minute or hour holding `utc` (seconds since 1970). O(1).
    #[must_use]
    pub const fn find_start(self, utc: i64) -> i64 {
        utc - utc.rem_euclid(self.to_seconds())
    }
}

/// Adds up the seconds of the current minute and hands it over once the minute is over.
#[derive(Debug, Clone, Default)]
pub struct MinuteAccumulator {
    current: Option<(i64, Aggregate)>,
}

impl MinuteAccumulator {
    /// Returns the minute that just ended, if `utc` starts a new one. O(1).
    pub fn record_second(&mut self, utc: i64, sample: &SecondSample) -> Option<(i64, Aggregate)> {
        let start = Granularity::Minute.find_start(utc);
        let finished = match self.current {
            Some((current_start, _)) if current_start != start => self.current.take(),
            _ => None,
        };
        self.current
            .get_or_insert_with(|| (start, Aggregate::default()))
            .1
            .record_second(sample);
        finished
    }

    /// Hands over the minute in progress (closing, new network). O(1).
    pub fn take_current(&mut self) -> Option<(i64, Aggregate)> {
        self.current.take()
    }
}

/// One second of the last hour, for the one-hour chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondPoint {
    pub utc: i64,
    pub latency: Option<Duration>,
    pub state: Option<CountedState>,
}

/// The last hour, second by second; the oldest point leaves when a new one comes. O(1) per
/// second, about 3 600 points.
#[derive(Debug, Clone, Default)]
pub struct SecondRing {
    points: VecDeque<SecondPoint>,
}

impl SecondRing {
    /// O(1).
    pub fn record(&mut self, point: SecondPoint) {
        if self.points.len() == RING_SECONDS {
            self.points.pop_front();
        }
        self.points.push_back(point);
    }

    /// Oldest first. O(n) for n ≤ 3 600 points.
    #[must_use]
    pub fn list_points(&self) -> Vec<SecondPoint> {
        self.points.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_sample(latency_ms: u64, state: CountedState) -> SecondSample {
        SecondSample {
            latency: Some(Duration::from_millis(latency_ms)),
            jitter: Some(Duration::from_millis(2)),
            probes_sent: 2,
            probes_lost: 0,
            state: Some(state),
        }
    }

    #[test]
    fn finds_percentiles_from_a_histogram() {
        // Arrange
        let mut histogram = LatencyHistogram::default();
        for latency in [10, 12, 15, 20, 300] {
            histogram.record(Duration::from_millis(latency));
        }

        // Act
        let median = histogram.compute_percentile(50.0).expect("samples");
        let p90 = histogram.compute_percentile(90.0).expect("samples");

        // Assert
        assert!(median >= Duration::from_millis(12) && median < Duration::from_millis(20));
        assert!(p90 >= Duration::from_millis(300));
        assert_eq!(LatencyHistogram::default().compute_percentile(90.0), None);
    }

    #[test]
    fn keeps_extreme_latencies_in_the_end_buckets() {
        // Arrange
        let mut histogram = LatencyHistogram::default();

        // Act
        histogram.record(Duration::from_micros(300));
        histogram.record(Duration::from_secs(5));

        // Assert
        assert_eq!(histogram.0[0], 1);
        assert_eq!(histogram.0[LATENCY_BUCKETS - 1], 1);
    }

    #[test]
    fn stores_and_reads_back_a_histogram() {
        // Arrange
        let mut histogram = LatencyHistogram::default();
        histogram.record(Duration::from_millis(42));

        // Act
        let read = LatencyHistogram::from_bytes(&histogram.to_bytes());

        // Assert
        assert_eq!(read, Some(histogram));
        assert_eq!(LatencyHistogram::from_bytes(&[0; 7]), None);
    }

    #[test]
    fn merges_two_minutes_into_the_aggregate_of_both() {
        // Arrange
        let mut first = Aggregate::default();
        first.record_second(&create_sample(20, CountedState::Solid));
        let mut second = Aggregate::default();
        second.record_second(&create_sample(180, CountedState::Unstable));

        // Act
        first.merge(&second);

        // Assert
        assert_eq!(first.probes_sent, 4);
        assert_eq!(first.jitter_count, 2);
        assert_eq!((first.seconds.solid, first.seconds.unstable), (1, 1));
    }

    #[test]
    fn hands_over_a_minute_once_the_next_one_starts() {
        // Arrange
        let mut accumulator = MinuteAccumulator::default();
        let minute = 1_700_000_040;
        for second in 0..60 {
            accumulator.record_second(minute + second, &create_sample(20, CountedState::Solid));
        }

        // Act
        let finished =
            accumulator.record_second(minute + 60, &create_sample(20, CountedState::Solid));

        // Assert
        let (start, aggregate) = finished.expect("a finished minute");
        assert_eq!(start, minute);
        assert_eq!(aggregate.seconds.solid, 60);
    }

    #[test]
    fn counts_no_state_while_measuring() {
        // Arrange
        let mut aggregate = Aggregate::default();
        let sample = SecondSample {
            state: None,
            ..create_sample(20, CountedState::Solid)
        };

        // Act
        aggregate.record_second(&sample);

        // Assert
        assert_eq!(aggregate.seconds, StateSeconds::default());
        assert_eq!(aggregate.probes_sent, 2);
    }

    #[test]
    fn keeps_only_the_last_hour_in_the_ring() {
        // Arrange
        let mut ring = SecondRing::default();

        // Act
        // Five seconds more than the ring holds.
        for utc in 0..3_605 {
            ring.record(SecondPoint {
                utc,
                latency: None,
                state: None,
            });
        }

        // Assert
        let points = ring.list_points();
        assert_eq!(points.len(), RING_SECONDS);
        assert_eq!(points.first().map(|point| point.utc), Some(5));
    }

    #[test]
    fn finds_the_start_of_a_minute_and_an_hour() {
        // Act & Assert
        assert_eq!(Granularity::Minute.find_start(3_725), 3_720);
        assert_eq!(Granularity::Hour.find_start(3_725), 3_600);
    }
}
