use std::net::Ipv4Addr;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use domain::{
    ConnectionContext, ConnectionKind, DegradationWatch, Diagnosis, Fault, GatewayWindow, Grade,
    GradeBounds, Hysteresis, LatencyWindow, LocalSegment, LossRate, LossWindow, MeasuringCause,
    Metric, OfflineDetector, ProbeOutcome, Scalable, Settings, Target, Thresholds, Unmeasurable,
    Verdict, WifiLink, compute_median,
};
use futures_util::future::{join, join_all, join3};
use tokio::sync::{mpsc, oneshot};

use crate::{
    Clock, ConnectionContextReader, GatewayProbe, HistoryRecorder, HistoryTracker, LatencyProbe,
    LiveHistory, MeasuredSecond, Notifier, VerdictPresenter, WifiReader, list_targets,
};

/// Seconds with an answer needed before the first verdict.
pub const WARM_UP_SAMPLES: u32 = 10;
/// Reading the connection costs milliseconds of system calls: every 10 seconds, not every tick.
pub const CONTEXT_REFRESH_SECONDS: u32 = 10;
/// A box that answered within this time while nothing else does has lost its Internet access.
const GATEWAY_ANSWER_SECONDS: usize = 5;
/// A wall-clock jump this long between two ticks, or during one, means the computer slept: a
/// slow tick lasts a probe timeout, never this long.
const WAKE_GAP_SECONDS: i64 = 10;

/// Whether the engine measures: it stops at the sleep notice and starts again at the wake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wakefulness {
    /// The wall clock at the previous tick, to notice a sleep the system did not announce.
    Awake {
        last_tick_utc: Option<i64>,
    },
    Asleep {
        since: i64,
    },
}

/// O(1).
const fn is_wake_gap(from: i64, to: i64) -> bool {
    to - from > WAKE_GAP_SECONDS
}

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
    /// The last hour and the current network, for the dashboard.
    ReadLiveHistory(oneshot::Sender<LiveHistory>),
    /// The system is about to sleep: write the minute in progress, answer, then measure nothing.
    Sleep(SyncSender<()>),
    /// The system woke up: measure again from scratch.
    Wake,
    /// The user saved the settings: other servers mean measuring again from scratch.
    SettingsChanged(Settings),
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
pub struct MeasurementEngine<P, C, V, R, G, W, H, N> {
    history_recorder: H,
    notifier: N,
    degradation: DegradationWatch,
    /// The setting « Prévenir quand la connexion se dégrade ».
    notifies_on_degradation: bool,
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
    wakefulness: Wakefulness,
}

/// The ports the engine talks to, gathered so the constructor stays readable.
pub struct EnginePorts<P, C, V, R, G, W, H, N> {
    pub history_recorder: H,
    pub notifier: N,
    pub probe: P,
    pub clock: C,
    pub presenter: V,
    pub context_reader: R,
    pub gateway_probe: G,
    pub wifi_reader: W,
}

impl<P, C, V, R, G, W, H, N> MeasurementEngine<P, C, V, R, G, W, H, N>
where
    H: HistoryRecorder,
    N: Notifier,
    P: LatencyProbe,
    C: Clock,
    V: VerdictPresenter,
    R: ConnectionContextReader,
    G: GatewayProbe,
    W: WifiReader,
{
    #[must_use]
    pub fn new(ports: EnginePorts<P, C, V, R, G, W, H, N>, targets: Vec<Target>) -> Self {
        let EnginePorts {
            history_recorder,
            notifier,
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
            notifier,
            degradation: DegradationWatch::default(),
            notifies_on_degradation: Settings::default().notifies_on_degradation,
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
            wakefulness: Wakefulness::Awake {
                last_tick_utc: None,
            },
        }
    }

    /// Probes the targets concurrently, so one tick costs one probe timeout at most.
    /// O(k log k + n log n) time and O(k + n) space, k targets and n ≤ 15 window samples.
    pub async fn handle(&mut self, message: EngineMessage) {
        match message {
            EngineMessage::Tick => self.measure_one_second().await,
            EngineMessage::NetworkChanged => self.seconds_since_context = CONTEXT_REFRESH_SECONDS,
            EngineMessage::Flush(answer) => self.history.flush(answer, &self.history_recorder),
            EngineMessage::ReadLiveHistory(answer) => {
                // The asker may have given up waiting: nothing to do then.
                let _ = answer.send(self.history.read_live());
            }
            EngineMessage::Sleep(answer) => {
                let utc = self.clock.read_utc_seconds();
                self.history
                    .record_sleep(utc, answer, &self.history_recorder);
                self.wakefulness = Wakefulness::Asleep { since: utc };
            }
            EngineMessage::SettingsChanged(settings) => self.apply_settings(settings),
            // Already awake: the gap between two ticks came first, and was handled then.
            EngineMessage::Wake => {
                if matches!(self.wakefulness, Wakefulness::Asleep { .. }) {
                    let utc = self.clock.read_utc_seconds();
                    self.wake(utc);
                }
            }
        }
    }

    /// Runs the actor until every sender is dropped. `urgent` (sleep, quitting) jumps the queue
    /// of ticks: a notice waits for one tick in progress at most, never for those behind it.
    pub async fn run(
        mut self,
        mut inbox: mpsc::Receiver<EngineMessage>,
        mut urgent: mpsc::Receiver<EngineMessage>,
    ) {
        loop {
            let message = tokio::select! {
                biased;
                Some(message) = urgent.recv() => message,
                Some(message) = inbox.recv() => message,
                else => break,
            };
            self.handle(message).await;
        }
    }

    async fn measure_one_second(&mut self) {
        // Read before the probes: they may take a second, and two ticks must not share one.
        let utc = self.clock.read_utc_seconds();
        match self.wakefulness {
            // A wake notice can be lost, and the clock set back: either way the computer is awake.
            Wakefulness::Asleep { since } if !is_wake_gap(since, utc) && utc >= since => return,
            Wakefulness::Asleep { .. } => self.wake(utc),
            Wakefulness::Awake {
                last_tick_utc: Some(last),
            } if is_wake_gap(last, utc) => self.wake(utc),
            Wakefulness::Awake { .. } => {}
        }
        self.wakefulness = Wakefulness::Awake {
            last_tick_utc: Some(utc),
        };
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
        let after = self.clock.read_utc_seconds();
        if is_wake_gap(utc, after) {
            // These answers waited through a sleep: they describe nothing.
            self.wake(after);
            return;
        }
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
        self.present(&view);
    }

    /// Shows the view, and announces a durable degradation when the user wants it. O(1).
    fn present(&mut self, view: &VerdictView) {
        self.presenter.present(view);
        let is_degraded = self
            .degradation
            .observe(view.verdict, self.clock.read_now());
        if is_degraded && self.notifies_on_degradation {
            self.notifier.notify_degradation(view);
        }
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
                    self.restart_for(MeasuringCause::NewNetwork);
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
            Verdict::Measuring => self.compose_measuring_diagnosis(),
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

    /// The servers change the measurement; the notifications only what is announced. O(k) for
    /// k targets.
    fn apply_settings(&mut self, settings: Settings) {
        self.notifies_on_degradation = settings.notifies_on_degradation;
        let targets = list_targets(settings.servers);
        if targets == self.targets {
            return;
        }
        self.jitter = vec![LatencyWindow::default(); targets.len()];
        self.targets = targets;
        self.restart_for(MeasuringCause::NewServers);
    }

    const fn compose_measuring_diagnosis(&self) -> Diagnosis {
        Diagnosis::Measuring {
            collected: if self.samples_since_restart < WARM_UP_SAMPLES {
                self.samples_since_restart
            } else {
                WARM_UP_SAMPLES
            },
            needed: WARM_UP_SAMPLES,
            cause: self.measuring_cause,
        }
    }

    /// Measures again from scratch, reads the connection at once and says so right away.
    /// O(n log n) for the n ≤ 15 gateway samples behind the local segment.
    fn wake(&mut self, utc: i64) {
        self.wakefulness = Wakefulness::Awake {
            last_tick_utc: Some(utc),
        };
        self.restart_for(MeasuringCause::Wake);
        self.offline = OfflineDetector::default();
        self.history.record_wake(utc, &self.history_recorder);
        self.present(&VerdictView {
            verdict: Verdict::Measuring,
            diagnosis: self.compose_measuring_diagnosis(),
            latency: None,
            jitter: None,
            loss: None,
            context: self.context,
            local: self.compute_local_segment(),
            wifi: self.wifi.clone(),
        });
    }

    /// Another network, or back from sleep: nothing measured before applies, the box included.
    fn restart_for(&mut self, cause: MeasuringCause) {
        self.restart();
        self.gateway = GatewayWindow::default();
        self.measuring_cause = cause;
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
