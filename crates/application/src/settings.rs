//! What may be saved: a new own server must answer first, or the dot would measure nothing.

use domain::{MeasurementServers, ProbeOutcome};

use crate::LatencyProbe;

/// The public servers, and a server already saved, need no proof; a new own server is probed
/// once, and an answer or a refused connection both show it is there. At most one probe timeout.
pub async fn can_measure_against(
    previous: MeasurementServers,
    next: MeasurementServers,
    probe: &impl LatencyProbe,
) -> bool {
    match next {
        MeasurementServers::Default => true,
        MeasurementServers::Custom(_) if next == previous => true,
        MeasurementServers::Custom(target) => {
            probe.send_probe(target).await != ProbeOutcome::Unreachable
        }
    }
}
