use std::time::Duration;

use application::LatencyProbe;
use domain::Target;
use ping_async::{IcmpEchoRequestor, IcmpEchoStatus};

/// A reply slower than this counts as lost, so a probe never outlives its one-second tick.
const PROBE_TIMEOUT: Duration = Duration::from_millis(900);

/// Unprivileged ICMP echo: DGRAM sockets on macOS and Linux, `IcmpSendEcho2Ex` on Windows.
/// On Linux it needs `net.ipv4.ping_group_range` to include the user, otherwise every probe is lost.
#[derive(Debug, Clone, Copy, Default)]
pub struct IcmpLatencyProbe;

impl LatencyProbe for IcmpLatencyProbe {
    async fn measure_round_trip(&self, target: Target) -> Option<Duration> {
        // shortcut: one requestor (socket) per probe, two per second; cache them per target if the cost shows.
        let requestor =
            IcmpEchoRequestor::new(target.address(), None, None, Some(PROBE_TIMEOUT)).ok()?;
        let reply = requestor.send().await.ok()?;
        (reply.status() == IcmpEchoStatus::Success).then(|| reply.round_trip_time())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Needs network access and ICMP rights, so it only runs on demand: `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore = "real network"]
    async fn measures_a_round_trip_to_a_public_resolver() {
        // Arrange
        let probe = IcmpLatencyProbe;
        let target = application::list_default_targets()[0];

        // Act
        let round_trip = probe.measure_round_trip(target).await;

        // Assert
        assert!(round_trip.is_some_and(|rtt| rtt < PROBE_TIMEOUT));
    }
}
