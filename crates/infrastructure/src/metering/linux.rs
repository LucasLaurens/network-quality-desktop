//! The `Metered` property of `NetworkManager` over the system bus. Without `NetworkManager`
//! (systemd-networkd, containers), the answer is `Unknown` and the domain guesses from the link.

use std::time::Duration;

use dbus::blocking::Connection;
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;
use domain::Metering;

const NETWORK_MANAGER: &str = "org.freedesktop.NetworkManager";
const NETWORK_MANAGER_PATH: &str = "/org/freedesktop/NetworkManager";
const METERED_PROPERTY: &str = "Metered";
/// `NMMetered` values: yes and guessed yes, no and guessed no.
const METERED: [u32; 2] = [1, 3];
const UNMETERED: [u32; 2] = [2, 4];
/// Read every 10 seconds: a slow bus must not hold the context back.
const BUS_TIMEOUT: Duration = Duration::from_millis(500);

/// O(1).
pub fn read_metering() -> Metering {
    read_metered_property().map_or(Metering::Unknown, to_metering)
}

fn read_metered_property() -> Result<u32, dbus::Error> {
    let connection = Connection::new_system()?;
    connection
        .with_proxy(NETWORK_MANAGER, NETWORK_MANAGER_PATH, BUS_TIMEOUT)
        .get(NETWORK_MANAGER, METERED_PROPERTY)
}

/// 0 and anything newer mean unknown. O(1).
fn to_metering(metered: u32) -> Metering {
    if METERED.contains(&metered) {
        Metering::Metered
    } else if UNMETERED.contains(&metered) {
        Metering::Unmetered
    } else {
        Metering::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusts_network_manager_guesses_as_well_as_its_answers() {
        // Act & Assert
        assert_eq!(to_metering(1), Metering::Metered);
        assert_eq!(to_metering(3), Metering::Metered);
        assert_eq!(to_metering(2), Metering::Unmetered);
        assert_eq!(to_metering(4), Metering::Unmetered);
        assert_eq!(to_metering(0), Metering::Unknown);
    }
}
