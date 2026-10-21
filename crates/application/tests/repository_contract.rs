//! The in-memory repository honours the contract the database also runs.

use application::repository_contract::check_contract;
use application::test_support::InMemoryMeasurementRepository;

#[test]
fn honours_the_repository_contract() {
    // Act & Assert
    check_contract(InMemoryMeasurementRepository::default);
}
