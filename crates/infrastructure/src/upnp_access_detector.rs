//! Asks the box about its line over `UPnP`: `WANCommonInterfaceConfig:1`, read only. The app never
//! calls an action that changes the box (no port mapping), and gives up after a few seconds.

use std::collections::HashMap;
use std::pin::pin;
use std::time::Duration;

use application::AccessTechnologyDetector;
use domain::{Bitrate, BoxLine, WanAccessType};
use futures_util::StreamExt;
use rupnp::ssdp::{SearchTarget, URN};

const WAN_COMMON_INTERFACE: URN = URN::service("schemas-upnp-org", "WANCommonInterfaceConfig", 1);
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(2);
/// Discovery included: a box that does not answer must not hold the details panel.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Copy, Default)]
pub struct UpnpAccessDetector;

impl AccessTechnologyDetector for UpnpAccessDetector {
    async fn read_box_line(&self) -> Option<BoxLine> {
        tokio::time::timeout(ANSWER_TIMEOUT, read_box_line_now())
            .await
            .ok()
            .flatten()
    }
}

/// The first box offering the service that answers. O(d) for d devices answering the search.
async fn read_box_line_now() -> Option<BoxLine> {
    let target = SearchTarget::URN(WAN_COMMON_INTERFACE);
    let devices = match rupnp::discover(&target, DISCOVERY_TIMEOUT, None).await {
        Ok(devices) => devices,
        Err(error) => {
            eprintln!("Searching the box over UPnP failed: {error}");
            return None;
        }
    };
    let mut devices = pin!(devices);
    while let Some(device) = devices.next().await {
        let Ok(device) = device else {
            continue;
        };
        let Some(service) = device.find_service(&WAN_COMMON_INTERFACE) else {
            continue;
        };
        let Ok(properties) = service
            .action(device.url(), "GetCommonLinkProperties", "")
            .await
        else {
            continue;
        };
        let received = service
            .action(device.url(), "GetTotalBytesReceived", "")
            .await
            .ok();
        let sent = service
            .action(device.url(), "GetTotalBytesSent", "")
            .await
            .ok();
        return Some(to_box_line(
            &properties,
            received
                .as_ref()
                .and_then(|answer| read_count(answer, "NewTotalBytesReceived")),
            sent.as_ref()
                .and_then(|answer| read_count(answer, "NewTotalBytesSent")),
        ));
    }
    None
}

/// A rate of zero means the box does not know it. O(1).
fn to_box_line(
    properties: &HashMap<String, String>,
    total_received: Option<u64>,
    total_sent: Option<u64>,
) -> BoxLine {
    let read_rate = |name: &str| {
        read_count(properties, name)
            .filter(|bits| *bits > 0)
            .map(Bitrate::from_bits_per_second)
    };
    BoxLine {
        access_type: properties
            .get("NewWANAccessType")
            .map_or(WanAccessType::Unknown, |value| {
                WanAccessType::from_upnp(value)
            }),
        downstream_max: read_rate("NewLayer1DownstreamMaxBitRate"),
        upstream_max: read_rate("NewLayer1UpstreamMaxBitRate"),
        total_received,
        total_sent,
    }
}

/// O(length).
fn read_count(answer: &HashMap<String, String>, name: &str) -> Option<u64> {
    answer.get(name)?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use application::{AccessTechnologyDetector, LineReading, detect_access};

    use super::*;

    fn create_answer(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn reads_the_line_from_the_box_answer() {
        // Arrange
        let properties = create_answer(&[
            ("NewWANAccessType", "DSL"),
            ("NewLayer1DownstreamMaxBitRate", "17408000"),
            ("NewLayer1UpstreamMaxBitRate", "0"),
            ("NewPhysicalLinkStatus", "Up"),
        ]);

        // Act
        let line = to_box_line(&properties, Some(9_000), None);

        // Assert
        assert_eq!(
            line,
            BoxLine {
                access_type: WanAccessType::Dsl,
                downstream_max: Some(Bitrate::from_bits_per_second(17_408_000)),
                upstream_max: None,
                total_received: Some(9_000),
                total_sent: None,
            }
        );
    }

    // Asks the real box: `cargo test -p infrastructure -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "asks the box of the network the computer is on"]
    async fn asks_the_box_about_its_line() {
        // Arrange
        let detector = UpnpAccessDetector;

        // Act
        let line = detector.read_box_line().await;
        let LineReading { access, .. } = detect_access(&detector, &[], None, None).await;

        // Assert
        println!("{line:?} {access:?}");
    }
}
