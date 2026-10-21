//! The history as three RFC 4180 files: measures, events and throughput tests. Rows go to the
//! writer one at a time, so an export of years holds a single row in memory. Neither the gateway's
//! MAC nor the latency histogram leaves the app.

use std::fmt::Write as _;
use std::io::{self, Write};
use std::time::Duration;

use domain::{
    Aggregate, Bitrate, CivilDate, ConnectionKind, Granularity, SECONDS_PER_DAY, SECONDS_PER_HOUR,
    SECONDS_PER_MINUTE, TestProfile,
};

use crate::{
    EventKind, HistoryEvent, HistoryStream, MINUTE_RETENTION, NetworkKey, RepositoryError,
    ThroughputRecord,
};

const MEASURE_HEADER: [&str; 13] = [
    "debut_utc",
    "granularite",
    "reseau",
    "type_connexion",
    "latence_p90_ms",
    "gigue_moyenne_ms",
    "perte_pct",
    "sondes_envoyees",
    "sondes_perdues",
    "secondes_solide",
    "secondes_instable",
    "secondes_degrade",
    "secondes_hors_ligne",
];
const EVENT_HEADER: [&str; 4] = ["horodatage_utc", "reseau", "type_connexion", "evenement"];
const TEST_HEADER: [&str; 9] = [
    "horodatage_utc",
    "reseau",
    "type_connexion",
    "profil",
    "reception_kbps",
    "envoi_kbps",
    "latence_en_charge_ms",
    "octets",
    "duree_ms",
];
const BITS_PER_KILOBIT: u64 = 1000;
const MINUTE_RETENTION_SECONDS: i64 = MINUTE_RETENTION.as_secs().cast_signed();

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
            Self::LastDay => now_utc - SECONDS_PER_DAY,
            Self::LastWeek => now_utc - 7 * SECONDS_PER_DAY,
            Self::LastMonth => now_utc - 30 * SECONDS_PER_DAY,
            Self::All => i64::MIN,
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

/// Hours before the minutes still kept, then the minutes: each second once, at its finest grain.
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
    let minutes_from = Granularity::Hour.find_start(now_utc - MINUTE_RETENTION_SECONDS);
    if from < minutes_from {
        write_measures(stream, Granularity::Hour, from, minutes_from, writer)?;
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
    let date = CivilDate::from_days(utc.div_euclid(SECONDS_PER_DAY));
    let second_of_day = utc.rem_euclid(SECONDS_PER_DAY);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        date.year,
        date.month,
        date.day,
        second_of_day / SECONDS_PER_HOUR,
        second_of_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        second_of_day % SECONDS_PER_MINUTE
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
        to_network_name(network).to_owned(),
        to_kind_text(network.kind).to_owned(),
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
        format_utc_iso8601(event.utc).as_str(),
        to_network_name(&event.network),
        to_kind_text(event.network.kind),
        to_event_text(event.kind),
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
        to_network_name(&record.network).to_owned(),
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

/// "12.6": a dot, as a spreadsheet in any language reads a CSV. O(1).
fn format_tenths_of_millis(duration: Duration) -> String {
    let tenths = duration.as_micros() / 100;
    let mut text = String::new();
    let _ = write!(text, "{}.{}", tenths / 10, tenths % 10);
    text
}

fn to_network_name(network: &NetworkKey) -> &str {
    network.name.as_deref().unwrap_or_default()
}

const fn to_granularity_text(granularity: Granularity) -> &'static str {
    match granularity {
        Granularity::Minute => "minute",
        Granularity::Hour => "heure",
    }
}

const fn to_kind_text(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::Ethernet => "ethernet",
        ConnectionKind::WiFi => "wifi",
        ConnectionKind::Cellular => "cellulaire",
        ConnectionKind::PhoneHotspot => "partage_de_connexion",
        ConnectionKind::Other => "autre",
    }
}

const fn to_event_text(kind: EventKind) -> &'static str {
    match kind {
        EventKind::NetworkChanged => "changement_de_reseau",
        EventKind::OfflineStarted => "debut_coupure",
        EventKind::OfflineEnded => "fin_coupure",
        EventKind::Sleep => "veille",
        EventKind::Wake => "reveil",
    }
}

const fn to_profile_text(profile: TestProfile) -> &'static str {
    match profile {
        TestProfile::Quick => "rapide",
        TestProfile::Full => "complet",
    }
}
