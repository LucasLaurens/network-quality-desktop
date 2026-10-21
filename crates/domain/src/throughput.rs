//! Throughput tests: what each profile transfers, how a test is summed up, and how its result
//! grades the connection.

use std::time::Duration;

use crate::{Grade, compute_median};

const KILOBYTE: u64 = 1000;
const MEGABYTE: u64 = 1000 * KILOBYTE;
const BITS_PER_BYTE: u64 = 8;
/// How long a slow test holds the verdict down, unless another network takes over first.
pub const THROUGHPUT_CAP_DURATION: Duration = Duration::from_mins(10);
/// A size whose transfer lasts longer than this already measures the line well: no bigger one.
pub const ENOUGH_TRANSFER_TIME: Duration = Duration::from_secs(1);
/// Below this size, a transfer mostly measures the request's own round trip.
const RATE_MIN_BYTES: u64 = MEGABYTE;
/// Download and upload grade bounds, in bits per second, for video calls.
const DOWNLOAD_SOLID_FROM: u64 = 2_500_000;
const UPLOAD_SOLID_FROM: u64 = 1_500_000;
const DEGRADED_BELOW: u64 = 600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Download,
    Upload,
}

/// One size, repeated `count` times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub bytes: u64,
    pub count: u32,
}

const QUICK_DOWNLOAD: &[Step] = &[
    Step {
        bytes: 100 * KILOBYTE,
        count: 1,
    },
    Step {
        bytes: MEGABYTE,
        count: 3,
    },
];
const QUICK_UPLOAD: &[Step] = &[
    Step {
        bytes: 100 * KILOBYTE,
        count: 1,
    },
    Step {
        bytes: MEGABYTE / 2,
        count: 2,
    },
];
const FULL_DOWNLOAD: &[Step] = &[
    Step {
        bytes: 100 * KILOBYTE,
        count: 4,
    },
    Step {
        bytes: MEGABYTE,
        count: 6,
    },
    Step {
        bytes: 10 * MEGABYTE,
        count: 4,
    },
    Step {
        bytes: 25 * MEGABYTE,
        count: 2,
    },
];
const FULL_UPLOAD: &[Step] = &[
    Step {
        bytes: 100 * KILOBYTE,
        count: 4,
    },
    Step {
        bytes: MEGABYTE,
        count: 4,
    },
    Step {
        bytes: 10 * MEGABYTE,
        count: 3,
    },
    Step {
        bytes: 25 * MEGABYTE,
        count: 1,
    },
];

/// The two tests a user can start; there is no automatic one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestProfile {
    /// About 3 s and 5 MB.
    Quick,
    /// Sizes up to 25 MB, 15 s at most per direction.
    Full,
}

impl TestProfile {
    /// Smallest first: a test climbs until a size takes long enough. O(1).
    #[must_use]
    pub const fn list_steps(self, direction: Direction) -> &'static [Step] {
        match (self, direction) {
            (Self::Quick, Direction::Download) => QUICK_DOWNLOAD,
            (Self::Quick, Direction::Upload) => QUICK_UPLOAD,
            (Self::Full, Direction::Download) => FULL_DOWNLOAD,
            (Self::Full, Direction::Upload) => FULL_UPLOAD,
        }
    }

    /// The time one direction may take; the transfer in progress always ends. O(1).
    #[must_use]
    pub const fn to_direction_budget(self) -> Duration {
        match self {
            Self::Quick => Duration::from_millis(1500),
            Self::Full => Duration::from_secs(15),
        }
    }

    /// The most a test can transfer, both directions together. O(s) for s steps.
    #[must_use]
    pub fn compute_max_bytes(self) -> u64 {
        [Direction::Download, Direction::Upload]
            .into_iter()
            .flat_map(|direction| self.list_steps(direction))
            .map(|step| step.bytes * u64::from(step.count))
            .sum()
    }
}

/// One transfer as the probe timed it, with the round trips measured while it ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    pub bytes: u64,
    pub elapsed: Duration,
    pub loaded_round_trips: Vec<Duration>,
}

impl Transfer {
    /// Large enough for its rate to measure the line rather than the request. O(1).
    #[must_use]
    pub const fn is_large_enough_to_rate(&self) -> bool {
        self.bytes >= RATE_MIN_BYTES
    }

    /// `None` for a transfer too short to time. O(1).
    #[must_use]
    pub fn compute_rate(&self) -> Option<Bitrate> {
        let micros = u64::try_from(self.elapsed.as_micros()).ok()?;
        (micros > 0).then(|| Bitrate(self.bytes * BITS_PER_BYTE * 1_000_000 / micros))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bitrate(u64);

impl Bitrate {
    #[must_use]
    pub const fn from_bits_per_second(bits: u64) -> Self {
        Self(bits)
    }

    #[must_use]
    pub const fn to_bits_per_second(self) -> u64 {
        self.0
    }
}

/// What a test measured. A direction without any timed transfer has no rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThroughputResult {
    pub profile: TestProfile,
    pub download: Option<Bitrate>,
    pub upload: Option<Bitrate>,
    /// Round trip while the line was busy: shown, never graded.
    pub loaded_latency: Option<Duration>,
    pub bytes: u64,
    pub duration: Duration,
}

impl ThroughputResult {
    /// Sums up the transfers of both directions: the 90th percentile of each direction's rates
    /// (the transfers of 1 MB or more when there are some), and the median loaded round trip.
    /// O(n log n) for n transfers.
    #[must_use]
    pub fn compose(profile: TestProfile, downloads: &[Transfer], uploads: &[Transfer]) -> Self {
        let transfers = || downloads.iter().chain(uploads);
        let round_trips: Vec<Option<Duration>> = transfers()
            .flat_map(|transfer| transfer.loaded_round_trips.iter().copied().map(Some))
            .collect();
        Self {
            profile,
            download: compute_rate_p90(downloads),
            upload: compute_rate_p90(uploads),
            loaded_latency: compute_median(&round_trips),
            bytes: transfers().map(|transfer| transfer.bytes).sum(),
            duration: transfers().map(|transfer| transfer.elapsed).sum(),
        }
    }

    /// The worse of the two directions; `None` when neither was measured. O(1).
    #[must_use]
    pub fn compute_grade(&self) -> Option<Grade> {
        let download = self
            .download
            .map(|rate| grade_rate(rate, DOWNLOAD_SOLID_FROM));
        let upload = self.upload.map(|rate| grade_rate(rate, UPLOAD_SOLID_FROM));
        download.into_iter().chain(upload).max()
    }
}

fn grade_rate(rate: Bitrate, solid_from: u64) -> Grade {
    if rate.0 >= solid_from {
        Grade::Solid
    } else if rate.0 >= DEGRADED_BELOW {
        Grade::Unstable
    } else {
        Grade::Degraded
    }
}

/// Nearest rank, as for latency. O(n log n).
fn compute_rate_p90(transfers: &[Transfer]) -> Option<Bitrate> {
    let has_large = transfers.iter().any(Transfer::is_large_enough_to_rate);
    let mut rates: Vec<Bitrate> = transfers
        .iter()
        .filter(|transfer| !has_large || transfer.is_large_enough_to_rate())
        .filter_map(Transfer::compute_rate)
        .collect();
    if rates.is_empty() {
        return None;
    }
    rates.sort_unstable();
    let rank = (rates.len() * 9).div_ceil(10).max(1);
    rates.get(rank - 1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_transfer(bytes: u64, millis: u64, round_trips_ms: &[u64]) -> Transfer {
        Transfer {
            bytes,
            elapsed: Duration::from_millis(millis),
            loaded_round_trips: round_trips_ms
                .iter()
                .copied()
                .map(Duration::from_millis)
                .collect(),
        }
    }

    #[test]
    fn keeps_the_quick_test_near_five_megabytes_and_the_full_one_under_160() {
        // Act & Assert
        assert!((4 * MEGABYTE..=6 * MEGABYTE).contains(&TestProfile::Quick.compute_max_bytes()));
        assert!(TestProfile::Full.compute_max_bytes() <= 160 * MEGABYTE);
    }

    #[test]
    fn rates_a_transfer_from_its_size_and_time() {
        // Act
        let rate = create_transfer(MEGABYTE, 800, &[]).compute_rate();

        // Assert
        assert_eq!(rate, Some(Bitrate::from_bits_per_second(10_000_000)));
    }

    #[test]
    fn sums_up_a_test_from_its_large_transfers_and_loaded_round_trips() {
        // Arrange
        let downloads = [
            create_transfer(100 * KILOBYTE, 400, &[]),
            create_transfer(MEGABYTE, 800, &[40, 60]),
            create_transfer(MEGABYTE, 400, &[50]),
        ];
        let uploads = [create_transfer(MEGABYTE / 2, 2000, &[90])];

        // Act
        let result = ThroughputResult::compose(TestProfile::Quick, &downloads, &uploads);

        // Assert
        assert_eq!(
            result.download,
            Some(Bitrate::from_bits_per_second(20_000_000))
        );
        assert_eq!(
            result.upload,
            Some(Bitrate::from_bits_per_second(2_000_000))
        );
        assert_eq!(result.loaded_latency, Some(Duration::from_millis(55)));
        assert_eq!(result.bytes, 2_600_000);
        assert_eq!(result.duration, Duration::from_millis(3600));
    }

    #[test]
    fn grades_a_test_by_its_worse_direction() {
        // Arrange
        let result = |download: u64, upload: u64| ThroughputResult {
            profile: TestProfile::Quick,
            download: Some(Bitrate::from_bits_per_second(download)),
            upload: Some(Bitrate::from_bits_per_second(upload)),
            loaded_latency: None,
            bytes: 0,
            duration: Duration::ZERO,
        };

        // Act & Assert
        assert_eq!(
            result(50_000_000, 10_000_000).compute_grade(),
            Some(Grade::Solid)
        );
        assert_eq!(
            result(50_000_000, 1_000_000).compute_grade(),
            Some(Grade::Unstable)
        );
        assert_eq!(
            result(500_000, 10_000_000).compute_grade(),
            Some(Grade::Degraded)
        );
    }
}
