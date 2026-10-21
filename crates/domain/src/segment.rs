//! The two segments of the path: from the computer to its gateway (local), and from the gateway
//! to the Internet. The verdict comes from the Internet segment alone; the local one only tells
//! where a problem lies.

use std::collections::VecDeque;
use std::time::Duration;

use crate::{ConnectionKind, LatencyWindow, LossRate, WifiLink};

/// Above this, the link to the gateway is the problem: a box answers within a few milliseconds.
pub const LOCAL_LATENCY_LIMIT: Duration = Duration::from_millis(30);
pub const LOCAL_LOSS_LIMIT: LossRate = LossRate::from_percent(1.0);
/// Boxes drop the odd ping aimed at themselves when busy: one loss proves nothing.
pub const LOCAL_LOST_PINGS_MIN: u32 = 2;
/// One ping per second: loss is judged over a minute, and only after ten pings.
const LOSS_WINDOW_SECONDS: usize = 60;
const LOSS_MIN_PINGS: u32 = 10;

/// Why the local segment cannot be measured. There is no fallback: a TCP connection to a box
/// says nothing reliable about the Wi-Fi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unmeasurable {
    /// The system refuses unprivileged pings (Linux without `net.ipv4.ping_group_range`).
    NotPermitted,
    /// No gateway to ping: no default route, a VPN hiding it, or a cellular modem.
    NoGateway,
    /// The gateway answered no ping for a minute: some boxes are set up to ignore them.
    NoAnswer,
    /// Too few pings yet to judge the losses.
    TooFewPings,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocalSegment {
    Unmeasurable(Unmeasurable),
    Measured {
        /// p90 over 15 s; `None` when no ping was answered in that time.
        latency: Option<Duration>,
        /// `None` until enough pings were sent.
        loss: Option<LossRate>,
        /// Pings lost over the loss window.
        lost: u32,
    },
}

/// One ping of the gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayEcho {
    Answered(Duration),
    Lost,
    NotPermitted,
}

/// What the gateway answered over the last minute.
#[derive(Debug, Clone, Default)]
pub struct GatewayWindow {
    latency: LatencyWindow,
    /// One entry per second, newest last: whether the ping was answered.
    answers: VecDeque<bool>,
    lost: u32,
    is_refused: bool,
}

impl GatewayWindow {
    /// O(1).
    pub fn record_second(&mut self, echo: GatewayEcho) {
        let round_trip = match echo {
            GatewayEcho::Answered(round_trip) => Some(round_trip),
            GatewayEcho::Lost => None,
            GatewayEcho::NotPermitted => {
                self.is_refused = true;
                return;
            }
        };
        self.latency.record_second(round_trip);
        if self.answers.len() == LOSS_WINDOW_SECONDS && self.answers.pop_front() == Some(false) {
            self.lost -= 1;
        }
        self.answers.push_back(round_trip.is_some());
        self.lost += u32::from(round_trip.is_none());
    }

    /// O(n log n) for n ≤ 15 latency samples.
    #[must_use]
    pub fn compute_segment(&self) -> LocalSegment {
        if self.is_refused {
            return LocalSegment::Unmeasurable(Unmeasurable::NotPermitted);
        }
        // At most 60 entries: the conversion cannot fail.
        let sent = u32::try_from(self.answers.len()).unwrap_or(u32::MAX);
        // A broken link would cut the Internet too: a gateway silent for a minute ignores pings.
        if self.lost == sent {
            return LocalSegment::Unmeasurable(Unmeasurable::NoAnswer);
        }
        LocalSegment::Measured {
            latency: self.latency.compute_p90(),
            loss: (sent >= LOSS_MIN_PINGS)
                .then(|| LossRate::from_percent(f64::from(self.lost) * 100.0 / f64::from(sent))),
            lost: self.lost,
        }
    }

    /// Whether the gateway answered within the last `seconds`. O(seconds).
    #[must_use]
    pub fn has_answered_within(&self, seconds: usize) -> bool {
        self.answers
            .iter()
            .rev()
            .take(seconds)
            .any(|answered| *answered)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalProblem {
    /// The Wi-Fi signal is under −70 dBm: the most likely reason for the slow or lossy link.
    WeakSignal,
    Loss,
    Latency,
}

/// Where a problem seen on the Internet segment comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Between the computer and its gateway: Wi-Fi, cable, link with the phone.
    LocalLink(LocalProblem),
    /// Past the box, with a good local link.
    Operator,
    /// Past the phone or the modem, with a good local link.
    MobileNetwork,
    Undetermined(Unmeasurable),
}

impl Fault {
    /// Applies the attribution rules once the Internet segment is known to be bad. The local
    /// link is cleared only once its losses can be judged; a bad local link over a weak Wi-Fi
    /// signal is put down to the signal. `wifi` is the link carrying the traffic, if any. O(1).
    #[must_use]
    pub fn attribute(kind: ConnectionKind, local: LocalSegment, wifi: Option<&WifiLink>) -> Self {
        if kind == ConnectionKind::Cellular {
            return Self::MobileNetwork;
        }
        match Self::attribute_link(kind, local) {
            Self::LocalLink(_) if wifi.is_some_and(WifiLink::is_signal_weak) => {
                Self::LocalLink(LocalProblem::WeakSignal)
            }
            fault => fault,
        }
    }

    fn attribute_link(kind: ConnectionKind, local: LocalSegment) -> Self {
        match local {
            LocalSegment::Unmeasurable(reason) => Self::Undetermined(reason),
            LocalSegment::Measured {
                loss: Some(rate),
                lost,
                ..
            } if rate > LOCAL_LOSS_LIMIT && lost >= LOCAL_LOST_PINGS_MIN => {
                Self::LocalLink(LocalProblem::Loss)
            }
            LocalSegment::Measured {
                latency: Some(latency),
                ..
            } if latency > LOCAL_LATENCY_LIMIT => Self::LocalLink(LocalProblem::Latency),
            LocalSegment::Measured { loss: None, .. } => {
                Self::Undetermined(Unmeasurable::TooFewPings)
            }
            LocalSegment::Measured { .. } if kind == ConnectionKind::PhoneHotspot => {
                Self::MobileNetwork
            }
            LocalSegment::Measured { .. } => Self::Operator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_LOCAL: LocalSegment = LocalSegment::Measured {
        latency: Some(Duration::from_millis(4)),
        loss: Some(LossRate::from_percent(0.0)),
        lost: 0,
    };

    fn record(window: &mut GatewayWindow, echo: GatewayEcho, seconds: usize) {
        for _ in 0..seconds {
            window.record_second(echo);
        }
    }

    #[test]
    fn blames_the_operator_when_the_local_link_is_good() {
        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::WiFi, GOOD_LOCAL, None),
            Fault::Operator
        );
    }

    #[test]
    fn blames_the_mobile_network_behind_a_good_link_with_the_phone() {
        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::PhoneHotspot, GOOD_LOCAL, None),
            Fault::MobileNetwork
        );
    }

    #[test]
    fn blames_the_local_link_when_pings_are_lost_before_the_gateway() {
        // Arrange
        let lossy = LocalSegment::Measured {
            latency: Some(Duration::from_millis(4)),
            loss: Some(LossRate::from_percent(5.0)),
            lost: 3,
        };

        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::WiFi, lossy, None),
            Fault::LocalLink(LocalProblem::Loss)
        );
    }

    #[test]
    fn forgives_a_single_lost_ping() {
        // Arrange
        let mut window = GatewayWindow::default();
        record(
            &mut window,
            GatewayEcho::Answered(Duration::from_millis(4)),
            59,
        );
        window.record_second(GatewayEcho::Lost);

        // Act
        let fault = Fault::attribute(ConnectionKind::WiFi, window.compute_segment(), None);

        // Assert
        assert_eq!(fault, Fault::Operator);
    }

    #[test]
    fn blames_the_local_link_when_the_gateway_answers_slowly() {
        // Arrange
        let slow = LocalSegment::Measured {
            latency: Some(Duration::from_millis(45)),
            loss: Some(LossRate::from_percent(0.0)),
            lost: 0,
        };

        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::WiFi, slow, None),
            Fault::LocalLink(LocalProblem::Latency)
        );
    }

    #[test]
    fn waits_for_enough_pings_before_clearing_the_local_link() {
        // Arrange
        let mut window = GatewayWindow::default();
        record(
            &mut window,
            GatewayEcho::Answered(Duration::from_millis(4)),
            5,
        );

        // Act
        let fault = Fault::attribute(ConnectionKind::WiFi, window.compute_segment(), None);

        // Assert
        assert_eq!(fault, Fault::Undetermined(Unmeasurable::TooFewPings));
    }

    #[test]
    fn judges_a_very_lossy_link_on_its_losses_even_without_recent_answers() {
        // Arrange
        let mut window = GatewayWindow::default();
        window.record_second(GatewayEcho::Answered(Duration::from_millis(4)));
        record(&mut window, GatewayEcho::Lost, 20);

        // Act
        let fault = Fault::attribute(ConnectionKind::WiFi, window.compute_segment(), None);

        // Assert
        assert_eq!(fault, Fault::LocalLink(LocalProblem::Loss));
    }

    #[test]
    fn says_nothing_it_cannot_measure() {
        // Act & Assert
        assert_eq!(
            Fault::attribute(
                ConnectionKind::WiFi,
                LocalSegment::Unmeasurable(Unmeasurable::NotPermitted),
                None
            ),
            Fault::Undetermined(Unmeasurable::NotPermitted)
        );
    }

    #[test]
    fn blames_the_mobile_network_for_a_built_in_modem() {
        // Act & Assert
        assert_eq!(
            Fault::attribute(
                ConnectionKind::Cellular,
                LocalSegment::Unmeasurable(Unmeasurable::NoGateway),
                None
            ),
            Fault::MobileNetwork
        );
    }

    #[test]
    fn calls_a_gateway_silent_for_a_minute_unmeasurable() {
        // Arrange
        let mut window = GatewayWindow::default();

        // Act
        record(&mut window, GatewayEcho::Lost, LOSS_WINDOW_SECONDS);

        // Assert
        assert_eq!(
            window.compute_segment(),
            LocalSegment::Unmeasurable(Unmeasurable::NoAnswer)
        );
    }

    #[test]
    fn remembers_that_the_system_refuses_pings() {
        // Arrange
        let mut window = GatewayWindow::default();

        // Act
        window.record_second(GatewayEcho::NotPermitted);

        // Assert
        assert_eq!(
            window.compute_segment(),
            LocalSegment::Unmeasurable(Unmeasurable::NotPermitted)
        );
    }

    #[test]
    fn knows_whether_the_gateway_answered_lately() {
        // Arrange
        let mut window = GatewayWindow::default();
        window.record_second(GatewayEcho::Answered(Duration::from_millis(4)));
        record(&mut window, GatewayEcho::Lost, 5);

        // Act & Assert
        assert!(window.has_answered_within(6));
        assert!(!window.has_answered_within(5));
    }

    #[test]
    fn puts_a_bad_local_link_down_to_a_weak_wifi_signal() {
        // Arrange
        let slow = LocalSegment::Measured {
            latency: Some(Duration::from_millis(45)),
            loss: Some(LossRate::from_percent(0.0)),
            lost: 0,
        };
        let weak = WifiLink {
            name: None,
            signal_dbm: Some(-78),
            rate_mbps: None,
            channel: None,
            band: crate::WifiBand::Unknown,
        };

        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::WiFi, slow, Some(&weak)),
            Fault::LocalLink(LocalProblem::WeakSignal)
        );
        assert_eq!(
            Fault::attribute(ConnectionKind::WiFi, GOOD_LOCAL, Some(&weak)),
            Fault::Operator
        );
    }
}
