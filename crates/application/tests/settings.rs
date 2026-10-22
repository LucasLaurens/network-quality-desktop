//! Settings seam: which servers can be saved.

use application::can_measure_against;
use application::test_support::{InMemoryNetwork, generate_targets};
use domain::{MeasurementServers, ProbeOutcome};

#[tokio::test]
async fn admits_a_new_server_that_answers() {
    // Arrange
    let [own_server] = generate_targets();
    let network = InMemoryNetwork::default();
    network.set_outcome(
        own_server,
        ProbeOutcome::Answered(std::time::Duration::from_millis(12)),
    );

    // Act
    let admitted = can_measure_against(
        MeasurementServers::Default,
        MeasurementServers::Custom(own_server),
        &network,
    )
    .await;

    // Assert
    assert!(admitted);
}

#[tokio::test]
async fn refuses_a_new_server_that_cannot_be_reached() {
    // Arrange
    let [own_server, other_server] = generate_targets();
    let network = InMemoryNetwork::default();

    // Act
    let admitted = can_measure_against(
        MeasurementServers::Custom(other_server),
        MeasurementServers::Custom(own_server),
        &network,
    )
    .await;

    // Assert
    assert!(!admitted);
}

#[tokio::test]
async fn keeps_a_server_already_saved_without_probing_it_again() {
    // Arrange
    let [own_server] = generate_targets();
    // Unreachable now: changing the look must not be refused for it.
    let network = InMemoryNetwork::default();

    // Act
    let admitted = can_measure_against(
        MeasurementServers::Custom(own_server),
        MeasurementServers::Custom(own_server),
        &network,
    )
    .await;

    // Assert
    assert!(admitted);
}

#[tokio::test]
async fn always_admits_the_public_servers() {
    // Arrange
    let [own_server] = generate_targets();
    let network = InMemoryNetwork::default();

    // Act
    let admitted = can_measure_against(
        MeasurementServers::Custom(own_server),
        MeasurementServers::Default,
        &network,
    )
    .await;

    // Assert
    assert!(admitted);
}
