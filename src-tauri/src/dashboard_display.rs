//! The dashboard's figures in words: the day so far, the evolution chart and the networks.

use std::time::Duration;

use application::{
    DayOverview, Evolution, NetworkKey, NetworkStay, NetworkSummary, OutageEnd, Period,
};
use domain::{
    Aggregate, CountedState, GradeBounds, LossRate, Metric, SECONDS_PER_DAY, Scalable, Thresholds,
};
use fluent_bundle::FluentArgs;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::display::{DisplayState, Displayable, to_lower_name_key};
use crate::network_display::select_stored_network_keys;
use crate::text_catalog::TextCatalog;
use crate::time_display::{format_date_hour, format_duration, format_time_of_day};

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
    /// What the share of the day rests on, « mesuré 18 h sur 24 h »; empty before any measure.
    pub today_coverage: String,
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
    /// How long the network was measured, which its shares rest on.
    pub measured: String,
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
        now: i64,
        offset: i64,
        catalog: &TextCatalog,
    ) -> Self {
        Self {
            overview: OverviewDisplay::compose(overview, now, offset, catalog),
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
    pub fn compose(overview: &DayOverview, now: i64, offset: i64, catalog: &TextCatalog) -> Self {
        let seconds = overview.today.seconds;
        let has_measures = overview.today.count_measured_seconds() > 0;
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
            today_coverage: if has_measures {
                describe_coverage(
                    i64::from(seconds.count_total()),
                    now - compute_day_start(now, offset),
                    catalog,
                )
            } else {
                String::new()
            },
            outages: describe_outages(overview, now, offset, catalog),
            worst_moment: if has_measures {
                describe_worst_minute(overview.worst_minute.as_ref(), offset, catalog)
            } else {
                catalog.render("today-not-enough", None)
            },
            on_network_since: match overview.network_stay {
                NetworkStay::NoNetwork => catalog.render("since-no-network", None),
                NetworkStay::BeyondHistory => catalog.render("since-beyond", None),
                NetworkStay::Since(since) => format_duration(now - since, catalog),
            },
        }
    }
}

impl NetworkRowDisplay {
    /// O(B), B = 24 histogram buckets.
    #[must_use]
    pub fn compose(summary: &NetworkSummary, catalog: &TextCatalog) -> Self {
        let (_, connection_key) = select_stored_network_keys(summary.network.kind);
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
            label: compose_network_name(&summary.network, catalog),
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
            measured: format_duration(i64::from(total), catalog),
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
        let thresholds = Thresholds::VIDEO_CALLS;
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
            .map(|aggregate| aggregate.compute_dominant_state().map(DisplayState::from))
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
                thresholds.latency,
                catalog,
            ),
            jitter: compose_series(
                &aggregates,
                Aggregate::compute_mean_jitter,
                to_milliseconds,
                thresholds.jitter,
                catalog,
            ),
            loss: compose_series(
                &aggregates,
                Aggregate::compute_loss,
                LossRate::to_percent,
                thresholds.loss,
                catalog,
            ),
            network_changes: evolution
                .network_changes
                .iter()
                .map(|event| {
                    let mut args = FluentArgs::new();
                    args.set("network", compose_network_name(&event.network, catalog));
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
                // Rounded down, as every share: 100 % only for a fully measured period.
                format!("{:.0}", (evolution.coverage * 100.0).floor()),
                catalog,
            ),
        }
    }
}

/// O(n·B).
fn compose_series<T: Displayable + Scalable>(
    aggregates: &[&Aggregate],
    read: fn(&Aggregate) -> Option<T>,
    to_number: fn(T) -> f64,
    bounds: GradeBounds<T>,
    catalog: &TextCatalog,
) -> SeriesDisplay {
    let unstable_from = bounds.read_unstable_from();
    let degraded_above = bounds.read_degraded_above();
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
        .compute_worst_metric(&Thresholds::VIDEO_CALLS)
        .filter(|_| minute.compute_dominant_state() != Some(CountedState::Offline));
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
            args.set("metric", catalog.render(to_lower_name_key(metric), None));
            args.set("value", value);
            catalog.render("worst-moment", Some(&args))
        }
        None => catalog.render("worst-outage", Some(&args)),
    }
}

/// The Wi-Fi name kept in the history, a generic name otherwise. O(1).
fn compose_network_name(network: &NetworkKey, catalog: &TextCatalog) -> String {
    let (name_key, _) = select_stored_network_keys(network.kind);
    network
        .name
        .clone()
        .unwrap_or_else(|| catalog.render(name_key, None))
}

pub fn format_or_unknown<T: Displayable>(value: Option<T>, catalog: &TextCatalog) -> String {
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

/// « mesuré 18 h sur 24 h »: the seconds counted in a state out of those elapsed. O(1).
fn describe_coverage(measured: i64, elapsed: i64, catalog: &TextCatalog) -> String {
    let mut args = FluentArgs::new();
    args.set("measured", format_duration(measured, catalog));
    args.set("elapsed", format_duration(elapsed, catalog));
    catalog.render("today-coverage", Some(&args))
}

#[cfg(test)]
mod tests {
    use application::test_support::generate_network_name;
    use application::{HistoryEvent, NetworkKey, Outage};
    use domain::{ConnectionKind, MacAddress, SECONDS_PER_HOUR, SecondSample};

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
            network_stay: NetworkStay::NoNetwork,
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
    fn insert_no_break_spaces(text: &str) -> String {
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
    fn says_how_solid_the_day_was_and_when_it_was_hardest() {
        // Arrange
        let catalog = TextCatalog::load();
        let mut today = create_aggregate(90, CountedState::Solid, 20);
        let worst = create_aggregate(10, CountedState::Degraded, 340);
        today.merge(&worst);
        let overview = DayOverview {
            worst_minute: Some((MIDNIGHT + 11 * 3600 + 20 * 60, worst)),
            network_stay: NetworkStay::Since(MIDNIGHT),
            ..create_overview(today)
        };

        // Act
        let display =
            OverviewDisplay::compose(&overview, MIDNIGHT + 3 * 3600 + 12 * 60, PARIS, &catalog);

        // Assert
        assert_eq!(
            display.today,
            insert_no_break_spaces("Solide 90~% du temps")
        );
        // The day started at local midnight, an hour before midnight UTC in Paris.
        assert_eq!(
            display.today_coverage,
            insert_no_break_spaces("mesuré 1~min sur 4~h 12~min")
        );
        // The histogram keeps the upper edge of the bucket holding the p90.
        let p90 = worst
            .compute_latency_p90()
            .expect("latencies")
            .format(&catalog);
        assert_eq!(
            display.worst_moment,
            insert_no_break_spaces(&format!("12~h~20, latence {p90}"))
        );
        assert_eq!(
            display.on_network_since,
            insert_no_break_spaces("3~h 12~min")
        );
        assert_eq!(display.outages, "Aucune");
    }

    #[test]
    fn waits_for_measures_before_judging_the_day() {
        // Arrange
        let catalog = TextCatalog::load();

        // Act
        let display = OverviewDisplay::compose(
            &create_overview(Aggregate::default()),
            MIDNIGHT,
            PARIS,
            &catalog,
        );

        // Assert
        assert_eq!(display.today, "Pas encore assez de mesures");
        assert_eq!(display.today_coverage, "");
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
            .map(insert_no_break_spaces)
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
        assert_eq!(described, insert_no_break_spaces("1~h~00, coupure"));
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
        assert_eq!(row.solid_label, insert_no_break_spaces("75~% Solide"));
        assert_eq!(
            row.shares_label,
            insert_no_break_spaces("75~% Solide, 25~% Instable, 0~% Dégradée, 0~% Hors ligne")
        );
        assert_eq!(row.loss, insert_no_break_spaces("0~%"));
        assert_eq!(row.measured, insert_no_break_spaces("1~min"));
    }

    #[test]
    fn counts_a_network_measured_for_days_in_days_and_hours() {
        // Arrange
        let catalog = TextCatalog::load();
        let mut aggregate = Aggregate::default();
        aggregate.seconds.solid =
            u32::try_from(2 * SECONDS_PER_DAY + 14 * SECONDS_PER_HOUR).expect("fits a u32");
        let summary = NetworkSummary {
            network: create_network(None),
            aggregate,
            is_current: false,
            is_most_reliable: false,
        };

        // Act
        let row = NetworkRowDisplay::compose(&summary, &catalog);

        // Assert
        assert_eq!(row.measured, insert_no_break_spaces("2~j 14~h"));
    }

    #[test]
    fn lays_out_the_chart_with_its_gaps_thresholds_and_network_changes() {
        // Arrange
        let catalog = TextCatalog::load();
        let name = generate_network_name();
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
                network: create_network(Some(name.as_str())),
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
        assert_eq!(
            display.latency.unstable_label,
            insert_no_break_spaces("Instable 100~ms")
        );
        assert!((display.loss.degraded_above - 5.0).abs() < 1e-9);
        assert!((display.network_changes[0].position - 0.5).abs() < 1e-9);
        assert_eq!(
            display.network_changes[0].label,
            insert_no_break_spaces(&format!("Changement de réseau~: {name}"))
        );
        assert_eq!(
            display.coverage,
            insert_no_break_spaces("Mesuré 50~% de la période")
        );
        assert_eq!(display.times[0], insert_no_break_spaces("1~h~00"));
        assert!(!display.is_empty);
        assert_eq!(display.state_labels[3].label, "Hors ligne");
    }
}
