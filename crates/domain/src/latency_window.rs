use std::collections::VecDeque;
use std::time::Duration;

const WINDOW_SECONDS: usize = 15;
const PERCENTILE: usize = 90;

/// Last 15 one-second latency samples; a second with no answer counts as no sample.
#[derive(Debug, Clone, Default)]
pub struct LatencyWindow {
    seconds: VecDeque<Option<Duration>>,
}

impl LatencyWindow {
    /// O(1) time, O(1) space: the window never holds more than 15 entries.
    pub fn record_second(&mut self, sample: Option<Duration>) {
        if self.seconds.len() == WINDOW_SECONDS {
            self.seconds.pop_front();
        }
        self.seconds.push_back(sample);
    }

    /// Nearest-rank p90 of the samples in the window, `None` without any sample.
    /// O(n log n) time, O(n) space with n ≤ 15, so effectively constant.
    #[must_use]
    pub fn compute_p90(&self) -> Option<Duration> {
        compute_nearest_rank(self.seconds.iter().flatten().copied().collect(), PERCENTILE)
    }
}

impl LatencyWindow {
    /// Mean gap between successive answered seconds, the quantity RFC 3550's jitter estimates;
    /// `None` without two successive answers. O(n) time and space, n ≤ 15.
    #[must_use]
    pub fn compute_jitter(&self) -> Option<Duration> {
        let gaps: Vec<Duration> = self
            .seconds
            .iter()
            .zip(self.seconds.iter().skip(1))
            .filter_map(|(previous, current)| Some(current.as_ref()?.abs_diff(*previous.as_ref()?)))
            .collect();
        compute_mean_duration(&gaps)
    }
}

/// The value at the nearest rank of `percentile` (out of 100), `None` without any value.
/// O(n log n) time for the sort, O(1) extra space.
#[must_use]
pub fn compute_nearest_rank<T: Ord + Copy>(mut values: Vec<T>, percentile: usize) -> Option<T> {
    values.sort_unstable();
    let rank = (values.len() * percentile).div_ceil(100).max(1);
    values.get(rank - 1).copied()
}

/// `None` without any duration. O(n) time, O(1) space.
#[must_use]
pub fn compute_mean_duration(durations: &[Duration]) -> Option<Duration> {
    let count = u32::try_from(durations.len())
        .ok()
        .filter(|count| *count > 0)?;
    Some(durations.iter().sum::<Duration>() / count)
}

/// Median of the round trips that came back, `None` when no target answered.
/// O(k log k) time, O(k) space, k being the number of targets (2 by default).
#[must_use]
pub fn compute_median(round_trips: &[Option<Duration>]) -> Option<Duration> {
    let mut answered: Vec<Duration> = round_trips.iter().flatten().copied().collect();
    if answered.is_empty() {
        return None;
    }
    answered.sort_unstable();
    let middle = answered.len() / 2;
    if answered.len() % 2 == 1 {
        answered.get(middle).copied()
    } else {
        Some((answered[middle - 1] + answered[middle]) / 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_nearest_rank() {
        // Arrange
        let values: Vec<u32> = (1..=20).collect();

        // Act & Assert
        assert_eq!(compute_nearest_rank(values, 90), Some(18));
        assert_eq!(compute_nearest_rank(vec![7_u32], 90), Some(7));
        assert_eq!(compute_nearest_rank(Vec::<u32>::new(), 90), None);
    }

    #[test]
    fn averages_durations() {
        // Arrange
        let durations = [Duration::from_millis(10), Duration::from_millis(30)];

        // Act & Assert
        assert_eq!(
            compute_mean_duration(&durations),
            Some(Duration::from_millis(20))
        );
        assert_eq!(compute_mean_duration(&[]), None);
    }
}
