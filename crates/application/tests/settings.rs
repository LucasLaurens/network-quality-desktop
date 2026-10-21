//! Settings seam: which servers can be saved.

use application::can_measure_against;
use application::test_support::InMemoryNetwork;
use domain::{MeasurementServers, ProbeOutcome, Target};

const OWN_SERVER: &str = "203.0.113.20";
const OTHER_SERVER: &str = "203.0.113.21";

fn create_own(address: &str) -> MeasurementServers {
    MeasurementServers::Custom(Target::parse(address))
}

#[tokio::test]
async fn admits_a_new_server_that_answers() {
    // Arrange
    let network = InMemoryNetwork::default();
    network.set_outcome(
        Target::parse(OWN_SERVER),
        ProbeOutcome::Answered(std::time::Duration::from_millis(12)),
    );

    // Act
    let admitted = can_measure_against(
        MeasurementServers::Default,
        create_own(OWN_SERVER),
        &network,
    )
    .await;

    // Assert
    assert!(admitted);
}

#[tokio::test]
async fn refuses_a_new_server_that_cannot_be_reached() {
    // Arrange
    let network = InMemoryNetwork::default();

    // Act
    let admitted =
        can_measure_against(create_own(OTHER_SERVER), create_own(OWN_SERVER), &network).await;

    // Assert
    assert!(!admitted);
}

#[tokio::test]
async fn keeps_a_server_already_saved_without_probing_it_again() {
    // Arrange
    // Unreachable now: changing the look must not be refused for it.
    let network = InMemoryNetwork::default();

    // Act
    let admitted =
        can_measure_against(create_own(OWN_SERVER), create_own(OWN_SERVER), &network).await;

    // Assert
    assert!(admitted);
}

#[tokio::test]
async fn always_admits_the_public_servers() {
    // Arrange
    let network = InMemoryNetwork::default();

    // Act
    let admitted = can_measure_against(
        create_own(OWN_SERVER),
        MeasurementServers::Default,
        &network,
    )
    .await;

    // Assert
    assert!(admitted);
}
