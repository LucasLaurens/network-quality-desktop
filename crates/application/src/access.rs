//! What carries the line: the box asked first, then the estimate from this network's tests.

use std::time::Duration;

use domain::{ACCESS_TESTS, AccessEstimate, Bitrate, BoxLine};

use crate::{AccessTechnologyDetector, NetworkKey, ThroughputRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineReading {
    /// `None` when the box does not answer.
    pub line: Option<BoxLine>,
    pub access: AccessEstimate,
}

/// `tests` newest first, as the history lists them; `network` the current one, `None` before
/// any was seen: no test counts then. `round_trip` is the latency measured at rest.
/// O(t) for t tests, plus the box's answer time.
pub async fn detect_access(
    detector: &impl AccessTechnologyDetector,
    tests: &[ThroughputRecord],
    network: Option<&NetworkKey>,
    round_trip: Option<Duration>,
) -> LineReading {
    let line = detector.read_box_line().await;
    let uploads: Vec<Bitrate> = tests
        .iter()
        .filter(|test| network.is_some_and(|network| test.network.is_likely_same(network)))
        .filter_map(|test| test.result.upload)
        .take(ACCESS_TESTS)
        .collect();
    LineReading {
        line,
        access: AccessEstimate::estimate(line.as_ref(), &uploads, round_trip),
    }
}
