//! The two segments of the path: from the computer to its gateway (local), and from the gateway
//! to the Internet. The verdict comes from the Internet segment alone; the local one only tells
//! where a problem lies.

use std::time::Duration;

use crate::{ConnectionKind, LatencyWindow, LossRate, LossWindow};

/// Above this, the link to the gateway is the problem: a box answers within a few milliseconds.
pub const LOCAL_LATENCY_LIMIT: Duration = Duration::from_millis(30);
pub const LOCAL_LOSS_LIMIT: LossRate = LossRate::from_percent(1.0);

/// Why the local segment cannot be measured. There is no fallback: a TCP connection to a box
/// says nothing reliable about the Wi-Fi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unmeasurable {
    /// The system refuses unprivileged pings (Linux without `net.ipv4.ping_group_range`).
    NotPermitted,
    /// No gateway to ping: no default route, a VPN hiding it, or a cellular modem.
    NoGateway,
    /// The gateway ignores pings: some boxes are set up that way.
    NoAnswer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocalSegment {
    Unmeasurable(Unmeasurable),
    Measured {
        /// p90 over the latency window.
        latency: Duration,
        /// `None` until the loss window is long enough to grade.
        loss: Option<LossRate>,
    },
}

/// One ping of the gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayEcho {
    Answered(Duration),
    Lost,
    NotPermitted,
}

/// What the gateway answered over the last seconds, with the same windows as the Internet
/// segment.
#[derive(Debug, Clone, Default)]
pub struct GatewayWindow {
    latency: LatencyWindow,
    loss: LossWindow,
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
        self.loss.record_second(1, u32::from(round_trip.is_none()));
    }

    /// A gateway that never answered in the window is ignoring pings, not a broken link: a
    /// broken link would cut the Internet too. O(n log n) for n ≤ 15 samples.
    #[must_use]
    pub fn compute_segment(&self) -> LocalSegment {
        if self.is_refused {
            return LocalSegment::Unmeasurable(Unmeasurable::NotPermitted);
        }
        self.latency.compute_p90().map_or(
            LocalSegment::Unmeasurable(Unmeasurable::NoAnswer),
            |latency| LocalSegment::Measured {
                latency,
                loss: self.loss.compute_rate(),
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalProblem {
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
    /// Applies the attribution rules once the Internet segment is known to be bad. O(1).
    #[must_use]
    pub fn attribute(kind: ConnectionKind, local: LocalSegment) -> Self {
        if kind == ConnectionKind::Cellular {
            return Self::MobileNetwork;
        }
        match local {
            LocalSegment::Unmeasurable(reason) => Self::Undetermined(reason),
            LocalSegment::Measured {
                loss: Some(loss), ..
            } if loss > LOCAL_LOSS_LIMIT => Self::LocalLink(LocalProblem::Loss),
            LocalSegment::Measured { latency, .. } if latency > LOCAL_LATENCY_LIMIT => {
                Self::LocalLink(LocalProblem::Latency)
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
        latency: Duration::from_millis(4),
        loss: Some(LossRate::from_percent(0.0)),
    };

    #[test]
    fn blames_the_operator_when_the_local_link_is_good() {
        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::WiFi, GOOD_LOCAL),
            Fault::Operator
        );
    }

    #[test]
    fn blames_the_mobile_network_behind_a_good_link_with_the_phone() {
        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::PhoneHotspot, GOOD_LOCAL),
            Fault::MobileNetwork
        );
    }

    #[test]
    fn blames_the_local_link_when_packets_are_lost_before_the_gateway() {
        // Arrange
        let lossy = LocalSegment::Measured {
            latency: Duration::from_millis(4),
            loss: Some(LossRate::from_percent(3.0)),
        };

        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::WiFi, lossy),
            Fault::LocalLink(LocalProblem::Loss)
        );
    }

    #[test]
    fn blames_the_local_link_when_the_gateway_answers_slowly() {
        // Arrange
        let slow = LocalSegment::Measured {
            latency: Duration::from_millis(45),
            loss: None,
        };

        // Act & Assert
        assert_eq!(
            Fault::attribute(ConnectionKind::WiFi, slow),
            Fault::LocalLink(LocalProblem::Latency)
        );
    }

    #[test]
    fn says_nothing_it_cannot_measure() {
        // Act & Assert
        assert_eq!(
            Fault::attribute(
                ConnectionKind::WiFi,
                LocalSegment::Unmeasurable(Unmeasurable::NotPermitted)
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
                LocalSegment::Unmeasurable(Unmeasurable::NoGateway)
            ),
            Fault::MobileNetwork
        );
    }

    #[test]
    fn measures_the_gateway_from_its_answers() {
        // Arrange
        let mut window = GatewayWindow::default();
        let round_trip = Duration::from_millis(3);

        // Act
        window.record_second(GatewayEcho::Answered(round_trip));
        window.record_second(GatewayEcho::Lost);

        // Assert
        assert_eq!(
            window.compute_segment(),
            LocalSegment::Measured {
                latency: round_trip,
                loss: None,
            }
        );
    }

    #[test]
    fn calls_a_silent_gateway_unmeasurable() {
        // Arrange
        let mut window = GatewayWindow::default();

        // Act
        window.record_second(GatewayEcho::Lost);

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
}
