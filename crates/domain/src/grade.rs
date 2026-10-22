use std::time::Duration;

use crate::LossRate;

/// Quality level of one metric, ordered from best to worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
    Solid,
    Unstable,
    Degraded,
}

/// A value is "near" the next grade once it reaches this share of the next boundary.
const NEAR_NEXT_SHARE: f64 = 0.8;

/// Boundaries of one metric: Solid below `unstable_from`, Degraded above `degraded_above`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradeBounds<T> {
    unstable_from: T,
    degraded_above: T,
}

/// A measured quantity that can be compared with a scaled boundary.
pub trait Scalable: Copy + PartialOrd {
    #[must_use]
    fn scale_by(self, factor: f64) -> Self;
}

impl Scalable for Duration {
    fn scale_by(self, factor: f64) -> Self {
        self.mul_f64(factor)
    }
}

impl<T: Scalable> GradeBounds<T> {
    #[must_use]
    pub const fn new(unstable_from: T, degraded_above: T) -> Self {
        Self {
            unstable_from,
            degraded_above,
        }
    }

    /// O(1).
    #[must_use]
    pub fn compute_grade(&self, value: T) -> Grade {
        if value < self.unstable_from {
            Grade::Solid
        } else if value <= self.degraded_above {
            Grade::Unstable
        } else {
            Grade::Degraded
        }
    }

    /// The boundary worth showing next to the value: the one it crossed, or the next one while Solid. O(1).
    #[must_use]
    pub fn find_reference_threshold(&self, value: T) -> T {
        match self.compute_grade(value) {
            Grade::Solid | Grade::Unstable => self.unstable_from,
            Grade::Degraded => self.degraded_above,
        }
    }

    #[must_use]
    pub const fn read_unstable_from(&self) -> T {
        self.unstable_from
    }

    #[must_use]
    pub const fn read_degraded_above(&self) -> T {
        self.degraded_above
    }

    /// True when the value is within 20 % of the boundary that would make it worse. O(1).
    #[must_use]
    pub fn is_near_next_threshold(&self, value: T) -> bool {
        match self.compute_grade(value) {
            Grade::Solid => value >= self.unstable_from.scale_by(NEAR_NEXT_SHARE),
            Grade::Unstable => value >= self.degraded_above.scale_by(NEAR_NEXT_SHARE),
            Grade::Degraded => false,
        }
    }
}

/// Single home of every metric threshold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    pub latency: GradeBounds<Duration>,
    pub jitter: GradeBounds<Duration>,
    pub loss: GradeBounds<LossRate>,
}

impl Thresholds {
    /// Defaults calibrated for video calls (ticket « Seuils de qualité pour la visio »).
    pub const VIDEO_CALLS: Self = Self {
        latency: GradeBounds::new(Duration::from_millis(100), Duration::from_millis(300)),
        jitter: GradeBounds::new(Duration::from_millis(30), Duration::from_millis(50)),
        loss: GradeBounds::new(LossRate::from_percent(1.0), LossRate::from_percent(5.0)),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const LATENCY: GradeBounds<Duration> = Thresholds::VIDEO_CALLS.latency;

    fn create_millis(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn grades_below_between_and_above_the_bounds() {
        // Act & Assert
        assert_eq!(LATENCY.compute_grade(create_millis(99)), Grade::Solid);
        assert_eq!(LATENCY.compute_grade(create_millis(100)), Grade::Unstable);
        assert_eq!(LATENCY.compute_grade(create_millis(300)), Grade::Unstable);
        assert_eq!(LATENCY.compute_grade(create_millis(301)), Grade::Degraded);
    }

    #[test]
    fn shows_the_boundary_crossed_or_the_next_one() {
        // Act & Assert
        assert_eq!(
            LATENCY.find_reference_threshold(create_millis(20)),
            create_millis(100)
        );
        assert_eq!(
            LATENCY.find_reference_threshold(create_millis(200)),
            create_millis(100)
        );
        assert_eq!(
            LATENCY.find_reference_threshold(create_millis(400)),
            create_millis(300)
        );
        assert_eq!(LATENCY.read_unstable_from(), create_millis(100));
        assert_eq!(LATENCY.read_degraded_above(), create_millis(300));
    }

    #[test]
    fn warns_within_a_fifth_of_the_next_boundary() {
        // Act & Assert
        assert!(!LATENCY.is_near_next_threshold(create_millis(79)));
        assert!(LATENCY.is_near_next_threshold(create_millis(80)));
        assert!(!LATENCY.is_near_next_threshold(create_millis(239)));
        assert!(LATENCY.is_near_next_threshold(create_millis(240)));
        assert!(!LATENCY.is_near_next_threshold(create_millis(500)));
    }

    #[test]
    fn grades_loss_with_the_same_rules() {
        // Arrange
        let loss = Thresholds::VIDEO_CALLS.loss;

        // Act & Assert
        assert_eq!(
            loss.compute_grade(LossRate::from_percent(0.5)),
            Grade::Solid
        );
        assert_eq!(
            loss.compute_grade(LossRate::from_percent(6.0)),
            Grade::Degraded
        );
        assert!(loss.is_near_next_threshold(LossRate::from_percent(0.8)));
    }
}
