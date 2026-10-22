//! When the state shown got durably worse: the only moment worth a notification.

use std::time::Duration;

use crate::{DEGRADE_AFTER, Grade, OFFLINE_AFTER_SECONDS, Timestamp, Verdict};

/// A cut is announced once it has lasted as long as a worse grade must: « Hors ligne » already
/// waited `OFFLINE_AFTER_SECONDS` before showing.
const OFFLINE_SHOWN_BEFORE_NOTICE: Duration =
    DEGRADE_AFTER.saturating_sub(Duration::from_secs(OFFLINE_AFTER_SECONDS as u64));

/// How bad a measured state is: the grades in their own order, then the cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
    Graded(Grade),
    Offline,
}

/// Follows the verdicts shown. A worse grade is already confirmed by the hysteresis; an
/// improvement, or the first grade after « Mesure en cours », is never a degradation. A cut long
/// enough is always one, unless it was announced already.
#[derive(Debug, Clone, Default)]
pub struct DegradationWatch {
    /// The last state measured, forgotten when measuring starts over.
    last_known: Option<Severity>,
    /// When the cut began to show, until it is announced.
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
            Verdict::Graded(grade) => {
                self.offline_since = None;
                let shown = Severity::Graded(grade);
                let is_worse = self.last_known.is_some_and(|previous| shown > previous);
                self.last_known = Some(shown);
                is_worse
            }
        }
    }

    /// O(1).
    fn observe_offline(&mut self, now: Timestamp) -> bool {
        if self.last_known == Some(Severity::Offline) {
            return false;
        }
        let since = *self.offline_since.get_or_insert(now);
        if now.compute_elapsed_since(since) < OFFLINE_SHOWN_BEFORE_NOTICE {
            return false;
        }
        self.last_known = Some(Severity::Offline);
        self.offline_since = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOLID: Verdict = Verdict::Graded(Grade::Solid);
    const UNSTABLE: Verdict = Verdict::Graded(Grade::Unstable);
    const DEGRADED: Verdict = Verdict::Graded(Grade::Degraded);

    fn to_timestamp(seconds: u64) -> Timestamp {
        Timestamp::from_elapsed(Duration::from_secs(seconds))
    }

    /// Feeds one verdict per second from `start`, and returns what each answered.
    fn observe_all(watch: &mut DegradationWatch, start: u64, shown: &[Verdict]) -> Vec<bool> {
        (start..)
            .zip(shown)
            .map(|(second, verdict)| watch.observe(*verdict, to_timestamp(second)))
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
        watch.observe(SOLID, to_timestamp(0));

        // Act
        // Offline shows after 5 s without any server: 5 s more make 10 s of cut.
        let answers = observe_all(&mut watch, 1, &[Verdict::Offline; 8]);

        // Assert
        let announced: Vec<usize> = answers
            .iter()
            .enumerate()
            .filter_map(|(second, answer)| answer.then_some(second))
            .collect();
        assert_eq!(announced, vec![5]);
    }

    #[test]
    fn announces_a_long_cut_even_right_after_measuring_started_over() {
        // Arrange
        let mut watch = DegradationWatch::default();
        watch.observe(Verdict::Measuring, to_timestamp(0));

        // Act
        let answers = observe_all(&mut watch, 1, &[Verdict::Offline; 6]);

        // Assert
        assert_eq!(answers.iter().filter(|answer| **answer).count(), 1);
    }

    #[test]
    fn says_nothing_of_a_short_cut() {
        // Arrange
        let mut watch = DegradationWatch::default();
        watch.observe(SOLID, to_timestamp(0));

        // Act
        let answers = observe_all(&mut watch, 1, &[Verdict::Offline; 5]);

        // Assert
        assert!(answers.iter().all(|answer| !answer));
    }
}
