//! What the dashboard shows of the history: the day so far, a period's evolution, and how each
//! network fared over 30 days. Reads only: the writer is never waited for.

use std::cmp::Reverse;
use std::collections::HashMap;

use domain::{
    Aggregate, ConnectionKind, CountedState, Granularity, MacAddress, SECONDS_PER_DAY,
    SECONDS_PER_HOUR, SECONDS_PER_MINUTE,
};

use crate::{EventKind, HistoryEvent, HistoryReader, LiveHistory, NetworkKey, RepositoryError};

/// How far back the network table and the events go.
pub const SUMMARY_DAYS: i64 = 30;
const SUMMARY_SECONDS: i64 = SUMMARY_DAYS * SECONDS_PER_DAY;
/// A network measured for less than an hour is listed, but never compared with the others.
const COMPARABLE_SECONDS: i64 = SECONDS_PER_HOUR;
/// How hard a second is on a call, by state: a cut is worse than a degraded line.
const UNSTABLE_WEIGHT: u64 = 1;
const DEGRADED_WEIGHT: u64 = 2;
const OFFLINE_WEIGHT: u64 = 3;

/// The periods the evolution chart offers, each ending now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Hour,
    Day,
    Week,
    Month,
}

impl Period {
    #[must_use]
    pub const fn to_seconds(self) -> i64 {
        match self {
            Self::Hour => SECONDS_PER_HOUR,
            Self::Day => SECONDS_PER_DAY,
            Self::Week => 7 * SECONDS_PER_DAY,
            Self::Month => SUMMARY_SECONDS,
        }
    }

    /// One chart point per bucket: 60 to 240 points whatever the period. The last hour goes by
    /// the minute, the grain of the stored minutes it starts from after a restart.
    #[must_use]
    pub const fn to_bucket_seconds(self) -> i64 {
        match self {
            Self::Hour => SECONDS_PER_MINUTE,
            Self::Day => 10 * SECONDS_PER_MINUTE,
            Self::Week => SECONDS_PER_HOUR,
            Self::Month => 3 * SECONDS_PER_HOUR,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Evolution {
    /// Each bucket's start and aggregate, oldest first; an empty aggregate is a gap.
    pub buckets: Vec<(i64, Aggregate)>,
    pub bucket_seconds: i64,
    /// Moves to another network during the period; a restart on the same one is not a move.
    pub network_changes: Vec<HistoryEvent>,
    /// Share of the period spent measured, from 0 to 1.
    pub coverage: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutageEnd {
    At(i64),
    /// Still offline now.
    Ongoing,
    /// The app stopped before it ended.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outage {
    pub start: i64,
    pub end: OutageEnd,
}

/// How long the computer has been on the current network, as far as 30 days of history tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkStay {
    NoNetwork,
    /// Joined before the oldest event kept.
    BeyondHistory,
    Since(i64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayOverview {
    /// Every network's minutes since local midnight, added up.
    pub today: Aggregate,
    /// Oldest first, including one that began before midnight and ended after.
    pub outages: Vec<Outage>,
    /// The minute of the day hardest on calls; `None` when every minute was solid.
    pub worst_minute: Option<(i64, Aggregate)>,
    pub network_stay: NetworkStay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSummary {
    pub network: NetworkKey,
    pub aggregate: Aggregate,
    pub is_current: bool,
    /// The network solid most often, among at least two measured for an hour or more.
    pub is_most_reliable: bool,
}

/// The chart's buckets over `period`, the last hour from memory and the rest from the history.
/// O(r + s + e + n) for r rows, s ≤ 3 600 seconds in memory, e events and n ≤ 241 buckets.
///
/// # Errors
///
/// The history could not be read.
pub fn read_evolution(
    reader: &(impl HistoryReader + ?Sized),
    live: &LiveHistory,
    period: Period,
    now: i64,
) -> Result<Evolution, RepositoryError> {
    let width = period.to_bucket_seconds();
    let start = now - period.to_seconds();
    let from = start - start.rem_euclid(width);
    let mut buckets = Buckets::create(from, now, width);
    match period {
        Period::Hour => {
            // Memory holds the seconds since launch; stored minutes cover the hour before.
            let memory_start = live.last_hour.first().map_or(now, |point| {
                let minute = Granularity::Minute.find_start(point.utc);
                if minute == point.utc {
                    minute
                } else {
                    minute + Granularity::Minute.to_seconds()
                }
            });
            buckets.merge_all(reader.list_all_aggregates(
                Granularity::Minute,
                from,
                memory_start,
            )?);
            for point in live
                .last_hour
                .iter()
                .filter(|point| point.utc >= memory_start)
            {
                if let Some(bucket) = buckets.find_mut(point.utc) {
                    bucket.record_second(&point.sample);
                }
            }
        }
        Period::Day => {
            buckets.merge_all(reader.list_all_aggregates(Granularity::Minute, from, now)?);
        }
        Period::Week | Period::Month => buckets.merge_all(list_rows_until_now(reader, from, now)?),
    }
    let events = reader.list_events(now - SUMMARY_SECONDS, now)?;
    let measured: u32 = buckets
        .aggregates
        .iter()
        .map(Aggregate::count_measured_seconds)
        .sum();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a period holds a few million seconds"
    )]
    let coverage = (f64::from(measured) / period.to_seconds() as f64).min(1.0);
    Ok(Evolution {
        buckets: buckets.into_list(),
        bucket_seconds: width,
        network_changes: select_network_changes(&events, from),
        coverage,
    })
}

/// The day since `day_start` (local midnight, in UTC seconds). O(m + e) for m minutes and e
/// events.
///
/// # Errors
///
/// The history could not be read.
pub fn read_day_overview(
    reader: &(impl HistoryReader + ?Sized),
    live: &LiveHistory,
    day_start: i64,
    now: i64,
) -> Result<DayOverview, RepositoryError> {
    let minutes = reader.list_all_aggregates(Granularity::Minute, day_start, now)?;
    let mut today = Aggregate::default();
    for (_, _, minute) in &minutes {
        today.merge(minute);
    }
    // A minute shared by two networks (a move mid-minute) counts once per network.
    let worst_minute = minutes
        .iter()
        .filter(|(_, _, minute)| compute_hardship(minute) > 0)
        .max_by_key(|(_, start, minute)| (compute_hardship(minute), *start))
        .map(|(_, start, minute)| (*start, *minute));
    let events = reader.list_events(now - SUMMARY_SECONDS, now)?;
    let is_offline = live
        .last_hour
        .last()
        .is_some_and(|point| point.sample.state == Some(CountedState::Offline));
    Ok(DayOverview {
        today,
        outages: list_outages(&events, is_offline)
            .into_iter()
            .filter(|outage| match outage.end {
                OutageEnd::At(end) => end >= day_start,
                OutageEnd::Ongoing => true,
                OutageEnd::Unknown => outage.start >= day_start,
            })
            .collect(),
        worst_minute,
        network_stay: live
            .network
            .as_ref()
            .map_or(NetworkStay::NoNetwork, |network| {
                compute_network_start(&events, network)
                    .map_or(NetworkStay::BeyondHistory, NetworkStay::Since)
            }),
    })
}

/// Each network of the last 30 days, the most measured first. O(r + k log k) for r rows and k
/// networks.
///
/// # Errors
///
/// The history could not be read.
pub fn list_network_summaries(
    reader: &(impl HistoryReader + ?Sized),
    live: &LiveHistory,
    now: i64,
) -> Result<Vec<NetworkSummary>, RepositoryError> {
    let mut indexes: HashMap<(Option<MacAddress>, ConnectionKind), usize> = HashMap::new();
    let mut summaries: Vec<NetworkSummary> = Vec::new();
    for (network, _, aggregate) in list_rows_until_now(reader, now - SUMMARY_SECONDS, now)? {
        let index = *indexes
            .entry((network.gateway, network.kind))
            .or_insert_with(|| {
                summaries.push(NetworkSummary {
                    is_current: live
                        .network
                        .as_ref()
                        .is_some_and(|current| current.is_same_identity(&network)),
                    network,
                    aggregate: Aggregate::default(),
                    is_most_reliable: false,
                });
                summaries.len() - 1
            });
        summaries[index].aggregate.merge(&aggregate);
    }
    summaries.retain(|summary| summary.aggregate.count_measured_seconds() > 0);
    summaries.sort_by_key(|summary| Reverse(summary.aggregate.count_measured_seconds()));
    mark_most_reliable(&mut summaries);
    Ok(summaries)
}

/// Closed hours, then the minutes of the hour in progress. O(log n + r).
fn list_rows_until_now(
    reader: &(impl HistoryReader + ?Sized),
    from: i64,
    now: i64,
) -> Result<Vec<(NetworkKey, i64, Aggregate)>, RepositoryError> {
    let current_hour = Granularity::Hour.find_start(now);
    let mut rows = reader.list_all_aggregates(Granularity::Hour, from, current_hour)?;
    rows.extend(reader.list_all_aggregates(Granularity::Minute, current_hour.max(from), now)?);
    Ok(rows)
}

/// O(e).
fn select_network_changes(events: &[HistoryEvent], from: i64) -> Vec<HistoryEvent> {
    let mut previous: Option<&NetworkKey> = None;
    let mut changes = Vec::new();
    for event in events
        .iter()
        .filter(|event| event.kind == EventKind::NetworkChanged)
    {
        if event.utc >= from
            && previous.is_some_and(|before| !before.is_likely_same(&event.network))
        {
            changes.push(event.clone());
        }
        previous = Some(&event.network);
    }
    changes
}

/// A start with no end before the next start was cut by a stop of the app. O(e).
fn list_outages(events: &[HistoryEvent], is_offline: bool) -> Vec<Outage> {
    let mut outages = Vec::new();
    let mut open: Option<i64> = None;
    for event in events {
        match (event.kind, open) {
            (EventKind::OfflineStarted, previous) => {
                if let Some(start) = previous {
                    outages.push(Outage {
                        start,
                        end: OutageEnd::Unknown,
                    });
                }
                open = Some(event.utc);
            }
            (EventKind::OfflineEnded, Some(start)) => {
                outages.push(Outage {
                    start,
                    end: OutageEnd::At(event.utc),
                });
                open = None;
            }
            (EventKind::OfflineEnded | EventKind::NetworkChanged, _) => {}
        }
    }
    if let Some(start) = open {
        outages.push(Outage {
            start,
            end: if is_offline {
                OutageEnd::Ongoing
            } else {
                OutageEnd::Unknown
            },
        });
    }
    outages
}

/// The earliest of the latest run of arrivals on `network`. O(e).
fn compute_network_start(events: &[HistoryEvent], network: &NetworkKey) -> Option<i64> {
    let mut since = None;
    for event in events
        .iter()
        .rev()
        .filter(|event| event.kind == EventKind::NetworkChanged)
    {
        if !event.network.is_likely_same(network) {
            break;
        }
        since = Some(event.utc);
    }
    since
}

/// Seconds weighted by how hard they are on a call. O(1).
fn compute_hardship(aggregate: &Aggregate) -> u64 {
    let seconds = aggregate.seconds;
    UNSTABLE_WEIGHT * u64::from(seconds.unstable)
        + DEGRADED_WEIGHT * u64::from(seconds.degraded)
        + OFFLINE_WEIGHT * u64::from(seconds.offline)
}

/// The highest share of solid seconds, compared without floats; the more measured wins a tie.
/// O(k).
fn mark_most_reliable(summaries: &mut [NetworkSummary]) {
    let comparable: Vec<usize> = (0..summaries.len())
        .filter(|index| {
            i64::from(summaries[*index].aggregate.count_measured_seconds()) >= COMPARABLE_SECONDS
        })
        .collect();
    if comparable.len() < 2 {
        return;
    }
    let share = |index: usize| {
        let seconds = summaries[index].aggregate.seconds;
        (u64::from(seconds.solid), u64::from(seconds.count_total()))
    };
    let best = comparable
        .iter()
        .copied()
        .fold(comparable[0], |best, index| {
            let (solid, total) = share(index);
            let (best_solid, best_total) = share(best);
            if solid * best_total > best_solid * total {
                index
            } else {
                best
            }
        });
    summaries[best].is_most_reliable = true;
}

/// Fixed-width buckets from `from` to `to`, ready to receive rows.
struct Buckets {
    from: i64,
    width: i64,
    aggregates: Vec<Aggregate>,
}

impl Buckets {
    fn create(from: i64, to: i64, width: i64) -> Self {
        let count = usize::try_from((to - from + width - 1) / width).unwrap_or(0);
        Self {
            from,
            width,
            aggregates: vec![Aggregate::default(); count],
        }
    }

    fn find_mut(&mut self, utc: i64) -> Option<&mut Aggregate> {
        let index = usize::try_from((utc - self.from).div_euclid(self.width)).ok()?;
        self.aggregates.get_mut(index)
    }

    /// Rows outside the buckets are left out. O(r).
    fn merge_all(&mut self, rows: Vec<(NetworkKey, i64, Aggregate)>) {
        for (_, start, aggregate) in rows {
            if let Some(bucket) = self.find_mut(start) {
                bucket.merge(&aggregate);
            }
        }
    }

    fn into_list(self) -> Vec<(i64, Aggregate)> {
        let Self {
            from,
            width,
            aggregates,
        } = self;
        (0_i64..)
            .map(|index| from + index * width)
            .zip(aggregates)
            .collect()
    }
}
