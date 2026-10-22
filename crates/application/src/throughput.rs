//! A throughput test, started by the user only: each direction climbs the profile's sizes until a
//! transfer takes long enough to measure the line, within the direction's time budget.

use std::time::Duration;

use domain::{Direction, ENOUGH_TRANSFER_TIME, TestProfile, ThroughputResult, Transfer};

use crate::ThroughputProbe;

const BITS_PER_BYTE: u64 = 8;
const MICROS_PER_SECOND: u64 = 1_000_000;

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
/// last one. No transfer starts that would end past the budget at the rate of the last large
/// transfer; until one is timed, the next size always runs: a warm-up's small transfer measures
/// the request more than the line. O(s).
async fn measure_direction(
    probe: &impl ThroughputProbe,
    profile: TestProfile,
    direction: Direction,
) -> Result<Vec<Transfer>, ThroughputFailure> {
    let budget = profile.to_direction_budget();
    let mut spent = Duration::ZERO;
    let mut transfers: Vec<Transfer> = Vec::new();
    for step in profile.list_steps(direction) {
        let mut is_long_enough = false;
        for _ in 0..step.count {
            let expected = transfers
                .iter()
                .rev()
                .find(|transfer| transfer.is_large_enough_to_rate())
                .map_or(Duration::ZERO, |large| predict_elapsed(large, step.bytes));
            if spent + expected > budget {
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

/// How long `bytes` would take at `last`'s rate, in whole microseconds. O(1).
fn predict_elapsed(last: &Transfer, bytes: u64) -> Duration {
    last.compute_rate()
        .filter(|rate| rate.to_bits_per_second() > 0)
        .map_or(Duration::ZERO, |rate| {
            Duration::from_micros(
                bytes.saturating_mul(BITS_PER_BYTE * MICROS_PER_SECOND) / rate.to_bits_per_second(),
            )
        })
}
