//! Engine seam: messages in, presented verdicts out, with in-memory ports and a manual clock.

use std::time::Duration;

use application::test_support::{InMemoryNetwork, InMemoryVerdictPresenter, ManualClock};
use application::{EngineMessage, MeasurementEngine};
use domain::{Grade, Target, Verdict};

// Documentation-only addresses (RFC 5737): the engine must not depend on which targets are used.
const FIRST_TARGET: &str = "192.0.2.1";
const SECOND_TARGET: &str = "198.51.100.1";
const FAST: Duration = Duration::from_millis(20);
const SLOW: Duration = Duration::from_millis(180);
const VERY_SLOW: Duration = Duration::from_millis(400);

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
    fn set_latency_to_every_target(&self, round_trip: Option<Duration>) {
        self.network
            .set_round_trip(Target::parse(FIRST_TARGET), round_trip);
        self.network
            .set_round_trip(Target::parse(SECOND_TARGET), round_trip);
    }

    async fn run_seconds(&mut self, seconds: u32) {
        for _ in 0..seconds {
            self.clock.advance(Duration::from_secs(1));
            self.engine.handle(EngineMessage::Tick).await;
        }
    }

    fn read_presented_verdict(&self) -> Verdict {
        self.presenter
            .read_last()
            .expect("a view was presented")
            .verdict
    }
}

#[tokio::test]
async fn presents_solid_when_latency_is_low() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency_to_every_target(Some(FAST));

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn presents_measuring_while_no_target_has_answered() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency_to_every_target(None);

    // Act
    harness.run_seconds(3).await;

    // Assert
    assert_eq!(harness.read_presented_verdict(), Verdict::Measuring);
}

#[tokio::test]
async fn presents_the_p90_latency_over_the_last_15_seconds() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency_to_every_target(Some(FAST));
    harness.run_seconds(13).await;
    harness.set_latency_to_every_target(Some(SLOW));

    // Act
    harness.run_seconds(2).await;

    // Assert
    let view = harness.presenter.read_last().expect("a view was presented");
    assert_eq!(view.latency_p90, Some(SLOW));
}

#[tokio::test]
async fn uses_the_median_of_the_targets_that_answered() {
    // Arrange
    let mut harness = create_harness();
    harness
        .network
        .set_round_trip(Target::parse(FIRST_TARGET), Some(FAST));
    harness
        .network
        .set_round_trip(Target::parse(SECOND_TARGET), None);

    // Act
    harness.run_seconds(1).await;

    // Assert
    let view = harness.presenter.read_last().expect("a view was presented");
    assert_eq!(view.latency_p90, Some(FAST));
    assert_eq!(view.verdict, Verdict::Graded(Grade::Solid));
}

#[tokio::test]
async fn shows_a_degradation_only_after_10_sustained_seconds() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency_to_every_target(Some(FAST));
    harness.run_seconds(20).await;
    harness.set_latency_to_every_target(Some(VERY_SLOW));
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
    harness.set_latency_to_every_target(Some(VERY_SLOW));
    harness.run_seconds(20).await;
    harness.set_latency_to_every_target(Some(FAST));
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
    harness.set_latency_to_every_target(Some(FAST));
    harness.run_seconds(20).await;

    // Act
    for _ in 0..6 {
        harness.set_latency_to_every_target(Some(VERY_SLOW));
        harness.run_seconds(1).await;
        harness.set_latency_to_every_target(Some(FAST));
        harness.run_seconds(15).await;
    }

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Solid)
    );
}

#[tokio::test]
async fn grades_latency_between_100_and_300_ms_as_unstable() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency_to_every_target(Some(SLOW));

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Unstable)
    );
}

#[tokio::test]
async fn falls_back_to_measuring_once_a_whole_window_went_unanswered() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency_to_every_target(Some(FAST));
    harness.run_seconds(20).await;
    harness.set_latency_to_every_target(None);

    // Act
    harness.run_seconds(15).await;

    // Assert
    assert_eq!(harness.read_presented_verdict(), Verdict::Measuring);
}

#[tokio::test]
async fn shows_the_first_grade_measured_after_an_outage_at_once() {
    // Arrange
    let mut harness = create_harness();
    harness.set_latency_to_every_target(Some(FAST));
    harness.run_seconds(20).await;
    harness.set_latency_to_every_target(None);
    harness.run_seconds(15).await;
    harness.set_latency_to_every_target(Some(VERY_SLOW));

    // Act
    harness.run_seconds(1).await;

    // Assert
    assert_eq!(
        harness.read_presented_verdict(),
        Verdict::Graded(Grade::Degraded)
    );
}
