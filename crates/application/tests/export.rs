//! Export seam: three RFC 4180 files written row by row, with neither MAC nor histogram.

use std::time::Duration;

use application::test_support::{InMemoryMeasurementRepository, generate_network_name};
use application::{
    EventKind, ExportPeriod, HistoryEvent, MINUTE_RETENTION, MeasurementRepository, NetworkKey,
    ThroughputRecord, encode_csv_line, export_events, export_measures, export_throughput_tests,
    format_utc_iso8601,
};
use domain::{
    Aggregate, Bitrate, ConnectionKind, CountedState, Granularity, MacAddress, SECONDS_PER_DAY,
    SECONDS_PER_HOUR, SECONDS_PER_MINUTE, SecondSample, TestProfile, ThroughputResult,
};

/// 2023-11-14 22:13:20 UTC.
const SOME_UTC: i64 = 1_700_000_000;

fn create_network() -> NetworkKey {
    create_network_named(&generate_network_name())
}

fn create_network_named(name: &str) -> NetworkKey {
    NetworkKey {
        gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, 0x41]),
        kind: ConnectionKind::WiFi,
        name: Some(name.to_owned()),
    }
}

fn create_minute() -> Aggregate {
    let mut aggregate = Aggregate::default();
    for _ in 0..60 {
        aggregate.record_second(&SecondSample {
            latency: Some(Duration::from_millis(20)),
            jitter: Some(Duration::from_millis(2)),
            probes_sent: 2,
            probes_lost: 0,
            state: Some(CountedState::Solid),
        });
    }
    aggregate
}

/// The three files, as text.
fn export(repository: &InMemoryMeasurementRepository, period: ExportPeriod) -> [String; 3] {
    let now = SOME_UTC + SECONDS_PER_HOUR;
    let mut measures = Vec::new();
    let mut events = Vec::new();
    let mut tests = Vec::new();
    export_measures(repository, period, now, &mut measures).expect("the measures");
    export_events(repository, period, now, &mut events).expect("the events");
    export_throughput_tests(repository, period, now, &mut tests).expect("the tests");
    [measures, events, tests].map(|bytes| String::from_utf8(bytes).expect("UTF-8"))
}

#[test]
fn quotes_only_the_fields_that_need_it() {
    // Act
    let line = encode_csv_line(&["Box", "Wi-Fi, 5 GHz", "dit \"bonjour\"", "a\nb"]);

    // Assert
    assert_eq!(
        line,
        "Box,\"Wi-Fi, 5 GHz\",\"dit \"\"bonjour\"\"\",\"a\nb\"\r\n"
    );
}

#[test]
fn writes_times_in_utc_iso_8601() {
    // Act & Assert
    assert_eq!(format_utc_iso8601(SOME_UTC), "2023-11-14T22:13:20Z");
    assert_eq!(format_utc_iso8601(951_782_400), "2000-02-29T00:00:00Z");
}

#[test]
fn exports_the_measures_by_network_name_without_mac_nor_histogram() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let network = create_network();
    let name = network.name.clone().expect("a named network");
    repository
        .add_minute(&network, SOME_UTC - 20, &create_minute())
        .expect("a minute");

    // Act
    let [measures, ..] = export(&repository, ExportPeriod::LastDay);

    // Assert
    let lines: Vec<&str> = measures.split("\r\n").collect();
    assert!(lines[0].starts_with(
        "start_utc,granularity,network,connection_type,latency_p50_ms,latency_p90_ms"
    ));
    assert!(lines[1].starts_with(&format!("2023-11-14T22:13:00Z,minute,{name},wifi,")));
    assert!(!measures.contains("54:9b"));
    assert_eq!(lines.len(), 3, "a header, one row and the last line break");
}

#[test]
fn exports_events_and_throughput_tests_in_their_own_files() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let network = create_network();
    let name = network.name.clone().expect("a named network");
    repository
        .record_event(&HistoryEvent {
            utc: SOME_UTC,
            network: network.clone(),
            kind: EventKind::OfflineStarted,
        })
        .expect("an event");
    repository
        .record_throughput_test(&ThroughputRecord {
            utc: SOME_UTC,
            network,
            result: ThroughputResult {
                profile: TestProfile::Quick,
                download: Some(Bitrate::from_bits_per_second(85_000_000)),
                upload: None,
                loaded_latency: Some(Duration::from_millis(46)),
                bytes: 4_200_000,
                duration: Duration::from_millis(3_100),
            },
        })
        .expect("a test");

    // Act
    let [_, events, tests] = export(&repository, ExportPeriod::All);

    // Assert
    assert!(events.contains(&format!(
        "2023-11-14T22:13:20Z,{name},wifi,offline_started\r\n"
    )));
    assert!(tests.contains(&format!(
        "2023-11-14T22:13:20Z,{name},wifi,quick,85000,,46,4200000,3100\r\n"
    )));
}

#[test]
fn leaves_out_what_is_older_than_the_period() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    repository
        .add_minute(
            &create_network(),
            SOME_UTC - 3 * SECONDS_PER_DAY,
            &create_minute(),
        )
        .expect("an old minute");

    // Act
    let [measures, ..] = export(&repository, ExportPeriod::LastDay);

    // Assert
    assert_eq!(measures.split("\r\n").count(), 2, "the header alone");
}

#[test]
fn exports_hours_where_minutes_are_gone_and_never_a_second_twice() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let network = create_network();
    let recent_hour = SOME_UTC - SOME_UTC % SECONDS_PER_HOUR;
    repository
        .save_hour(
            &network,
            recent_hour - 40 * SECONDS_PER_DAY,
            &create_minute(),
        )
        .expect("an old hour");
    repository
        .add_minute(
            &network,
            recent_hour - 40 * SECONDS_PER_DAY,
            &create_minute(),
        )
        .expect("its minute, not purged yet");
    repository
        .save_hour(&network, recent_hour, &create_minute())
        .expect("a recent hour");
    repository
        .add_minute(&network, recent_hour, &create_minute())
        .expect("its minute");

    // Act
    let [measures, ..] = export(&repository, ExportPeriod::All);

    // Assert
    let grains: Vec<&str> = measures
        .split("\r\n")
        .skip(1)
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split(',').nth(1))
        .collect();
    assert_eq!(grains, ["hour", "minute"]);
}

#[test]
fn exports_the_hour_where_the_purge_cut_its_minutes() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let network = create_network();
    let purge_cutoff = SOME_UTC + SECONDS_PER_HOUR - MINUTE_RETENTION.as_secs().cast_signed();
    let cut_hour = Granularity::Hour.find_start(purge_cutoff);
    repository
        .save_hour(&network, cut_hour, &create_minute())
        .expect("the cut hour");
    let kept_minute = Granularity::Minute.find_start(purge_cutoff) + SECONDS_PER_MINUTE;
    repository
        .add_minute(&network, kept_minute, &create_minute())
        .expect("a minute after the cutoff");

    // Act
    let [measures, ..] = export(&repository, ExportPeriod::All);

    // Assert
    let grains: Vec<&str> = measures
        .split("\r\n")
        .skip(1)
        .filter_map(|line| line.split(',').nth(1))
        .collect();
    assert_eq!(grains, ["hour"], "the hour already holds its minutes");
}

#[test]
fn keeps_a_network_name_from_becoming_a_spreadsheet_formula() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    repository
        .add_minute(
            &create_network_named("=HYPERLINK(\"x\")"),
            SOME_UTC - 20,
            &create_minute(),
        )
        .expect("a minute");

    // Act
    let [measures, ..] = export(&repository, ExportPeriod::LastDay);

    // Assert
    assert!(measures.contains(",minute,\"'=HYPERLINK(\"\"x\"\")\",wifi,"));
}

#[test]
fn starts_the_last_month_with_the_whole_hour_it_falls_in() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    let month_start = SOME_UTC + SECONDS_PER_HOUR - 30 * SECONDS_PER_DAY;
    let network = create_network();
    let name = network.name.clone().expect("a named network");
    repository
        .save_hour(
            &network,
            Granularity::Hour.find_start(month_start),
            &create_minute(),
        )
        .expect("the first hour");

    // Act
    let [measures, ..] = export(&repository, ExportPeriod::LastMonth);

    // Assert
    assert!(measures.contains(&format!(",hour,{name},")));
}

#[test]
fn gives_the_median_beside_the_ninetieth_percentile() {
    // Arrange
    let mut repository = InMemoryMeasurementRepository::default();
    repository
        .add_minute(&create_network(), SOME_UTC - 20, &create_minute())
        .expect("a minute");

    // Act
    let [measures, ..] = export(&repository, ExportPeriod::LastDay);

    // Assert
    let row: Vec<&str> = measures
        .split("\r\n")
        .nth(1)
        .expect("a row")
        .split(',')
        .collect();
    let p50: f64 = row[4].parse().expect("a p50");
    let p90: f64 = row[5].parse().expect("a p90");
    assert!(p50 > 0.0 && p50 <= p90, "{p50} then {p90}");
}
