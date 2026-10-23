//! The history as three RFC 4180 files: measures, events and throughput tests. Rows go to the
//! writer one at a time, so an export of years holds a single row in memory. Neither the gateway's
//! MAC nor the latency histogram leaves the app.

use std::io::{self, Write};
use std::time::Duration;

use domain::{Aggregate, Bitrate, CivilDateTime, ConnectionKind, Granularity, TestProfile};

use crate::history::MINUTE_RETENTION_SECONDS;
use crate::{
    EventKind, HistoryEvent, HistoryStream, NetworkKey, Period, RepositoryError, ThroughputRecord,
};

const MEASURE_HEADER: [&str; 14] = [
    "start_utc",
    "granularity",
    "network",
    "connection_type",
    "latency_p50_ms",
    "latency_p90_ms",
    "jitter_mean_ms",
    "loss_pct",
    "probes_sent",
    "probes_lost",
    "seconds_solid",
    "seconds_unstable",
    "seconds_degraded",
    "seconds_offline",
];
const EVENT_HEADER: [&str; 4] = ["time_utc", "network", "connection_type", "event"];
const TEST_HEADER: [&str; 9] = [
    "time_utc",
    "network",
    "connection_type",
    "profile",
    "download_kbps",
    "upload_kbps",
    "loaded_latency_ms",
    "bytes",
    "duration_ms",
];
const BITS_PER_KILOBIT: u64 = 1000;
/// What a spreadsheet reads as the start of a formula.
const FORMULA_STARTS: [char; 6] = ['=', '+', '-', '@', '\t', '\r'];

/// How far back an export reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPeriod {
    LastDay,
    LastWeek,
    LastMonth,
    All,
}

impl ExportPeriod {
    /// The first second exported. O(1).
    #[must_use]
    pub const fn find_start(self, now_utc: i64) -> i64 {
        match self {
            Self::LastDay => now_utc - Period::Day.to_seconds(),
            Self::LastWeek => now_utc - Period::Week.to_seconds(),
            Self::LastMonth => now_utc - Period::Month.to_seconds(),
            // Nothing was measured before 1970.
            Self::All => 0,
        }
    }
}

#[derive(Debug)]
pub enum ExportError {
    Storage(RepositoryError),
    Write(io::Error),
}

impl From<RepositoryError> for ExportError {
    fn from(error: RepositoryError) -> Self {
        Self::Storage(error)
    }
}

impl From<io::Error> for ExportError {
    fn from(error: io::Error) -> Self {
        Self::Write(error)
    }
}

/// Hours up to the one the purge cut into, then the minutes after it: each second once. The hour
/// holding the start of the period comes whole, a few minutes before it included.
/// O(r) time for r rows, O(1) space.
///
/// # Errors
///
/// The history could not be read, or the file written.
pub fn export_measures(
    stream: &impl HistoryStream,
    period: ExportPeriod,
    now_utc: i64,
    writer: &mut impl Write,
) -> Result<(), ExportError> {
    writer.write_all(encode_csv_line(&MEASURE_HEADER).as_bytes())?;
    let from = period.find_start(now_utc);
    // The purge keeps a minute until its hour is kept: past the cut hour, every minute is there.
    let minutes_from = Granularity::Hour.find_start(now_utc - MINUTE_RETENTION_SECONDS)
        + Granularity::Hour.to_seconds();
    if from < minutes_from {
        let hours_from = Granularity::Hour.find_start(from);
        write_measures(stream, Granularity::Hour, hours_from, minutes_from, writer)?;
    }
    write_measures(
        stream,
        Granularity::Minute,
        from.max(minutes_from),
        now_utc + 1,
        writer,
    )
}

/// O(r) time for r rows, O(1) space.
fn write_measures(
    stream: &impl HistoryStream,
    granularity: Granularity,
    from: i64,
    to: i64,
    writer: &mut impl Write,
) -> Result<(), ExportError> {
    stream.stream_all_aggregates(granularity, from, to, |network, start, aggregate| {
        let line = encode_measure_line(granularity, network, start, aggregate);
        writer.write_all(line.as_bytes()).map_err(ExportError::from)
    })
}

/// O(e) time for e events, O(1) space.
///
/// # Errors
///
/// The history could not be read, or the file written.
pub fn export_events(
    stream: &impl HistoryStream,
    period: ExportPeriod,
    now_utc: i64,
    writer: &mut impl Write,
) -> Result<(), ExportError> {
    writer.write_all(encode_csv_line(&EVENT_HEADER).as_bytes())?;
    stream.stream_events(period.find_start(now_utc), now_utc + 1, |event| {
        let line = encode_event_line(event);
        writer.write_all(line.as_bytes()).map_err(ExportError::from)
    })
}

/// O(t) time for t tests, O(1) space.
///
/// # Errors
///
/// The history could not be read, or the file written.
pub fn export_throughput_tests(
    stream: &impl HistoryStream,
    period: ExportPeriod,
    now_utc: i64,
    writer: &mut impl Write,
) -> Result<(), ExportError> {
    writer.write_all(encode_csv_line(&TEST_HEADER).as_bytes())?;
    stream.stream_throughput_tests(period.find_start(now_utc), now_utc + 1, |record| {
        let line = encode_test_line(record);
        writer.write_all(line.as_bytes()).map_err(ExportError::from)
    })
}

/// One CRLF-ended record; a field holding a comma, a quote or a line break is quoted, its quotes
/// doubled. O(length of the fields).
#[must_use]
pub fn encode_csv_line<S: AsRef<str>>(fields: &[S]) -> String {
    let mut line = String::new();
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            line.push(',');
        }
        let field = field.as_ref();
        if field.contains([',', '"', '\r', '\n']) {
            line.push('"');
            line.push_str(&field.replace('"', "\"\""));
            line.push('"');
        } else {
            line.push_str(field);
        }
    }
    line.push_str("\r\n");
    line
}

/// "2023-11-14T22:13:20Z". O(1).
#[must_use]
pub fn format_utc_iso8601(utc: i64) -> String {
    let moment = CivilDateTime::from_seconds(utc);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        moment.date.year,
        moment.date.month,
        moment.date.day,
        moment.hour,
        moment.minute,
        moment.second
    )
}

/// O(1).
fn encode_measure_line(
    granularity: Granularity,
    network: &NetworkKey,
    start: i64,
    aggregate: &Aggregate,
) -> String {
    let seconds = aggregate.seconds;
    encode_csv_line(&[
        format_utc_iso8601(start),
        to_granularity_text(granularity).to_owned(),
        to_network_name(network),
        to_kind_text(network.kind).to_owned(),
        format_or_empty(aggregate.compute_latency_p50(), format_tenths_of_millis),
        format_or_empty(aggregate.compute_latency_p90(), format_tenths_of_millis),
        format_or_empty(aggregate.compute_mean_jitter(), format_tenths_of_millis),
        format_or_empty(aggregate.compute_loss(), |loss| {
            format!("{:.2}", loss.to_percent())
        }),
        aggregate.probes_sent.to_string(),
        aggregate.probes_lost.to_string(),
        seconds.solid.to_string(),
        seconds.unstable.to_string(),
        seconds.degraded.to_string(),
        seconds.offline.to_string(),
    ])
}

/// O(1).
fn encode_event_line(event: &HistoryEvent) -> String {
    encode_csv_line(&[
        format_utc_iso8601(event.utc),
        to_network_name(&event.network),
        to_kind_text(event.network.kind).to_owned(),
        to_event_text(event.kind).to_owned(),
    ])
}

/// O(1).
fn encode_test_line(record: &ThroughputRecord) -> String {
    let result = &record.result;
    let kilobits = |rate: Option<Bitrate>| {
        format_or_empty(rate, |rate| {
            (rate.to_bits_per_second() / BITS_PER_KILOBIT).to_string()
        })
    };
    encode_csv_line(&[
        format_utc_iso8601(record.utc),
        to_network_name(&record.network),
        to_kind_text(record.network.kind).to_owned(),
        to_profile_text(result.profile).to_owned(),
        kilobits(result.download),
        kilobits(result.upload),
        format_or_empty(result.loaded_latency, |latency| {
            latency.as_millis().to_string()
        }),
        result.bytes.to_string(),
        result.duration.as_millis().to_string(),
    ])
}

/// An unmeasured value is an empty field. O(1).
fn format_or_empty<T>(value: Option<T>, format: impl FnOnce(T) -> String) -> String {
    value.map_or_else(String::new, format)
}

/// "12.6": a dot, as RFC 4180 tools read numbers. O(1).
fn format_tenths_of_millis(duration: Duration) -> String {
    let tenths = duration.as_micros() / 100;
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// A Wi-Fi name is anyone's choice: one that a spreadsheet would run as a formula is quoted with
/// a leading apostrophe. O(length).
fn to_network_name(network: &NetworkKey) -> String {
    let name = network.name.as_deref().unwrap_or_default();
    if name.starts_with(FORMULA_STARTS) {
        format!("'{name}")
    } else {
        name.to_owned()
    }
}

const fn to_granularity_text(granularity: Granularity) -> &'static str {
    match granularity {
        Granularity::Minute => "minute",
        Granularity::Hour => "hour",
    }
}

const fn to_kind_text(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::Ethernet => "ethernet",
        ConnectionKind::WiFi => "wifi",
        ConnectionKind::Cellular => "cellular",
        ConnectionKind::PhoneHotspot => "phone_hotspot",
        ConnectionKind::Other => "other",
    }
}

const fn to_event_text(kind: EventKind) -> &'static str {
    match kind {
        EventKind::NetworkChanged => "network_changed",
        EventKind::OfflineStarted => "offline_started",
        EventKind::OfflineEnded => "offline_ended",
        EventKind::Sleep => "sleep",
        EventKind::Wake => "wake",
    }
}

const fn to_profile_text(profile: TestProfile) -> &'static str {
    match profile {
        TestProfile::Quick => "quick",
        TestProfile::Full => "full",
    }
}
