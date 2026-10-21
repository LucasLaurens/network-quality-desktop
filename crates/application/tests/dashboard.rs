//! Dashboard seam: the day so far, a period's evolution and the networks, read from the history.

use std::time::Duration;

use application::test_support::InMemoryMeasurementRepository;
use application::{
    EventKind, HistoryEvent, LiveHistory, MeasurementRepository, NetworkKey, NetworkStay,
    OutageEnd, Period, list_network_summaries, read_day_overview, read_evolution,
};
use domain::{Aggregate, ConnectionKind, CountedState, MacAddress, SecondPoint, SecondSample};

/// A midnight, UTC, in seconds since 1970.
const MIDNIGHT: i64 = 1_699_920_000;
const MINUTE: i64 = 60;
const HOUR: i64 = 3600;
const DAY: i64 = 86_400;

fn create_network(last_octet: u8, name: &str) -> NetworkKey {
    NetworkKey {
        gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, last_octet]),
        kind: ConnectionKind::WiFi,
        name: Some(name.to_owned()),
    }
}

fn create_sample(state: CountedState) -> SecondSample {
    SecondSample {
        latency: Some(Duration::from_millis(20)),
        jitter: Some(Duration::from_millis(2)),
        probes_sent: 2,
        probes_lost: 0,
        state: Some(state),
    }
}

fn create_aggregate(seconds: u32, state: CountedState) -> Aggregate {
    let mut aggregate = Aggregate::default();
    for _ in 0..seconds {
        aggregate.record_second(&create_sample(state));
    }
    aggregate
}

fn create_event(utc: i64, network: &NetworkKey, kind: EventKind) -> HistoryEvent {
    HistoryEvent {
        utc,
        network: network.clone(),
        kind,
    }
}

fn create_live(network: Option<NetworkKey>) -> LiveHistory {
    LiveHistory {
        last_hour: Vec::new(),
        network,
    }
}

fn add_minute(
    repository: &mut InMemoryMeasurementRepository,
    network: &NetworkKey,
    start: i64,
    aggregate: &Aggregate,
) {
    repository
        .add_minute(network, start, aggregate)
        .expect("a minute");
}

fn record_event(repository: &mut InMemoryMeasurementRepository, event: &HistoryEvent) {
    repository.record_event(event).expect("an event");
}

#[test]
fn charts_a_day_from_minutes_and_leaves_the_unmeasured_buckets_empty() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(1, "Box");
    let now = MIDNIGHT + DAY;
    let minute = now - HOUR;
    add_minute(
        &mut repository,
        &home,
        minute,
        &create_aggregate(60, CountedState::Solid),
    );

    // Act
    let evolution = read_evolution(&repository, &create_live(Some(home)), Period::Day, now)
        .expect("an evolution");

    // Assert
    assert_eq!(evolution.buckets.len(), 144);
    let measured: Vec<i64> = evolution
        .buckets
        .iter()
        .filter(|(_, aggregate)| aggregate.count_measured_seconds() > 0)
        .map(|(start, _)| *start)
        .collect();
    assert_eq!(measured, vec![minute]);
    assert!((evolution.coverage - 60.0 / 86_400.0).abs() < 1e-9);
}

#[test]
fn charts_the_last_hour_from_the_seconds_in_memory() {
    // Arrange
    let repository = InMemoryMeasurementRepository::default();
    let now = MIDNIGHT + 10 * HOUR;
    let live = LiveHistory {
        last_hour: (1..=80)
            .map(|ago| SecondPoint {
                utc: now - ago,
                sample: create_sample(CountedState::Unstable),
            })
            .rev()
            .collect(),
        network: None,
    };

    // Act
    let evolution = read_evolution(&repository, &live, Period::Hour, now).expect("an evolution");

    // Assert
    assert_eq!(evolution.bucket_seconds, 60);
    let last_two: Vec<u32> = evolution
        .buckets
        .iter()
        .rev()
        .take(2)
        .map(|(_, aggregate)| aggregate.seconds.unstable)
        .collect();
    // The minute memory starts in mid-way is left to the stored minutes.
    assert_eq!(last_two, vec![60, 0]);
}

#[test]
fn charts_the_start_of_the_hour_from_stored_minutes_after_a_restart() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(15, "Box");
    let now = MIDNIGHT + 10 * HOUR;
    let restart = now - 2 * MINUTE + 30;
    for start in [now - 10 * MINUTE, now - 3 * MINUTE] {
        add_minute(
            &mut repository,
            &home,
            start,
            &create_aggregate(60, CountedState::Degraded),
        );
    }
    add_minute(
        &mut repository,
        &home,
        now - 2 * MINUTE,
        &create_aggregate(30, CountedState::Degraded),
    );
    let live = LiveHistory {
        last_hour: (restart..now)
            .map(|utc| SecondPoint {
                utc,
                sample: create_sample(CountedState::Solid),
            })
            .collect(),
        network: Some(home),
    };

    // Act
    let evolution = read_evolution(&repository, &live, Period::Hour, now).expect("an evolution");

    // Assert
    let seconds = |start: i64| {
        evolution
            .buckets
            .iter()
            .find(|(bucket, _)| *bucket == start)
            .map(|(_, aggregate)| aggregate.seconds)
            .expect("a bucket")
    };
    assert_eq!(seconds(now - 10 * MINUTE).degraded, 60);
    assert_eq!(seconds(now - 3 * MINUTE).degraded, 60);
    assert_eq!(seconds(now - 2 * MINUTE).degraded, 30);
    assert_eq!(seconds(now - 2 * MINUTE).solid, 0);
    assert_eq!(seconds(now - MINUTE).solid, 60);
}

#[test]
fn charts_a_week_from_hours_and_the_current_hour_from_minutes() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(2, "Box");
    let now = MIDNIGHT + 10 * HOUR + 30 * MINUTE;
    repository
        .save_hour(
            &home,
            MIDNIGHT + 8 * HOUR,
            &create_aggregate(3600, CountedState::Solid),
        )
        .expect("an hour");
    add_minute(
        &mut repository,
        &home,
        MIDNIGHT + 10 * HOUR,
        &create_aggregate(60, CountedState::Degraded),
    );

    // Act
    let evolution = read_evolution(&repository, &create_live(Some(home)), Period::Week, now)
        .expect("an evolution");

    // Assert
    let seconds = |start: i64| {
        evolution
            .buckets
            .iter()
            .find(|(bucket, _)| *bucket == start)
            .map(|(_, aggregate)| aggregate.seconds)
            .expect("a bucket")
    };
    assert_eq!(seconds(MIDNIGHT + 8 * HOUR).solid, 3600);
    assert_eq!(seconds(MIDNIGHT + 9 * HOUR).count_total(), 0);
    assert_eq!(seconds(MIDNIGHT + 10 * HOUR).degraded, 60);
}

#[test]
fn marks_a_change_of_network_but_not_a_restart_on_the_same_one() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(3, "Box");
    let phone = create_network(4, "iPhone");
    let now = MIDNIGHT + 12 * HOUR;
    let unresolved_home = NetworkKey {
        gateway: None,
        ..home.clone()
    };
    for event in [
        create_event(now - 5 * HOUR, &home, EventKind::NetworkChanged),
        create_event(now - 4 * HOUR, &unresolved_home, EventKind::NetworkChanged),
        create_event(now - 3 * HOUR, &home, EventKind::NetworkChanged),
        create_event(now - 2 * HOUR, &phone, EventKind::NetworkChanged),
    ] {
        record_event(&mut repository, &event);
    }

    // Act
    let evolution = read_evolution(
        &repository,
        &create_live(Some(phone.clone())),
        Period::Day,
        now,
    )
    .expect("an evolution");

    // Assert
    let changes: Vec<(i64, Option<String>)> = evolution
        .network_changes
        .iter()
        .map(|event| (event.utc, event.network.name.clone()))
        .collect();
    assert_eq!(changes, vec![(now - 2 * HOUR, phone.name)]);
}

#[test]
fn marks_a_move_between_two_named_networks_even_before_the_gateway_is_read() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(16, "Box");
    let office_unresolved = NetworkKey {
        gateway: None,
        ..create_network(17, "Bureau")
    };
    let now = MIDNIGHT + 12 * HOUR;
    for event in [
        create_event(now - 3 * HOUR, &home, EventKind::NetworkChanged),
        create_event(now - HOUR, &office_unresolved, EventKind::NetworkChanged),
    ] {
        record_event(&mut repository, &event);
    }

    // Act
    let evolution = read_evolution(
        &repository,
        &create_live(Some(office_unresolved)),
        Period::Day,
        now,
    )
    .expect("an evolution");

    // Assert
    assert_eq!(evolution.network_changes.len(), 1);
}

#[test]
fn counts_an_outage_that_began_before_midnight_and_ended_after() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(18, "Box");
    for (utc, kind) in [
        (MIDNIGHT - 30, EventKind::OfflineStarted),
        (MIDNIGHT + 90, EventKind::OfflineEnded),
    ] {
        record_event(&mut repository, &create_event(utc, &home, kind));
    }

    // Act
    let overview = read_day_overview(
        &repository,
        &create_live(Some(home)),
        MIDNIGHT,
        MIDNIGHT + HOUR,
    )
    .expect("an overview");

    // Assert
    let outages: Vec<(i64, OutageEnd)> = overview
        .outages
        .iter()
        .map(|outage| (outage.start, outage.end))
        .collect();
    assert_eq!(outages, vec![(MIDNIGHT - 30, OutageEnd::At(MIDNIGHT + 90))]);
}

#[test]
fn says_when_no_network_is_in_use() {
    // Arrange
    let repository = InMemoryMeasurementRepository::default();

    // Act
    let overview = read_day_overview(&repository, &create_live(None), MIDNIGHT, MIDNIGHT + HOUR)
        .expect("an overview");

    // Assert
    assert_eq!(overview.network_stay, NetworkStay::NoNetwork);
}

#[test]
fn sums_the_day_since_local_midnight_only() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(5, "Box");
    add_minute(
        &mut repository,
        &home,
        MIDNIGHT - MINUTE,
        &create_aggregate(60, CountedState::Degraded),
    );
    add_minute(
        &mut repository,
        &home,
        MIDNIGHT + HOUR,
        &create_aggregate(60, CountedState::Solid),
    );

    // Act
    let overview = read_day_overview(
        &repository,
        &create_live(Some(home)),
        MIDNIGHT,
        MIDNIGHT + 2 * HOUR,
    )
    .expect("an overview");

    // Assert
    assert_eq!(overview.today.seconds.solid, 60);
    assert_eq!(overview.today.seconds.degraded, 0);
}

#[test]
fn lists_the_outages_of_the_day_with_how_they_ended() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(6, "Box");
    let now = MIDNIGHT + 12 * HOUR;
    for (utc, kind) in [
        (MIDNIGHT - HOUR, EventKind::OfflineStarted),
        (MIDNIGHT - HOUR + 30, EventKind::OfflineEnded),
        (MIDNIGHT + HOUR, EventKind::OfflineStarted),
        (MIDNIGHT + HOUR + 40, EventKind::OfflineEnded),
        (MIDNIGHT + 2 * HOUR, EventKind::OfflineStarted),
        (MIDNIGHT + 3 * HOUR, EventKind::OfflineStarted),
        (now - 20, EventKind::OfflineEnded),
        (now - 5, EventKind::OfflineStarted),
    ] {
        record_event(&mut repository, &create_event(utc, &home, kind));
    }
    let live = LiveHistory {
        last_hour: vec![SecondPoint {
            utc: now - 1,
            sample: create_sample(CountedState::Offline),
        }],
        network: Some(home),
    };

    // Act
    let overview = read_day_overview(&repository, &live, MIDNIGHT, now).expect("an overview");

    // Assert
    let outages: Vec<(i64, OutageEnd)> = overview
        .outages
        .iter()
        .map(|outage| (outage.start, outage.end))
        .collect();
    assert_eq!(
        outages,
        vec![
            (MIDNIGHT + HOUR, OutageEnd::At(MIDNIGHT + HOUR + 40)),
            (MIDNIGHT + 2 * HOUR, OutageEnd::Unknown),
            (MIDNIGHT + 3 * HOUR, OutageEnd::At(now - 20)),
            (now - 5, OutageEnd::Ongoing),
        ]
    );
}

#[test]
fn finds_the_minute_of_the_day_hardest_on_calls() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(7, "Box");
    let mut mixed = create_aggregate(50, CountedState::Solid);
    mixed.merge(&create_aggregate(10, CountedState::Unstable));
    add_minute(&mut repository, &home, MIDNIGHT + HOUR, &mixed);
    add_minute(
        &mut repository,
        &home,
        MIDNIGHT + 2 * HOUR,
        &create_aggregate(30, CountedState::Degraded),
    );
    add_minute(
        &mut repository,
        &home,
        MIDNIGHT + 3 * HOUR,
        &create_aggregate(60, CountedState::Solid),
    );

    // Act
    let overview = read_day_overview(
        &repository,
        &create_live(Some(home)),
        MIDNIGHT,
        MIDNIGHT + 4 * HOUR,
    )
    .expect("an overview");

    // Assert
    assert_eq!(
        overview.worst_minute.map(|(start, _)| start),
        Some(MIDNIGHT + 2 * HOUR)
    );
}

#[test]
fn names_no_worst_minute_on_a_day_without_trouble() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(8, "Box");
    add_minute(
        &mut repository,
        &home,
        MIDNIGHT + HOUR,
        &create_aggregate(60, CountedState::Solid),
    );

    // Act
    let overview = read_day_overview(
        &repository,
        &create_live(Some(home)),
        MIDNIGHT,
        MIDNIGHT + 2 * HOUR,
    )
    .expect("an overview");

    // Assert
    assert_eq!(overview.worst_minute, None);
}

#[test]
fn says_since_when_the_computer_is_on_the_current_network() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(9, "Box");
    let phone = create_network(10, "iPhone");
    let now = MIDNIGHT + 12 * HOUR;
    for event in [
        create_event(now - 6 * HOUR, &phone, EventKind::NetworkChanged),
        create_event(now - 5 * HOUR, &home, EventKind::NetworkChanged),
        create_event(now - 4 * HOUR, &home, EventKind::OfflineStarted),
        create_event(now - HOUR, &home, EventKind::NetworkChanged),
    ] {
        record_event(&mut repository, &event);
    }

    // Act
    let overview = read_day_overview(&repository, &create_live(Some(home)), MIDNIGHT, now)
        .expect("an overview");

    // Assert
    assert_eq!(overview.network_stay, NetworkStay::Since(now - 5 * HOUR));
}

#[test]
fn sums_each_network_over_30_days_and_names_the_most_reliable() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(11, "Box");
    let phone = create_network(12, "iPhone");
    let now = MIDNIGHT + 12 * HOUR + 10 * MINUTE;
    let mut home_hour = create_aggregate(3000, CountedState::Solid);
    home_hour.merge(&create_aggregate(600, CountedState::Unstable));
    repository
        .save_hour(&home, MIDNIGHT, &home_hour)
        .expect("home hour");
    repository
        .save_hour(&home, MIDNIGHT - 40 * DAY, &home_hour)
        .expect("an hour too old");
    repository
        .save_hour(
            &phone,
            MIDNIGHT + HOUR,
            &create_aggregate(3600, CountedState::Solid),
        )
        .expect("phone hour");
    add_minute(
        &mut repository,
        &phone,
        MIDNIGHT + 12 * HOUR,
        &create_aggregate(60, CountedState::Solid),
    );

    // Act
    let summaries =
        list_network_summaries(&repository, &create_live(Some(phone)), now).expect("summaries");

    // Assert
    let listed: Vec<(Option<&str>, u32, bool, bool)> = summaries
        .iter()
        .map(|summary| {
            (
                summary.network.name.as_deref(),
                summary.aggregate.count_measured_seconds(),
                summary.is_current,
                summary.is_most_reliable,
            )
        })
        .collect();
    assert_eq!(
        listed,
        vec![
            (Some("iPhone"), 3660, true, true),
            (Some("Box"), 3600, false, false),
        ]
    );
}

#[test]
fn names_no_most_reliable_network_without_another_to_compare() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let home = create_network(13, "Box");
    let phone = create_network(14, "iPhone");
    repository
        .save_hour(
            &home,
            MIDNIGHT,
            &create_aggregate(3600, CountedState::Solid),
        )
        .expect("home hour");
    // Too short to compare.
    add_minute(
        &mut repository,
        &phone,
        MIDNIGHT + HOUR,
        &create_aggregate(60, CountedState::Solid),
    );

    // Act
    let summaries =
        list_network_summaries(&repository, &create_live(None), MIDNIGHT + HOUR + MINUTE)
            .expect("summaries");

    // Assert
    assert_eq!(summaries.len(), 2);
    assert!(summaries.iter().all(|summary| !summary.is_most_reliable));
}
