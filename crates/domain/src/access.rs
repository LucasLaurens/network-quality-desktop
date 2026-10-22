//! What carries the line beyond the box: ADSL or fibre. The box's own word when it says « DSL »
//! over `UPnP`; otherwise an estimate, only when the upload rate and the round trip agree.

use std::time::Duration;

use crate::Bitrate;

/// Fibre uploads tens of Mbit/s and adds a few milliseconds at most.
const FIBRE_UPLOAD_FROM: Bitrate = Bitrate::from_bits_per_second(20_000_000);
const FIBRE_ROUND_TRIP_UP_TO: Duration = Duration::from_millis(8);
/// ADSL uploads about 1 Mbit/s (3 with Annex M); its interleaving adds 10 to 40 ms.
const ADSL_UPLOAD_UP_TO: Bitrate = Bitrate::from_bits_per_second(2_000_000);
const ADSL_ROUND_TRIP_FROM: Duration = Duration::from_millis(10);
/// The upload signal is the median of the latest tests: one slow server does not decide.
pub const ACCESS_TESTS: usize = 5;
/// Fewer tests than this say nothing yet.
pub const ACCESS_MIN_TESTS: usize = 3;

/// `WANAccessType` of the `UPnP` `WANCommonInterfaceConfig` service. Only « DSL » proves anything:
/// a fibre box sees its link as Ethernet, as a cable box or a box behind another one does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WanAccessType {
    Dsl,
    Ethernet,
    Cable,
    Pots,
    Unknown,
}

impl WanAccessType {
    /// Vendor extensions (`X_AVM-DE_Fiber`) read as unknown. O(length).
    #[must_use]
    pub fn from_upnp(value: &str) -> Self {
        match value.trim() {
            "DSL" => Self::Dsl,
            "Ethernet" => Self::Ethernet,
            "Cable" => Self::Cable,
            "POTS" => Self::Pots,
            _ => Self::Unknown,
        }
    }
}

/// What the box says of its line over `UPnP`. The maximum rates are the box's sync rate, or a value
/// set in its configuration: shown as announced, never as measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxLine {
    pub access_type: WanAccessType,
    pub downstream_max: Option<Bitrate>,
    pub upstream_max: Option<Bitrate>,
    /// Bytes through the box's WAN link since it started, by every device behind it.
    pub total_received: Option<u64>,
    pub total_sent: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessEstimate {
    /// The box says « DSL ».
    ReportedDsl,
    EstimatedFibre,
    EstimatedAdsl,
    Unknown,
}

impl AccessEstimate {
    /// `uploads` are the latest throughput tests on this network, any order, `ACCESS_MIN_TESTS`
    /// at least; `round_trip` the latency measured at rest. Fibre and cable look alike, and VDSL falls in between: both
    /// stay unknown rather than guessed. O(u log u) for u uploads.
    #[must_use]
    pub fn estimate(
        line: Option<&BoxLine>,
        uploads: &[Bitrate],
        round_trip: Option<Duration>,
    ) -> Self {
        if line.is_some_and(|line| line.access_type == WanAccessType::Dsl) {
            return Self::ReportedDsl;
        }
        let Some(round_trip) = round_trip else {
            return Self::Unknown;
        };
        let Some(upload) = compute_median_rate(uploads) else {
            return Self::Unknown;
        };
        if upload >= FIBRE_UPLOAD_FROM && round_trip <= FIBRE_ROUND_TRIP_UP_TO {
            Self::EstimatedFibre
        } else if upload <= ADSL_UPLOAD_UP_TO && round_trip >= ADSL_ROUND_TRIP_FROM {
            Self::EstimatedAdsl
        } else {
            Self::Unknown
        }
    }
}

/// The middle rate, the mean of the two middles on an even count; `None` below
/// `ACCESS_MIN_TESTS` rates. O(u log u).
fn compute_median_rate(rates: &[Bitrate]) -> Option<Bitrate> {
    if rates.len() < ACCESS_MIN_TESTS {
        return None;
    }
    let mut sorted: Vec<u64> = rates.iter().map(|rate| rate.to_bits_per_second()).collect();
    sorted.sort_unstable();
    let upper = sorted[sorted.len() / 2];
    let lower = sorted[(sorted.len() - 1) / 2];
    Some(Bitrate::from_bits_per_second(lower.midpoint(upper)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_line(access_type: WanAccessType) -> BoxLine {
        BoxLine {
            access_type,
            downstream_max: None,
            upstream_max: None,
            total_received: None,
            total_sent: None,
        }
    }

    fn create_megabits(megabits: u64) -> Bitrate {
        Bitrate::from_bits_per_second(megabits * 1_000_000)
    }

    #[test]
    fn takes_the_box_at_its_word_only_for_dsl() {
        // Arrange
        let dsl = create_line(WanAccessType::from_upnp("DSL"));
        let ethernet = create_line(WanAccessType::from_upnp("Ethernet"));

        // Act
        let reported = AccessEstimate::estimate(Some(&dsl), &[], None);
        let unproven = AccessEstimate::estimate(Some(&ethernet), &[], None);

        // Assert
        assert_eq!(reported, AccessEstimate::ReportedDsl);
        assert_eq!(unproven, AccessEstimate::Unknown);
        assert_eq!(
            WanAccessType::from_upnp("X_AVM-DE_Fiber"),
            WanAccessType::Unknown
        );
    }

    #[test]
    fn estimates_only_when_upload_and_round_trip_agree() {
        // Arrange
        let fast = [
            create_megabits(300),
            create_megabits(2),
            create_megabits(250),
        ];
        let slow = [create_megabits(1), create_megabits(1), create_megabits(2)];

        // Act
        let fibre = AccessEstimate::estimate(None, &fast, Some(Duration::from_millis(4)));
        let adsl = AccessEstimate::estimate(None, &slow, Some(Duration::from_millis(28)));
        let fast_but_far = AccessEstimate::estimate(None, &fast, Some(Duration::from_millis(9)));
        let slow_but_near = AccessEstimate::estimate(None, &slow, Some(Duration::from_millis(9)));

        // Assert
        assert_eq!(fibre, AccessEstimate::EstimatedFibre);
        assert_eq!(adsl, AccessEstimate::EstimatedAdsl);
        assert_eq!(fast_but_far, AccessEstimate::Unknown);
        assert_eq!(slow_but_near, AccessEstimate::Unknown);
    }

    #[test]
    fn waits_for_three_tests_and_splits_an_even_count_evenly() {
        // Arrange
        let two_slow = [create_megabits(1), create_megabits(1)];
        let split = [
            create_megabits(1),
            create_megabits(1),
            create_megabits(300),
            create_megabits(300),
        ];
        let near = Some(Duration::from_millis(4));
        let far = Some(Duration::from_millis(30));

        // Act
        let too_few = AccessEstimate::estimate(None, &two_slow, far);
        let undecided_far = AccessEstimate::estimate(None, &split, far);
        let fibre_near = AccessEstimate::estimate(None, &split, near);

        // Assert
        assert_eq!(too_few, AccessEstimate::Unknown);
        assert_eq!(
            undecided_far,
            AccessEstimate::Unknown,
            "150 Mbit/s is no ADSL"
        );
        assert_eq!(fibre_near, AccessEstimate::EstimatedFibre);
    }

    #[test]
    fn needs_both_signals() {
        // Arrange
        let slow = [create_megabits(1), create_megabits(1), create_megabits(1)];

        // Act
        let without_test = AccessEstimate::estimate(None, &[], Some(Duration::from_millis(30)));
        let without_round_trip = AccessEstimate::estimate(None, &slow, None);

        // Assert
        assert_eq!(without_test, AccessEstimate::Unknown);
        assert_eq!(without_round_trip, AccessEstimate::Unknown);
    }
}
