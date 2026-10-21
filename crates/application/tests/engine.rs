//! Engine seam: messages in, presented verdicts out, with in-memory ports and a manual clock.

use std::time::Duration;

use application::test_support::{InMemoryNetwork, InMemoryVerdictPresenter, ManualClock};
use application::{EngineMessage, MeasurementEngine, VerdictView};
use domain::{Grade, ProbeOutcome, Target, Verdict};

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
    engine: MeasurementEngine<InMemoryNetwork, ManualClock, InMemoryVerdictPresenter>,
}

fn create_harness() -> Harness {
    let network = InMemoryNetwork::default();
    let clock = ManualClock::default();
    let presenter = InMemoryVerdictPresenter::default();
    let targets = vec![Target::parse(FIRST_TARGET), Target::parse(SECOND_TARGET)];
    let engine = MeasurementEngine::new(network.clone(), clock.clone(), presenter.clone(), targets);
    Harness {
        network,
        clock,
        presenter,
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
