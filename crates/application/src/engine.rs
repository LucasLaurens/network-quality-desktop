use std::time::Duration;

use domain::{
    Grade, GradeBounds, Hysteresis, LatencyWindow, LossRate, LossWindow, OfflineDetector,
    ProbeOutcome, Scalable, Target, Thresholds, Verdict, compute_median,
};
use futures_util::future::join_all;
use tokio::sync::mpsc;

use crate::{Clock, LatencyProbe, VerdictPresenter};

/// Seconds with an answer needed before the first verdict.
const WARM_UP_SAMPLES: u32 = 10;

/// Messages the single measurement actor reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMessage {
    /// One second elapsed: probe every target and recompute the verdict.
    Tick,
}

/// One metric as shown to the user: its value, its grade and whether it is close to getting worse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricReading<T> {
    pub value: Option<T>,
    pub grade: Option<Grade>,
    pub is_near_next_threshold: bool,
}

impl<T: Scalable> MetricReading<T> {
    fn compute(value: Option<T>, bounds: &GradeBounds<T>) -> Self {
        Self {
            value,
            grade: value.map(|value| bounds.compute_grade(value)),
            is_near_next_threshold: value.is_some_and(|value| bounds.is_near_next_threshold(value)),
        }
    }
}

/// What the engine hands to the presenters after each tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerdictView {
    pub verdict: Verdict,
    pub latency: MetricReading<Duration>,
    pub jitter: MetricReading<Duration>,
    pub loss: MetricReading<LossRate>,
}

/// Owns every piece of measurement state; nothing else mutates it.
pub struct MeasurementEngine<P, C, V> {
    probe: P,
    clock: C,
    presenter: V,
    targets: Vec<Target>,
    thresholds: Thresholds,
    latency: LatencyWindow,
    loss: LossWindow,
    offline: OfflineDetector,
    hysteresis: Hysteresis,
    samples_since_restart: u32,
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
            latency: LatencyWindow::default(),
            loss: LossWindow::default(),
            offline: OfflineDetector::default(),
            hysteresis: Hysteresis::default(),
            samples_since_restart: 0,
        }
    }

    /// Probes the targets concurrently, so one tick costs one probe timeout at most.
    /// O(k log k + n log n) time and O(k + n) space, k targets and n ≤ 15 window samples.
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
        let outcomes = join_all(
            self.targets
                .iter()
                .map(|target| self.probe.send_probe(*target)),
        )
        .await;
        self.record_outcomes(&outcomes);
        let view = self.compute_view(&outcomes);
        self.presenter.present(&view);
    }

    /// O(k log k) time, O(k) space for k targets.
    fn record_outcomes(&mut self, outcomes: &[ProbeOutcome]) {
        let round_trips: Vec<Option<Duration>> = outcomes
            .iter()
            .map(|outcome| match outcome {
                ProbeOutcome::Answered(round_trip) => Some(*round_trip),
                ProbeOutcome::LostButReachable | ProbeOutcome::Unreachable => None,
            })
            .collect();
        let median = compute_median(&round_trips);
        self.latency.record_second(median);
        let sent = u32::try_from(outcomes.len()).unwrap_or(u32::MAX);
        let lost = u32::try_from(
            round_trips
                .iter()
                .filter(|round_trip| round_trip.is_none())
                .count(),
        )
        .unwrap_or(sent);
        self.loss.record_second(sent, lost);
        if median.is_some() {
            self.samples_since_restart += 1;
        }
    }

    /// O(n log n) time, O(n) space, n ≤ 15 window samples.
    fn compute_view(&mut self, outcomes: &[ProbeOutcome]) -> VerdictView {
        let any_reachable = outcomes
            .iter()
            .any(|outcome| *outcome != ProbeOutcome::Unreachable);
        let latency = MetricReading::compute(self.latency.compute_p90(), &self.thresholds.latency);
        let jitter = MetricReading::compute(self.latency.compute_jitter(), &self.thresholds.jitter);
        let loss = MetricReading::compute(self.loss.compute_rate(), &self.thresholds.loss);
        let verdict = if self.offline.record_second(any_reachable) {
            self.restart();
            Verdict::Offline
        } else if self.samples_since_restart < WARM_UP_SAMPLES {
            Verdict::Measuring
        } else {
            let worst = [latency.grade, jitter.grade, loss.grade]
                .into_iter()
                .flatten()
                .max();
            match worst {
                Some(grade) => self.hysteresis.observe(grade, self.clock.read_now()),
                None => Verdict::Measuring,
            }
        };
        VerdictView {
            verdict,
            latency,
            jitter,
            loss,
        }
    }

    /// After an outage the history no longer describes the connection: measure again from scratch.
    fn restart(&mut self) {
        self.latency = LatencyWindow::default();
        self.loss = LossWindow::default();
        self.hysteresis = Hysteresis::default();
        self.samples_since_restart = 0;
    }
}
