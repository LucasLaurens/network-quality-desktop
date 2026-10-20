use std::time::Duration;

use crate::{Grade, Timestamp, Verdict};

const DEGRADE_AFTER: Duration = Duration::from_secs(10);
const IMPROVE_AFTER: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Worse,
    Better,
}

/// Keeps the dot steady: a worse grade shows after 10 sustained seconds, a better one after 30.
#[derive(Debug, Clone, Default)]
pub struct Hysteresis {
    displayed: Option<Grade>,
    pending_since: Option<(Direction, Timestamp)>,
}

impl Hysteresis {
    /// Feeds the grade observed now and returns the verdict to display. O(1) time and space.
    pub fn observe(&mut self, observed: Grade, now: Timestamp) -> Verdict {
        let Some(displayed) = self.displayed else {
            self.displayed = Some(observed);
            return Verdict::Graded(observed);
        };
        let direction = match observed.cmp(&displayed) {
            std::cmp::Ordering::Equal => {
                self.pending_since = None;
                return Verdict::Graded(displayed);
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
            self.displayed = Some(observed);
            self.pending_since = None;
            return Verdict::Graded(observed);
        }
        Verdict::Graded(displayed)
    }

    /// The verdict shown before any grade could be observed.
    #[must_use]
    pub fn read_displayed(&self) -> Verdict {
        self.displayed.map_or(Verdict::Measuring, Verdict::Graded)
    }
}
