use std::time::Duration;

/// Quality level of one metric, ordered from best to worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
    Solid,
    Unstable,
    Degraded,
}

/// Boundaries of one metric: Solid below `unstable_from`, Degraded above `degraded_above`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GradeBounds {
    unstable_from: Duration,
    degraded_above: Duration,
}

impl GradeBounds {
    #[must_use]
    pub const fn new(unstable_from: Duration, degraded_above: Duration) -> Self {
        Self {
            unstable_from,
            degraded_above,
        }
    }

    #[must_use]
    pub fn compute_grade(&self, value: Duration) -> Grade {
        if value < self.unstable_from {
            Grade::Solid
        } else if value <= self.degraded_above {
            Grade::Unstable
        } else {
            Grade::Degraded
        }
    }
}

/// Single home of every verdict threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    pub latency: GradeBounds,
}

impl Thresholds {
    /// Defaults calibrated for video calls (ticket « Seuils de qualité pour la visio »).
    #[must_use]
    pub const fn for_video_calls() -> Self {
        Self {
            latency: GradeBounds::new(Duration::from_millis(100), Duration::from_millis(300)),
        }
    }
}
