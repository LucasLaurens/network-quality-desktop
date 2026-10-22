//! The behaviour every `MeasurementRepository` and `HistoryReader` share. Each adapter runs these checks on a fresh
//! repository, so the in-memory fake and the database cannot drift apart.

use std::time::Duration;

use domain::{
    Aggregate, Appearance, Bitrate, ConnectionKind, CountedState, Granularity, MacAddress,
    MeasurementServers, SecondSample, Settings, Target, TestProfile, ThroughputResult,
};

use crate::{
    EventKind, HistoryEvent, HistoryReader, MeasurementRepository, NetworkKey, SettingsRepository,
    ThroughputRecord,
};

const HOUR: i64 = 3600;
const MINUTE: i64 = 60;
/// An hour boundary, in seconds since 1970 UTC.
const SOME_HOUR: i64 = 1_700_002_800;

fn create_network(last_octet: u8, name: Option<&str>) -> NetworkKey {
    NetworkKey {
        gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, last_octet]),
        kind: ConnectionKind::WiFi,
        name: name.map(str::to_owned),
    }
}

fn create_aggregate(seconds: u32, latency_ms: u64) -> Aggregate {
    let mut aggregate = Aggregate::default();
    for _ in 0..seconds {
        aggregate.record_second(&SecondSample {
            latency: Some(Duration::from_millis(latency_ms)),
            jitter: Some(Duration::from_millis(1)),
            probes_sent: 2,
            probes_lost: 0,
            state: Some(CountedState::Solid),
        });
    }
    aggregate
}

/// Runs every check on fresh repositories made by `create`.
///
/// # Panics
///
/// When the repository breaks the contract.
pub fn check_contract<R: MeasurementRepository + HistoryReader>(create: impl Fn() -> R) {
    adds_to_a_minute_already_kept(&mut create());
    replaces_an_hour_already_kept(&mut create());
    lists_rows_in_range_oldest_first(&mut create());
    keeps_networks_apart(&mut create());
    finds_hours_left_open(&mut create());
    purges_only_old_minutes(&mut create());
    keeps_old_minutes_whose_hour_is_not_kept(&mut create());
    records_events_in_order(&mut create());
    keeps_a_name_learnt_later(&mut create());
    keeps_a_name_when_seen_again_without_it(&mut create());
    learns_a_name_from_an_event(&mut create());
    groups_networks_without_a_readable_gateway_by_kind(&mut create());
    lists_the_rows_of_every_network_in_range(&mut create());
    keeps_every_kind_of_event(&mut create());
    keeps_throughput_tests_newest_first(&mut create());
}

/// Rates in whole kbit/s and round trips in whole milliseconds: the precision the storage keeps.
fn keeps_throughput_tests_newest_first(
    repository: &mut (impl MeasurementRepository + HistoryReader),
) {
    // Arrange
    let network = create_network(23, Some("Box"));
    let older = ThroughputRecord {
        utc: SOME_HOUR,
        network: network.clone(),
        result: ThroughputResult {
            profile: TestProfile::Quick,
            download: Some(Bitrate::from_bits_per_second(85_000_000)),
            upload: None,
            loaded_latency: None,
            bytes: 4_200_000,
            duration: Duration::from_millis(3_100),
        },
    };
    let newer = ThroughputRecord {
        utc: SOME_HOUR + 120,
        network,
        result: ThroughputResult {
            profile: TestProfile::Full,
            download: Some(Bitrate::from_bits_per_second(412_000_000)),
            upload: Some(Bitrate::from_bits_per_second(96_000_000)),
            loaded_latency: Some(Duration::from_millis(46)),
            bytes: 138_000_000,
            duration: Duration::from_millis(24_500),
        },
    };

    // Act
    repository
        .record_throughput_test(&older)
        .expect("an older test");
    repository
        .record_throughput_test(&newer)
        .expect("a newer test");

    // Assert
    let tests = repository
        .list_throughput_tests(SOME_HOUR, SOME_HOUR + HOUR)
        .expect("tests");
    assert_eq!(tests, vec![newer, older]);
}

fn keeps_every_kind_of_event(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let network = create_network(22, Some("Box"));
    let kinds = [
        EventKind::NetworkChanged,
        EventKind::OfflineStarted,
        EventKind::OfflineEnded,
        EventKind::Sleep,
        EventKind::Wake,
    ];

    // Act
    for (offset, kind) in (0..).zip(kinds) {
        repository
            .record_event(&HistoryEvent {
                utc: SOME_HOUR + offset,
                network: network.clone(),
                kind,
            })
            .expect("an event");
    }

    // Assert
    let read: Vec<EventKind> = repository
        .list_events(SOME_HOUR, SOME_HOUR + MINUTE)
        .expect("events")
        .iter()
        .map(|event| event.kind)
        .collect();
    assert_eq!(read, kinds);
}

fn lists_the_rows_of_every_network_in_range(
    repository: &mut (impl MeasurementRepository + HistoryReader),
) {
    // Arrange
    let home = create_network(20, Some("Box"));
    let office = create_network(21, None);
    repository
        .add_minute(&office, SOME_HOUR + MINUTE, &create_aggregate(60, 30))
        .expect("office minute");
    repository
        .add_minute(&home, SOME_HOUR, &create_aggregate(60, 15))
        .expect("home minute");
    repository
        .add_minute(&home, SOME_HOUR + HOUR, &create_aggregate(60, 15))
        .expect("minute out of range");
    repository
        .save_hour(&home, SOME_HOUR, &create_aggregate(60, 15))
        .expect("hour");

    // Act
    let rows = repository
        .list_all_aggregates(Granularity::Minute, SOME_HOUR, SOME_HOUR + HOUR)
        .expect("rows");

    // Assert
    let listed: Vec<(Option<&str>, i64)> = rows
        .iter()
        .map(|(network, start, _)| (network.name.as_deref(), *start))
        .collect();
    assert_eq!(
        listed,
        vec![(Some("Box"), SOME_HOUR), (None, SOME_HOUR + MINUTE)]
    );
}

fn adds_to_a_minute_already_kept(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let network = create_network(1, None);
    repository
        .add_minute(&network, SOME_HOUR, &create_aggregate(20, 15))
        .expect("first part");

    // Act
    repository
        .add_minute(&network, SOME_HOUR, &create_aggregate(40, 15))
        .expect("second part");

    // Assert
    let rows = repository
        .list_aggregates(&network, Granularity::Minute, SOME_HOUR, SOME_HOUR + MINUTE)
        .expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.seconds.solid, 60);
}

fn replaces_an_hour_already_kept(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let network = create_network(2, None);
    repository
        .save_hour(&network, SOME_HOUR, &create_aggregate(10, 15))
        .expect("first hour");
    let merged = create_aggregate(3600, 15);

    // Act
    repository
        .save_hour(&network, SOME_HOUR, &merged)
        .expect("second hour");

    // Assert
    let rows = repository
        .list_aggregates(&network, Granularity::Hour, SOME_HOUR, SOME_HOUR + HOUR)
        .expect("rows");
    assert_eq!(rows, vec![(SOME_HOUR, merged)]);
}

fn lists_rows_in_range_oldest_first(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let network = create_network(3, None);
    for minute in [2, 0, 1] {
        repository
            .add_minute(
                &network,
                SOME_HOUR + minute * MINUTE,
                &create_aggregate(60, 15),
            )
            .expect("minute");
    }

    // Act
    let rows = repository
        .list_aggregates(
            &network,
            Granularity::Minute,
            SOME_HOUR,
            SOME_HOUR + 2 * MINUTE,
        )
        .expect("rows");

    // Assert
    let starts: Vec<i64> = rows.iter().map(|(start, _)| *start).collect();
    assert_eq!(starts, vec![SOME_HOUR, SOME_HOUR + MINUTE]);
}

fn keeps_networks_apart(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let home = create_network(4, None);
    let office = create_network(5, None);
    repository
        .add_minute(&home, SOME_HOUR, &create_aggregate(60, 15))
        .expect("home");

    // Act
    repository
        .add_minute(&office, SOME_HOUR, &create_aggregate(30, 80))
        .expect("office");

    // Assert
    let office_rows = repository
        .list_aggregates(&office, Granularity::Minute, SOME_HOUR, SOME_HOUR + MINUTE)
        .expect("rows");
    assert_eq!(office_rows[0].1.seconds.solid, 30);
}

fn finds_hours_left_open(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let network = create_network(6, None);
    repository
        .add_minute(&network, SOME_HOUR + 5 * MINUTE, &create_aggregate(60, 15))
        .expect("open hour");
    repository
        .add_minute(&network, SOME_HOUR + HOUR, &create_aggregate(60, 15))
        .expect("closed hour");
    repository
        .save_hour(&network, SOME_HOUR + HOUR, &create_aggregate(60, 15))
        .expect("hour row");

    // Act
    let open = repository
        .list_unmerged_hours(SOME_HOUR + 2 * HOUR)
        .expect("open hours");

    // Assert
    let hours: Vec<i64> = open.iter().map(|(_, hour)| *hour).collect();
    assert_eq!(hours, vec![SOME_HOUR]);
}

fn purges_only_old_minutes(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let network = create_network(7, None);
    repository
        .add_minute(&network, SOME_HOUR, &create_aggregate(60, 15))
        .expect("old minute");
    repository
        .add_minute(&network, SOME_HOUR + HOUR, &create_aggregate(60, 15))
        .expect("recent minute");
    repository
        .save_hour(&network, SOME_HOUR, &create_aggregate(60, 15))
        .expect("old hour");

    // Act
    repository
        .purge_minutes_before(SOME_HOUR + HOUR)
        .expect("purge");

    // Assert
    let minutes = repository
        .list_aggregates(&network, Granularity::Minute, 0, i64::MAX)
        .expect("minutes");
    let hours = repository
        .list_aggregates(&network, Granularity::Hour, 0, i64::MAX)
        .expect("hours");
    assert_eq!(minutes.len(), 1);
    assert_eq!(hours.len(), 1);
}

fn records_events_in_order(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let network = create_network(8, Some("Box"));
    let started = HistoryEvent {
        utc: SOME_HOUR,
        network: network.clone(),
        kind: EventKind::OfflineStarted,
    };
    let ended = HistoryEvent {
        utc: SOME_HOUR + 12,
        network,
        kind: EventKind::OfflineEnded,
    };

    // Act
    repository.record_event(&started).expect("start");
    repository.record_event(&ended).expect("end");

    // Assert
    let events = repository
        .list_events(SOME_HOUR, SOME_HOUR + MINUTE)
        .expect("events");
    assert_eq!(events, vec![started, ended]);
}

fn keeps_a_name_learnt_later(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let unnamed = create_network(9, None);
    let named = create_network(9, Some("Livebox"));
    repository
        .add_minute(&unnamed, SOME_HOUR, &create_aggregate(60, 15))
        .expect("unnamed");

    // Act
    repository
        .add_minute(&named, SOME_HOUR + MINUTE, &create_aggregate(60, 15))
        .expect("named");

    // Assert
    let open = repository
        .list_unmerged_hours(SOME_HOUR + HOUR)
        .expect("open hours");
    assert_eq!(open[0].0.name.as_deref(), Some("Livebox"));
}

fn keeps_a_name_when_seen_again_without_it(
    repository: &mut (impl MeasurementRepository + HistoryReader),
) {
    // Arrange
    let named = create_network(10, Some("Livebox"));
    let unnamed = create_network(10, None);
    repository
        .add_minute(&named, SOME_HOUR, &create_aggregate(60, 15))
        .expect("named");

    // Act
    repository
        .add_minute(&unnamed, SOME_HOUR + MINUTE, &create_aggregate(60, 15))
        .expect("unnamed");

    // Assert
    let open = repository
        .list_unmerged_hours(SOME_HOUR + HOUR)
        .expect("open hours");
    assert_eq!(open[0].0.name.as_deref(), Some("Livebox"));
}

fn learns_a_name_from_an_event(repository: &mut (impl MeasurementRepository + HistoryReader)) {
    // Arrange
    let unnamed = create_network(11, None);
    repository
        .add_minute(&unnamed, SOME_HOUR, &create_aggregate(60, 15))
        .expect("unnamed");
    let event = HistoryEvent {
        utc: SOME_HOUR + MINUTE,
        network: create_network(11, Some("Bureau")),
        kind: EventKind::NetworkChanged,
    };

    // Act
    repository.record_event(&event).expect("event");

    // Assert
    let open = repository
        .list_unmerged_hours(SOME_HOUR + HOUR)
        .expect("open hours");
    assert_eq!(open[0].0.name.as_deref(), Some("Bureau"));
}

fn groups_networks_without_a_readable_gateway_by_kind(
    repository: &mut (impl MeasurementRepository + HistoryReader),
) {
    // Arrange
    let unreadable = NetworkKey {
        gateway: None,
        kind: ConnectionKind::Ethernet,
        name: None,
    };
    repository
        .add_minute(&unreadable, SOME_HOUR, &create_aggregate(20, 15))
        .expect("first");

    // Act
    repository
        .add_minute(&unreadable, SOME_HOUR, &create_aggregate(40, 15))
        .expect("second");

    // Assert
    let rows = repository
        .list_aggregates(
            &unreadable,
            Granularity::Minute,
            SOME_HOUR,
            SOME_HOUR + MINUTE,
        )
        .expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1.seconds.solid, 60);
}

fn keeps_old_minutes_whose_hour_is_not_kept(
    repository: &mut (impl MeasurementRepository + HistoryReader),
) {
    // Arrange
    let network = create_network(12, None);
    repository
        .add_minute(&network, SOME_HOUR, &create_aggregate(60, 15))
        .expect("minute of an open hour");

    // Act
    repository
        .purge_minutes_before(SOME_HOUR + HOUR)
        .expect("purge");

    // Assert
    assert_eq!(
        repository
            .list_aggregates(&network, Granularity::Minute, 0, i64::MAX)
            .expect("minutes")
            .len(),
        1
    );
}

/// Runs every settings check on fresh repositories made by `create`.
///
/// # Panics
///
/// When the repository breaks the contract.
pub fn check_settings_contract<R: SettingsRepository>(create: impl Fn() -> R) {
    reads_the_defaults_before_any_save(&create());
    reads_back_what_was_saved(&mut create());
    goes_back_to_the_public_servers(&mut create());
}

fn reads_the_defaults_before_any_save(repository: &impl SettingsRepository) {
    // Act
    let settings = repository.read_settings().expect("settings");

    // Assert
    assert_eq!(settings, Settings::default());
}

fn reads_back_what_was_saved(repository: &mut impl SettingsRepository) {
    // Arrange
    let saved = Settings {
        launch_at_login: false,
        appearance: Appearance::Dark,
        servers: MeasurementServers::Custom(Target::parse("2001:db8::7")),
        notifies_on_degradation: false,
        allows_throughput_tests: true,
    };

    // Act
    repository.save_settings(&saved).expect("a save");

    // Assert
    assert_eq!(repository.read_settings().expect("settings"), saved);
}

fn goes_back_to_the_public_servers(repository: &mut impl SettingsRepository) {
    // Arrange
    repository
        .save_settings(&Settings {
            servers: MeasurementServers::Custom(Target::parse("192.0.2.9")),
            ..Settings::default()
        })
        .expect("a custom server");

    // Act
    repository
        .save_settings(&Settings {
            appearance: Appearance::Light,
            ..Settings::default()
        })
        .expect("the public servers");

    // Assert
    let settings = repository.read_settings().expect("settings");
    assert_eq!(settings.servers, MeasurementServers::Default);
    assert_eq!(settings.appearance, Appearance::Light);
}
