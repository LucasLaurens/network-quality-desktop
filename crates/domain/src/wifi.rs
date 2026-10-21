//! The Wi-Fi link to the box: radio values the system reports, and the network name when the
//! location permission lets the app read it.

/// Below this, the Wi-Fi signal is too weak for a steady video call.
pub const WEAK_SIGNAL_DBM: i32 = -70;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WifiBand {
    TwoPointFourGhz,
    FiveGhz,
    SixGhz,
    Unknown,
}

impl WifiBand {
    /// O(1).
    #[must_use]
    pub const fn from_frequency(megahertz: u32) -> Self {
        match megahertz {
            2400..=2500 => Self::TwoPointFourGhz,
            5150..=5895 => Self::FiveGhz,
            5925..=7125 => Self::SixGhz,
            _ => Self::Unknown,
        }
    }
}

/// The channel number of a centre frequency, as the IEEE 802.11 plans define it; 6 GHz
/// channel 2 sits below the start of its plan. O(1).
#[must_use]
pub const fn compute_channel(megahertz: u32) -> Option<u32> {
    let (start, megahertz) = match WifiBand::from_frequency(megahertz) {
        WifiBand::TwoPointFourGhz if megahertz == 2484 => return Some(14),
        WifiBand::SixGhz if megahertz == 5935 => return Some(2),
        WifiBand::TwoPointFourGhz => (2407, megahertz),
        WifiBand::FiveGhz => (5000, megahertz),
        WifiBand::SixGhz => (5950, megahertz),
        WifiBand::Unknown => return None,
    };
    match megahertz.checked_sub(start) {
        Some(offset) => Some(offset / 5),
        None => None,
    }
}

/// Systems report the link rate in kbit/s; the app shows Mbit/s. O(1).
#[must_use]
pub const fn convert_kilobits_to_megabits(kilobits: u32) -> u32 {
    kilobits / 1000
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiLink {
    /// `None` when the system hides it: no location permission, or a name it cannot decode.
    pub name: Option<String>,
    pub signal_dbm: Option<i32>,
    pub rate_mbps: Option<u32>,
    pub channel: Option<u32>,
    pub band: WifiBand,
}

impl WifiLink {
    #[must_use]
    pub fn is_signal_weak(&self) -> bool {
        self.signal_dbm
            .is_some_and(|signal| signal < WEAK_SIGNAL_DBM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_link(signal_dbm: Option<i32>) -> WifiLink {
        WifiLink {
            name: None,
            signal_dbm,
            rate_mbps: None,
            channel: None,
            band: WifiBand::Unknown,
        }
    }

    #[test]
    fn calls_a_signal_under_minus_70_dbm_weak() {
        // Act & Assert
        assert!(create_link(Some(-75)).is_signal_weak());
        assert!(!create_link(Some(WEAK_SIGNAL_DBM)).is_signal_weak());
        assert!(!create_link(None).is_signal_weak());
    }

    #[test]
    fn finds_the_band_and_channel_of_a_frequency() {
        // Act & Assert
        assert_eq!(WifiBand::from_frequency(2437), WifiBand::TwoPointFourGhz);
        assert_eq!(compute_channel(2437), Some(6));
        assert_eq!(compute_channel(2484), Some(14));
        assert_eq!(WifiBand::from_frequency(5180), WifiBand::FiveGhz);
        assert_eq!(compute_channel(5180), Some(36));
        assert_eq!(WifiBand::from_frequency(5955), WifiBand::SixGhz);
        assert_eq!(compute_channel(5955), Some(1));
        assert_eq!(compute_channel(5935), Some(2));
        assert_eq!(compute_channel(5925), None);
        assert_eq!(compute_channel(900), None);
    }

    #[test]
    fn shows_a_link_rate_in_whole_megabits() {
        // Act & Assert
        assert_eq!(convert_kilobits_to_megabits(866_700), 866);
    }
}
