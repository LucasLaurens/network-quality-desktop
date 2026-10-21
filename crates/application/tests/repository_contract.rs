//! The in-memory repositories honour the contracts the database also runs.

use application::repository_contract::{check_contract, check_settings_contract};
use application::test_support::{InMemoryMeasurementRepository, InMemorySettingsRepository};

#[test]
fn honours_the_repository_contract() {
    // Act & Assert
    check_contract(InMemoryMeasurementRepository::default);
}

#[test]
fn honours_the_settings_contract() {
    // Act & Assert
    check_settings_contract(InMemorySettingsRepository::default);
}
