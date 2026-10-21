//! When the state shown got durably worse: the only moment worth a notification.

use std::time::Duration;

use crate::{Grade, Timestamp, Verdict};

/// Offline shows after 5 s without any server; a cut is announced once it has lasted 10 s more.
const OFFLINE_NOTICE_AFTER: Duration = Duration::from_secs(10);

/// Follows the verdicts shown. A worse grade is already confirmed by the hysteresis (10 s); an
/// improvement, or the first verdict after « Mesure en cours », is never a degradation.
#[derive(Debug, Clone, Default)]
pub struct DegradationWatch {
    /// The last state measured, forgotten when measuring starts over.
    last_known: Option<Verdict>,
    /// When the cut began, until it is announced.
    offline_since: Option<Timestamp>,
}

impl DegradationWatch {
    /// Returns whether `shown` is a degradation to announce now. O(1).
    pub fn observe(&mut self, shown: Verdict, now: Timestamp) -> bool {
        match shown {
            Verdict::Measuring => {
                self.last_known = None;
                self.offline_since = None;
                false
            }
            Verdict::Offline => self.observe_offline(now),
            Verdict::Graded(_) => {
                self.offline_since = None;
                let is_worse = self
                    .last_known
                    .is_some_and(|previous| rank(shown) > rank(previous));
                self.last_known = Some(shown);
                is_worse
            }
        }
    }

    /// O(1).
    fn observe_offline(&mut self, now: Timestamp) -> bool {
        if !self
            .last_known
            .is_some_and(|previous| matches!(previous, Verdict::Graded(_)))
        {
            return false;
        }
        let since = *self.offline_since.get_or_insert(now);
        if now.compute_elapsed_since(since) < OFFLINE_NOTICE_AFTER {
            return false;
        }
        self.last_known = Some(Verdict::Offline);
        self.offline_since = None;
        true
    }
}

/// Worse is higher; « Mesure en cours » ranks nowhere. O(1).
const fn rank(verdict: Verdict) -> Option<u8> {
    match verdict {
        Verdict::Graded(Grade::Solid) => Some(0),
        Verdict::Graded(Grade::Unstable) => Some(1),
        Verdict::Graded(Grade::Degraded) => Some(2),
        Verdict::Offline => Some(3),
        Verdict::Measuring => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOLID: Verdict = Verdict::Graded(Grade::Solid);
    const UNSTABLE: Verdict = Verdict::Graded(Grade::Unstable);
    const DEGRADED: Verdict = Verdict::Graded(Grade::Degraded);

    fn at(seconds: u64) -> Timestamp {
        Timestamp::from_elapsed(Duration::from_secs(seconds))
    }

    /// Feeds one verdict per second from `start`, and returns what each answered.
    fn observe_all(watch: &mut DegradationWatch, start: u64, shown: &[Verdict]) -> Vec<bool> {
        (start..)
            .zip(shown)
            .map(|(second, verdict)| watch.observe(*verdict, at(second)))
            .collect()
    }

    #[test]
    fn announces_a_state_getting_worse_once() {
        // Arrange
        let mut watch = DegradationWatch::default();

        // Act
        let answers = observe_all(&mut watch, 0, &[SOLID, SOLID, UNSTABLE, UNSTABLE, DEGRADED]);

        // Assert
        assert_eq!(answers, [false, false, true, false, true]);
    }

    #[test]
    fn never_announces_an_improvement() {
        // Arrange
        let mut watch = DegradationWatch::default();

        // Act
        let answers = observe_all(&mut watch, 0, &[DEGRADED, UNSTABLE, SOLID]);

        // Assert
        assert_eq!(answers, [false, false, false]);
    }

    #[test]
    fn says_nothing_of_the_first_state_after_measuring() {
        // Arrange
        let mut watch = DegradationWatch::default();

        // Act
        let answers = observe_all(&mut watch, 0, &[SOLID, Verdict::Measuring, DEGRADED]);

        // Assert
        assert_eq!(answers, [false, false, false]);
    }

    #[test]
    fn announces_a_cut_once_it_has_lasted_ten_seconds() {
        // Arrange
        let mut watch = DegradationWatch::default();
        watch.observe(SOLID, at(0));
        let mut shown = vec![Verdict::Offline; 12];
        shown.push(Verdict::Offline);

        // Act
        let answers = observe_all(&mut watch, 1, &shown);

        // Assert
        let announced: Vec<usize> = answers
            .iter()
            .enumerate()
            .filter_map(|(second, answer)| answer.then_some(second))
            .collect();
        assert_eq!(announced, vec![10]);
    }

    #[test]
    fn says_nothing_of_a_short_cut() {
        // Arrange
        let mut watch = DegradationWatch::default();
        watch.observe(SOLID, at(0));

        // Act
        let answers = observe_all(&mut watch, 1, &[Verdict::Offline; 9]);

        // Assert
        assert!(answers.iter().all(|answer| !answer));
    }
}
