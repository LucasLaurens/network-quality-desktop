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
