//! History seam: minutes and events from the seconds, hours from the minutes.

use std::net::Ipv4Addr;
use std::sync::mpsc;
use std::time::Duration;

use application::test_support::{InMemoryHistoryRecorder, InMemoryMeasurementRepository};
use application::{
    EventKind, HistoryCommand, HistoryTracker, HistoryWriter, MINUTE_RETENTION, MeasuredSecond,
    MeasurementRepository, NetworkKey,
};
use domain::{
    Aggregate, ConnectionContext, ConnectionKind, CountedState, Grade, Granularity, MacAddress,
    Medium, Metering, SecondSample, Verdict,
};

/// An hour boundary, in seconds since 1970 UTC.
const SOME_HOUR: i64 = 1_700_002_800;
const MINUTE: i64 = 60;

fn create_context(last_octet: u8) -> ConnectionContext {
    ConnectionContext {
        kind: ConnectionKind::WiFi,
        is_vpn: false,
        gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, last_octet]),
        gateway_address: Some(Ipv4Addr::new(192, 168, last_octet, 1)),
        metering: Metering::Unmetered,
        medium: Medium::WiFi,
    }
}

fn create_key(last_octet: u8) -> NetworkKey {
    let context = create_context(last_octet);
    NetworkKey {
        gateway: context.gateway,
        kind: context.kind,
        name: None,
    }
}

fn create_second(utc: i64, verdict: Verdict) -> MeasuredSecond {
    MeasuredSecond {
        utc,
        latency: Some(Duration::from_millis(20)),
        probes_sent: 2,
        probes_lost: 0,
        verdict,
    }
}

fn create_minute() -> Aggregate {
    let mut aggregate = Aggregate::default();
    for _ in 0..60 {
        aggregate.record_second(&SecondSample {
            latency: Some(Duration::from_millis(20)),
            jitter: None,
            probes_sent: 2,
            probes_lost: 0,
            state: Some(CountedState::Solid),
        });
    }
    aggregate
}

fn save_minute(
    writer: &mut HistoryWriter<InMemoryMeasurementRepository>,
    network: &NetworkKey,
    start: i64,
) {
    writer
        .apply(
            HistoryCommand::SaveMinute {
                network: network.clone(),
                start,
                aggregate: create_minute(),
            },
            start + MINUTE,
        )
        .expect("minute saved");
}

#[test]
fn closes_the_hour_with_its_last_minute() {
    // Arrange
    let network = create_key(1);
    let mut writer = HistoryWriter::new(InMemoryMeasurementRepository::default());
    for minute in 0..59 {
        save_minute(&mut writer, &network, SOME_HOUR + minute * MINUTE);
    }

    // Act
    save_minute(&mut writer, &network, SOME_HOUR + 59 * MINUTE);

    // Assert
    let repository = writer.into_repository();
    let hours = repository
        .list_aggregates(&network, Granularity::Hour, SOME_HOUR, SOME_HOUR + 3600)
        .expect("hours");
    assert_eq!(hours.len(), 1);
    assert_eq!(hours[0].1.seconds.solid, 3600);
}

#[test]
fn finishes_the_hours_left_open_at_startup() {
    // Arrange
    let network = create_key(2);
    let mut writer = HistoryWriter::new(InMemoryMeasurementRepository::default());
    save_minute(&mut writer, &network, SOME_HOUR + 10 * MINUTE);

    // Act
    writer
        .finish_pending_hours(SOME_HOUR + 3600 + MINUTE)
        .expect("finished");

    // Assert
    let repository = writer.into_repository();
    let hours = repository
        .list_aggregates(&network, Granularity::Hour, SOME_HOUR, SOME_HOUR + 3600)
        .expect("hours");
    assert_eq!(hours[0].1.seconds.solid, 60);
}

#[test]
fn drops_minutes_older_than_thirty_days_once_their_hour_is_kept() {
    // Arrange
    let network = create_key(3);
    let mut writer = HistoryWriter::new(InMemoryMeasurementRepository::default());
    save_minute(&mut writer, &network, SOME_HOUR);
    let retention = i64::try_from(MINUTE_RETENTION.as_secs()).expect("retention");

    // Act
    writer
        .finish_pending_hours(SOME_HOUR + retention + 3600)
        .expect("finished");

    // Assert
    let repository = writer.into_repository();
    assert!(
        repository
            .list_aggregates(&network, Granularity::Minute, 0, i64::MAX)
            .expect("minutes")
            .is_empty()
    );
    assert_eq!(
        repository
            .list_aggregates(&network, Granularity::Hour, 0, i64::MAX)
            .expect("hours")
            .len(),
        1
    );
}

#[test]
fn hands_over_a_minute_and_marks_a_new_network() {
    // Arrange
    let recorder = InMemoryHistoryRecorder::default();
    let mut tracker = HistoryTracker::default();
    let solid = Verdict::Graded(Grade::Solid);
    for second in 0..60 {
        tracker.record_second(
            &create_second(SOME_HOUR + second, solid),
            Some(create_context(4)),
            None,
            &recorder,
        );
    }

    // Act
    tracker.record_second(
        &create_second(SOME_HOUR + 60, solid),
        Some(create_context(5)),
        None,
        &recorder,
    );

    // Assert
    let minutes = recorder.list_saved_minutes();
    assert_eq!(minutes.len(), 1);
    assert_eq!(minutes[0].0, create_key(4));
    assert_eq!(minutes[0].2.seconds.solid, 60);
    let kinds: Vec<EventKind> = recorder
        .list_events()
        .iter()
        .map(|event| event.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![EventKind::NetworkChanged, EventKind::NetworkChanged]
    );
}

#[test]
fn marks_the_start_and_end_of_an_outage() {
    // Arrange
    let recorder = InMemoryHistoryRecorder::default();
    let mut tracker = HistoryTracker::default();
    let context = Some(create_context(6));
    tracker.record_second(
        &create_second(SOME_HOUR, Verdict::Graded(Grade::Solid)),
        context,
        None,
        &recorder,
    );
    tracker.record_second(
        &create_second(SOME_HOUR + 1, Verdict::Offline),
        None,
        None,
        &recorder,
    );

    // Act
    tracker.record_second(
        &create_second(SOME_HOUR + 2, Verdict::Graded(Grade::Solid)),
        context,
        None,
        &recorder,
    );

    // Assert
    let kinds: Vec<EventKind> = recorder
        .list_events()
        .iter()
        .map(|event| event.kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            EventKind::NetworkChanged,
            EventKind::OfflineStarted,
            EventKind::OfflineEnded
        ]
    );
}

#[test]
fn writes_the_minute_in_progress_on_flush() {
    // Arrange
    let recorder = InMemoryHistoryRecorder::default();
    let mut tracker = HistoryTracker::default();
    tracker.record_second(
        &create_second(SOME_HOUR, Verdict::Graded(Grade::Solid)),
        Some(create_context(7)),
        None,
        &recorder,
    );
    let (answer, answered) = mpsc::sync_channel(1);

    // Act
    tracker.flush(answer, &recorder);

    // Assert
    assert_eq!(recorder.list_saved_minutes().len(), 1);
    assert!(answered.recv().is_ok());
}

#[test]
fn keeps_the_last_hour_second_by_second() {
    // Arrange
    let recorder = InMemoryHistoryRecorder::default();
    let mut tracker = HistoryTracker::default();

    // Act
    for second in 0..3 {
        tracker.record_second(
            &create_second(SOME_HOUR + second, Verdict::Measuring),
            Some(create_context(8)),
            None,
            &recorder,
        );
    }

    // Assert
    let points = tracker.list_last_hour();
    assert_eq!(points.len(), 3);
    assert_eq!(points[2].utc, SOME_HOUR + 2);
}

#[test]
fn closes_the_hours_of_a_network_left_before_the_hour_ended() {
    // Arrange
    let home = create_key(10);
    let office = create_key(11);
    let mut writer = HistoryWriter::new(InMemoryMeasurementRepository::default());
    save_minute(&mut writer, &home, SOME_HOUR + 10 * MINUTE);

    // Act
    save_minute(&mut writer, &office, SOME_HOUR + 3600 + 59 * MINUTE);

    // Assert
    let repository = writer.into_repository();
    assert_eq!(
        repository
            .list_aggregates(&home, Granularity::Hour, SOME_HOUR, SOME_HOUR + 3600)
            .expect("hours")
            .len(),
        1
    );
}

#[test]
fn counts_the_seconds_before_any_network_under_no_network() {
    // Arrange
    let recorder = InMemoryHistoryRecorder::default();
    let mut tracker = HistoryTracker::default();
    tracker.record_second(
        &create_second(SOME_HOUR, Verdict::Offline),
        None,
        None,
        &recorder,
    );

    // Act
    tracker.record_second(
        &create_second(SOME_HOUR + 1, Verdict::Measuring),
        Some(create_context(12)),
        None,
        &recorder,
    );

    // Assert
    let minutes = recorder.list_saved_minutes();
    assert_eq!(minutes.len(), 1);
    assert_eq!(minutes[0].0.kind, ConnectionKind::Other);
    assert_eq!(minutes[0].2.seconds.offline, 1);
    assert_eq!(tracker.list_last_hour().len(), 2);
}

#[test]
fn keeps_the_gateway_learnt_earlier_when_a_reading_misses_it() {
    // Arrange
    let recorder = InMemoryHistoryRecorder::default();
    let mut tracker = HistoryTracker::default();
    let solid = Verdict::Graded(Grade::Solid);
    tracker.record_second(
        &create_second(SOME_HOUR, solid),
        Some(create_context(13)),
        None,
        &recorder,
    );
    let missed = ConnectionContext {
        gateway: None,
        ..create_context(13)
    };

    // Act
    tracker.record_second(
        &create_second(SOME_HOUR + 1, solid),
        Some(missed),
        None,
        &recorder,
    );
    tracker.record_second(
        &create_second(SOME_HOUR + 60, solid),
        Some(missed),
        None,
        &recorder,
    );

    // Assert
    assert_eq!(recorder.list_saved_minutes()[0].0, create_key(13));
}

#[test]
fn measures_no_jitter_across_a_gap() {
    // Arrange
    let recorder = InMemoryHistoryRecorder::default();
    let mut tracker = HistoryTracker::default();
    let context = Some(create_context(14));
    let solid = Verdict::Graded(Grade::Solid);
    tracker.record_second(&create_second(SOME_HOUR, solid), context, None, &recorder);
    let after_sleep = MeasuredSecond {
        latency: Some(Duration::from_millis(400)),
        ..create_second(SOME_HOUR + 30, solid)
    };

    // Act
    tracker.record_second(&after_sleep, context, None, &recorder);
    tracker.record_second(
        &create_second(SOME_HOUR + 60, solid),
        context,
        None,
        &recorder,
    );

    // Assert
    assert_eq!(recorder.list_saved_minutes()[0].2.jitter_count, 0);
}

#[test]
fn writes_a_throughput_test_under_its_network() {
    // Arrange
    let home = create_key(14);
    let record = application::ThroughputRecord {
        utc: SOME_HOUR + 30,
        network: home,
        result: domain::ThroughputResult {
            profile: domain::TestProfile::Quick,
            download: Some(domain::Bitrate::from_bits_per_second(50_000_000)),
            upload: Some(domain::Bitrate::from_bits_per_second(12_000_000)),
            loaded_latency: Some(Duration::from_millis(41)),
            bytes: 4_200_000,
            duration: Duration::from_millis(3_200),
        },
    };
    let mut writer = HistoryWriter::new(InMemoryMeasurementRepository::default());

    // Act
    writer
        .apply(
            HistoryCommand::RecordThroughput(record.clone()),
            SOME_HOUR + 31,
        )
        .expect("a write");

    // Assert
    let repository = writer.into_repository();
    assert_eq!(
        application::HistoryReader::list_throughput_tests(
            &repository,
            SOME_HOUR,
            SOME_HOUR + MINUTE
        )
        .expect("tests"),
        vec![record]
    );
}

#[test]
fn erases_everything_written_before() {
    // Arrange
    let mut writer = HistoryWriter::new(InMemoryMeasurementRepository::default());
    let network = create_key(30);
    save_minute(&mut writer, &network, SOME_HOUR);

    // Act
    writer
        .apply(HistoryCommand::EraseAll, SOME_HOUR + MINUTE)
        .expect("an erase");

    // Assert
    let repository = writer.into_repository();
    let minutes = repository
        .list_aggregates(&network, Granularity::Minute, 0, SOME_HOUR + MINUTE)
        .expect("minutes");
    assert!(minutes.is_empty());
}

#[test]
fn starts_afresh_on_erase_without_the_minute_in_progress() {
    // Arrange
    let recorder = InMemoryHistoryRecorder::default();
    let mut tracker = HistoryTracker::default();
    let context = Some(create_context(31));
    tracker.record_second(
        &create_second(SOME_HOUR, Verdict::Offline),
        context,
        None,
        &recorder,
    );
    let (answer, answered) = mpsc::sync_channel(1);

    // Act
    tracker.erase(answer, &recorder);

    // Assert
    tracker.record_second(
        &create_second(SOME_HOUR + 1, Verdict::Offline),
        context,
        None,
        &recorder,
    );
    let kinds: Vec<EventKind> = recorder
        .list_events()
        .iter()
        .map(|event| event.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            EventKind::NetworkChanged,
            EventKind::OfflineStarted,
            EventKind::NetworkChanged,
            EventKind::OfflineStarted,
        ],
        "the outage and the network are noted again after the erase"
    );
    assert!(recorder.list_saved_minutes().is_empty());
    assert_eq!(tracker.list_last_hour().len(), 1);
    assert!(answered.recv().is_ok());
}
