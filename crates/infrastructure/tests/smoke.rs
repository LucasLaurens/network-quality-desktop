//! Smoke tests: each real adapter, on the system the test runs on, answers in time. Ignored by
//! default as they need a network and a real system; CI runs them on macOS, Linux and Windows
//! with `cargo test -p infrastructure --test smoke -- --ignored`.

use std::net::Ipv4Addr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use application::{
    Clock, ConnectionContextReader, LatencyProbe, TrayHost, WifiReader, list_default_targets,
};
use domain::ProbeOutcome;
use infrastructure::{
    FallbackLatencyProbe, IcmpEchoSender, NetdevContextReader, SystemClock, SystemTrayHost,
    SystemWifiReader, TcpConnectionOpener,
};
use tokio::time::timeout;

/// Far beyond what any adapter needs: past it, the adapter hangs.
const ANSWER_WITHIN: Duration = Duration::from_secs(10);
/// The system clock and the one the test reads apart.
const CLOCK_TOLERANCE_SECONDS: i64 = 5;

#[tokio::test]
#[ignore = "smoke test: real network"]
async fn reaches_a_default_target_by_icmp_or_tcp() {
    // Arrange
    let probe = FallbackLatencyProbe::new(IcmpEchoSender, TcpConnectionOpener);
    let mut outcomes = Vec::new();

    // Act
    for target in list_default_targets() {
        let outcome = timeout(ANSWER_WITHIN, probe.send_probe(target))
            .await
            .expect("the probe answers in time");
        outcomes.push(outcome);
    }

    // Assert
    // CI runners often drop ICMP: the TCP fallback must then reach the targets.
    assert!(
        outcomes
            .iter()
            .any(|outcome| *outcome != ProbeOutcome::Unreachable),
        "{outcomes:?}"
    );
}

#[tokio::test]
#[ignore = "smoke test: real network interfaces"]
async fn reads_the_connection_of_the_default_route() {
    // Arrange
    let reader = NetdevContextReader::new(Ipv4Addr::new(1, 1, 1, 1).into());

    // Act
    let context = timeout(ANSWER_WITHIN, reader.read_context())
        .await
        .expect("the reader answers in time");

    // Assert
    assert!(context.is_some(), "a runner has a default route");
}

#[tokio::test]
#[ignore = "smoke test: real Wi-Fi"]
async fn reads_the_wifi_link_or_its_absence() {
    // Act
    let link = timeout(ANSWER_WITHIN, SystemWifiReader.read_wifi()).await;

    // Assert
    // A runner has no Wi-Fi: answering `None` in time is the expected outcome there.
    assert!(link.is_ok(), "the Wi-Fi reader answers in time");
}

#[tokio::test]
#[ignore = "smoke test: real clock"]
async fn reads_a_steady_clock_and_the_wall_clock() {
    // Arrange
    let clock = SystemClock::start();
    let wall_clock = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after 1970")
            .as_secs(),
    )
    .expect("seconds in range");

    // Act
    let first = clock.read_now();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let second = clock.read_now();
    let utc = clock.read_utc_seconds();

    // Assert
    assert!(second > first);
    assert!((utc - wall_clock).abs() <= CLOCK_TOLERANCE_SECONDS);
}

#[tokio::test]
#[ignore = "smoke test: real system tray"]
async fn says_whether_the_system_shows_a_tray() {
    // Act
    let available = timeout(
        ANSWER_WITHIN,
        tokio::task::spawn_blocking(|| SystemTrayHost.is_tray_available()),
    )
    .await
    .expect("the tray host answers in time")
    .expect("the question does not panic");

    // Assert
    // A Linux runner has no desktop, so no tray to promise there.
    if !cfg!(target_os = "linux") {
        assert!(available);
    }
}
