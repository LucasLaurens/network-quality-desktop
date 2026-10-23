//! Asks the box about its line over `UPnP`: `WANCommonInterfaceConfig:1`, read only. The app never
//! calls an action that changes the box (no port mapping), and gives up after a few seconds.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::pin::pin;
use std::time::Duration;

use application::AccessTechnologyDetector;
use domain::{Bitrate, BoxLine, WanAccessType};
use futures_util::StreamExt;
use rupnp::Device;
use rupnp::http::Uri;
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

/// The gateway's answer. Only the description the gateway announces is fetched: any device on the
/// network may answer the search, and following its address would let it make the app download
/// without limit, or reach any other host. Without a known gateway, nothing tells the box apart,
/// so nothing is asked. O(r) for r answers to the search.
async fn read_box_line_now(gateway: Option<Ipv4Addr>) -> Option<BoxLine> {
    let gateway = gateway?;
    let target = SearchTarget::URN(WAN_COMMON_INTERFACE);
    let responses = match rupnp::ssdp::search(&target, DISCOVERY_TIMEOUT, 3, None).await {
        Ok(responses) => responses,
        Err(error) => {
            eprintln!("Searching the box over UPnP failed: {error}");
            return None;
        }
    };
    let mut responses = pin!(responses);
    while let Some(response) = responses.next().await {
        let Some(url) = response
            .ok()
            .and_then(|response| parse_gateway_location(response.location(), gateway))
        else {
            continue;
        };
        let Ok(device) = Device::from_url(url).await else {
            continue;
        };
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

/// The description address, when the gateway announced it over plain HTTP. O(length).
fn parse_gateway_location(location: &str, gateway: Ipv4Addr) -> Option<Uri> {
    let url: Uri = location.parse().ok()?;
    let is_gateway =
        url.scheme_str() == Some("http") && url.host() == Some(gateway.to_string().as_str());
    is_gateway.then_some(url)
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
    use application::test_support::generate_address;
    use application::{AccessTechnologyDetector, ConnectionContextReader};

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

    #[test]
    fn follows_only_what_the_gateway_announces() {
        // Arrange
        let gateway = generate_address();
        let other = Ipv4Addr::from(u32::from(gateway).wrapping_add(1));

        // Act
        let from_gateway =
            parse_gateway_location(&format!("http://{gateway}:49000/desc.xml"), gateway);
        let from_elsewhere =
            parse_gateway_location(&format!("http://{other}:49000/desc.xml"), gateway);
        let elsewhere_on_the_box =
            parse_gateway_location(&format!("http://{gateway}.example/desc.xml"), gateway);
        let not_http = parse_gateway_location(&format!("file://{gateway}/etc/passwd"), gateway);
        let garbled = parse_gateway_location("not a url", gateway);

        // Assert
        assert!(from_gateway.is_some());
        assert!(from_elsewhere.is_none());
        assert!(elsewhere_on_the_box.is_none());
        assert!(not_http.is_none());
        assert!(garbled.is_none());
    }

    #[tokio::test]
    async fn asks_nothing_when_the_gateway_is_unknown() {
        // Act
        let line = UpnpAccessDetector::new(None).read_box_line().await;

        // Assert
        assert_eq!(line, None, "any device could answer for the box");
    }

    // Asks the real box: `cargo test -p infrastructure -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "asks the box of the network the computer is on"]
    async fn asks_the_box_about_its_line_within_the_time_allowed() {
        // Arrange
        let gateway = crate::NetdevContextReader::new(Ipv4Addr::new(1, 1, 1, 1).into())
            .read_context()
            .await
            .and_then(|context| context.gateway_address);
        let detector = UpnpAccessDetector::new(gateway);
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
