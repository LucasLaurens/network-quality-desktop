//! Access seam: the box's word first, then the latest uploads of this network with the round trip.

use std::time::Duration;

use application::test_support::InMemoryAccessDetector;
use application::{NetworkKey, ThroughputRecord, detect_access};
use domain::{
    AccessEstimate, Bitrate, BoxLine, ConnectionKind, MacAddress, TestProfile, ThroughputResult,
    WanAccessType,
};

const NEAR: Option<Duration> = Some(Duration::from_millis(4));

fn create_network(last_octet: u8) -> NetworkKey {
    NetworkKey {
        gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, last_octet]),
        kind: ConnectionKind::Ethernet,
        name: None,
    }
}

fn create_test(network: &NetworkKey, upload_megabits: u64) -> ThroughputRecord {
    ThroughputRecord {
        utc: 1_700_000_000,
        network: network.clone(),
        result: ThroughputResult {
            profile: TestProfile::Quick,
            download: None,
            upload: Some(Bitrate::from_bits_per_second(upload_megabits * 1_000_000)),
            loaded_latency: None,
            bytes: 0,
            duration: Duration::ZERO,
        },
    }
}

#[tokio::test]
async fn estimates_from_the_tests_of_this_network_only() {
    // Arrange
    let here = create_network(1);
    let elsewhere = create_network(2);
    let detector = InMemoryAccessDetector::default();
    let mut tests: Vec<ThroughputRecord> = (0..3).map(|_| create_test(&elsewhere, 1)).collect();
    tests.extend((0..3).map(|_| create_test(&here, 300)));

    // Act
    let reading = detect_access(&detector, &tests, Some(&here), NEAR).await;

    // Assert
    assert_eq!(reading.access, AccessEstimate::EstimatedFibre);
    assert_eq!(reading.line, None);
}

#[tokio::test]
async fn reads_only_the_latest_tests() {
    // Arrange
    let here = create_network(3);
    let detector = InMemoryAccessDetector::default();
    let mut tests: Vec<ThroughputRecord> = (0..5).map(|_| create_test(&here, 300)).collect();
    tests.extend((0..5).map(|_| create_test(&here, 1)));

    // Act
    let reading = detect_access(&detector, &tests, Some(&here), NEAR).await;

    // Assert
    assert_eq!(
        reading.access,
        AccessEstimate::EstimatedFibre,
        "newest first"
    );
}

#[tokio::test]
async fn takes_the_box_line_when_it_answers() {
    // Arrange
    let line = BoxLine {
        access_type: WanAccessType::Dsl,
        downstream_max: Some(Bitrate::from_bits_per_second(17_000_000)),
        upstream_max: Some(Bitrate::from_bits_per_second(1_000_000)),
        total_received: Some(9_000_000_000),
        total_sent: Some(700_000_000),
    };
    let detector = InMemoryAccessDetector::answering(line);

    // Act
    let reading = detect_access(&detector, &[], None, None).await;

    // Assert
    assert_eq!(reading.access, AccessEstimate::ReportedDsl);
    assert_eq!(reading.line, Some(line));
}
