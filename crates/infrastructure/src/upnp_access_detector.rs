//! Asks the box about its line over `UPnP`: `WANCommonInterfaceConfig:1`, read only. The app never
//! calls an action that changes the box (no port mapping), and gives up after a few seconds.

use std::collections::HashMap;
use std::net::Ipv4Addr;
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
const LINK_PROPERTIES_ACTION: &str = "GetCommonLinkProperties";
const BYTES_RECEIVED_ACTION: &str = "GetTotalBytesReceived";
const BYTES_SENT_ACTION: &str = "GetTotalBytesSent";
const ACCESS_TYPE_ARGUMENT: &str = "NewWANAccessType";
const DOWNSTREAM_MAX_ARGUMENT: &str = "NewLayer1DownstreamMaxBitRate";
const UPSTREAM_MAX_ARGUMENT: &str = "NewLayer1UpstreamMaxBitRate";
const BYTES_RECEIVED_ARGUMENT: &str = "NewTotalBytesReceived";
const BYTES_SENT_ARGUMENT: &str = "NewTotalBytesSent";

#[derive(Debug, Clone, Copy)]
pub struct UpnpAccessDetector {
    /// The default gateway, when known: a mesh node or a second router may answer too.
    gateway: Option<Ipv4Addr>,
}

impl UpnpAccessDetector {
    #[must_use]
    pub const fn new(gateway: Option<Ipv4Addr>) -> Self {
        Self { gateway }
    }
}

impl AccessTechnologyDetector for UpnpAccessDetector {
    async fn read_box_line(&self) -> Option<BoxLine> {
        tokio::time::timeout(ANSWER_TIMEOUT, read_box_line_now(self.gateway))
            .await
            .ok()
            .flatten()
    }
}

/// The gateway's answer, or the first one when the gateway is unknown. O(d) for d devices
/// answering the search.
async fn read_box_line_now(gateway: Option<Ipv4Addr>) -> Option<BoxLine> {
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
        let is_gateway =
            gateway.is_none_or(|gateway| device.url().host() == Some(gateway.to_string().as_str()));
        if !is_gateway {
            continue;
        }
        let Some(service) = device.find_service(&WAN_COMMON_INTERFACE) else {
            continue;
        };
        let Ok(properties) = service
            .action(device.url(), LINK_PROPERTIES_ACTION, "")
            .await
        else {
            continue;
        };
        let received = service
            .action(device.url(), BYTES_RECEIVED_ACTION, "")
            .await
            .ok();
        let sent = service
            .action(device.url(), BYTES_SENT_ACTION, "")
            .await
            .ok();
        return Some(to_box_line(
            &properties,
            received
                .as_ref()
                .and_then(|answer| read_count(answer, BYTES_RECEIVED_ARGUMENT)),
            sent.as_ref()
                .and_then(|answer| read_count(answer, BYTES_SENT_ARGUMENT)),
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
            .get(ACCESS_TYPE_ARGUMENT)
            .map_or(WanAccessType::Other, |value| {
                WanAccessType::from_upnp(value)
            }),
        downstream_max: read_rate(DOWNSTREAM_MAX_ARGUMENT),
        upstream_max: read_rate(UPSTREAM_MAX_ARGUMENT),
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
    use application::AccessTechnologyDetector;

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
    async fn asks_the_box_about_its_line_within_the_time_allowed() {
        // Arrange
        let detector = UpnpAccessDetector::new(None);
        let started = std::time::Instant::now();

        // Act
        let line = detector.read_box_line().await;

        // Assert
        println!("{line:?}");
        assert!(started.elapsed() < ANSWER_TIMEOUT + Duration::from_secs(1));
        if let Some(line) = line {
            assert!(
                line.downstream_max
                    .is_none_or(|rate| rate.to_bits_per_second() > 0)
            );
        }
    }
}
