//! `CoreWLAN` for the radio values, `CoreLocation` for the permission that reveals the name.

use domain::{WifiBand, WifiLink};
use objc2_core_location::{CLAuthorizationStatus, CLLocationManager};
use objc2_core_wlan::{CWChannelBand, CWWiFiClient};

use super::LocationRequest;

/// O(1).
pub fn read_wifi() -> Option<WifiLink> {
    // SAFETY: CoreWLAN getters only read the state of the default interface; every object they
    // return is retained by `Retained` for as long as it is used.
    unsafe {
        let interface = CWWiFiClient::sharedWiFiClient().interface()?;
        let signal = interface.rssiValue();
        // CoreWLAN answers 0 when the interface is not associated with any network.
        if signal == 0 {
            return None;
        }
        let channel = interface.wlanChannel();
        Some(WifiLink {
            name: interface.ssid().map(|name| name.to_string()),
            signal_dbm: i32::try_from(signal).ok(),
            noise_dbm: i32::try_from(interface.noiseMeasurement())
                .ok()
                .filter(|noise| *noise != 0),
            rate_mbps: to_whole_mbps(interface.transmitRate()),
            channel: channel
                .as_ref()
                .and_then(|channel| u32::try_from(channel.channelNumber()).ok()),
            band: channel.map_or(WifiBand::Unknown, |channel| to_band(channel.channelBand())),
        })
    }
}

fn to_band(band: CWChannelBand) -> WifiBand {
    match band {
        CWChannelBand::Band2GHz => WifiBand::TwoPointFourGhz,
        CWChannelBand::Band5GHz => WifiBand::FiveGhz,
        CWChannelBand::Band6GHz => WifiBand::SixGhz,
        _ => WifiBand::Unknown,
    }
}

/// The rate is a whole number of Mbit/s on every Wi-Fi standard; 0 means unknown.
fn to_whole_mbps(rate: f64) -> Option<u32> {
    // Rounded and range-checked before the conversion, which cannot truncate.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = rate.round() as u32;
    (rate.is_finite() && rate >= 1.0 && rate <= f64::from(u32::MAX)).then_some(whole)
}

pub fn request_location_permission() -> LocationRequest {
    // SAFETY: called on the main thread, as CoreLocation requires; the manager is kept alive
    // until the user answers, then never used again.
    unsafe {
        let manager = CLLocationManager::new();
        if manager.authorizationStatus() != CLAuthorizationStatus::NotDetermined {
            return LocationRequest::NeedsSettings;
        }
        manager.requestWhenInUseAuthorization();
        // Dropping the manager would cancel the prompt: one manager per request is leaked.
        std::mem::forget(manager);
        LocationRequest::Prompted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_the_transmit_rate_and_drops_an_unknown_one() {
        // Act & Assert
        assert_eq!(to_whole_mbps(866.7), Some(867));
        assert_eq!(to_whole_mbps(0.0), None);
        assert_eq!(to_whole_mbps(f64::NAN), None);
    }

    // Depends on the machine running the test: run on demand with `-- --ignored`.
    #[test]
    #[ignore = "reads the Wi-Fi link of the machine running the test"]
    fn reads_the_wifi_link_of_this_machine() {
        // Act
        let link = read_wifi();

        // Assert
        println!("{link:?}");
    }
}
