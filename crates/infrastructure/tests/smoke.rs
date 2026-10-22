//! Smoke tests: each real adapter, on the system the test runs on, answers in time. Ignored by
//! default as they need a network and a real system; CI runs them on macOS, Linux and Windows
//! with `pnpm test:smoke`.

use std::future::Future;
use std::net::Ipv4Addr;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use application::{
    Clock, ConnectionContextReader, LatencyProbe, TrayHost, WifiReader, list_default_targets,
};
use domain::ProbeOutcome;
use infrastructure::{
    FallbackLatencyProbe, IcmpEchoSender, NetdevContextReader, SystemClock, SystemTrayHost,
    SystemWifiReader, TcpConnectionOpener,
};

/// Far beyond what any adapter needs: past it, the adapter hangs.
const ANSWER_WITHIN: Duration = Duration::from_secs(10);
/// The system clock and the one the test reads apart.
const CLOCK_TOLERANCE_SECONDS: i64 = 5;

/// Runs `work` on a thread of its own and waits for it at most `ANSWER_WITHIN`: a hung adapter
/// fails the test, and the thread left behind does not keep the run from ending.
fn run_within_deadline<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (answer, answered) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = answer.send(work());
    });
    answered
        .recv_timeout(ANSWER_WITHIN)
        .expect("the adapter answers in time")
}

fn run_to_completion<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(future)
}

#[test]
#[ignore = "smoke test: real network"]
fn reaches_a_default_target_by_icmp_or_tcp() {
    // Arrange
    let probe = FallbackLatencyProbe::new(IcmpEchoSender, TcpConnectionOpener);

    // Act
    let outcomes = run_within_deadline(move || {
        run_to_completion(async {
            let mut outcomes = Vec::new();
            for target in list_default_targets() {
                outcomes.push(probe.send_probe(target).await);
            }
            outcomes
        })
    });

    // Assert
    // CI runners often drop ICMP: the TCP fallback must then reach the targets.
    assert!(
        outcomes
            .iter()
            .any(|outcome| *outcome != ProbeOutcome::Unreachable),
        "{outcomes:?}"
    );
}

#[test]
#[ignore = "smoke test: real network interfaces"]
fn reads_the_connection_of_the_default_route() {
    // Arrange
    let reader = NetdevContextReader::new(Ipv4Addr::new(1, 1, 1, 1).into());

    // Act
    let context = run_within_deadline(move || run_to_completion(reader.read_context()));

    // Assert
    assert!(context.is_some(), "a runner has a default route");
}

#[test]
#[ignore = "smoke test: real Wi-Fi"]
fn reads_the_wifi_link_or_its_absence() {
    // Act
    let link = run_within_deadline(|| run_to_completion(SystemWifiReader.read_wifi()));

    // Assert
    // A runner has no Wi-Fi: `None` in time is the expected answer there; a name never comes empty.
    assert!(link.is_none_or(|link| link.name.as_deref() != Some("")));
}

#[test]
#[ignore = "smoke test: real clock"]
fn reads_a_steady_clock_and_the_wall_clock() {
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
    std::thread::sleep(Duration::from_millis(20));
    let second = clock.read_now();
    let utc = clock.read_utc_seconds();

    // Assert
    assert!(second > first);
    assert!((utc - wall_clock).abs() <= CLOCK_TOLERANCE_SECONDS);
}

#[test]
#[ignore = "smoke test: real system tray"]
fn says_whether_the_system_shows_a_tray() {
    // Act
    let available = run_within_deadline(|| SystemTrayHost.is_tray_available());

    // Assert
    // A Linux runner has no desktop, so no tray to promise there.
    if !cfg!(target_os = "linux") {
        assert!(available);
    }
}
