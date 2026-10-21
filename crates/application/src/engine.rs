use std::time::Duration;

use domain::{
    ConnectionContext, Diagnosis, Grade, GradeBounds, Hysteresis, LatencyWindow, LossRate,
    LossWindow, Metric, OfflineDetector, ProbeOutcome, Scalable, Target, Thresholds, Verdict,
    compute_median,
};
use futures_util::future::join_all;
use tokio::sync::mpsc;

use crate::{Clock, ConnectionContextReader, LatencyProbe, VerdictPresenter};

/// Seconds with an answer needed before the first verdict.
pub const WARM_UP_SAMPLES: u32 = 10;
/// Reading the connection costs milliseconds of system calls: every 10 seconds, not every tick.
pub const CONTEXT_REFRESH_SECONDS: u32 = 10;

/// Messages the single measurement actor reacts to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineMessage {
    /// One second elapsed: probe every target and recompute the verdict.
    Tick,
}

/// One metric as shown to the user: its value, its grade and whether it is close to getting worse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricReading<T> {
    pub value: T,
    pub grade: Grade,
    pub is_near_next_threshold: bool,
    /// The thresholds the value was graded against, so presenters never redefine them.
    pub bounds: GradeBounds<T>,
}

impl<T: Scalable> MetricReading<T> {
    fn compute(value: T, bounds: &GradeBounds<T>) -> Self {
        Self {
            value,
            grade: bounds.compute_grade(value),
            is_near_next_threshold: bounds.is_near_next_threshold(value),
            bounds: *bounds,
        }
    }
}

/// What the engine hands to the presenters after each tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerdictView {
    pub verdict: Verdict,
    pub diagnosis: Diagnosis,
    /// `None` while the metric cannot be computed yet, and while offline.
    pub latency: Option<MetricReading<Duration>>,
    pub jitter: Option<MetricReading<Duration>>,
    pub loss: Option<MetricReading<LossRate>>,
    /// `None` while no default route exists, or before the first read.
    pub context: Option<ConnectionContext>,
}

/// Owns every piece of measurement state; nothing else mutates it.
pub struct MeasurementEngine<P, C, V, R> {
    probe: P,
    clock: C,
    presenter: V,
    context_reader: R,
    context: Option<ConnectionContext>,
    seconds_since_context: u32,
    targets: Vec<Target>,
    thresholds: Thresholds,
    latency: LatencyWindow,
    /// One window per target: jitter compares a target with itself, never two targets' median.
    jitter: Vec<LatencyWindow>,
    loss: LossWindow,
    offline: OfflineDetector,
    hysteresis: Hysteresis,
    samples_since_restart: u32,
}

impl<P: LatencyProbe, C: Clock, V: VerdictPresenter, R: ConnectionContextReader>
    MeasurementEngine<P, C, V, R>
{
    #[must_use]
    pub fn new(probe: P, clock: C, presenter: V, context_reader: R, targets: Vec<Target>) -> Self {
        let jitter = vec![LatencyWindow::default(); targets.len()];
        Self {
            probe,
            clock,
            presenter,
            context_reader,
            context: None,
            // Due at once: the first tick reads the connection.
            seconds_since_context: CONTEXT_REFRESH_SECONDS,
            targets,
            thresholds: Thresholds::for_video_calls(),
            latency: LatencyWindow::default(),
            jitter,
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
        if self.seconds_since_context >= CONTEXT_REFRESH_SECONDS {
            self.context = self.context_reader.read_context().await;
            self.seconds_since_context = 0;
        }
        self.seconds_since_context += 1;
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
        for (window, round_trip) in self.jitter.iter_mut().zip(&round_trips) {
            window.record_second(*round_trip);
        }
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
            self.samples_since_restart = self.samples_since_restart.saturating_add(1);
        }
    }

    /// O(k·n log n) time, O(k·n) space, k targets and n ≤ 15 window samples.
    fn compute_view(&mut self, outcomes: &[ProbeOutcome]) -> VerdictView {
        let any_reachable = outcomes
            .iter()
            .any(|outcome| *outcome != ProbeOutcome::Unreachable);
        if self.offline.record_second(any_reachable) {
            self.restart();
            return VerdictView {
                verdict: Verdict::Offline,
                diagnosis: Diagnosis::NoServerReachable,
                latency: None,
                jitter: None,
                loss: None,
                context: self.context,
            };
        }
        let thresholds = self.thresholds;
        let latency = self
            .latency
            .compute_p90()
            .map(|value| MetricReading::compute(value, &thresholds.latency));
        let jitter = self
            .compute_jitter()
            .map(|value| MetricReading::compute(value, &thresholds.jitter));
        let loss = self
            .loss
            .compute_rate()
            .map(|value| MetricReading::compute(value, &thresholds.loss));
        let worst = [
            latency.map(|reading| reading.grade),
            jitter.map(|reading| reading.grade),
            loss.map(|reading| reading.grade),
        ]
        .into_iter()
        .flatten()
        .max();
        let verdict = match worst {
            Some(grade) if self.samples_since_restart >= WARM_UP_SAMPLES => {
                self.hysteresis.observe(grade, self.clock.read_now())
            }
            _ => Verdict::Measuring,
        };
        let grades = [
            (
                Metric::Latency,
                latency.map(|reading| (reading.grade, reading.is_near_next_threshold)),
            ),
            (
                Metric::Jitter,
                jitter.map(|reading| (reading.grade, reading.is_near_next_threshold)),
            ),
            (
                Metric::Loss,
                loss.map(|reading| (reading.grade, reading.is_near_next_threshold)),
            ),
        ];
        let diagnosis = match verdict {
            Verdict::Offline => Diagnosis::NoServerReachable,
            Verdict::Measuring => Diagnosis::Measuring {
                collected: self.samples_since_restart.min(WARM_UP_SAMPLES),
                needed: WARM_UP_SAMPLES,
            },
            Verdict::Graded(displayed) => diagnose_graded(displayed, &grades),
        };
        VerdictView {
            verdict,
            diagnosis,
            latency,
            jitter,
            loss,
            context: self.context,
        }
    }

    /// Mean of the per-target jitters that can be computed. O(k·n) time, O(k) space.
    fn compute_jitter(&self) -> Option<Duration> {
        let jitters: Vec<Duration> = self
            .jitter
            .iter()
            .filter_map(LatencyWindow::compute_jitter)
            .collect();
        let count = u32::try_from(jitters.len())
            .ok()
            .filter(|count| *count > 0)?;
        Some(jitters.iter().sum::<Duration>() / count)
    }

    /// After an outage the history no longer describes the connection: measure again from scratch.
    fn restart(&mut self) {
        self.latency = LatencyWindow::default();
        self.jitter.fill(LatencyWindow::default());
        self.loss = LossWindow::default();
        self.hysteresis = Hysteresis::default();
        self.samples_since_restart = 0;
    }
}

/// Names the metric behind a graded verdict. While Solid, a metric close to (or already past) its
/// threshold is reported, since the hysteresis may not show it yet; while a worse state is still
/// shown with every metric back to Solid, the connection is recovering. O(1): three metrics.
fn diagnose_graded(displayed: Grade, grades: &[(Metric, Option<(Grade, bool)>); 3]) -> Diagnosis {
    let worst = grades
        .iter()
        .filter_map(|(metric, reading)| reading.map(|(grade, near)| (grade, near, *metric)))
        .max_by_key(|(grade, _, metric)| (*grade, std::cmp::Reverse(*metric)));
    match (displayed, worst) {
        (Grade::Solid, Some((grade, _, metric))) if grade > Grade::Solid => {
            Diagnosis::NearThreshold(metric)
        }
        (Grade::Solid, _) => grades
            .iter()
            .find(|(_, reading)| reading.is_some_and(|(_, near)| near))
            .map_or(Diagnosis::Healthy, |(metric, _)| {
                Diagnosis::NearThreshold(*metric)
            }),
        (_, Some((grade, _, metric))) if grade > Grade::Solid => Diagnosis::OutOfThreshold(metric),
        // Worse state still shown while every metric is back to Solid: the hysteresis is waiting.
        _ => Diagnosis::Recovering,
    }
}
