//! Throughput seam: how a test climbs the sizes, stops, and sums up, with an in-memory server.

use std::time::Duration;

use application::run_throughput_test;
use application::test_support::InMemoryThroughputServer;
use domain::{Bitrate, Direction, TestProfile};

const MEGABYTE: u64 = 1_000_000;

#[tokio::test]
async fn stops_climbing_once_a_size_takes_long_enough() {
    // Arrange
    // 1 MB takes 1.6 s at 5 Mbit/s: enough to measure the line, no bigger size is needed.
    let server = InMemoryThroughputServer::with_rates(5_000_000, 5_000_000);

    // Act
    let result = run_throughput_test(&server, TestProfile::Full).await;

    // Assert
    assert!(result.is_ok());
    let largest = server
        .list_transfers(Direction::Download)
        .into_iter()
        .max()
        .expect("downloads");
    assert_eq!(largest, MEGABYTE);
}

#[tokio::test]
async fn starts_no_transfer_that_would_run_past_the_direction_budget() {
    // Arrange
    // At 9 Mbit/s, 10 MB takes 8.9 s: the first ends at 14.6 s, a second would end at 23.5 s.
    let server = InMemoryThroughputServer::with_rates(9_000_000, 9_000_000);

    // Act
    let result = run_throughput_test(&server, TestProfile::Full).await;

    // Assert
    assert!(result.is_ok());
    let downloads = server.list_transfers(Direction::Download);
    let ten_megabytes = downloads
        .iter()
        .filter(|bytes| **bytes == 10 * MEGABYTE)
        .count();
    assert_eq!(ten_megabytes, 1);
    let spent: u64 = downloads
        .iter()
        .map(|bytes| bytes * 8 * 1000 / 9_000_000)
        .sum();
    assert!(spent <= 15_000, "{spent} ms spent downloading");
}

#[tokio::test]
async fn times_at_least_one_large_transfer_on_a_slow_line() {
    // Arrange
    // At 1 Mbit/s the warm-up's 100 kB says little; 1 MB takes 8 s, past the quick budget.
    let server = InMemoryThroughputServer::with_rates(1_000_000, 1_000_000);

    // Act
    let result = run_throughput_test(&server, TestProfile::Quick).await;

    // Assert
    assert_eq!(
        result.expect("a result").download,
        Some(Bitrate::from_bits_per_second(1_000_000))
    );
    assert_eq!(
        server.list_transfers(Direction::Download),
        vec![100_000, MEGABYTE]
    );
}

#[tokio::test]
async fn measures_both_directions_and_the_loaded_round_trip() {
    // Arrange
    let server = InMemoryThroughputServer::with_rates(80_000_000, 20_000_000);

    // Act
    let result = run_throughput_test(&server, TestProfile::Quick)
        .await
        .expect("a result");

    // Assert
    assert_eq!(
        result.download,
        Some(Bitrate::from_bits_per_second(80_000_000))
    );
    assert_eq!(
        result.upload,
        Some(Bitrate::from_bits_per_second(20_000_000))
    );
    assert_eq!(
        result.loaded_latency,
        Some(InMemoryThroughputServer::LOADED_ROUND_TRIP)
    );
    assert!(result.duration > Duration::ZERO);
}

#[tokio::test]
async fn fails_when_the_test_server_does_not_answer() {
    // Arrange
    let server = InMemoryThroughputServer::create_unreachable();

    // Act
    let result = run_throughput_test(&server, TestProfile::Quick).await;

    // Assert
    assert!(result.is_err());
}
