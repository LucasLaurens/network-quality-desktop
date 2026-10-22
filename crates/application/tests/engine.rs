//! Engine seam: messages in, presented verdicts out, with in-memory ports and a manual clock.

use std::net::Ipv4Addr;
use std::time::Duration;

use application::test_support::{
    InMemoryConnection, InMemoryGateway, InMemoryHistoryRecorder, InMemoryNetwork,
    InMemoryNotifier, InMemoryVerdictPresenter, InMemoryWifi, MANUAL_CLOCK_EPOCH, ManualClock,
    generate_targets,
};
use application::{
    CONTEXT_REFRESH_SECONDS, EngineMessage, EnginePorts, EventKind, MeasurementEngine, VerdictView,
};
use domain::{
    Appearance, Bitrate, ConnectionContext, ConnectionKind, Diagnosis, Fault, GatewayEcho, Grade,
    LocalProblem, LocalSegment, MacAddress, MeasurementServers, MeasuringCause, Medium, Metering,
    Metric, ProbeOutcome, Settings, Target, TestProfile, ThroughputResult, Unmeasurable, Verdict,
    WifiBand, WifiLink,
};

const FAST: Duration = Duration::from_millis(20);
const NEAR_LIMIT: Duration = Duration::from_millis(85);
const SLOW: Duration = Duration::from_millis(180);
const VERY_SLOW: Duration = Duration::from_millis(400);
/// Seconds the engine needs before its first verdict.
const WARM_UP: u32 = 10;

struct Harness {
    /// Generated: the engine must not depend on which targets are used.
    first_target: Target,
    second_target: Target,
    own_server: Target,
    network: InMemoryNetwork,
    clock: ManualClock,
    presenter: InMemoryVerdictPresenter,
    connection: InMemoryConnection,
    gateway: InMemoryGateway,
    wifi: InMemoryWifi,
    history: InMemoryHistoryRecorder,
    notifier: InMemoryNotifier,
    engine: MeasurementEngine<
        InMemoryNetwork,
        ManualClock,
        InMemoryVerdictPresenter,
        InMemoryConnection,
        InMemoryGateway,
        InMemoryWifi,
        InMemoryHistoryRecorder,
        InMemoryNotifier,
    >,
}

fn create_harness() -> Harness {
    let network = InMemoryNetwork::default();
    let clock = ManualClock::default();
    let presenter = InMemoryVerdictPresenter::default();
    let connection = InMemoryConnection::default();
    let gateway = InMemoryGateway::default();
    let wifi = InMemoryWifi::default();
    let history = InMemoryHistoryRecorder::default();
    let notifier = InMemoryNotifier::default();
    let [first_target, second_target, own_server] = generate_targets();
    let engine = MeasurementEngine::new(
        EnginePorts {
            probe: network.clone(),
            clock: clock.clone(),
            presenter: presenter.clone(),
            context_reader: connection.clone(),
            gateway_probe: gateway.clone(),
            wifi_reader: wifi.clone(),
            history_recorder: history.clone(),
            notifier: notifier.clone(),
        },
        vec![first_target, second_target],
    );
    Harness {
        first_target,
        second_target,
        own_server,
        network,
        clock,
        presenter,
        connection,
        gateway,
        wifi,
        history,
        notifier,
        engine,
    }
}

impl Harness {
    fn set_every_target(&self, outcome: ProbeOutcome) {
        self.network.set_outcome(self.first_target, outcome);
        self.network.set_outcome(self.second_target, outcome);
    }

    fn set_latency(&self, round_trip: Duration) {
        self.set_every_target(ProbeOutcome::Answered(round_trip));
    }

    async fn run_seconds(&mut self, seconds: u32) {
        for _ in 0..seconds {
            self.clock.advance(Duration::from_secs(1));
            self.engine.handle(EngineMessage::Tick).await;
        }
    }

    async fn warm_up_with_latency(&mut self, round_trip: Duration) {
        self.set_latency(round_trip);
        self.run_seconds(WARM_UP).await;
    }

    fn read_last_view(&self) -> VerdictView {
        self.presenter.read_last().expect("a view was presented")
    }

    fn read_presented_verdict(&self) -> Verdict {
        self.read_last_view().verdict
    }
}

// Warm-up and latency

#[tokio::test]
async fn presents_measuring_until_ten_samples_were_collected() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency(FAST);
    harness.run_seconds(WARM_UP - 1).await;
    let before = harness.read_presented_verdict();

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(before, Verdict::Measuring);
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn grades_latency_between_100_and_300_ms_as_unstable() {
    // Arrange
    let mut harness = create_harness();

    // Act
    harness.warm_up_with_latency(SLOW).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Unstable)
    );
}

#[tokio::test]
async fn presents_the_p90_latency_over_the_last_15_seconds() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency(FAST);
    harness.run_seconds(13).await;
    harness.set_latency(SLOW);

    // Act
    harness.run_seconds(2).await;

    // Assert
    assert_eq!(
        harness
            .read_last_view()
            .latency
            .map(|latency| latency.value),
        Some(SLOW)
    );
}

#[tokio::test]
async fn uses_the_median_of_the_targets_that_answered() {
    // Arrange
    let mut harness = create_harness();
    harness
        .network
        .set_outcome(harness.first_target, ProbeOutcome::Answered(FAST));
    harness
        .network
        .set_outcome(harness.second_target, ProbeOutcome::LostButReachable);

    // Act
    harness.run_seconds(WARM_UP).await;

    // Assert
    assert_eq!(
        harness
            .read_last_view()
            .latency
            .map(|latency| latency.value),
        Some(FAST)
    );
}

// Hysteresis

#[tokio::test]
async fn shows_a_degradation_only_after_10_sustained_seconds() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.run_seconds(10).await;
    harness.set_latency(VERY_SLOW);
    // The p90 over 15 samples turns bad from the second slow second on.
    harness.run_seconds(11).await;
    let still_solid = harness.read_presented_verdict();

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(still_solid, Verdict::Graded(Grade::Solid));
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Degraded)
    );
}

#[tokio::test]
async fn shows_an_improvement_only_after_30_healthy_seconds() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(VERY_SLOW).await;
    harness.run_seconds(10).await;
    harness.set_latency(FAST);
    // The p90 over 15 samples turns good once at most one slow sample remains (14 seconds).
    harness.run_seconds(43).await;
    let still_degraded = harness.read_presented_verdict();

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(still_degraded, Verdict::Graded(Grade::Degraded));
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn ignores_isolated_latency_spikes() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;

    // Act
    for _ in 0..6 {
        harness.set_latency(Duration::from_millis(95));
        harness.run_seconds(1).await;
        harness.set_latency(FAST);
        harness.run_seconds(15).await;
    }

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

// Jitter and loss

#[tokio::test]
async fn grades_jitter_between_30_and_50_ms_as_unstable() {
    // Arrange
    let mut harness = create_harness();

    // Act
    for second in 0..WARM_UP {
        harness.set_latency(if second % 2 == 0 {
            FAST
        } else {
            FAST + Duration::from_millis(40)
        });
        harness.run_seconds(1).await;
    }

    // Assert
    let view = harness.read_last_view();
    assert_eq!(
        view.jitter.map(|jitter| jitter.value),
        Some(Duration::from_millis(40))
    );
    assert_eq!(view.verdict, Verdict::Graded(Grade::Unstable));
}

#[tokio::test]
async fn counts_lost_probes_over_the_last_minute_across_targets() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.run_seconds(50).await;
    harness
        .network
        .set_outcome(harness.second_target, ProbeOutcome::LostButReachable);

    // Act: 3 lost probes out of 120 sent in the last 60 seconds.
    harness.run_seconds(3).await;

    // Assert
    let view = harness.read_last_view();
    let loss = view.loss.expect("a loss reading");
    assert!((loss.value.to_percent() - 2.5).abs() < f64::EPSILON);
    assert_eq!(loss.grade, Grade::Unstable);
}

#[tokio::test]
async fn shows_the_worst_metric_as_the_verdict() {
    // Arrange
    let mut harness = create_harness();
    harness
        .network
        .set_outcome(harness.first_target, ProbeOutcome::Answered(FAST));
    harness
        .network
        .set_outcome(harness.second_target, ProbeOutcome::LostButReachable);

    // Act: half of the probes are lost while the latency stays low. The loss is graded from 30 s
    // on, then the hysteresis needs 10 more seconds to show the degradation.
    harness.run_seconds(40).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(
        view.latency.map(|latency| latency.grade),
        Some(Grade::Solid)
    );
    assert_eq!(view.loss.map(|loss| loss.grade), Some(Grade::Degraded));
    assert_eq!(view.verdict, Verdict::Graded(Grade::Degraded));
}

#[tokio::test]
async fn flags_a_metric_within_20_percent_of_its_next_threshold() {
    // Arrange
    let mut harness = create_harness();

    // Act
    harness.warm_up_with_latency(NEAR_LIMIT).await;

    // Assert
    let view = harness.read_last_view();
    let latency = view.latency.expect("a latency reading");
    assert_eq!(latency.grade, Grade::Solid);
    assert!(latency.is_near_next_threshold);
    assert!(
        !view
            .jitter
            .expect("a jitter reading")
            .is_near_next_threshold
    );
}

// Offline

#[tokio::test]
async fn shows_offline_after_5_seconds_without_any_reachable_target() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);
    harness.run_seconds(4).await;
    let after_four_seconds = harness.read_presented_verdict();

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(after_four_seconds, Verdict::Graded(Grade::Solid));
    assert_eq!(harness.read_presented_verdict(), Verdict::Offline);
}

#[tokio::test]
async fn does_not_show_offline_while_a_target_is_reachable_over_tcp() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness
        .network
        .set_outcome(harness.first_target, ProbeOutcome::Unreachable);
    harness
        .network
        .set_outcome(harness.second_target, ProbeOutcome::LostButReachable);

    // Act
    harness.run_seconds(20).await;

    // Assert
    assert_ne!(harness.read_presented_verdict(), Verdict::Offline);
}

#[tokio::test]
async fn measures_again_from_scratch_once_back_online() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);
    harness.run_seconds(5).await;
    harness.set_latency(VERY_SLOW);

    // Act
    harness.run_seconds(WARM_UP - 1).await;
    let while_warming_up = harness.read_presented_verdict();
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(while_warming_up, Verdict::Measuring);
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Degraded)
    );
}

#[tokio::test]
async fn shows_no_stale_measurement_while_offline() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(SLOW).await;
    harness.set_every_target(ProbeOutcome::Unreachable);

    // Act
    harness.run_seconds(5).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.verdict, Verdict::Offline);
    assert_eq!((view.latency, view.jitter, view.loss), (None, None, None));
}

#[tokio::test]
async fn does_not_invent_jitter_when_one_target_drops_packets() {
    // Arrange
    let mut harness = create_harness();
    harness
        .network
        .set_outcome(harness.first_target, ProbeOutcome::Answered(FAST));

    // Act: the second target, steadily slower, loses every other echo.
    for second in 0..WARM_UP * 2 {
        let outcome = if second % 2 == 0 {
            ProbeOutcome::Answered(SLOW)
        } else {
            ProbeOutcome::LostButReachable
        };
        harness.network.set_outcome(harness.second_target, outcome);
        harness.run_seconds(1).await;
    }

    // Assert
    let jitter = harness.read_last_view().jitter.expect("a jitter value");
    assert_eq!(jitter.value, Duration::ZERO);
}

#[tokio::test]
async fn does_not_grade_loss_before_30_seconds_of_probes() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency(FAST);
    harness.run_seconds(WARM_UP).await;
    harness
        .network
        .set_outcome(harness.second_target, ProbeOutcome::LostButReachable);

    // Act: one lost probe out of 22 would read as Unstable on so few probes.
    harness.run_seconds(1).await;
    let early = harness.read_last_view();
    harness
        .network
        .set_outcome(harness.second_target, ProbeOutcome::Answered(FAST));
    harness.run_seconds(19).await;

    // Assert
    assert_eq!(early.verdict, Verdict::Graded(Grade::Solid));
    assert_eq!(early.loss, None);
    // 1 lost probe out of the 60 sent in 30 seconds.
    assert_eq!(
        harness.read_last_view().loss.map(|loss| loss.grade),
        Some(Grade::Unstable)
    );
}

// Diagnosis

#[tokio::test]
async fn diagnoses_a_healthy_connection() {
    // Arrange
    let mut harness = create_harness();

    // Act
    harness.warm_up_with_latency(FAST).await;

    // Assert
    assert_eq!(harness.read_last_view().diagnosis, Diagnosis::Healthy);
}

#[tokio::test]
async fn diagnoses_a_metric_close_to_its_next_threshold() {
    // Arrange
    let mut harness = create_harness();

    // Act
    harness.warm_up_with_latency(NEAR_LIMIT).await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::NearThreshold(Metric::Latency)
    );
}

#[tokio::test]
async fn reports_warm_up_progress_while_measuring() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency(FAST);

    // Act
    harness.run_seconds(6).await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::Measuring {
            collected: 6,
            needed: WARM_UP,
            cause: MeasuringCause::FirstMeasures,
        }
    );
}

#[tokio::test]
async fn diagnoses_no_reachable_server_while_offline() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 20)));
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);

    // Act
    harness.run_seconds(5).await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::NoServerReachable
    );
}

#[tokio::test]
async fn names_the_worst_metric_when_out_of_threshold() {
    // Arrange
    let mut harness = create_harness();

    // Act
    harness.warm_up_with_latency(SLOW).await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::OutOfThreshold {
            metric: Metric::Latency,
            fault: Fault::Undetermined(Unmeasurable::NoGateway),
        }
    );
}

#[tokio::test]
async fn diagnoses_a_recovery_while_the_state_waits_to_improve() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(VERY_SLOW).await;
    harness.run_seconds(10).await;
    harness.set_latency(FAST);

    // Act
    harness.run_seconds(20).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.verdict, Verdict::Graded(Grade::Degraded));
    assert_eq!(view.diagnosis, Diagnosis::Recovering);
}

#[tokio::test]
async fn gives_each_reading_the_bounds_it_was_graded_against() {
    // Arrange
    let mut harness = create_harness();

    // Act
    harness.warm_up_with_latency(SLOW).await;

    // Assert
    let latency = harness.read_last_view().latency.expect("a latency reading");
    assert_eq!(latency.bounds, domain::Thresholds::VIDEO_CALLS.latency);
    assert_eq!(
        latency.bounds.find_reference_threshold(latency.value),
        Duration::from_millis(100)
    );
}

// Connection context

fn create_context(kind: ConnectionKind, last_octet: u8) -> ConnectionContext {
    ConnectionContext {
        kind,
        is_vpn: false,
        gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, last_octet]),
        gateway_address: Some(Ipv4Addr::new(192, 168, last_octet, 1)),
        metering: Metering::Unknown,
        medium: if kind == ConnectionKind::WiFi {
            Medium::WiFi
        } else {
            Medium::Ethernet
        },
    }
}

#[tokio::test]
async fn presents_the_connection_from_the_first_second() {
    // Arrange
    let mut harness = create_harness();
    let context = create_context(ConnectionKind::WiFi, 1);
    harness.connection.set_context(Some(context));
    harness.set_latency(FAST);

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(harness.read_last_view().context, Some(context));
}

#[tokio::test]
async fn reads_the_connection_again_every_ten_seconds() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 1)));
    harness.set_latency(FAST);
    harness.run_seconds(1).await;
    let ethernet = create_context(ConnectionKind::Ethernet, 2);
    harness.connection.set_context(Some(ethernet));
    harness.run_seconds(CONTEXT_REFRESH_SECONDS - 1).await;
    let before_refresh = harness.read_last_view().context;

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_ne!(before_refresh, Some(ethernet));
    assert_eq!(harness.read_last_view().context, Some(ethernet));
}

#[tokio::test]
async fn keeps_the_connection_while_offline() {
    // Arrange
    let mut harness = create_harness();
    let context = create_context(ConnectionKind::Ethernet, 3);
    harness.connection.set_context(Some(context));
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);

    // Act
    harness.run_seconds(5).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.verdict, Verdict::Offline);
    assert_eq!(view.context, Some(context));
}

#[tokio::test]
async fn reads_the_connection_again_right_after_an_outage() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 4)));
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);
    harness.run_seconds(5).await;
    let ethernet = create_context(ConnectionKind::Ethernet, 5);
    harness.connection.set_context(Some(ethernet));
    harness.set_latency(FAST);

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(harness.read_last_view().context, Some(ethernet));
}

// Network changes

#[tokio::test]
async fn measures_again_as_soon_as_the_network_changes() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 6)));
    harness.warm_up_with_latency(FAST).await;
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 7)));
    harness.engine.handle(EngineMessage::NetworkChanged).await;

    // Act
    harness.run_seconds(1).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.verdict, Verdict::Measuring);
    assert_eq!(
        view.diagnosis,
        // The second that saw the change already measured the new network.
        Diagnosis::Measuring {
            collected: 1,
            needed: WARM_UP,
            cause: MeasuringCause::NewNetwork,
        }
    );
}

#[tokio::test]
async fn tries_icmp_again_on_another_network() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 61)));
    harness.warm_up_with_latency(FAST).await;
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 62)));
    harness.engine.handle(EngineMessage::NetworkChanged).await;

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(harness.network.count_forgotten_fallbacks(), 1);
}

#[tokio::test]
async fn keeps_its_fallbacks_while_the_network_stays_the_same() {
    // Arrange
    let mut harness = create_harness();
    let wifi = create_context(ConnectionKind::WiFi, 63);
    harness.connection.set_context(Some(wifi));
    harness.warm_up_with_latency(FAST).await;
    harness.engine.handle(EngineMessage::NetworkChanged).await;

    // Act
    harness.run_seconds(5).await;

    // Assert
    assert_eq!(harness.network.count_forgotten_fallbacks(), 0);
}

#[tokio::test]
async fn keeps_its_verdict_when_a_change_leaves_the_network_as_it_was() {
    // Arrange
    let mut harness = create_harness();
    let wifi = create_context(ConnectionKind::WiFi, 8);
    harness.connection.set_context(Some(wifi));
    harness.warm_up_with_latency(FAST).await;
    harness.connection.set_context(Some(ConnectionContext {
        metering: Metering::Metered,
        ..wifi
    }));
    harness.engine.handle(EngineMessage::NetworkChanged).await;

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn notices_a_new_network_at_the_next_reading_without_any_event() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::Ethernet, 9)));
    harness.warm_up_with_latency(FAST).await;
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 10)));
    // The warm-up read the connection at its first second: the next reading is due now.

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(harness.read_presented_verdict(), Verdict::Measuring);
}

#[tokio::test]
async fn notices_a_new_network_reached_through_a_moment_without_route() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 11)));
    harness.warm_up_with_latency(FAST).await;
    harness.connection.set_context(None);
    harness.engine.handle(EngineMessage::NetworkChanged).await;
    harness.run_seconds(1).await;
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 12)));
    harness.engine.handle(EngineMessage::NetworkChanged).await;

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(harness.read_presented_verdict(), Verdict::Measuring);
}

#[tokio::test]
async fn notices_another_box_at_the_same_address_once_its_mac_is_read() {
    // Arrange
    let mut harness = create_harness();
    let first_box = create_context(ConnectionKind::WiFi, 13);
    harness.connection.set_context(Some(first_box));
    harness.warm_up_with_latency(FAST).await;
    harness.connection.set_context(Some(ConnectionContext {
        gateway: None,
        ..first_box
    }));
    harness.engine.handle(EngineMessage::NetworkChanged).await;
    harness.run_seconds(1).await;
    harness.connection.set_context(Some(ConnectionContext {
        gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, 14]),
        ..first_box
    }));
    harness.engine.handle(EngineMessage::NetworkChanged).await;

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(harness.read_presented_verdict(), Verdict::Measuring);
}

// Segment diagnosis

async fn run_slow_internet_behind(
    harness: &mut Harness,
    kind: ConnectionKind,
    gateway: GatewayEcho,
) {
    harness
        .connection
        .set_context(Some(create_context(kind, 30)));
    harness.gateway.set_echo(gateway);
    harness.warm_up_with_latency(SLOW).await;
    // Gateway pings start once the first reading named the gateway: one more second for ten.
    harness.run_seconds(1).await;
}

#[tokio::test]
async fn blames_the_operator_when_the_box_answers_well() {
    // Arrange
    let mut harness = create_harness();

    // Act
    run_slow_internet_behind(
        &mut harness,
        ConnectionKind::WiFi,
        GatewayEcho::Answered(Duration::from_millis(3)),
    )
    .await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::OutOfThreshold {
            metric: Metric::Latency,
            fault: Fault::Operator,
        }
    );
}

#[tokio::test]
async fn blames_the_local_link_when_the_box_answers_slowly() {
    // Arrange
    let mut harness = create_harness();

    // Act
    run_slow_internet_behind(
        &mut harness,
        ConnectionKind::WiFi,
        GatewayEcho::Answered(Duration::from_millis(45)),
    )
    .await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::OutOfThreshold {
            metric: Metric::Latency,
            fault: Fault::LocalLink(LocalProblem::Latency),
        }
    );
}

#[tokio::test]
async fn says_the_cause_is_undetermined_when_pings_are_refused() {
    // Arrange
    let mut harness = create_harness();

    // Act
    run_slow_internet_behind(
        &mut harness,
        ConnectionKind::WiFi,
        GatewayEcho::NotPermitted,
    )
    .await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::OutOfThreshold {
            metric: Metric::Latency,
            fault: Fault::Undetermined(Unmeasurable::NotPermitted),
        }
    );
}

#[tokio::test]
async fn blames_the_mobile_network_of_a_built_in_modem() {
    // Arrange
    let mut harness = create_harness();

    // Act
    run_slow_internet_behind(&mut harness, ConnectionKind::Cellular, GatewayEcho::Lost).await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::OutOfThreshold {
            metric: Metric::Latency,
            fault: Fault::MobileNetwork,
        }
    );
}

#[tokio::test]
async fn says_the_box_lost_internet_when_only_the_box_answers() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 31)));
    harness
        .gateway
        .set_echo(GatewayEcho::Answered(Duration::from_millis(3)));
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);

    // Act
    harness.run_seconds(5).await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::GatewayWithoutInternet
    );
}

#[tokio::test]
async fn says_no_network_is_connected_without_any_route() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);

    // Act
    harness.run_seconds(5).await;

    // Assert
    assert_eq!(harness.read_last_view().diagnosis, Diagnosis::NoNetwork);
}

#[tokio::test]
async fn never_blames_the_box_or_the_operator_for_a_vpn() {
    // Arrange
    let mut harness = create_harness();
    harness.connection.set_context(Some(ConnectionContext {
        is_vpn: true,
        ..create_context(ConnectionKind::WiFi, 32)
    }));
    harness
        .gateway
        .set_echo(GatewayEcho::Answered(Duration::from_millis(3)));

    // Act
    harness.warm_up_with_latency(SLOW).await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::OutOfThreshold {
            metric: Metric::Latency,
            fault: Fault::Undetermined(Unmeasurable::NoGateway),
        }
    );
}

#[tokio::test]
async fn keeps_measuring_the_box_while_the_internet_is_cut() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 33)));
    harness
        .gateway
        .set_echo(GatewayEcho::Answered(Duration::from_millis(3)));
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);

    // Act
    harness.run_seconds(6).await;

    // Assert
    assert!(matches!(
        harness.read_last_view().local,
        LocalSegment::Measured { .. }
    ));
}

#[tokio::test]
async fn stays_solid_when_only_the_link_to_the_box_is_slow() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 34)));
    harness
        .gateway
        .set_echo(GatewayEcho::Answered(Duration::from_millis(45)));

    // Act
    harness.warm_up_with_latency(FAST).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.verdict, Verdict::Graded(Grade::Solid));
    assert_eq!(view.diagnosis, Diagnosis::Healthy);
}

#[tokio::test]
async fn puts_a_slow_wifi_down_to_its_weak_signal() {
    // Arrange
    let mut harness = create_harness();
    harness.wifi.set_link(Some(WifiLink {
        name: None,
        signal_dbm: Some(-80),
        rate_mbps: None,
        channel: None,
        band: WifiBand::TwoPointFourGhz,
    }));

    // Act
    run_slow_internet_behind(
        &mut harness,
        ConnectionKind::WiFi,
        GatewayEcho::Answered(Duration::from_millis(45)),
    )
    .await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::OutOfThreshold {
            metric: Metric::Latency,
            fault: Fault::LocalLink(LocalProblem::WeakSignal),
        }
    );
}

#[tokio::test]
async fn ignores_a_weak_wifi_while_a_cable_carries_the_traffic() {
    // Arrange
    let mut harness = create_harness();
    harness.wifi.set_link(Some(WifiLink {
        name: None,
        signal_dbm: Some(-80),
        rate_mbps: None,
        channel: None,
        band: WifiBand::TwoPointFourGhz,
    }));

    // Act
    run_slow_internet_behind(
        &mut harness,
        ConnectionKind::Ethernet,
        GatewayEcho::Answered(Duration::from_millis(45)),
    )
    .await;

    // Assert
    assert_eq!(
        harness.read_last_view().diagnosis,
        Diagnosis::OutOfThreshold {
            metric: Metric::Latency,
            fault: Fault::LocalLink(LocalProblem::Latency),
        }
    );
}

// History

#[tokio::test]
async fn hands_each_finished_minute_to_the_history() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 40)));
    harness.set_latency(FAST);

    // Act
    harness.run_seconds(61).await;

    // Assert
    let minutes = harness.history.list_saved_minutes();
    assert_eq!(minutes.len(), 1);
    assert_eq!(minutes[0].1, MANUAL_CLOCK_EPOCH);
    // The clock moves before each tick: the first minute holds seconds 1 to 59, two targets.
    assert_eq!(minutes[0].2.probes_sent, 118);
}

#[tokio::test]
async fn writes_the_minute_in_progress_when_asked_to_flush() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 41)));
    harness.set_latency(FAST);
    harness.run_seconds(5).await;
    let (answer, answered) = std::sync::mpsc::sync_channel(1);

    // Act
    harness.engine.handle(EngineMessage::Flush(answer)).await;

    // Assert
    assert_eq!(harness.history.list_saved_minutes().len(), 1);
    assert!(answered.recv().is_ok());
}

#[tokio::test]
async fn answers_with_the_last_hour_and_the_current_network() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 42)));
    harness.set_latency(FAST);
    harness.run_seconds(3).await;
    let (answer, answered) = tokio::sync::oneshot::channel();

    // Act
    harness
        .engine
        .handle(EngineMessage::ReadLiveHistory(answer))
        .await;

    // Assert
    let live = answered.await.expect("an answer");
    assert_eq!(live.last_hour.len(), 3);
    assert_eq!(live.last_hour[0].sample.latency, Some(FAST));
    assert_eq!(
        live.network.map(|network| network.kind),
        Some(ConnectionKind::WiFi)
    );
}

// Sleep and wake

#[tokio::test]
async fn writes_the_minute_in_progress_before_allowing_sleep() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 51)));
    harness.set_latency(FAST);
    harness.run_seconds(5).await;
    let (answer, answered) = std::sync::mpsc::sync_channel(1);

    // Act
    harness.engine.handle(EngineMessage::Sleep(answer)).await;

    // Assert
    assert!(answered.recv().is_ok());
    assert_eq!(harness.history.list_saved_minutes().len(), 1);
    assert_eq!(
        harness.history.list_events().last().map(|event| event.kind),
        Some(EventKind::Sleep)
    );
}

#[tokio::test]
async fn measures_nothing_between_sleep_and_wake() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    let (answer, _answered) = std::sync::mpsc::sync_channel(1);
    harness.engine.handle(EngineMessage::Sleep(answer)).await;
    let presented = harness.presenter.count_views();

    // Act
    harness.run_seconds(3).await;

    // Assert
    assert_eq!(harness.presenter.count_views(), presented);
}

#[tokio::test]
async fn tries_icmp_again_on_wake() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 64)));
    harness.warm_up_with_latency(FAST).await;
    let (answer, _answered) = std::sync::mpsc::sync_channel(1);
    harness.engine.handle(EngineMessage::Sleep(answer)).await;
    harness.clock.advance(Duration::from_secs(3600));

    // Act
    harness.engine.handle(EngineMessage::Wake).await;

    // Assert
    assert_eq!(harness.network.count_forgotten_fallbacks(), 1);
}

#[tokio::test]
async fn measures_again_from_scratch_on_wake() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 52)));
    harness.warm_up_with_latency(FAST).await;
    let (answer, _answered) = std::sync::mpsc::sync_channel(1);
    harness.engine.handle(EngineMessage::Sleep(answer)).await;
    harness.clock.advance(Duration::from_secs(3600));

    // Act
    harness.engine.handle(EngineMessage::Wake).await;
    harness.run_seconds(1).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.verdict, Verdict::Measuring);
    assert!(matches!(
        view.diagnosis,
        Diagnosis::Measuring {
            collected: 1,
            cause: MeasuringCause::Wake,
            ..
        }
    ));
    assert_eq!(
        harness.history.list_events().last().map(|event| event.kind),
        Some(EventKind::Wake)
    );
}

#[tokio::test]
async fn wakes_once_when_the_system_says_so() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    let (answer, _answered) = std::sync::mpsc::sync_channel(1);
    harness.engine.handle(EngineMessage::Sleep(answer)).await;
    harness.clock.advance(Duration::from_secs(3600));

    // Act
    harness.engine.handle(EngineMessage::Wake).await;
    harness.run_seconds(2).await;

    // Assert
    let wakes = harness
        .history
        .list_events()
        .iter()
        .filter(|event| event.kind == EventKind::Wake)
        .count();
    assert_eq!(wakes, 1);
}

#[tokio::test]
async fn ignores_a_wake_notice_that_comes_after_the_gap_was_noticed() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.clock.advance(Duration::from_secs(600));
    harness.run_seconds(3).await;

    // Act
    harness.engine.handle(EngineMessage::Wake).await;
    harness.run_seconds(1).await;

    // Assert
    let wakes = harness
        .history
        .list_events()
        .iter()
        .filter(|event| event.kind == EventKind::Wake)
        .count();
    assert_eq!(wakes, 1);
    assert!(matches!(
        harness.read_last_view().diagnosis,
        Diagnosis::Measuring { collected: 4, .. }
    ));
}

#[tokio::test]
async fn ends_an_outage_at_its_last_second_when_the_gap_reveals_a_sleep() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 54)));
    harness.set_every_target(ProbeOutcome::Unreachable);
    harness.run_seconds(8).await;
    let last_offline_second = MANUAL_CLOCK_EPOCH + 8;
    harness.clock.advance(Duration::from_secs(600));

    // Act
    harness.set_latency(FAST);
    harness.run_seconds(1).await;

    // Assert
    let ended: Vec<i64> = harness
        .history
        .list_events()
        .iter()
        .filter(|event| event.kind == EventKind::OfflineEnded)
        .map(|event| event.utc)
        .collect();
    assert_eq!(ended, vec![last_offline_second]);
}

#[tokio::test]
async fn wakes_when_the_clock_was_set_back_during_the_sleep() {
    // Arrange
    let mut harness = create_harness();
    harness.clock.advance(Duration::from_secs(120));
    harness.warm_up_with_latency(FAST).await;
    let (answer, _answered) = std::sync::mpsc::sync_channel(1);
    harness.engine.handle(EngineMessage::Sleep(answer)).await;
    let presented = harness.presenter.count_views();

    // Act
    harness.clock.set_back(Duration::from_secs(60));
    harness.engine.handle(EngineMessage::Tick).await;

    // Assert
    assert!(harness.presenter.count_views() > presented);
}

#[tokio::test]
async fn treats_a_long_gap_between_two_ticks_as_a_wake() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;

    // Act
    // The system slept without telling: the wall clock jumped between two ticks.
    harness.clock.advance(Duration::from_secs(600));
    harness.engine.handle(EngineMessage::Tick).await;

    // Assert
    assert!(matches!(
        harness.read_last_view().diagnosis,
        Diagnosis::Measuring {
            cause: MeasuringCause::Wake,
            ..
        }
    ));
    assert_eq!(
        harness.history.list_events().last().map(|event| event.kind),
        Some(EventKind::Wake)
    );
}

#[tokio::test]
async fn wakes_up_by_itself_when_the_wake_notice_never_comes() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    let (answer, _answered) = std::sync::mpsc::sync_channel(1);
    harness.engine.handle(EngineMessage::Sleep(answer)).await;
    harness.clock.advance(Duration::from_secs(600));

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(harness.read_presented_verdict(), Verdict::Measuring);
}

#[tokio::test]
async fn keeps_an_outage_from_running_on_through_sleep() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 53)));
    harness.set_every_target(ProbeOutcome::Unreachable);
    harness.run_seconds(8).await;
    let (answer, _answered) = std::sync::mpsc::sync_channel(1);

    // Act
    harness.engine.handle(EngineMessage::Sleep(answer)).await;

    // Assert
    let kinds: Vec<EventKind> = harness
        .history
        .list_events()
        .iter()
        .map(|event| event.kind)
        .collect();
    assert_eq!(
        kinds[kinds.len() - 2..],
        [EventKind::OfflineEnded, EventKind::Sleep]
    );
}

/// A network whose next probe lasts as long as a sleep: the wall clock jumps while it waits.
#[derive(Clone)]
struct DozingNetwork {
    network: InMemoryNetwork,
    clock: ManualClock,
    doze: std::sync::Arc<std::sync::Mutex<Option<Duration>>>,
}

impl DozingNetwork {
    fn doze_during_next_probe(&self, duration: Duration) {
        *self.doze.lock().expect("a doze") = Some(duration);
    }
}

impl application::LatencyProbe for DozingNetwork {
    fn send_probe(&self, target: Target) -> impl std::future::Future<Output = ProbeOutcome> + Send {
        if let Some(duration) = self.doze.lock().expect("a doze").take() {
            self.clock.advance(duration);
        }
        self.network.send_probe(target)
    }

    fn forget_fallbacks(&self) {
        self.network.forget_fallbacks();
    }
}

#[tokio::test]
async fn discards_the_probes_that_waited_through_a_sleep() {
    // Arrange
    let clock = ManualClock::default();
    let network = InMemoryNetwork::default();
    let presenter = InMemoryVerdictPresenter::default();
    let [target] = generate_targets();
    let dozing = DozingNetwork {
        network: network.clone(),
        clock: clock.clone(),
        doze: std::sync::Arc::default(),
    };
    let mut engine = MeasurementEngine::new(
        EnginePorts {
            probe: dozing.clone(),
            clock: clock.clone(),
            presenter: presenter.clone(),
            context_reader: InMemoryConnection::default(),
            gateway_probe: InMemoryGateway::default(),
            wifi_reader: InMemoryWifi::default(),
            history_recorder: InMemoryHistoryRecorder::default(),
            notifier: InMemoryNotifier::default(),
        },
        vec![target],
    );
    network.set_outcome(target, ProbeOutcome::Answered(FAST));
    for _ in 0..WARM_UP {
        clock.advance(Duration::from_secs(1));
        engine.handle(EngineMessage::Tick).await;
    }
    network.set_outcome(target, ProbeOutcome::Unreachable);
    dozing.doze_during_next_probe(Duration::from_secs(600));

    // Act
    clock.advance(Duration::from_secs(1));
    engine.handle(EngineMessage::Tick).await;

    // Assert
    let view = presenter.read_last().expect("a view");
    assert!(matches!(
        view.diagnosis,
        Diagnosis::Measuring {
            collected: 0,
            cause: MeasuringCause::Wake,
            ..
        }
    ));
}

#[tokio::test]
async fn handles_a_sleep_notice_before_the_ticks_already_waiting() {
    // Arrange
    let harness = create_harness();
    let presenter = harness.presenter.clone();
    let (ticks, inbox) = tokio::sync::mpsc::channel(4);
    let (notices, urgent) = tokio::sync::mpsc::channel(1);
    for _ in 0..3 {
        ticks
            .send(EngineMessage::Tick)
            .await
            .expect("room for a tick");
    }
    let (answer, answered) = std::sync::mpsc::sync_channel(1);
    notices
        .send(EngineMessage::Sleep(answer))
        .await
        .expect("room for the notice");
    drop((ticks, notices));

    // Act
    harness.engine.run(inbox, urgent).await;

    // Assert
    assert!(answered.recv().is_ok());
    assert_eq!(presenter.count_views(), 0);
}

// Settings

fn create_settings(servers: MeasurementServers) -> Settings {
    Settings {
        servers,
        ..Settings::default()
    }
}

/// Warm on the public servers, with only the own server answering from now on.
async fn create_harness_moving_to_the_own_server() -> Harness {
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);
    harness
        .network
        .set_outcome(harness.own_server, ProbeOutcome::Answered(FAST));
    harness
        .engine
        .handle(EngineMessage::SettingsChanged(create_settings(
            MeasurementServers::Custom(harness.own_server),
        )))
        .await;
    harness
}

#[tokio::test]
async fn tries_icmp_again_against_new_servers() {
    // Act
    let harness = create_harness_moving_to_the_own_server().await;

    // Assert
    assert_eq!(harness.network.count_forgotten_fallbacks(), 1);
}

#[tokio::test]
async fn measures_again_from_scratch_against_new_servers() {
    // Arrange
    let mut harness = create_harness_moving_to_the_own_server().await;

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert!(matches!(
        harness.read_last_view().diagnosis,
        Diagnosis::Measuring {
            collected: 1,
            cause: MeasuringCause::NewServers,
            ..
        }
    ));
}

#[tokio::test]
async fn measures_the_own_server_alone_once_picked() {
    // Arrange
    let mut harness = create_harness_moving_to_the_own_server().await;

    // Act
    harness.run_seconds(WARM_UP).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn keeps_measuring_when_the_servers_stay_the_same() {
    // Arrange
    let mut harness = create_harness();
    let own_server = MeasurementServers::Custom(harness.own_server);
    harness
        .network
        .set_outcome(harness.own_server, ProbeOutcome::Answered(FAST));
    harness
        .engine
        .handle(EngineMessage::SettingsChanged(create_settings(own_server)))
        .await;
    harness.run_seconds(WARM_UP).await;

    // Act
    harness
        .engine
        .handle(EngineMessage::SettingsChanged(Settings {
            appearance: Appearance::Dark,
            ..create_settings(own_server)
        }))
        .await;
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

// Notifications

#[tokio::test]
async fn notifies_once_when_the_connection_gets_durably_worse() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.set_latency(SLOW);

    // Act
    harness.run_seconds(15).await;

    // Assert
    let verdicts: Vec<Verdict> = harness
        .notifier
        .list_notified()
        .iter()
        .map(|view| view.verdict)
        .collect();
    assert_eq!(verdicts, vec![Verdict::Graded(Grade::Unstable)]);
}

#[tokio::test]
async fn never_notifies_an_improvement() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(SLOW).await;
    harness.set_latency(FAST);

    // Act
    // 15 s for the slow samples to leave the p90 window, 30 s of improvement, and a margin.
    harness.run_seconds(50).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
    assert!(harness.notifier.list_notified().is_empty());
}

#[tokio::test]
async fn stays_silent_when_notifications_are_turned_off() {
    // Arrange
    let mut harness = create_harness();
    harness
        .engine
        .handle(EngineMessage::SettingsChanged(Settings {
            notifies_on_degradation: false,
            ..create_settings(MeasurementServers::Custom(harness.first_target))
        }))
        .await;
    harness.warm_up_with_latency(FAST).await;
    harness.set_latency(SLOW);

    // Act
    harness.run_seconds(15).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Unstable)
    );
    assert!(harness.notifier.list_notified().is_empty());
}

#[tokio::test]
async fn notifies_at_the_tick_the_worse_state_shows() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.set_latency(SLOW);
    let mut ticks = Vec::new();

    // Act
    for _ in 0..20 {
        harness.run_seconds(1).await;
        ticks.push((
            harness.read_presented_verdict(),
            harness.notifier.list_notified().len(),
        ));
    }

    // Assert
    let first_shown = ticks
        .iter()
        .position(|(verdict, _)| *verdict == Verdict::Graded(Grade::Unstable));
    let first_notified = ticks.iter().position(|(_, notified)| *notified > 0);
    assert!(first_shown.is_some());
    assert_eq!(first_notified, first_shown);
}

#[tokio::test]
async fn notifies_a_cut_that_lasts() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness.set_every_target(ProbeOutcome::Unreachable);

    // Act
    harness.run_seconds(20).await;

    // Assert
    let verdicts: Vec<Verdict> = harness
        .notifier
        .list_notified()
        .iter()
        .map(|view| view.verdict)
        .collect();
    assert_eq!(verdicts, vec![Verdict::Offline]);
}

// Throughput tests

fn create_throughput(download_bits: u64) -> ThroughputResult {
    ThroughputResult {
        profile: TestProfile::Quick,
        download: Some(Bitrate::from_bits_per_second(download_bits)),
        upload: Some(Bitrate::from_bits_per_second(10_000_000)),
        loaded_latency: Some(Duration::from_millis(90)),
        bytes: 4_200_000,
        duration: Duration::from_secs(3),
    }
}

/// Solid, then a test too slow for a call.
async fn create_harness_after_a_slow_test() -> Harness {
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 61)));
    harness.warm_up_with_latency(FAST).await;
    harness
        .engine
        .handle(EngineMessage::ThroughputEnded(Some(create_throughput(
            400_000,
        ))))
        .await;
    harness
}

#[tokio::test]
async fn caps_the_verdict_after_a_slow_test() {
    // Arrange
    let mut harness = create_harness_after_a_slow_test().await;

    // Act
    harness.run_seconds(1).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.verdict, Verdict::Graded(Grade::Degraded));
    assert_eq!(view.diagnosis, Diagnosis::LowThroughput);
}

#[tokio::test]
async fn lifts_the_cap_ten_minutes_later() {
    // Arrange
    let mut harness = create_harness_after_a_slow_test().await;

    // Act
    harness.run_seconds(601).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn lifts_the_cap_on_another_network() {
    // Arrange
    let mut harness = create_harness_after_a_slow_test().await;
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::Ethernet, 62)));
    harness.engine.handle(EngineMessage::NetworkChanged).await;

    // Act
    harness.run_seconds(WARM_UP + 1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn never_caps_after_a_test_fast_enough() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness
        .engine
        .handle(EngineMessage::ThroughputEnded(Some(create_throughput(
            90_000_000,
        ))))
        .await;

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn keeps_every_test_in_the_history_under_its_network() {
    // Arrange
    let mut harness = create_harness();
    harness
        .connection
        .set_context(Some(create_context(ConnectionKind::WiFi, 63)));
    harness.run_seconds(2).await;

    // Act
    harness
        .engine
        .handle(EngineMessage::ThroughputEnded(Some(create_throughput(
            90_000_000,
        ))))
        .await;

    // Assert
    let kept = harness.history.list_throughput_tests();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].result, create_throughput(90_000_000));
    assert_eq!(kept[0].network.kind, ConnectionKind::WiFi);
}

#[tokio::test]
async fn keeps_the_loaded_line_out_of_the_verdict_during_a_test() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness
        .engine
        .handle(EngineMessage::ThroughputStarted)
        .await;
    let presented = harness.presenter.count_views();
    // The test fills the line: the probes come back slow.
    harness.set_latency(VERY_SLOW);

    // Act
    harness.run_seconds(15).await;

    // Assert
    assert_eq!(harness.presenter.count_views(), presented);
    assert!(harness.notifier.list_notified().is_empty());
}

#[tokio::test]
async fn measures_on_from_where_it_was_once_the_test_ends() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness
        .engine
        .handle(EngineMessage::ThroughputStarted)
        .await;
    harness.run_seconds(5).await;

    // Act
    harness
        .engine
        .handle(EngineMessage::ThroughputEnded(None))
        .await;
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn measures_again_by_itself_when_a_test_never_says_it_ended() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    harness
        .engine
        .handle(EngineMessage::ThroughputStarted)
        .await;
    let presented = harness.presenter.count_views();

    // Act
    harness.run_seconds(70).await;

    // Assert
    assert!(harness.presenter.count_views() > presented);
}

#[tokio::test]
async fn announces_no_degradation_for_a_slow_test_already_on_screen() {
    // Arrange
    let mut harness = create_harness_after_a_slow_test().await;

    // Act
    harness.run_seconds(5).await;

    // Assert
    assert_eq!(harness.read_last_view().diagnosis, Diagnosis::LowThroughput);
    assert!(harness.notifier.list_notified().is_empty());
}

#[tokio::test]
async fn caps_nothing_after_a_test_that_ended_during_a_sleep() {
    // Arrange
    let mut harness = create_harness();
    harness.warm_up_with_latency(FAST).await;
    let (answer, _answered) = std::sync::mpsc::sync_channel(1);
    harness.engine.handle(EngineMessage::Sleep(answer)).await;
    harness
        .engine
        .handle(EngineMessage::ThroughputEnded(Some(create_throughput(
            400_000,
        ))))
        .await;
    harness.engine.handle(EngineMessage::Wake).await;

    // Act
    harness.run_seconds(WARM_UP + 1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}
