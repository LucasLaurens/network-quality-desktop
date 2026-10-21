//! The measurement history: what the engine hands over each minute, and how the single writer
//! turns minutes into hours and keeps 30 days of minutes.

use std::sync::mpsc::SyncSender;
use std::time::Duration;

use domain::{
    Aggregate, ConnectionContext, ConnectionKind, CountedState, Granularity, MacAddress,
    MinuteAccumulator, SecondPoint, SecondRing, SecondSample, Verdict,
};

use crate::{HistoryRecorder, MeasurementRepository};

/// Minutes older than this are dropped once their hour is kept.
pub const MINUTE_RETENTION: Duration = Duration::from_hours(30 * 24);

/// The network a row belongs to: its gateway and kind identify it, the name only labels it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkKey {
    /// `None` while the gateway's MAC cannot be read: such rows group by kind alone.
    pub gateway: Option<MacAddress>,
    pub kind: ConnectionKind,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    NetworkChanged,
    OfflineStarted,
    OfflineEnded,
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
    /// Answered once every earlier command is written.
    Flush(SyncSender<()>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError(pub String);

/// Applies history commands; the only owner of the repository, on a thread of its own.
pub struct HistoryWriter<R> {
    repository: R,
}

impl<R: MeasurementRepository> HistoryWriter<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Gives the repository back, e.g. to read what was written.
    #[must_use]
    pub fn into_repository(self) -> R {
        self.repository
    }

    /// The last minute of an hour closes that hour and purges old minutes. O(60·B) at worst,
    /// B = 24 histogram buckets.
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
                    self.purge_old_minutes(now_utc)?;
                }
                Ok(())
            }
            HistoryCommand::RecordEvent(event) => self.repository.record_event(&event),
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

    fn purge_old_minutes(&mut self, now_utc: i64) -> Result<(), RepositoryError> {
        let retention = i64::try_from(MINUTE_RETENTION.as_secs()).unwrap_or(i64::MAX);
        self.repository.purge_minutes_before(now_utc - retention)
    }
}

/// Builds the history from the engine's seconds: minutes, events and the last hour.
#[derive(Debug, Default)]
pub struct HistoryTracker {
    accumulator: MinuteAccumulator,
    ring: SecondRing,
    previous_latency: Option<Duration>,
    network: Option<(ConnectionContext, NetworkKey)>,
    was_offline: bool,
}

impl HistoryTracker {
    /// Records one second and sends what it completes. Without a route, the seconds still
    /// count for the last network seen. O(1).
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
        let Some((_, network)) = &self.network else {
            return;
        };
        let state = to_counted_state(second.verdict);
        let is_offline = state == Some(CountedState::Offline);
        if is_offline != self.was_offline {
            recorder.record(HistoryCommand::RecordEvent(HistoryEvent {
                utc: second.utc,
                network: network.clone(),
                kind: if is_offline {
                    EventKind::OfflineStarted
                } else {
                    EventKind::OfflineEnded
                },
            }));
            self.was_offline = is_offline;
        }
        let sample = SecondSample {
            latency: second.latency,
            jitter: second
                .latency
                .zip(self.previous_latency)
                .map(|(latency, previous)| latency.abs_diff(previous)),
            probes_sent: second.probes_sent,
            probes_lost: second.probes_lost,
            state,
        };
        self.previous_latency = second.latency;
        self.ring.record(SecondPoint {
            utc: second.utc,
            latency: second.latency,
            state,
        });
        if let Some((start, aggregate)) = self.accumulator.record_second(second.utc, &sample) {
            recorder.record(HistoryCommand::SaveMinute {
                network: network.clone(),
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

    /// The last hour, second by second, oldest first. O(n) for n ≤ 3 600.
    #[must_use]
    pub fn list_last_hour(&self) -> Vec<SecondPoint> {
        self.ring.list_points()
    }

    fn follow_network(
        &mut self,
        utc: i64,
        context: ConnectionContext,
        name: Option<String>,
        recorder: &impl HistoryRecorder,
    ) {
        let key = NetworkKey {
            gateway: context.gateway,
            kind: context.kind,
            name,
        };
        match &self.network {
            Some((previous, _)) if previous.is_same_network(&context) => {
                // Same network: keep what was learnt (a name read later, a MAC resolved later).
                self.network = Some((context.complete_with(previous), key));
            }
            previous => {
                if previous.is_some() {
                    self.save_current_minute(recorder);
                }
                recorder.record(HistoryCommand::RecordEvent(HistoryEvent {
                    utc,
                    network: key.clone(),
                    kind: EventKind::NetworkChanged,
                }));
                self.previous_latency = None;
                self.network = Some((context, key));
            }
        }
    }

    fn save_current_minute(&mut self, recorder: &impl HistoryRecorder) {
        if let (Some((start, aggregate)), Some((_, network))) =
            (self.accumulator.take_current(), &self.network)
        {
            recorder.record(HistoryCommand::SaveMinute {
                network: network.clone(),
                start,
                aggregate,
            });
        }
    }
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
