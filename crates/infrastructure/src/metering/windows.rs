//! `ConnectionCost` of the Internet connection profile (`WinRT`).

use domain::Metering;
use windows::Networking::Connectivity::{NetworkCostType, NetworkInformation};

/// No profile (offline) or no answer: `Unknown`. O(1).
pub fn read_metering() -> Metering {
    read_cost_type().map_or(Metering::Unknown, to_metering)
}

fn read_cost_type() -> windows::core::Result<NetworkCostType> {
    NetworkInformation::GetInternetConnectionProfile()?
        .GetConnectionCost()?
        .NetworkCostType()
}

/// `Fixed` is a capped plan, `Variable` billed by volume: both cost money per byte.
fn to_metering(cost_type: NetworkCostType) -> Metering {
    match cost_type {
        NetworkCostType::Unrestricted => Metering::Unmetered,
        NetworkCostType::Fixed | NetworkCostType::Variable => Metering::Metered,
        _ => Metering::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_capped_and_volume_plans_as_metered() {
        // Act & Assert
        assert_eq!(to_metering(NetworkCostType::Fixed), Metering::Metered);
        assert_eq!(to_metering(NetworkCostType::Variable), Metering::Metered);
        assert_eq!(
            to_metering(NetworkCostType::Unrestricted),
            Metering::Unmetered
        );
        assert_eq!(to_metering(NetworkCostType::Unknown), Metering::Unknown);
    }
}
