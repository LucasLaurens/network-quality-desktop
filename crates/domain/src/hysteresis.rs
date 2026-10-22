use std::time::Duration;

use crate::{Grade, Timestamp, Verdict};

/// How long a worse grade must last before it shows: the single home of « plus de 10 s ».
pub const DEGRADE_AFTER: Duration = Duration::from_secs(10);
const IMPROVE_AFTER: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Worse,
    Better,
}

/// Keeps the dot steady: a worse grade shows after 10 sustained seconds, a better one after 30.
/// The direction is what must persist: Solid, then Unstable, then Degraded shows Degraded 10 s
/// after the connection first got worse.
#[derive(Debug, Clone)]
pub struct Hysteresis {
    displayed: Verdict,
    pending_since: Option<(Direction, Timestamp)>,
}

impl Default for Hysteresis {
    fn default() -> Self {
        Self {
            displayed: Verdict::Measuring,
            pending_since: None,
        }
    }
}

impl Hysteresis {
    /// Feeds the grade observed now and returns the verdict to display. O(1) time and space.
    pub fn observe(&mut self, observed: Grade, now: Timestamp) -> Verdict {
        let Verdict::Graded(displayed) = self.displayed else {
            self.displayed = Verdict::Graded(observed);
            return self.displayed;
        };
        let direction = match observed.cmp(&displayed) {
            std::cmp::Ordering::Equal => {
                self.pending_since = None;
                return self.displayed;
            }
            std::cmp::Ordering::Greater => Direction::Worse,
            std::cmp::Ordering::Less => Direction::Better,
        };
        let since = match self.pending_since {
            Some((pending, since)) if pending == direction => since,
            _ => {
                self.pending_since = Some((direction, now));
                now
            }
        };
        let required = match direction {
            Direction::Worse => DEGRADE_AFTER,
            Direction::Better => IMPROVE_AFTER,
        };
        if now.compute_elapsed_since(since) >= required {
            self.displayed = Verdict::Graded(observed);
            self.pending_since = None;
        }
        self.displayed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_second(seconds: u64) -> Timestamp {
        Timestamp::from_elapsed(Duration::from_secs(seconds))
    }

    #[test]
    fn shows_the_first_grade_at_once() {
        // Arrange
        let mut hysteresis = Hysteresis::default();

        // Act
        let verdict = hysteresis.observe(Grade::Unstable, create_second(0));

        // Assert
        assert_eq!(verdict, Verdict::Graded(Grade::Unstable));
    }

    #[test]
    fn shows_a_worse_grade_after_ten_sustained_seconds() {
        // Arrange
        let mut hysteresis = Hysteresis::default();
        hysteresis.observe(Grade::Solid, create_second(0));
        hysteresis.observe(Grade::Unstable, create_second(1));

        // Act
        let early = hysteresis.observe(Grade::Degraded, create_second(10));
        let late = hysteresis.observe(Grade::Degraded, create_second(11));

        // Assert
        assert_eq!(early, Verdict::Graded(Grade::Solid));
        assert_eq!(
            late,
            Verdict::Graded(Grade::Degraded),
            "counted from the first worse second"
        );
    }

    #[test]
    fn shows_a_better_grade_after_thirty_seconds() {
        // Arrange
        let mut hysteresis = Hysteresis::default();
        hysteresis.observe(Grade::Degraded, create_second(0));
        hysteresis.observe(Grade::Solid, create_second(1));

        // Act
        let early = hysteresis.observe(Grade::Solid, create_second(30));
        let late = hysteresis.observe(Grade::Solid, create_second(31));

        // Assert
        assert_eq!(early, Verdict::Graded(Grade::Degraded));
        assert_eq!(late, Verdict::Graded(Grade::Solid));
    }

    #[test]
    fn starts_counting_again_when_the_grade_comes_back() {
        // Arrange
        let mut hysteresis = Hysteresis::default();
        hysteresis.observe(Grade::Solid, create_second(0));
        hysteresis.observe(Grade::Degraded, create_second(1));
        hysteresis.observe(Grade::Solid, create_second(5));

        // Act
        let verdict = hysteresis.observe(Grade::Degraded, create_second(12));

        // Assert
        assert_eq!(verdict, Verdict::Graded(Grade::Solid));
    }
}
