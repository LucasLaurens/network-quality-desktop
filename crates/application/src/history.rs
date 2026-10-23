//! The measurement history: what the engine hands over each minute, and how the single writer
//! turns minutes into hours and keeps 30 days of minutes.

use std::sync::mpsc::SyncSender;
use std::time::Duration;

use domain::{
    Aggregate, ConnectionContext, ConnectionKind, CountedState, Granularity, MacAddress,
    MinuteAccumulator, SecondPoint, SecondRing, SecondSample, ThroughputResult, Verdict, agree,
};

use crate::{HistoryRecorder, MeasurementRepository};

/// Minutes older than this are dropped once their hour is kept.
pub const MINUTE_RETENTION: Duration = Duration::from_hours(30 * 24);
pub(crate) const MINUTE_RETENTION_SECONDS: i64 = MINUTE_RETENTION.as_secs().cast_signed();

/// The network a row belongs to: its gateway and kind identify it, the name only labels it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkKey {
    /// `None` while the gateway's MAC cannot be read: such rows group by kind alone.
    pub gateway: Option<MacAddress>,
    pub kind: ConnectionKind,
    pub name: Option<String>,
}

impl NetworkKey {
    /// Gateway and kind: the identity rows are stored under, the name aside. O(1).
    #[must_use]
    pub const fn identify(&self) -> (Option<MacAddress>, ConnectionKind) {
        (self.gateway, self.kind)
    }

    /// O(1).
    #[must_use]
    pub fn is_same_identity(&self, other: &Self) -> bool {
        self.identify() == other.identify()
    }

    /// Same kind, and neither a gateway nor a name that tells them apart: a MAC read late is not
    /// a new network, two Wi-Fi names are. O(length of the names).
    #[must_use]
    pub fn is_likely_same(&self, other: &Self) -> bool {
        self.kind == other.kind
            && agree(self.gateway, other.gateway)
            && agree(self.name.as_deref(), other.name.as_deref())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    NetworkChanged,
    OfflineStarted,
    OfflineEnded,
    Sleep,
    Wake,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEvent {
    pub utc: i64,
    pub network: NetworkKey,
    pub kind: EventKind,
}

/// A write for the history thread.
#[derive(Debug)]
pub enum HistoryCommand {
    /// Added to any minute already kept for that start: a minute cut by a restart completes.
    SaveMinute {
        network: NetworkKey,
        start: i64,
        aggregate: Aggregate,
    },
    RecordEvent(HistoryEvent),
    RecordThroughput(ThroughputRecord),
    /// The user erases the history: everything written so far goes. Answered once it is done,
    /// never after a failure: the asker then sees its answer dropped.
    EraseAll(SyncSender<()>),
    /// Answered once every earlier command is written.
    Flush(SyncSender<()>),
}

/// A throughput test, under the network it measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThroughputRecord {
    pub utc: i64,
    pub network: NetworkKey,
    pub result: ThroughputResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError(pub String);

/// Applies history commands; the only owner of the repository, on a thread of its own.
pub struct HistoryWriter<R> {
    repository: R,
    /// The hour of the last minute saved: a minute of a later hour closes every earlier one.
    last_hour: Option<i64>,
}

impl<R: MeasurementRepository> HistoryWriter<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self {
            repository,
            last_hour: None,
        }
    }

    /// Gives the repository back, e.g. to read what was written.
    #[must_use]
    pub fn into_repository(self) -> R {
        self.repository
    }

    /// The last minute of an hour closes that hour; the first minute of a new hour closes every
    /// earlier hour of every network (left before its end, or interrupted by a sleep), then
    /// purges old minutes. O(60·B) per hour closed, B = 24 histogram buckets.
    ///
    /// # Errors
    ///
    /// Returns the repository's error; the command is lost, the next ones still apply.
    pub fn apply(&mut self, command: HistoryCommand, now_utc: i64) -> Result<(), RepositoryError> {
        match command {
            HistoryCommand::SaveMinute {
                network,
                start,
                aggregate,
            } => {
                self.repository.add_minute(&network, start, &aggregate)?;
                let hour = Granularity::Hour.find_start(start);
                if start + Granularity::Minute.to_seconds() == hour + Granularity::Hour.to_seconds()
                {
                    self.close_hour(&network, hour)?;
                }
                if self.last_hour.is_some_and(|last| last < hour) {
                    self.finish_pending_hours(now_utc)?;
                }
                self.last_hour = Some(hour);
                Ok(())
            }
            HistoryCommand::RecordEvent(event) => self.repository.record_event(&event),
            HistoryCommand::RecordThroughput(record) => {
                self.repository.record_throughput_test(&record)
            }
            HistoryCommand::EraseAll(answer) => {
                self.repository.erase_history()?;
                self.last_hour = None;
                let _ = answer.send(());
                Ok(())
            }
            HistoryCommand::Flush(answer) => {
                // The asker may have given up waiting: nothing to do then.
                let _ = answer.send(());
                Ok(())
            }
        }
    }

    /// Closes the hours left open by a stop, before the current hour. O(h·60·B) for h hours.
    ///
    /// # Errors
    ///
    /// Returns the repository's error.
    pub fn finish_pending_hours(&mut self, now_utc: i64) -> Result<(), RepositoryError> {
        let current_hour = Granularity::Hour.find_start(now_utc);
        for (network, hour) in self.repository.list_unmerged_hours(current_hour)? {
            self.close_hour(&network, hour)?;
        }
        self.purge_old_minutes(now_utc)
    }

    /// O(60·B).
    fn close_hour(&mut self, network: &NetworkKey, hour: i64) -> Result<(), RepositoryError> {
        let minutes = self.repository.list_aggregates(
            network,
            Granularity::Minute,
            hour,
            hour + Granularity::Hour.to_seconds(),
        )?;
        let mut merged = Aggregate::default();
        for (_, minute) in &minutes {
            merged.merge(minute);
        }
        self.repository.save_hour(network, hour, &merged)
    }

    /// Only minutes whose hour is kept go: the repository checks it.
    fn purge_old_minutes(&mut self, now_utc: i64) -> Result<(), RepositoryError> {
        self.repository
            .purge_minutes_before(now_utc - MINUTE_RETENTION_SECONDS)
    }
}

/// Seconds measured before any network was seen (no route at launch) count under this key.
const NO_NETWORK: NetworkKey = NetworkKey {
    gateway: None,
    kind: ConnectionKind::Other,
    name: None,
};

/// Builds the history from the engine's seconds: minutes, events and the last hour.
#[derive(Debug)]
pub struct HistoryTracker {
    accumulator: MinuteAccumulator,
    ring: SecondRing,
    /// The previous second, to measure jitter between consecutive seconds only.
    previous: Option<(i64, Duration)>,
    context: Option<ConnectionContext>,
    network: NetworkKey,
    was_offline: bool,
    /// The last second recorded: an outage cut by a sleep nobody announced ends there.
    last_second_utc: Option<i64>,
}

impl Default for HistoryTracker {
    fn default() -> Self {
        Self {
            accumulator: MinuteAccumulator::default(),
            ring: SecondRing::default(),
            previous: None,
            context: None,
            network: NO_NETWORK,
            was_offline: false,
            last_second_utc: None,
        }
    }
}

impl HistoryTracker {
    /// Records one second and sends what it completes. Without a route, the seconds count for
    /// the last network seen. O(1).
    pub fn record_second(
        &mut self,
        second: &MeasuredSecond,
        context: Option<ConnectionContext>,
        name: Option<String>,
        recorder: &impl HistoryRecorder,
    ) {
        if let Some(context) = context {
            self.follow_network(second.utc, context, name, recorder);
        }
        self.last_second_utc = Some(second.utc);
        let state = to_counted_state(second.verdict);
        let is_offline = state == Some(CountedState::Offline);
        if is_offline != self.was_offline {
            let kind = if is_offline {
                EventKind::OfflineStarted
            } else {
                EventKind::OfflineEnded
            };
            self.record_event(second.utc, kind, recorder);
            self.was_offline = is_offline;
        }
        let sample = SecondSample {
            latency: second.latency,
            jitter: second
                .latency
                .zip(self.previous)
                .and_then(|(latency, (utc, previous))| {
                    (second.utc - utc == 1).then(|| latency.abs_diff(previous))
                }),
            probes_sent: second.probes_sent,
            probes_lost: second.probes_lost,
            state,
        };
        self.previous = second.latency.map(|latency| (second.utc, latency));
        self.ring.record(SecondPoint {
            utc: second.utc,
            sample,
        });
        if let Some((start, aggregate)) = self.accumulator.record_second(second.utc, &sample) {
            recorder.record(HistoryCommand::SaveMinute {
                network: self.network.clone(),
                start,
                aggregate,
            });
        }
    }

    /// Hands over the minute in progress, then answers `answer` once everything is written.
    pub fn flush(&mut self, answer: SyncSender<()>, recorder: &impl HistoryRecorder) {
        self.save_current_minute(recorder);
        recorder.record(HistoryCommand::Flush(answer));
    }

    /// Forgets the last hour and the minute in progress, then has what was written erased; the
    /// writer answers `answer` once done. The next second notes its network and any outage
    /// again, as at launch. O(1).
    pub fn erase(&mut self, answer: SyncSender<()>, recorder: &impl HistoryRecorder) {
        *self = Self::default();
        recorder.record(HistoryCommand::EraseAll(answer));
    }

    /// Closes an outage in progress (nothing is measured while asleep), notes the sleep, then
    /// writes the minute in progress and answers `answer`. O(1).
    pub fn record_sleep(
        &mut self,
        utc: i64,
        answer: SyncSender<()>,
        recorder: &impl HistoryRecorder,
    ) {
        self.close_open_outage(utc, recorder);
        self.record_event(utc, EventKind::Sleep, recorder);
        self.flush(answer, recorder);
    }

    /// O(1).
    pub fn record_throughput(
        &self,
        utc: i64,
        result: ThroughputResult,
        recorder: &impl HistoryRecorder,
    ) {
        recorder.record(HistoryCommand::RecordThroughput(ThroughputRecord {
            utc,
            network: self.network.clone(),
            result,
        }));
    }

    /// No jitter is measured across the sleep. An outage still open (the sleep came unannounced)
    /// ends at the last second measured before it. O(1).
    pub fn record_wake(&mut self, utc: i64, recorder: &impl HistoryRecorder) {
        self.previous = None;
        self.close_open_outage(self.last_second_utc.unwrap_or(utc), recorder);
        self.record_event(utc, EventKind::Wake, recorder);
    }

    /// O(1).
    fn close_open_outage(&mut self, utc: i64, recorder: &impl HistoryRecorder) {
        if self.was_offline {
            self.record_event(utc, EventKind::OfflineEnded, recorder);
            self.was_offline = false;
        }
    }

    fn record_event(&self, utc: i64, kind: EventKind, recorder: &impl HistoryRecorder) {
        recorder.record(HistoryCommand::RecordEvent(HistoryEvent {
            utc,
            network: self.network.clone(),
            kind,
        }));
    }

    /// What only memory holds: the last hour and the network the minutes are saved under.
    /// O(n) for n ≤ 3 600.
    #[must_use]
    pub fn read_live(&self) -> LiveHistory {
        LiveHistory {
            last_hour: self.ring.list_points(),
            network: self.context.map(|_| self.network.clone()),
        }
    }

    /// Same network: keeps what was learnt (a MAC resolved, a name read later). Another one:
    /// closes the minute under the old one and notes the change. O(1).
    fn follow_network(
        &mut self,
        utc: i64,
        context: ConnectionContext,
        name: Option<String>,
        recorder: &impl HistoryRecorder,
    ) {
        if let Some(previous) = self
            .context
            .filter(|previous| previous.is_same_network(&context))
        {
            let completed = context.complete_with(&previous);
            self.network.gateway = completed.gateway;
            if name.is_some() {
                self.network.name = name;
            }
            self.context = Some(completed);
            return;
        }
        self.save_current_minute(recorder);
        self.network = NetworkKey {
            gateway: context.gateway,
            kind: context.kind,
            name,
        };
        self.context = Some(context);
        self.previous = None;
        self.record_event(utc, EventKind::NetworkChanged, recorder);
    }

    fn save_current_minute(&mut self, recorder: &impl HistoryRecorder) {
        if let Some((start, aggregate)) = self.accumulator.take_current() {
            recorder.record(HistoryCommand::SaveMinute {
                network: self.network.clone(),
                start,
                aggregate,
            });
        }
    }
}

/// The part of the history kept in memory by the engine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveHistory {
    /// Oldest first.
    pub last_hour: Vec<SecondPoint>,
    /// `None` before any network was seen.
    pub network: Option<NetworkKey>,
}

/// What the engine measured during one second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeasuredSecond {
    pub utc: i64,
    /// Median of the targets that answered.
    pub latency: Option<Duration>,
    pub probes_sent: u32,
    pub probes_lost: u32,
    pub verdict: Verdict,
}

const fn to_counted_state(verdict: Verdict) -> Option<CountedState> {
    match verdict {
        Verdict::Graded(domain::Grade::Solid) => Some(CountedState::Solid),
        Verdict::Graded(domain::Grade::Unstable) => Some(CountedState::Unstable),
        Verdict::Graded(domain::Grade::Degraded) => Some(CountedState::Degraded),
        Verdict::Offline => Some(CountedState::Offline),
        Verdict::Measuring => None,
    }
}
