//! Runs the throughput tests the user starts: one at a time, against Cloudflare.

use std::sync::atomic::{AtomicBool, Ordering};

use application::{ThroughputFailure, run_throughput_test};
use domain::{TestProfile, ThroughputResult};
use infrastructure::CloudflareThroughputProbe;

/// Why a test did not run to its end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabRefusal {
    /// Another test is running: never two at once.
    Busy,
    /// No HTTP client, or the test server did not answer.
    Unavailable,
}

pub struct ThroughputLab {
    /// `None` when no HTTP client could be built: every test is then unavailable.
    probe: Option<CloudflareThroughputProbe>,
    is_running: AtomicBool,
}

impl ThroughputLab {
    #[must_use]
    pub fn create() -> Self {
        let probe = CloudflareThroughputProbe::create()
            .map_err(|error| eprintln!("The throughput tests are unavailable: {error}"))
            .ok();
        Self {
            probe,
            is_running: AtomicBool::new(false),
        }
    }

    /// Takes the lab for one test: the caller tells the engine before the line fills.
    ///
    /// # Errors
    ///
    /// A test is already running, or no HTTP client could be built.
    pub fn begin(&self) -> Result<RunningTest<'_>, LabRefusal> {
        let probe = self.probe.as_ref().ok_or(LabRefusal::Unavailable)?;
        let guard = RunningGuard::start(&self.is_running).ok_or(LabRefusal::Busy)?;
        Ok(RunningTest {
            probe,
            _guard: guard,
        })
    }
}

/// The lab, held for one test until it is dropped.
pub struct RunningTest<'a> {
    probe: &'a CloudflareThroughputProbe,
    _guard: RunningGuard<'a>,
}

impl RunningTest<'_> {
    /// # Errors
    ///
    /// The test server did not answer, or broke off a transfer.
    pub async fn run(&self, profile: TestProfile) -> Result<ThroughputResult, LabRefusal> {
        run_throughput_test(self.probe, profile)
            .await
            .map_err(|ThroughputFailure::Unreachable| LabRefusal::Unavailable)
    }
}

/// Marks a test as running for as long as it lives, even when it ends early.
struct RunningGuard<'a>(&'a AtomicBool);

impl<'a> RunningGuard<'a> {
    fn start(is_running: &'a AtomicBool) -> Option<Self> {
        is_running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self(is_running))
    }
}

impl Drop for RunningGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lets_one_test_run_at_a_time() {
        // Arrange
        let is_running = AtomicBool::new(false);
        let first = RunningGuard::start(&is_running);

        // Act
        let second = RunningGuard::start(&is_running);

        // Assert
        assert!(first.is_some());
        assert!(second.is_none());
    }

    #[test]
    fn frees_the_lab_once_a_test_ends() {
        // Arrange
        let is_running = AtomicBool::new(false);
        drop(RunningGuard::start(&is_running));

        // Act
        let next = RunningGuard::start(&is_running);

        // Assert
        assert!(next.is_some());
    }
}
