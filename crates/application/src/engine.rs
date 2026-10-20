use std::time::Duration;

use domain::{Hysteresis, LatencyWindow, Target, Thresholds, Verdict, compute_median};
use futures_util::future::join_all;
use tokio::sync::mpsc;

use crate::{Clock, LatencyProbe, VerdictPresenter};

/// Messages the single measurement actor reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMessage {
    /// One second elapsed: probe every target and recompute the verdict.
    Tick,
}

/// What the engine hands to the presenters after each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerdictView {
    pub verdict: Verdict,
    pub latency_p90: Option<Duration>,
}

/// Owns every piece of measurement state; nothing else mutates it.
pub struct MeasurementEngine<P, C, V> {
    probe: P,
    clock: C,
    presenter: V,
    targets: Vec<Target>,
    thresholds: Thresholds,
    window: LatencyWindow,
    hysteresis: Hysteresis,
}

impl<P: LatencyProbe, C: Clock, V: VerdictPresenter> MeasurementEngine<P, C, V> {
    #[must_use]
    pub fn new(probe: P, clock: C, presenter: V, targets: Vec<Target>) -> Self {
        Self {
            probe,
            clock,
            presenter,
            targets,
            thresholds: Thresholds::for_video_calls(),
            window: LatencyWindow::default(),
            hysteresis: Hysteresis::default(),
        }
    }

    /// Probes the targets concurrently, so one tick costs one probe timeout at most.
    /// O(k log k) time for k targets plus O(1) window work.
    pub async fn handle(&mut self, message: EngineMessage) {
        match message {
            EngineMessage::Tick => self.measure_one_second().await,
        }
    }

    /// Runs the actor until every sender is dropped.
    pub async fn run(mut self, mut inbox: mpsc::Receiver<EngineMessage>) {
        while let Some(message) = inbox.recv().await {
            self.handle(message).await;
        }
    }

    async fn measure_one_second(&mut self) {
        let round_trips = join_all(
            self.targets
                .iter()
                .map(|target| self.probe.measure_round_trip(*target)),
        )
        .await;
        self.window.record_second(compute_median(&round_trips));
        let latency_p90 = self.window.compute_p90();
        let verdict = match latency_p90 {
            Some(p90) => {
                let observed = self.thresholds.latency.compute_grade(p90);
                self.hysteresis.observe(observed, self.clock.read_now())
            }
            None => self.hysteresis.read_displayed(),
        };
        self.presenter.present(&VerdictView {
            verdict,
            latency_p90,
        });
    }
}
