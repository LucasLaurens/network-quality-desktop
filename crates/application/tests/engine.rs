//! Engine seam: messages in, presented verdicts out, with in-memory ports and a manual clock.

use std::net::Ipv4Addr;
use std::time::Duration;

use application::test_support::{
    InMemoryConnection, InMemoryNetwork, InMemoryVerdictPresenter, ManualClock,
};
use application::{CONTEXT_REFRESH_SECONDS, EngineMessage, MeasurementEngine, VerdictView};
use domain::{
    ConnectionContext, ConnectionKind, Diagnosis, Grade, MacAddress, MeasuringCause, Metering,
    Metric, ProbeOutcome, Target, Verdict,
};

// Documentation-only addresses (RFC 5737): the engine must not depend on which targets are used.
const FIRST_TARGET: &str = "192.0.2.1";
const SECOND_TARGET: &str = "198.51.100.1";
const FAST: Duration = Duration::from_millis(20);
const NEAR_LIMIT: Duration = Duration::from_millis(85);
const SLOW: Duration = Duration::from_millis(180);
const VERY_SLOW: Duration = Duration::from_millis(400);
/// Seconds the engine needs before its first verdict.
const WARM_UP: u32 = 10;

struct Harness {
    network: InMemoryNetwork,
    clock: ManualClock,
    presenter: InMemoryVerdictPresenter,
    connection: InMemoryConnection,
    engine: MeasurementEngine<
        InMemoryNetwork,
        ManualClock,
        InMemoryVerdictPresenter,
        InMemoryConnection,
    >,
}

fn create_harness() -> Harness {
    let network = InMemoryNetwork::default();
    let clock = ManualClock::default();
    let presenter = InMemoryVerdictPresenter::default();
    let connection = InMemoryConnection::default();
    let targets = vec![Target::parse(FIRST_TARGET), Target::parse(SECOND_TARGET)];
    let engine = MeasurementEngine::new(
        network.clone(),
        clock.clone(),
        presenter.clone(),
        connection.clone(),
        targets,
    );
    Harness {
        network,
        clock,
        presenter,
        connection,
        engine,
    }
}

impl Harness {
    fn set_every_target(&self, outcome: ProbeOutcome) {
        self.network
            .set_outcome(Target::parse(FIRST_TARGET), outcome);
        self.network
            .set_outcome(Target::parse(SECOND_TARGET), outcome);
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
        .set_outcome(Target::parse(FIRST_TARGET), ProbeOutcome::Answered(FAST));
    harness
        .network
        .set_outcome(Target::parse(SECOND_TARGET), ProbeOutcome::LostButReachable);

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
        .set_outcome(Target::parse(SECOND_TARGET), ProbeOutcome::LostButReachable);

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
        .set_outcome(Target::parse(FIRST_TARGET), ProbeOutcome::Answered(FAST));
    harness
        .network
        .set_outcome(Target::parse(SECOND_TARGET), ProbeOutcome::LostButReachable);

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
        .set_outcome(Target::parse(FIRST_TARGET), ProbeOutcome::Unreachable);
    harness
        .network
        .set_outcome(Target::parse(SECOND_TARGET), ProbeOutcome::LostButReachable);

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
        .set_outcome(Target::parse(FIRST_TARGET), ProbeOutcome::Answered(FAST));

    // Act: the second target, steadily slower, loses every other echo.
    for second in 0..WARM_UP * 2 {
        let outcome = if second % 2 == 0 {
            ProbeOutcome::Answered(SLOW)
        } else {
            ProbeOutcome::LostButReachable
        };
        harness
            .network
            .set_outcome(Target::parse(SECOND_TARGET), outcome);
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
        .set_outcome(Target::parse(SECOND_TARGET), ProbeOutcome::LostButReachable);

    // Act: one lost probe out of 22 would read as Unstable on so few probes.
    harness.run_seconds(1).await;
    let early = harness.read_last_view();
    harness
        .network
        .set_outcome(Target::parse(SECOND_TARGET), ProbeOutcome::Answered(FAST));
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
        Diagnosis::OutOfThreshold(Metric::Latency)
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
    assert_eq!(
        latency.bounds,
        domain::Thresholds::for_video_calls().latency
    );
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
