//! The dashboard's figures in words: the day so far, the evolution chart and the networks.

use std::time::Duration;

use application::{DayOverview, Evolution, NetworkSummary, OutageEnd, Period};
use domain::{Aggregate, CountedState, LossRate, Metric, Thresholds};
use fluent_bundle::FluentArgs;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::display::{DisplayState, Displayable, to_name_key};
use crate::network_display::select_kind_keys;
use crate::text_catalog::TextCatalog;

const SECONDS_PER_MINUTE: i64 = 60;
const SECONDS_PER_HOUR: i64 = 3600;
const SECONDS_PER_DAY: i64 = 86_400;
/// Days from 0000-03-01 to 1970-01-01, for the civil calendar below.
const DAYS_TO_EPOCH: i64 = 719_468;
const DAYS_PER_ERA: i64 = 146_097;

/// The period picked above the chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PeriodChoice {
    Hour,
    Day,
    Week,
    Month,
}

impl From<PeriodChoice> for Period {
    fn from(choice: PeriodChoice) -> Self {
        match choice {
            PeriodChoice::Hour => Self::Hour,
            PeriodChoice::Day => Self::Day,
            PeriodChoice::Week => Self::Week,
            PeriodChoice::Month => Self::Month,
        }
    }
}

/// The four facts under the current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OverviewDisplay {
    pub today: String,
    pub outages: String,
    pub worst_moment: String,
    pub on_network_since: String,
}

/// Percent of the measured time spent in each state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StateShares {
    pub solid: f64,
    pub unstable: f64,
    pub degraded: f64,
    pub offline: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRowDisplay {
    pub label: String,
    pub connection: String,
    pub shares: StateShares,
    pub solid_label: String,
    /// Every share in words, for screen readers.
    pub shares_label: String,
    pub latency_p90: String,
    pub loss: String,
    pub is_current: bool,
    pub is_most_reliable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummaryDisplay {
    pub overview: OverviewDisplay,
    /// The most measured first; empty before the first minute is kept.
    pub networks: Vec<NetworkRowDisplay>,
}

/// One metric across the chart, in its own unit (ms or %).
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDisplay {
    /// One per point; `None` where nothing was measured: the line breaks there.
    pub values: Vec<Option<f64>>,
    pub texts: Vec<Option<String>>,
    pub unstable_from: f64,
    pub degraded_above: f64,
    pub unstable_label: String,
    pub degraded_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NetworkChangeDisplay {
    /// Where on the time axis, from 0 (left end) to 1 (right end).
    pub position: f64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StateLabelDisplay {
    pub state: DisplayState,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct EvolutionDisplay {
    /// When each point starts, in the user's time zone.
    pub times: Vec<String>,
    /// The state that lasted longest in each point, for the band under the chart.
    pub states: Vec<Option<DisplayState>>,
    pub latency: SeriesDisplay,
    pub jitter: SeriesDisplay,
    pub loss: SeriesDisplay,
    pub network_changes: Vec<NetworkChangeDisplay>,
    /// The band's legend, best state first; the tooltip names a state with it too.
    pub state_labels: Vec<StateLabelDisplay>,
    pub coverage: String,
    /// Nothing measured in the period yet: a sentence replaces the chart.
    pub is_empty: bool,
}

/// Local midnight, in UTC seconds, for a zone `offset` seconds ahead of UTC. O(1).
// shortcut: today's offset applies to the whole day, so a day with a clock change is off by an
// hour; take the zone's rules into account if that ever matters.
#[must_use]
pub const fn compute_day_start(now: i64, offset: i64) -> i64 {
    now - (now + offset).rem_euclid(SECONDS_PER_DAY)
}

impl DashboardSummaryDisplay {
    /// O(k) for k networks.
    #[must_use]
    pub fn compose(
        overview: &DayOverview,
        summaries: &[NetworkSummary],
        has_network: bool,
        now: i64,
        offset: i64,
        catalog: &TextCatalog,
    ) -> Self {
        Self {
            overview: OverviewDisplay::compose(overview, has_network, now, offset, catalog),
            networks: summaries
                .iter()
                .map(|summary| NetworkRowDisplay::compose(summary, catalog))
                .collect(),
        }
    }
}

impl OverviewDisplay {
    /// O(o) for o outages.
    #[must_use]
    pub fn compose(
        overview: &DayOverview,
        has_network: bool,
        now: i64,
        offset: i64,
        catalog: &TextCatalog,
    ) -> Self {
        let seconds = overview.today.seconds;
        let has_measures = seconds.count_total() > 0;
        Self {
            today: if has_measures {
                render_value(
                    "today-solid-share",
                    compute_percent(seconds.solid, seconds.count_total()).to_string(),
                    catalog,
                )
            } else {
                catalog.render("today-not-enough", None)
            },
            outages: describe_outages(overview, now, offset, catalog),
            worst_moment: if has_measures {
                describe_worst_minute(overview.worst_minute.as_ref(), offset, catalog)
            } else {
                catalog.render("today-not-enough", None)
            },
            on_network_since: match (has_network, overview.on_network_since) {
                (false, _) => catalog.render("since-no-network", None),
                (true, None) => catalog.render("since-beyond", None),
                (true, Some(since)) => format_duration(now - since, catalog),
            },
        }
    }
}

impl NetworkRowDisplay {
    /// O(B), B = 24 histogram buckets.
    #[must_use]
    pub fn compose(summary: &NetworkSummary, catalog: &TextCatalog) -> Self {
        let (name_key, connection_key) = select_kind_keys(summary.network.kind, false);
        let seconds = summary.aggregate.seconds;
        let total = seconds.count_total();
        let share = |state: CountedState| {
            if total == 0 {
                0.0
            } else {
                f64::from(seconds.count(state)) * 100.0 / f64::from(total)
            }
        };
        let percent =
            |state: CountedState| compute_percent(seconds.count(state), total).to_string();
        let mut args = FluentArgs::new();
        args.set("solid", percent(CountedState::Solid));
        args.set("unstable", percent(CountedState::Unstable));
        args.set("degraded", percent(CountedState::Degraded));
        args.set("offline", percent(CountedState::Offline));
        Self {
            label: summary
                .network
                .name
                .clone()
                .unwrap_or_else(|| catalog.render(name_key, None)),
            connection: catalog.render(connection_key, None),
            shares: StateShares {
                solid: share(CountedState::Solid),
                unstable: share(CountedState::Unstable),
                degraded: share(CountedState::Degraded),
                offline: share(CountedState::Offline),
            },
            solid_label: render_value("share-solid", percent(CountedState::Solid), catalog),
            shares_label: catalog.render("shares", Some(&args)),
            latency_p90: format_or_unknown(summary.aggregate.compute_latency_p90(), catalog),
            loss: format_or_unknown(summary.aggregate.compute_loss(), catalog),
            is_current: summary.is_current,
            is_most_reliable: summary.is_most_reliable,
        }
    }
}

impl EvolutionDisplay {
    /// O(n·B) for n ≤ 241 points, B = 24 histogram buckets.
    #[must_use]
    pub fn compose(
        evolution: &Evolution,
        period: Period,
        offset: i64,
        catalog: &TextCatalog,
    ) -> Self {
        let thresholds = Thresholds::for_video_calls();
        let aggregates: Vec<&Aggregate> = evolution
            .buckets
            .iter()
            .map(|(_, aggregate)| aggregate)
            .collect();
        let first = evolution.buckets.first().map_or(0, |(start, _)| *start);
        let span =
            i64::try_from(evolution.buckets.len()).unwrap_or(i64::MAX) * evolution.bucket_seconds;
        let states: Vec<Option<DisplayState>> = aggregates
            .iter()
            .map(|aggregate| aggregate.seconds.find_dominant().map(DisplayState::from))
            .collect();
        Self {
            times: evolution
                .buckets
                .iter()
                .map(|(start, _)| match period {
                    Period::Hour | Period::Day => format_time_of_day(*start, offset, catalog),
                    Period::Week | Period::Month => format_date_hour(*start, offset, catalog),
                })
                .collect(),
            is_empty: states.iter().all(Option::is_none),
            states,
            latency: compose_series(
                &aggregates,
                Aggregate::compute_latency_p90,
                to_milliseconds,
                (
                    thresholds.latency.read_unstable_from(),
                    thresholds.latency.read_degraded_above(),
                ),
                catalog,
            ),
            jitter: compose_series(
                &aggregates,
                Aggregate::compute_mean_jitter,
                to_milliseconds,
                (
                    thresholds.jitter.read_unstable_from(),
                    thresholds.jitter.read_degraded_above(),
                ),
                catalog,
            ),
            loss: compose_series(
                &aggregates,
                Aggregate::compute_loss,
                LossRate::to_percent,
                (
                    thresholds.loss.read_unstable_from(),
                    thresholds.loss.read_degraded_above(),
                ),
                catalog,
            ),
            network_changes: evolution
                .network_changes
                .iter()
                .map(|event| {
                    let (name_key, _) = select_kind_keys(event.network.kind, false);
                    let mut args = FluentArgs::new();
                    args.set(
                        "network",
                        event
                            .network
                            .name
                            .clone()
                            .unwrap_or_else(|| catalog.render(name_key, None)),
                    );
                    NetworkChangeDisplay {
                        position: compute_fraction(event.utc - first, span),
                        label: catalog.render("chart-network-change", Some(&args)),
                    }
                })
                .collect(),
            state_labels: [
                DisplayState::Solid,
                DisplayState::Unstable,
                DisplayState::Degraded,
                DisplayState::Offline,
            ]
            .into_iter()
            .map(|state| StateLabelDisplay {
                state,
                label: catalog.render(state.to_catalogue_key(), None),
            })
            .collect(),
            coverage: render_value(
                "chart-coverage",
                format!("{:.0}", evolution.coverage * 100.0),
                catalog,
            ),
        }
    }
}

/// O(n·B).
fn compose_series<T: Displayable>(
    aggregates: &[&Aggregate],
    read: fn(&Aggregate) -> Option<T>,
    to_number: fn(T) -> f64,
    (unstable_from, degraded_above): (T, T),
    catalog: &TextCatalog,
) -> SeriesDisplay {
    let readings: Vec<Option<T>> = aggregates.iter().map(|aggregate| read(aggregate)).collect();
    SeriesDisplay {
        values: readings.iter().map(|value| value.map(to_number)).collect(),
        texts: readings
            .iter()
            .map(|value| value.map(|value| value.format(catalog)))
            .collect(),
        unstable_from: to_number(unstable_from),
        degraded_above: to_number(degraded_above),
        unstable_label: render_value("threshold-unstable", unstable_from.format(catalog), catalog),
        degraded_label: render_value(
            "threshold-degraded",
            degraded_above.format(catalog),
            catalog,
        ),
    }
}

fn to_milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a chart spans a few million seconds"
)]
fn compute_fraction(part: i64, whole: i64) -> f64 {
    if whole <= 0 {
        return 0.0;
    }
    (part as f64 / whole as f64).clamp(0.0, 1.0)
}

/// Rounded down: 100 % only when every second was in that state. O(1).
fn compute_percent(part: u32, total: u32) -> u64 {
    if total == 0 {
        return 0;
    }
    u64::from(part) * 100 / u64::from(total)
}

/// The latest outage if it lasts, otherwise how many and the longest. O(o).
fn describe_outages(
    overview: &DayOverview,
    now: i64,
    offset: i64,
    catalog: &TextCatalog,
) -> String {
    let outages = &overview.outages;
    let Some(last) = outages.last() else {
        return catalog.render("outages-none", None);
    };
    let mut args = FluentArgs::new();
    if last.end == OutageEnd::Ongoing {
        args.set("duration", format_duration(now - last.start, catalog));
        return catalog.render("outages-ongoing", Some(&args));
    }
    let longest = outages
        .iter()
        .filter_map(|outage| match outage.end {
            OutageEnd::At(end) => Some(end - outage.start),
            OutageEnd::Ongoing | OutageEnd::Unknown => None,
        })
        .max();
    if let Some(longest) = longest {
        args.set("duration", format_duration(longest, catalog));
    }
    if outages.len() == 1 {
        args.set("time", format_time_of_day(last.start, offset, catalog));
        let key = if longest.is_some() {
            "outages-one"
        } else {
            "outages-one-unknown"
        };
        return catalog.render(key, Some(&args));
    }
    args.set("count", outages.len().to_string());
    let key = if longest.is_some() {
        "outages-many"
    } else {
        "outages-many-unknown"
    };
    catalog.render(key, Some(&args))
}

/// The minute and the metric furthest beyond its threshold, or the outage. O(B).
fn describe_worst_minute(
    worst: Option<&(i64, Aggregate)>,
    offset: i64,
    catalog: &TextCatalog,
) -> String {
    let Some((start, minute)) = worst else {
        return catalog.render("worst-none", None);
    };
    let mut args = FluentArgs::new();
    args.set("time", format_time_of_day(*start, offset, catalog));
    let metric = minute
        .find_worst_metric(&Thresholds::for_video_calls())
        .filter(|_| minute.seconds.find_dominant() != Some(CountedState::Offline));
    let value = match metric {
        Some(Metric::Latency) => minute
            .compute_latency_p90()
            .map(|value| value.format(catalog)),
        Some(Metric::Jitter) => minute
            .compute_mean_jitter()
            .map(|value| value.format(catalog)),
        Some(Metric::Loss) => minute.compute_loss().map(|value| value.format(catalog)),
        None => None,
    };
    match metric.zip(value) {
        Some((metric, value)) => {
            args.set(
                "metric",
                catalog.render(&format!("{}-lower", to_name_key(metric)), None),
            );
            args.set("value", value);
            catalog.render("worst-moment", Some(&args))
        }
        None => catalog.render("worst-outage", Some(&args)),
    }
}

fn format_or_unknown<T: Displayable>(value: Option<T>, catalog: &TextCatalog) -> String {
    value.map_or_else(
        || catalog.render("value-unknown", None),
        |value| value.format(catalog),
    )
}

fn render_value(key: &str, value: String, catalog: &TextCatalog) -> String {
    let mut args = FluentArgs::new();
    args.set("value", value);
    catalog.render(key, Some(&args))
}

/// "14 h 05" in the user's time zone. O(1).
fn format_time_of_day(utc: i64, offset: i64, catalog: &TextCatalog) -> String {
    let seconds = (utc + offset).rem_euclid(SECONDS_PER_DAY);
    let mut args = FluentArgs::new();
    args.set("hours", (seconds / SECONDS_PER_HOUR).to_string());
    args.set(
        "minutes",
        format!("{:02}", seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE),
    );
    catalog.render("time-of-day", Some(&args))
}

/// "12/10, 14 h" in the user's time zone. O(1).
fn format_date_hour(utc: i64, offset: i64, catalog: &TextCatalog) -> String {
    let local = utc + offset;
    let (month, day) = convert_days_to_month_day(local.div_euclid(SECONDS_PER_DAY));
    let mut args = FluentArgs::new();
    args.set("day", format!("{day:02}"));
    args.set("month", format!("{month:02}"));
    args.set(
        "hours",
        (local.rem_euclid(SECONDS_PER_DAY) / SECONDS_PER_HOUR).to_string(),
    );
    catalog.render("date-hour", Some(&args))
}

/// The month and day of a day counted from 1970-01-01, in the proleptic Gregorian calendar
/// (H. Hinnant's `civil_from_days`). O(1).
const fn convert_days_to_month_day(days: i64) -> (i64, i64) {
    let shifted = days + DAYS_TO_EPOCH;
    let day_of_era = shifted.rem_euclid(DAYS_PER_ERA);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // Months counted from March, so that February comes last.
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    (month, day)
}

/// The two largest units: "42 s", "12 min", "3 h 12 min", "2 j 5 h". O(1).
fn format_duration(seconds: i64, catalog: &TextCatalog) -> String {
    let seconds = seconds.max(0);
    let mut args = FluentArgs::new();
    if seconds < SECONDS_PER_MINUTE {
        args.set("value", seconds.to_string());
        return catalog.render("duration-seconds", Some(&args));
    }
    if seconds < SECONDS_PER_HOUR {
        args.set("value", (seconds / SECONDS_PER_MINUTE).to_string());
        return catalog.render("duration-minutes", Some(&args));
    }
    if seconds < SECONDS_PER_DAY {
        let minutes = seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
        args.set("hours", (seconds / SECONDS_PER_HOUR).to_string());
        args.set("minutes", minutes.to_string());
        let key = if minutes == 0 {
            "duration-hours"
        } else {
            "duration-hours-minutes"
        };
        return catalog.render(key, Some(&args));
    }
    let hours = seconds % SECONDS_PER_DAY / SECONDS_PER_HOUR;
    args.set("days", (seconds / SECONDS_PER_DAY).to_string());
    args.set("hours", hours.to_string());
    let key = if hours == 0 {
        "duration-days"
    } else {
        "duration-days-hours"
    };
    catalog.render(key, Some(&args))
}

#[cfg(test)]
mod tests {
    use application::{HistoryEvent, NetworkKey, Outage};
    use domain::{ConnectionKind, MacAddress, SecondSample};

    use super::*;

    const NBSP: char = '\u{a0}';
    /// 2023-11-14 00:00 UTC.
    const MIDNIGHT: i64 = 1_699_920_000;
    /// Paris in winter, one hour ahead of UTC.
    const PARIS: i64 = 3600;

    fn create_aggregate(seconds: u32, state: CountedState, latency_ms: u64) -> Aggregate {
        let mut aggregate = Aggregate::default();
        for _ in 0..seconds {
            aggregate.record_second(&SecondSample {
                latency: Some(Duration::from_millis(latency_ms)),
                jitter: Some(Duration::from_millis(2)),
                probes_sent: 2,
                probes_lost: 0,
                state: Some(state),
            });
        }
        aggregate
    }

    fn create_overview(today: Aggregate) -> DayOverview {
        DayOverview {
            today,
            outages: Vec::new(),
            worst_minute: None,
            on_network_since: None,
        }
    }

    fn create_network(name: Option<&str>) -> NetworkKey {
        NetworkKey {
            gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, 0x01]),
            kind: ConnectionKind::Ethernet,
            name: name.map(str::to_owned),
        }
    }

    /// `~` marks a no-break space.
    fn spaced(text: &str) -> String {
        text.replace('~', &NBSP.to_string())
    }

    #[test]
    fn starts_the_day_at_local_midnight() {
        // Act & Assert
        assert_eq!(
            compute_day_start(MIDNIGHT + 10 * 3600, PARIS),
            MIDNIGHT - PARIS
        );
        assert_eq!(
            compute_day_start(MIDNIGHT + 23 * 3600 + 30 * 60, PARIS),
            MIDNIGHT + 23 * 3600
        );
    }

    #[test]
    fn writes_times_and_dates_in_the_user_time_zone() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let time = format_time_of_day(MIDNIGHT + 13 * 3600 + 5 * 60, PARIS, &catalog);
        let date = format_date_hour(MIDNIGHT + 23 * 3600, PARIS, &catalog);
        let leap_day = convert_days_to_month_day(19_782);

        // Assert
        assert_eq!(time, spaced("14~h~05"));
        assert_eq!(date, spaced("15/11, 0~h"));
        assert_eq!(leap_day, (2, 29));
    }

    #[test]
    fn keeps_the_two_largest_units_of_a_duration() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let durations: Vec<String> = [
            42,
            12 * 60,
            3 * 3600,
            3 * 3600 + 12 * 60,
            2 * 86_400 + 5 * 3600,
        ]
        .into_iter()
        .map(|seconds| format_duration(seconds, &catalog))
        .collect();

        // Assert
        assert_eq!(
            durations,
            ["42~s", "12~min", "3~h", "3~h 12~min", "2~j 5~h"].map(spaced)
        );
    }

    #[test]
    fn says_how_solid_the_day_was_and_when_it_was_hardest() {
        // Arrange
        let catalog = TextCatalog::load();
        let mut today = create_aggregate(90, CountedState::Solid, 20);
        let worst = create_aggregate(10, CountedState::Degraded, 340);
        today.merge(&worst);
        let overview = DayOverview {
            worst_minute: Some((MIDNIGHT + 11 * 3600 + 20 * 60, worst)),
            on_network_since: Some(MIDNIGHT),
            ..create_overview(today)
        };

        // Act
        let display = OverviewDisplay::compose(
            &overview,
            true,
            MIDNIGHT + 3 * 3600 + 12 * 60,
            PARIS,
            &catalog,
        );

        // Assert
        assert_eq!(display.today, spaced("Solide 90~% du temps"));
        // The histogram keeps the upper edge of the bucket holding the p90.
        let p90 = worst
            .compute_latency_p90()
            .expect("latencies")
            .format(&catalog);
        assert_eq!(
            display.worst_moment,
            spaced(&format!("12~h~20, latence {p90}"))
        );
        assert_eq!(display.on_network_since, spaced("3~h 12~min"));
        assert_eq!(display.outages, "Aucune");
    }

    #[test]
    fn waits_for_measures_before_judging_the_day() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let display = OverviewDisplay::compose(
            &create_overview(Aggregate::default()),
            false,
            MIDNIGHT,
            PARIS,
            &catalog,
        );

        // Assert
        assert_eq!(display.today, "Pas encore assez de mesures");
        assert_eq!(display.worst_moment, "Pas encore assez de mesures");
        assert_eq!(display.on_network_since, "Aucun réseau");
    }

    #[test]
    fn describes_the_outages_of_the_day() {
        // Arrange
        let catalog = TextCatalog::load();
        let now = MIDNIGHT + 14 * 3600;
        let one = DayOverview {
            outages: vec![Outage {
                start: MIDNIGHT + 12 * 3600 + 48 * 60,
                end: OutageEnd::At(MIDNIGHT + 12 * 3600 + 50 * 60),
            }],
            ..create_overview(Aggregate::default())
        };
        let many = DayOverview {
            outages: vec![
                Outage {
                    start: MIDNIGHT,
                    end: OutageEnd::At(MIDNIGHT + 40),
                },
                Outage {
                    start: MIDNIGHT + 3600,
                    end: OutageEnd::Unknown,
                },
            ],
            ..create_overview(Aggregate::default())
        };
        let ongoing = DayOverview {
            outages: vec![Outage {
                start: now - 60,
                end: OutageEnd::Ongoing,
            }],
            ..create_overview(Aggregate::default())
        };

        // Act
        let described: Vec<String> = [one, many, ongoing]
            .iter()
            .map(|overview| describe_outages(overview, now, PARIS, &catalog))
            .collect();

        // Assert
        assert_eq!(
            described,
            [
                "1, à 13~h~48 (2~min)",
                "2, la plus longue 40~s",
                "En cours depuis 1~min"
            ]
            .map(spaced)
        );
    }

    #[test]
    fn calls_a_minute_without_internet_an_outage() {
        // Arrange
        let catalog = TextCatalog::load();
        let mut offline = Aggregate::default();
        offline.record_second(&SecondSample {
            latency: None,
            jitter: None,
            probes_sent: 2,
            probes_lost: 2,
            state: Some(CountedState::Offline),
        });

        // Act
        let described = describe_worst_minute(Some(&(MIDNIGHT, offline)), PARIS, &catalog);

        // Assert
        assert_eq!(described, spaced("1~h~00, coupure"));
    }

    #[test]
    fn names_a_network_and_its_time_per_state() {
        // Arrange
        let catalog = TextCatalog::load();
        let mut aggregate = create_aggregate(75, CountedState::Solid, 20);
        aggregate.merge(&create_aggregate(25, CountedState::Unstable, 20));
        let summary = NetworkSummary {
            network: create_network(None),
            aggregate,
            is_current: true,
            is_most_reliable: false,
        };

        // Act
        let row = NetworkRowDisplay::compose(&summary, &catalog);

        // Assert
        assert_eq!(row.label, "Réseau Ethernet");
        assert_eq!(row.connection, "Ethernet");
        assert!((row.shares.solid - 75.0).abs() < 1e-9);
        assert_eq!(row.solid_label, spaced("75~% Solide"));
        assert_eq!(
            row.shares_label,
            spaced("75~% Solide, 25~% Instable, 0~% Dégradée, 0~% Hors ligne")
        );
        assert_eq!(row.loss, spaced("0~%"));
    }

    #[test]
    fn lays_out_the_chart_with_its_gaps_thresholds_and_network_changes() {
        // Arrange
        let catalog = TextCatalog::load();
        let start = MIDNIGHT;
        let evolution = Evolution {
            buckets: vec![
                (start, create_aggregate(30, CountedState::Solid, 20)),
                (start + 30, Aggregate::default()),
                (
                    start + 60,
                    create_aggregate(30, CountedState::Unstable, 150),
                ),
                (start + 90, Aggregate::default()),
            ],
            bucket_seconds: 30,
            network_changes: vec![HistoryEvent {
                utc: start + 60,
                network: create_network(Some("Box")),
                kind: application::EventKind::NetworkChanged,
            }],
            coverage: 0.5,
        };

        // Act
        let display = EvolutionDisplay::compose(&evolution, Period::Hour, PARIS, &catalog);

        // Assert
        assert_eq!(display.latency.values[1], None);
        assert!(display.latency.values[2].is_some_and(|value| value >= 150.0));
        assert_eq!(display.states[2], Some(DisplayState::Unstable));
        assert_eq!(display.latency.unstable_label, spaced("Instable 100~ms"));
        assert!((display.loss.degraded_above - 5.0).abs() < 1e-9);
        assert!((display.network_changes[0].position - 0.5).abs() < 1e-9);
        assert_eq!(
            display.network_changes[0].label,
            spaced("Changement de réseau~: Box")
        );
        assert_eq!(display.coverage, spaced("Mesuré 50~% de la période"));
        assert_eq!(display.times[0], spaced("1~h~00"));
        assert!(!display.is_empty);
        assert_eq!(display.state_labels[3].label, "Hors ligne");
    }
}
