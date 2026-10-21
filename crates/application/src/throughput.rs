//! A throughput test, started by the user only: each direction climbs the profile's sizes until a
//! transfer takes long enough to measure the line, within the direction's time budget.

use std::time::Duration;

use domain::{Direction, ENOUGH_TRANSFER_TIME, TestProfile, ThroughputResult, Transfer};

use crate::ThroughputProbe;

/// Why a test gave no result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThroughputFailure {
    /// The test server did not answer, or broke off a transfer.
    Unreachable,
}

/// Download, then upload: the two never share the line. O(s) transfers for s steps.
///
/// # Errors
///
/// A transfer failed: a partial test would measure nothing reliable.
pub async fn run_throughput_test(
    probe: &impl ThroughputProbe,
    profile: TestProfile,
) -> Result<ThroughputResult, ThroughputFailure> {
    let downloads = measure_direction(probe, profile, Direction::Download).await?;
    let uploads = measure_direction(probe, profile, Direction::Upload).await?;
    Ok(ThroughputResult::compose(profile, &downloads, &uploads))
}

/// Each size runs its count; a size with a transfer longer than `ENOUGH_TRANSFER_TIME` is the
/// last one. No transfer starts once the budget is spent. O(s).
async fn measure_direction(
    probe: &impl ThroughputProbe,
    profile: TestProfile,
    direction: Direction,
) -> Result<Vec<Transfer>, ThroughputFailure> {
    let budget = profile.to_direction_budget();
    let mut spent = Duration::ZERO;
    let mut transfers = Vec::new();
    for step in profile.list_steps(direction) {
        let mut is_long_enough = false;
        for _ in 0..step.count {
            if spent >= budget {
                return Ok(transfers);
            }
            let transfer = probe.transfer(direction, step.bytes).await?;
            spent += transfer.elapsed;
            is_long_enough |= transfer.elapsed > ENOUGH_TRANSFER_TIME;
            transfers.push(transfer);
        }
        if is_long_enough {
            break;
        }
    }
    Ok(transfers)
}
