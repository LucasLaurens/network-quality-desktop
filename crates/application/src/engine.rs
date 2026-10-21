use std::net::Ipv4Addr;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use domain::{
    ConnectionContext, ConnectionKind, Diagnosis, Fault, GatewayWindow, Grade, GradeBounds,
    Hysteresis, LatencyWindow, LocalSegment, LossRate, LossWindow, MeasuringCause, Metric,
    OfflineDetector, ProbeOutcome, Scalable, SecondPoint, Target, Thresholds, Unmeasurable,
    Verdict, WifiLink, compute_median,
};
use futures_util::future::{join, join_all, join3};
use tokio::sync::{mpsc, oneshot};

use crate::{
    Clock, ConnectionContextReader, GatewayProbe, HistoryRecorder, HistoryTracker, LatencyProbe,
    MeasuredSecond, VerdictPresenter, WifiReader,
};

/// Seconds with an answer needed before the first verdict.
pub const WARM_UP_SAMPLES: u32 = 10;
/// Reading the connection costs milliseconds of system calls: every 10 seconds, not every tick.
pub const CONTEXT_REFRESH_SECONDS: u32 = 10;
/// A box that answered within this time while nothing else does has lost its Internet access.
const GATEWAY_ANSWER_SECONDS: usize = 5;

/// Messages the single measurement actor reacts to.
#[derive(Debug)]
pub enum EngineMessage {
    /// One second elapsed: probe every target and recompute the verdict.
    Tick,
    /// The system reports an interface or address change: read the connection at the next tick.
    /// A burst of events within one second costs a single reading.
    NetworkChanged,
    /// Closing: write the minute in progress, then answer.
    Flush(SyncSender<()>),
    /// The last hour, second by second, for the one-hour chart.
    ReadLastHour(oneshot::Sender<Vec<SecondPoint>>),
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
#[derive(Debug, Clone, PartialEq)]
pub struct VerdictView {
    pub verdict: Verdict,
    pub diagnosis: Diagnosis,
    /// `None` while the metric cannot be computed yet, and while offline.
    pub latency: Option<MetricReading<Duration>>,
    pub jitter: Option<MetricReading<Duration>>,
    pub loss: Option<MetricReading<LossRate>>,
    /// `None` while no default route exists, or before the first read.
    pub context: Option<ConnectionContext>,
    /// The link to the gateway: only says where a problem lies, never sets the verdict.
    pub local: LocalSegment,
    /// `None` without Wi-Fi, or before the first reading.
    pub wifi: Option<WifiLink>,
}

/// Owns every piece of measurement state; nothing else mutates it.
pub struct MeasurementEngine<P, C, V, R, G, W, H> {
    history_recorder: H,
    history: HistoryTracker,
    probe: P,
    clock: C,
    presenter: V,
    context_reader: R,
    gateway_probe: G,
    gateway: GatewayWindow,
    wifi_reader: W,
    wifi: Option<WifiLink>,
    context: Option<ConnectionContext>,
    /// The last network seen, kept through a moment without route: switching from one Wi-Fi to
    /// another passes through one.
    last_network: Option<ConnectionContext>,
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
    measuring_cause: MeasuringCause,
}

/// The ports the engine talks to, gathered so the constructor stays readable.
pub struct EnginePorts<P, C, V, R, G, W, H> {
    pub history_recorder: H,
    pub probe: P,
    pub clock: C,
    pub presenter: V,
    pub context_reader: R,
    pub gateway_probe: G,
    pub wifi_reader: W,
}

impl<P, C, V, R, G, W, H> MeasurementEngine<P, C, V, R, G, W, H>
where
    H: HistoryRecorder,
    P: LatencyProbe,
    C: Clock,
    V: VerdictPresenter,
    R: ConnectionContextReader,
    G: GatewayProbe,
    W: WifiReader,
{
    #[must_use]
    pub fn new(ports: EnginePorts<P, C, V, R, G, W, H>, targets: Vec<Target>) -> Self {
        let EnginePorts {
            history_recorder,
            probe,
            clock,
            presenter,
            context_reader,
            gateway_probe,
            wifi_reader,
        } = ports;
        let jitter = vec![LatencyWindow::default(); targets.len()];
        Self {
            history_recorder,
            history: HistoryTracker::default(),
            probe,
            clock,
            presenter,
            context_reader,
            gateway_probe,
            gateway: GatewayWindow::default(),
            wifi_reader,
            wifi: None,
            context: None,
            last_network: None,
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
            measuring_cause: MeasuringCause::FirstMeasures,
        }
    }

    /// Probes the targets concurrently, so one tick costs one probe timeout at most.
    /// O(k log k + n log n) time and O(k + n) space, k targets and n ≤ 15 window samples.
    pub async fn handle(&mut self, message: EngineMessage) {
        match message {
            EngineMessage::Tick => self.measure_one_second().await,
            EngineMessage::NetworkChanged => self.seconds_since_context = CONTEXT_REFRESH_SECONDS,
            EngineMessage::Flush(answer) => self.history.flush(answer, &self.history_recorder),
            EngineMessage::ReadLastHour(answer) => {
                // The asker may have given up waiting: nothing to do then.
                let _ = answer.send(self.history.list_last_hour());
            }
        }
    }

    /// Runs the actor until every sender is dropped.
    pub async fn run(mut self, mut inbox: mpsc::Receiver<EngineMessage>) {
        while let Some(message) = inbox.recv().await {
            self.handle(message).await;
        }
    }

    async fn measure_one_second(&mut self) {
        // Read before the probes: they may take a second, and two ticks must not share one.
        let utc = self.clock.read_utc_seconds();
        let is_context_due = self.seconds_since_context >= CONTEXT_REFRESH_SECONDS;
        let gateway = self.find_gateway();
        // The context and the gateway are read alongside the probes: none delays the others.
        let (outcomes, context, gateway_echo) = join3(
            join_all(
                self.targets
                    .iter()
                    .map(|target| self.probe.send_probe(*target)),
            ),
            async {
                if is_context_due {
                    // Read with the context: the radio changes slowly, and Windows shows its
                    // location icon on every reading.
                    Some(
                        join(
                            self.context_reader.read_context(),
                            self.wifi_reader.read_wifi(),
                        )
                        .await,
                    )
                } else {
                    None
                }
            },
            async {
                match gateway {
                    Some(address) => Some(self.gateway_probe.ping_gateway(address).await),
                    None => None,
                }
            },
        )
        .await;
        if let Some(echo) = gateway_echo {
            self.gateway.record_second(echo);
        }
        if let Some((context, wifi)) = context {
            self.record_context(context);
            self.wifi = wifi;
        }
        self.seconds_since_context += 1;
        let (latency, probes_lost) = self.record_outcomes(&outcomes);
        let view = self.compute_view(&outcomes);
        self.record_history(utc, latency, probes_lost, &view);
        self.presenter.present(&view);
    }

    /// Starts measuring again when another network took over; noise (an address renewed, a
    /// virtual interface coming and going) leaves the network and the verdict as they are. O(1).
    fn record_context(&mut self, context: Option<ConnectionContext>) {
        if let Some(current) = context {
            self.last_network = Some(match self.last_network {
                Some(previous) if previous.is_same_network(&current) => {
                    current.complete_with(&previous)
                }
                Some(_) => {
                    self.restart();
                    self.gateway = GatewayWindow::default();
                    self.measuring_cause = MeasuringCause::NewNetwork;
                    current
                }
                None => current,
            });
        }
        self.context = context;
        self.seconds_since_context = 0;
    }

    /// A cellular modem has no box to ping, and a VPN carries the targets past the box: pinging
    /// it would blame the box or the operator for the tunnel.
    fn find_gateway(&self) -> Option<Ipv4Addr> {
        self.context
            .filter(|context| context.kind != ConnectionKind::Cellular && !context.is_vpn)
            .and_then(|context| context.gateway_address)
    }

    /// O(n log n) for n ≤ 15 samples.
    fn compute_local_segment(&self) -> LocalSegment {
        if self.find_gateway().is_none() {
            return LocalSegment::Unmeasurable(Unmeasurable::NoGateway);
        }
        self.gateway.compute_segment()
    }

    /// Why nothing answers: no network at all, a box cut from the Internet, or no way to tell.
    /// Judged over a few seconds, so one lost ping does not flip the sentence. O(1).
    fn diagnose_offline(&self) -> Diagnosis {
        match self.context {
            None => Diagnosis::NoNetwork,
            Some(_) if self.gateway.has_answered_within(GATEWAY_ANSWER_SECONDS) => {
                Diagnosis::GatewayWithoutInternet
            }
            Some(_) => Diagnosis::NoServerReachable,
        }
    }

    /// O(k log k) time, O(k) space for k targets.
    /// Returns the second's median latency and its lost probes.
    fn record_outcomes(&mut self, outcomes: &[ProbeOutcome]) -> (Option<Duration>, u32) {
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
        (median, lost)
    }

    /// The Wi-Fi name labels the network only when Wi-Fi carries the traffic. O(1).
    fn record_history(
        &mut self,
        utc: i64,
        latency: Option<Duration>,
        probes_lost: u32,
        view: &VerdictView,
    ) {
        let name = self
            .wifi
            .as_ref()
            .filter(|_| {
                self.context
                    .is_some_and(|context| context.is_carried_by_wifi())
            })
            .and_then(|link| link.name.clone());
        let second = MeasuredSecond {
            utc,
            latency,
            probes_sent: u32::try_from(self.targets.len()).unwrap_or(u32::MAX),
            probes_lost,
            verdict: view.verdict,
        };
        self.history
            .record_second(&second, self.context, name, &self.history_recorder);
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
                diagnosis: self.diagnose_offline(),
                latency: None,
                jitter: None,
                loss: None,
                context: self.context,
                local: self.compute_local_segment(),
                wifi: self.wifi.clone(),
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
        let local = self.compute_local_segment();
        let wifi_in_use = self.wifi.as_ref().filter(|_| {
            self.context
                .is_some_and(|context| context.is_carried_by_wifi())
        });
        let fault = Fault::attribute(
            ConnectionKind::from_context(self.context),
            local,
            wifi_in_use,
        );
        let diagnosis = match verdict {
            Verdict::Offline => self.diagnose_offline(),
            Verdict::Measuring => Diagnosis::Measuring {
                collected: self.samples_since_restart.min(WARM_UP_SAMPLES),
                needed: WARM_UP_SAMPLES,
                cause: self.measuring_cause,
            },
            Verdict::Graded(displayed) => diagnose_graded(displayed, &grades, fault),
        };
        VerdictView {
            verdict,
            diagnosis,
            latency,
            jitter,
            loss,
            context: self.context,
            local,
            wifi: self.wifi.clone(),
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
        self.measuring_cause = MeasuringCause::FirstMeasures;
        // The way back may be another network: read it at once (every second while offline).
        self.seconds_since_context = CONTEXT_REFRESH_SECONDS;
    }
}

/// Names the metric behind a graded verdict. While Solid, a metric close to (or already past) its
/// threshold is reported, since the hysteresis may not show it yet; while a worse state is still
/// shown with every metric back to Solid, the connection is recovering. O(1): three metrics.
fn diagnose_graded(
    displayed: Grade,
    grades: &[(Metric, Option<(Grade, bool)>); 3],
    fault: Fault,
) -> Diagnosis {
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
        (_, Some((grade, _, metric))) if grade > Grade::Solid => {
            Diagnosis::OutOfThreshold { metric, fault }
        }
        // Worse state still shown while every metric is back to Solid: the hysteresis is waiting.
        _ => Diagnosis::Recovering,
    }
}
