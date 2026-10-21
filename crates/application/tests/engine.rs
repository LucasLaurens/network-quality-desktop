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
    // Act
    let mut harness = create_harness();
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
    assert_eq!(harness.read_last_view().latency.value, Some(SLOW));
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
    assert_eq!(harness.read_last_view().latency.value, Some(FAST));
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
    assert_eq!(view.jitter.value, Some(Duration::from_millis(40)));
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
    assert_eq!(view.loss.value.map(domain::LossRate::to_percent), Some(2.5));
    assert_eq!(view.loss.grade, Some(Grade::Unstable));
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

    // Act: half of the probes are lost while the latency stays low.
    harness.run_seconds(WARM_UP).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.latency.grade, Some(Grade::Solid));
    assert_eq!(view.loss.grade, Some(Grade::Degraded));
    assert_eq!(view.verdict, Verdict::Graded(Grade::Degraded));
}

#[tokio::test]
async fn flags_a_metric_within_20_percent_of_its_next_threshold() {
    // Act
    let mut harness = create_harness();
    harness.warm_up_with_latency(NEAR_LIMIT).await;

    // Assert
    let view = harness.read_last_view();
    assert_eq!(view.latency.grade, Some(Grade::Solid));
    assert!(view.latency.is_near_next_threshold);
    assert!(!view.jitter.is_near_next_threshold);
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
