use std::path::{Path, PathBuf};
use std::time::Duration;

use application::{
    EventKind, HistoryEvent, HistoryReader, HistoryStream, MeasurementRepository, NetworkKey,
    RepositoryError, SettingsRepository, ThroughputRecord,
};
use domain::{
    Aggregate, Appearance, Bitrate, ConnectionKind, Granularity, LatencyHistogram, MacAddress,
    MeasurementServers, Settings, StateSeconds, Target, TestProfile, ThroughputResult,
};
use rusqlite::{Connection, ErrorCode, OpenFlags, OptionalExtension, Row, Transaction, params};
use rusqlite_migration::{M, Migrations};

const MIGRATIONS: &[M<'static>] = &[
    M::up(include_str!("migrations/0001_create_history.sql")),
    M::up(include_str!(
        "migrations/0002_index_purge_and_unreadable_gateways.sql"
    )),
    M::up(include_str!(
        "migrations/0003_index_throughput_tests_by_time.sql"
    )),
];
/// The columns `read_throughput_record` reads, in its order.
const THROUGHPUT_COLUMNS: &str = "SELECT n.gateway_mac, n.connection_kind, n.label, t.ts, t.profile, \
     t.down_kbps, t.up_kbps, t.loaded_latency_ms, t.bytes, t.duration_ms FROM throughput_tests t \
     JOIN networks n ON n.id = t.network_id";
const MINUTE: i64 = 0;
const HOUR: i64 = 1;
const SECONDS_PER_HOUR: i64 = Granularity::Hour.to_seconds();
/// Another process holding the file briefly (a backup tool) is waited for, not taken for damage.
const BUSY_TIMEOUT: Duration = Duration::from_secs(2);
const AGGREGATE_COLUMNS: &str = "latency_hist, jitter_sum_us, jitter_count, probes_sent, \
     probes_lost, s_solid, s_unstable, s_degraded, s_offline";

/// The history file, and where an unreadable one was set aside, if it was.
pub struct OpenedHistory {
    pub repository: SqliteMeasurementRepository,
    pub set_aside: Option<PathBuf>,
}

pub struct SqliteMeasurementRepository {
    connection: Connection,
}

impl SqliteMeasurementRepository {
    /// Opens `path`. A damaged file (not a database, or failing its integrity check) is renamed
    /// next to it with its journals, kept, and a new history starts. Any other failure (busy,
    /// no permission, a schema from a newer version) is returned and the file left untouched.
    /// O(file) for the integrity check.
    ///
    /// # Errors
    ///
    /// The file could not be opened for another reason than damage, or a fresh one neither.
    pub fn open(path: &Path, now_utc: i64) -> Result<OpenedHistory, RepositoryError> {
        match Self::open_checked(path) {
            Ok(repository) => Ok(OpenedHistory {
                repository,
                set_aside: None,
            }),
            Err(OpenFailure::Damaged(reason)) => {
                eprintln!("The history is damaged, it is set aside: {reason}");
                let set_aside = move_aside(path, now_utc).map_err(to_error)?;
                let repository = Self::open_checked(path).map_err(OpenFailure::into_error)?;
                Ok(OpenedHistory {
                    repository,
                    set_aside: Some(set_aside),
                })
            }
            Err(OpenFailure::Other(error)) => Err(error),
        }
    }

    /// A fresh history in memory, for tests.
    ///
    /// # Errors
    ///
    /// The migrations failed.
    pub fn open_in_memory() -> Result<Self, RepositoryError> {
        Self::prepare(Connection::open_in_memory().map_err(to_error)?)
    }

    /// A second connection to a history the writer already opened, for the dashboard: it reads
    /// alongside the writer (WAL) and can write nothing.
    ///
    /// # Errors
    ///
    /// The file could not be opened.
    pub fn open_read_only(path: &Path) -> Result<Self, RepositoryError> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(to_error)?;
        connection.busy_timeout(BUSY_TIMEOUT).map_err(to_error)?;
        Ok(Self { connection })
    }

    /// Another writing connection to a history the writer already opened, for the settings: a
    /// rare write that waits for the writer at most `BUSY_TIMEOUT`.
    ///
    /// # Errors
    ///
    /// The file could not be opened.
    pub fn open_alongside(path: &Path) -> Result<Self, RepositoryError> {
        let connection = Connection::open(path).map_err(to_error)?;
        connection.busy_timeout(BUSY_TIMEOUT).map_err(to_error)?;
        Ok(Self { connection })
    }

    fn open_checked(path: &Path) -> Result<Self, OpenFailure> {
        let connection = Connection::open(path).map_err(OpenFailure::classify)?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .map_err(OpenFailure::classify)?;
        let check: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(OpenFailure::classify)?;
        if check != "ok" {
            return Err(OpenFailure::Damaged(format!("integrity check: {check}")));
        }
        Self::prepare(connection).map_err(OpenFailure::Other)
    }

    fn prepare(mut connection: Connection) -> Result<Self, RepositoryError> {
        // WAL with NORMAL sync: a power cut loses at most the last write, never a half one.
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(to_error)?;
        connection
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(to_error)?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(to_error)?;
        Migrations::from_slice(MIGRATIONS)
            .to_latest(&mut connection)
            .map_err(to_error)?;
        Ok(Self { connection })
    }

    fn write<T>(
        &mut self,
        work: impl FnOnce(&Transaction<'_>) -> rusqlite::Result<T>,
    ) -> Result<T, RepositoryError> {
        let transaction = self.connection.transaction().map_err(to_error)?;
        let result = work(&transaction).map_err(to_error)?;
        transaction.commit().map_err(to_error)?;
        Ok(result)
    }
}

impl MeasurementRepository for SqliteMeasurementRepository {
    /// O(B) plus one indexed lookup.
    fn add_minute(
        &mut self,
        network: &NetworkKey,
        start: i64,
        aggregate: &Aggregate,
    ) -> Result<(), RepositoryError> {
        self.write(|transaction| {
            let network_id = find_or_create_network(transaction, network)?;
            let mut merged = *aggregate;
            if let Some(kept) = transaction
                .query_row(
                    &format!(
                        "SELECT {AGGREGATE_COLUMNS} FROM aggregates \
                         WHERE network_id = ?1 AND granularity = ?2 AND start = ?3"
                    ),
                    params![network_id, MINUTE, start],
                    read_aggregate,
                )
                .optional()?
            {
                merged.merge(&kept);
            }
            save_aggregate(transaction, network_id, MINUTE, start, &merged)
        })
    }

    fn save_hour(
        &mut self,
        network: &NetworkKey,
        start: i64,
        aggregate: &Aggregate,
    ) -> Result<(), RepositoryError> {
        self.write(|transaction| {
            let network_id = find_or_create_network(transaction, network)?;
            save_aggregate(transaction, network_id, HOUR, start, aggregate)
        })
    }

    /// O(log n + k) on the primary key.
    fn list_aggregates(
        &self,
        network: &NetworkKey,
        granularity: Granularity,
        from: i64,
        to: i64,
    ) -> Result<Vec<(i64, Aggregate)>, RepositoryError> {
        let mut statement = self
            .connection
            .prepare(&format!(
                "SELECT a.start, {AGGREGATE_COLUMNS} FROM aggregates a \
                 JOIN networks n ON n.id = a.network_id \
                 WHERE n.gateway_mac IS ?1 AND n.connection_kind = ?2 \
                 AND a.granularity = ?3 AND a.start >= ?4 AND a.start < ?5 ORDER BY a.start"
            ))
            .map_err(to_error)?;
        let rows = statement
            .query_map(
                params![
                    to_mac_blob(network.gateway),
                    to_kind_text(network.kind),
                    to_granularity_code(granularity),
                    from,
                    to
                ],
                |row| Ok((row.get(0)?, read_aggregate_from(row, 1)?)),
            )
            .map_err(to_error)?;
        rows.collect::<rusqlite::Result<_>>().map_err(to_error)
    }

    /// O(m log m) for the m minutes kept (30 days at most), each checked on the primary key.
    fn list_unmerged_hours(&self, before: i64) -> Result<Vec<(NetworkKey, i64)>, RepositoryError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT n.gateway_mac, n.connection_kind, n.label, \
                 a.start - a.start % ?2 AS hour FROM aggregates a \
                 JOIN networks n ON n.id = a.network_id \
                 WHERE a.granularity = ?3 AND a.start < ?1 AND NOT EXISTS ( \
                     SELECT 1 FROM aggregates h WHERE h.network_id = a.network_id \
                     AND h.granularity = ?4 AND h.start = a.start - a.start % ?2) \
                 ORDER BY hour",
            )
            .map_err(to_error)?;
        let rows = statement
            .query_map(params![before, SECONDS_PER_HOUR, MINUTE, HOUR], |row| {
                Ok((read_network(row, 0)?, row.get(3)?))
            })
            .map_err(to_error)?;
        rows.collect::<rusqlite::Result<_>>().map_err(to_error)
    }

    /// Only minutes whose hour is kept go. O(log n + k) through the (granularity, start) index,
    /// each hour checked on the primary key.
    fn purge_minutes_before(&mut self, utc: i64) -> Result<(), RepositoryError> {
        self.write(|transaction| {
            transaction.execute(
                "DELETE FROM aggregates WHERE granularity = ?2 AND start < ?1 AND EXISTS ( \
                     SELECT 1 FROM aggregates h WHERE h.network_id = aggregates.network_id \
                     AND h.granularity = ?3 \
                     AND h.start = aggregates.start - aggregates.start % ?4)",
                params![utc, MINUTE, HOUR, SECONDS_PER_HOUR],
            )?;
            Ok(())
        })
    }

    /// Rates kept in kbit/s, round trips and durations in milliseconds. O(log n).
    fn record_throughput_test(&mut self, record: &ThroughputRecord) -> Result<(), RepositoryError> {
        let result = record.result;
        self.write(|transaction| {
            let network_id = find_or_create_network(transaction, &record.network)?;
            transaction.execute(
                "INSERT INTO throughput_tests (ts, network_id, profile, down_kbps, up_kbps, \
                 loaded_latency_ms, bytes, duration_ms, status) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.utc,
                    network_id,
                    to_profile_text(result.profile),
                    result.download.map(to_kilobits),
                    result.upload.map(to_kilobits),
                    result.loaded_latency.map(to_millis),
                    to_column(result.bytes),
                    to_millis(result.duration),
                    STATUS_DONE
                ],
            )?;
            Ok(())
        })
    }

    fn record_event(&mut self, event: &HistoryEvent) -> Result<(), RepositoryError> {
        self.write(|transaction| {
            let network_id = find_or_create_network(transaction, &event.network)?;
            transaction.execute(
                "INSERT INTO events (ts, network_id, kind) VALUES (?1, ?2, ?3)",
                params![event.utc, network_id, to_event_text(event.kind)],
            )?;
            Ok(())
        })
    }

    /// Deleted pages are zeroed as they go (`secure_delete`), then the file is rewritten and the
    /// journal emptied. The rows are gone once the transaction commits: a rewrite that fails, or a
    /// journal a reader still holds, is only logged. O(file).
    fn erase_history(&mut self) -> Result<(), RepositoryError> {
        self.connection
            .pragma_update(None, "secure_delete", true)
            .map_err(to_error)?;
        self.write(|transaction| {
            transaction.execute_batch(
                "DELETE FROM aggregates; DELETE FROM events; DELETE FROM throughput_tests; \
                 DELETE FROM networks;",
            )
        })?;
        if let Err(error) = self
            .connection
            .execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")
        {
            eprintln!("Rewriting the history after the erase failed: {error}");
        }
        Ok(())
    }
}

impl HistoryReader for SqliteMeasurementRepository {
    /// O(log n + k) through the (granularity, start) index.
    fn list_all_aggregates(
        &self,
        granularity: Granularity,
        from: i64,
        to: i64,
    ) -> Result<Vec<(NetworkKey, i64, Aggregate)>, RepositoryError> {
        let mut rows = Vec::new();
        self.stream_all_aggregates(granularity, from, to, |network, start, aggregate| {
            rows.push((network.clone(), start, *aggregate));
            Ok::<_, RepositoryError>(())
        })?;
        Ok(rows)
    }

    /// Newest first. O(t log t) for the t tests kept: a test is a click, so there are few.
    fn list_throughput_tests(
        &self,
        from: i64,
        to: i64,
    ) -> Result<Vec<ThroughputRecord>, RepositoryError> {
        let mut tests = Vec::new();
        self.stream_throughput_tests(from, to, |record| {
            tests.push(record.clone());
            Ok::<_, RepositoryError>(())
        })?;
        tests.reverse();
        Ok(tests)
    }

    /// O(log t + offset + limit) through the time index, for the t tests kept.
    fn list_throughput_page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ThroughputRecord>, RepositoryError> {
        let mut statement = self
            .connection
            .prepare(&format!(
                "{THROUGHPUT_COLUMNS} WHERE t.status = ?1 ORDER BY t.ts DESC, t.rowid DESC \
                 LIMIT ?2 OFFSET ?3"
            ))
            .map_err(to_error)?;
        let rows = statement
            .query_map(
                params![
                    STATUS_DONE,
                    to_column(limit as u64),
                    to_column(offset as u64)
                ],
                read_throughput_record,
            )
            .map_err(to_error)?;
        rows.collect::<rusqlite::Result<_>>().map_err(to_error)
    }

    /// O(t): one pass over the tests kept, a few hundred at most.
    fn count_throughput_tests(&self) -> Result<usize, RepositoryError> {
        let count: i64 = self
            .connection
            .query_row(
                "SELECT COUNT(*) FROM throughput_tests WHERE status = ?1",
                params![STATUS_DONE],
                |row| row.get(0),
            )
            .map_err(to_error)?;
        Ok(usize::try_from(count).unwrap_or(0))
    }

    /// O(1): each table is asked for one row at most.
    fn is_history_empty(&self) -> Result<bool, RepositoryError> {
        self.connection
            .query_row(
                "SELECT NOT EXISTS (SELECT 1 FROM aggregates) AND NOT EXISTS (SELECT 1 FROM events) \
                 AND NOT EXISTS (SELECT 1 FROM throughput_tests)",
                [],
                |row| row.get(0),
            )
            .map_err(to_error)
    }

    /// O(log n + k) through the time index.
    fn list_events(&self, from: i64, to: i64) -> Result<Vec<HistoryEvent>, RepositoryError> {
        let mut events = Vec::new();
        self.stream_events(from, to, |event| {
            events.push(event.clone());
            Ok::<_, RepositoryError>(())
        })?;
        Ok(events)
    }
}

/// One row read at a time from the cursor: an export of years stays in constant memory.
impl HistoryStream for SqliteMeasurementRepository {
    /// O(log n + k) time through the (granularity, start) index, O(1) space.
    fn stream_all_aggregates<E: From<RepositoryError>>(
        &self,
        granularity: Granularity,
        from: i64,
        to: i64,
        mut visit: impl FnMut(&NetworkKey, i64, &Aggregate) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut statement = self
            .connection
            .prepare(&format!(
                "SELECT n.gateway_mac, n.connection_kind, n.label, a.start, {AGGREGATE_COLUMNS} \
                 FROM aggregates a JOIN networks n ON n.id = a.network_id \
                 WHERE a.granularity = ?1 AND a.start >= ?2 AND a.start < ?3 ORDER BY a.start"
            ))
            .map_err(to_error)?;
        let rows = statement
            .query_map(params![to_granularity_code(granularity), from, to], |row| {
                Ok((
                    read_network(row, 0)?,
                    row.get::<_, i64>(3)?,
                    read_aggregate_from(row, 4)?,
                ))
            })
            .map_err(to_error)?;
        visit_rows(rows, |(network, start, aggregate)| {
            visit(&network, start, &aggregate)
        })
    }

    /// O(log n + k) time through the time index, O(1) space.
    fn stream_events<E: From<RepositoryError>>(
        &self,
        from: i64,
        to: i64,
        mut visit: impl FnMut(&HistoryEvent) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT n.gateway_mac, n.connection_kind, n.label, e.ts, e.kind FROM events e \
                 JOIN networks n ON n.id = e.network_id WHERE e.ts >= ?1 AND e.ts < ?2 \
                 ORDER BY e.ts, e.rowid",
            )
            .map_err(to_error)?;
        let rows = statement
            .query_map(params![from, to], |row| {
                Ok(HistoryEvent {
                    network: read_network(row, 0)?,
                    utc: row.get(3)?,
                    kind: from_event_text(&row.get::<_, String>(4)?),
                })
            })
            .map_err(to_error)?;
        visit_rows(rows, |row| visit(&row))
    }

    /// O(t log t) time for the t tests kept, O(1) space.
    fn stream_throughput_tests<E: From<RepositoryError>>(
        &self,
        from: i64,
        to: i64,
        mut visit: impl FnMut(&ThroughputRecord) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut statement = self
            .connection
            .prepare(&format!(
                "{THROUGHPUT_COLUMNS} WHERE t.ts >= ?1 AND t.ts < ?2 AND t.status = ?3 \
                 ORDER BY t.ts, t.rowid"
            ))
            .map_err(to_error)?;
        let rows = statement
            .query_map(params![from, to, STATUS_DONE], read_throughput_record)
            .map_err(to_error)?;
        visit_rows(rows, |row| visit(&row))
    }
}

fn read_throughput_record(row: &Row<'_>) -> rusqlite::Result<ThroughputRecord> {
    Ok(ThroughputRecord {
        network: read_network(row, 0)?,
        utc: row.get(3)?,
        result: ThroughputResult {
            profile: from_profile_text(&row.get::<_, String>(4)?),
            download: row.get::<_, Option<i64>>(5)?.map(from_kilobits),
            upload: row.get::<_, Option<i64>>(6)?.map(from_kilobits),
            loaded_latency: row.get::<_, Option<i64>>(7)?.map(from_millis),
            bytes: from_column(row.get(8)?),
            duration: from_millis(row.get(9)?),
        },
    })
}

/// Hands each row to `visit` as the cursor reads it; the first failure stops the reading.
/// O(k) for k rows, O(1) space.
fn visit_rows<T, E: From<RepositoryError>>(
    rows: impl Iterator<Item = rusqlite::Result<T>>,
    mut visit: impl FnMut(T) -> Result<(), E>,
) -> Result<(), E> {
    for row in rows {
        visit(row.map_err(to_error)?)?;
    }
    Ok(())
}

impl SettingsRepository for SqliteMeasurementRepository {
    /// The single typed row the first migration creates. O(1).
    fn read_settings(&self) -> Result<Settings, RepositoryError> {
        self.connection
            .query_row(
                "SELECT autostart, theme, ping_target, notify, tests_allowed FROM settings WHERE id = 1",
                [],
                |row| {
                    let ping_target: Option<String> = row.get(2)?;
                    Ok(Settings {
                        launch_at_login: row.get(0)?,
                        appearance: from_theme_text(&row.get::<_, String>(1)?),
                        // A server that no longer reads as an address falls back to the public ones.
                        servers: ping_target
                            .as_deref()
                            .and_then(Target::try_parse)
                            .map_or(MeasurementServers::Default, MeasurementServers::Custom),
                        notifies_on_degradation: row.get(3)?,
                        allows_throughput_tests: row.get(4)?,
                    })
                },
            )
            .map_err(to_error)
    }

    /// O(1).
    fn save_settings(&mut self, settings: &Settings) -> Result<(), RepositoryError> {
        let ping_target = settings
            .servers
            .to_own_target()
            .map(|target| target.get_address().to_string());
        self.write(|transaction| {
            transaction.execute(
                "UPDATE settings SET autostart = ?1, theme = ?2, ping_target = ?3, notify = ?4, \
                 tests_allowed = ?5 WHERE id = 1",
                params![
                    settings.launch_at_login,
                    to_theme_text(settings.appearance),
                    ping_target,
                    settings.notifies_on_degradation,
                    settings.allows_throughput_tests
                ],
            )?;
            Ok(())
        })
    }
}

/// The values the `theme` column accepts (its CHECK constraint).
const THEME_SYSTEM: &str = "system";
const THEME_LIGHT: &str = "light";
const THEME_DARK: &str = "dark";

const fn to_theme_text(appearance: Appearance) -> &'static str {
    match appearance {
        Appearance::System => THEME_SYSTEM,
        Appearance::Light => THEME_LIGHT,
        Appearance::Dark => THEME_DARK,
    }
}

fn from_theme_text(text: &str) -> Appearance {
    match text {
        THEME_LIGHT => Appearance::Light,
        THEME_DARK => Appearance::Dark,
        _ => Appearance::System,
    }
}

/// Only finished tests are kept; the column leaves room for others.
const STATUS_DONE: &str = "done";
const PROFILE_QUICK: &str = "quick";
const PROFILE_FULL: &str = "full";
const BITS_PER_KILOBIT: u64 = 1000;

const fn to_profile_text(profile: TestProfile) -> &'static str {
    match profile {
        TestProfile::Quick => PROFILE_QUICK,
        TestProfile::Full => PROFILE_FULL,
    }
}

fn from_profile_text(text: &str) -> TestProfile {
    match text {
        PROFILE_FULL => TestProfile::Full,
        _ => TestProfile::Quick,
    }
}

/// SQLite integers are signed: every count stored here is far below `i64::MAX`.
fn to_column(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// A negative value can only come from a damaged row: it reads as zero.
fn from_column(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn to_kilobits(rate: Bitrate) -> i64 {
    to_column(rate.to_bits_per_second() / BITS_PER_KILOBIT)
}

fn from_kilobits(kilobits: i64) -> Bitrate {
    Bitrate::from_bits_per_second(from_column(kilobits) * BITS_PER_KILOBIT)
}

fn to_millis(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn from_millis(millis: i64) -> Duration {
    Duration::from_millis(from_column(millis))
}

/// Keeps a name learnt later; a network without a readable name keeps the one it had. O(log n)
/// on the unique indexes.
fn find_or_create_network(
    transaction: &Transaction<'_>,
    network: &NetworkKey,
) -> rusqlite::Result<i64> {
    let mac = to_mac_blob(network.gateway);
    let kind = to_kind_text(network.kind);
    transaction.execute(
        "INSERT INTO networks (gateway_mac, connection_kind, label) \
         SELECT ?1, ?2, ?3 WHERE NOT EXISTS ( \
             SELECT 1 FROM networks WHERE gateway_mac IS ?1 AND connection_kind = ?2)",
        params![mac, kind, network.name],
    )?;
    if network.name.is_some() {
        transaction.execute(
            "UPDATE networks SET label = ?3 WHERE gateway_mac IS ?1 AND connection_kind = ?2",
            params![mac, kind, network.name],
        )?;
    }
    transaction.query_row(
        "SELECT id FROM networks WHERE gateway_mac IS ?1 AND connection_kind = ?2",
        params![mac, kind],
        |row| row.get(0),
    )
}

fn save_aggregate(
    transaction: &Transaction<'_>,
    network_id: i64,
    granularity: i64,
    start: i64,
    aggregate: &Aggregate,
) -> rusqlite::Result<()> {
    transaction.execute(
        &format!(
            "INSERT OR REPLACE INTO aggregates (network_id, granularity, start, {AGGREGATE_COLUMNS}) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
        ),
        params![
            network_id,
            granularity,
            start,
            aggregate.latency.to_bytes(),
            i64::try_from(aggregate.jitter_sum_us).unwrap_or(i64::MAX),
            aggregate.jitter_count,
            aggregate.probes_sent,
            aggregate.probes_lost,
            aggregate.seconds.solid,
            aggregate.seconds.unstable,
            aggregate.seconds.degraded,
            aggregate.seconds.offline,
        ],
    )?;
    Ok(())
}

fn read_aggregate(row: &Row<'_>) -> rusqlite::Result<Aggregate> {
    read_aggregate_from(row, 0)
}

fn read_aggregate_from(row: &Row<'_>, first: usize) -> rusqlite::Result<Aggregate> {
    let histogram: Vec<u8> = row.get(first)?;
    let jitter_sum_us: i64 = row.get(first + 1)?;
    Ok(Aggregate {
        // A malformed histogram reads as empty rather than losing the whole row.
        latency: LatencyHistogram::from_bytes(&histogram).unwrap_or_default(),
        jitter_sum_us: u64::try_from(jitter_sum_us).unwrap_or(0),
        jitter_count: row.get(first + 2)?,
        probes_sent: row.get(first + 3)?,
        probes_lost: row.get(first + 4)?,
        seconds: StateSeconds {
            solid: row.get(first + 5)?,
            unstable: row.get(first + 6)?,
            degraded: row.get(first + 7)?,
            offline: row.get(first + 8)?,
        },
    })
}

fn read_network(row: &Row<'_>, first: usize) -> rusqlite::Result<NetworkKey> {
    let mac: Option<Vec<u8>> = row.get(first)?;
    Ok(NetworkKey {
        gateway: mac
            .and_then(|bytes| <[u8; 6]>::try_from(bytes).ok())
            .and_then(MacAddress::from_octets),
        kind: from_kind_text(&row.get::<_, String>(first + 1)?),
        name: row.get(first + 2)?,
    })
}

fn to_mac_blob(gateway: Option<MacAddress>) -> Option<Vec<u8>> {
    gateway.map(|mac| mac.to_octets().to_vec())
}

const fn to_granularity_code(granularity: Granularity) -> i64 {
    match granularity {
        Granularity::Minute => MINUTE,
        Granularity::Hour => HOUR,
    }
}

const fn to_kind_text(kind: ConnectionKind) -> &'static str {
    match kind {
        ConnectionKind::Ethernet => "ethernet",
        ConnectionKind::WiFi => "wifi",
        ConnectionKind::Cellular => "cellular",
        ConnectionKind::PhoneHotspot => "hotspot",
        ConnectionKind::Other => "other",
    }
}

fn from_kind_text(text: &str) -> ConnectionKind {
    match text {
        "ethernet" => ConnectionKind::Ethernet,
        "wifi" => ConnectionKind::WiFi,
        "cellular" => ConnectionKind::Cellular,
        "hotspot" => ConnectionKind::PhoneHotspot,
        _ => ConnectionKind::Other,
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

fn from_event_text(text: &str) -> EventKind {
    match text {
        "offline_started" => EventKind::OfflineStarted,
        "offline_ended" => EventKind::OfflineEnded,
        "sleep" => EventKind::Sleep,
        "wake" => EventKind::Wake,
        _ => EventKind::NetworkChanged,
    }
}

/// Renames the file and its journals with a date, next to it. O(1).
fn move_aside(path: &Path, now_utc: i64) -> std::io::Result<PathBuf> {
    let stem = path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let set_aside = path.with_file_name(format!("{stem}.corrupt-{now_utc}.sqlite3"));
    std::fs::rename(path, &set_aside)?;
    for suffix in ["-wal", "-shm"] {
        // The journal may hold committed writes: it goes with the file it belongs to.
        let journal = append_to_name(path, suffix);
        if journal.exists() {
            std::fs::rename(&journal, append_to_name(&set_aside, suffix))?;
        }
    }
    Ok(set_aside)
}

fn append_to_name(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Damage sets the file aside; anything else leaves it alone.
enum OpenFailure {
    Damaged(String),
    Other(RepositoryError),
}

impl OpenFailure {
    fn classify(error: rusqlite::Error) -> Self {
        match error.sqlite_error_code() {
            Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt) => {
                Self::Damaged(error.to_string())
            }
            _ => Self::Other(to_error(error)),
        }
    }

    fn into_error(self) -> RepositoryError {
        match self {
            Self::Damaged(reason) => RepositoryError(reason),
            Self::Other(error) => error,
        }
    }
}

fn to_error(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use application::repository_contract::{check_contract, check_settings_contract};
    use application::test_support::generate_network_name;

    use super::*;

    /// One folder per test: tests run in parallel and the clock alone may not tell them apart.
    fn create_temporary_directory(test: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let directory =
            std::env::temp_dir().join(format!("history-{test}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        directory
    }

    #[test]
    fn honours_the_repository_contract() {
        // Act & Assert
        check_contract(|| SqliteMeasurementRepository::open_in_memory().expect("a history"));
    }

    #[test]
    fn keeps_the_settings_when_the_history_is_erased() {
        // Arrange
        let mut repository = SqliteMeasurementRepository::open_in_memory().expect("a history");
        let settings = Settings {
            launch_at_login: true,
            notifies_on_degradation: false,
            ..Settings::default()
        };
        repository.save_settings(&settings).expect("settings saved");

        // Act
        repository.erase_history().expect("an erase");

        // Assert
        assert_eq!(repository.read_settings().expect("settings"), settings);
    }

    #[test]
    fn honours_the_settings_contract() {
        // Act & Assert
        check_settings_contract(|| {
            SqliteMeasurementRepository::open_in_memory().expect("a history")
        });
    }

    #[test]
    fn migrates_an_empty_database_to_the_latest_schema() {
        // Act & Assert
        assert!(Migrations::from_slice(MIGRATIONS).validate().is_ok());
    }

    #[test]
    fn keeps_the_journal_of_a_damaged_history_next_to_it() {
        // Arrange
        let directory =
            create_temporary_directory("keeps_the_journal_of_a_damaged_history_next_to_it");
        let path = directory.join("history.sqlite3");
        std::fs::write(&path, b"not a database").expect("a broken file");
        std::fs::write(append_to_name(&path, "-wal"), b"journal").expect("a journal");

        // Act
        let set_aside = move_aside(&path, 1_700_000_000).expect("moved aside");

        // Assert
        assert_eq!(
            std::fs::read(append_to_name(&set_aside, "-wal")).expect("journal kept"),
            b"journal"
        );
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn leaves_a_history_from_a_newer_version_untouched() {
        // Arrange
        let directory =
            create_temporary_directory("leaves_a_history_from_a_newer_version_untouched");
        let path = directory.join("history.sqlite3");
        {
            let connection = Connection::open(&path).expect("a database");
            connection
                .pragma_update(None, "user_version", 999)
                .expect("a newer schema");
        }

        // Act
        let opened = SqliteMeasurementRepository::open(&path, 1_700_000_000);

        // Assert
        assert!(opened.is_err());
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn sets_an_unreadable_history_aside_and_starts_a_new_one() {
        // Arrange
        let directory =
            create_temporary_directory("sets_an_unreadable_history_aside_and_starts_a_new_one");
        let path = directory.join("history.sqlite3");
        std::fs::write(&path, b"not a database at all, just bytes").expect("a broken file");

        // Act
        let opened = SqliteMeasurementRepository::open(&path, 1_700_000_000).expect("a history");

        // Assert
        let set_aside = opened.set_aside.expect("the broken file kept");
        assert_eq!(
            std::fs::read(&set_aside).expect("kept"),
            b"not a database at all, just bytes"
        );
        assert!(
            opened
                .repository
                .list_events(0, i64::MAX)
                .expect("events")
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn reopens_a_healthy_history_as_it_was() {
        // Arrange
        let directory = create_temporary_directory("reopens_a_healthy_history_as_it_was");
        let path = directory.join("history.sqlite3");
        let event = HistoryEvent {
            utc: 1_700_000_000,
            network: NetworkKey {
                gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, 0x48]),
                kind: ConnectionKind::Ethernet,
                name: None,
            },
            kind: EventKind::NetworkChanged,
        };
        SqliteMeasurementRepository::open(&path, 0)
            .expect("a history")
            .repository
            .record_event(&event)
            .expect("an event");

        // Act
        let reopened = SqliteMeasurementRepository::open(&path, 0).expect("the same history");

        // Assert
        assert_eq!(reopened.set_aside, None);
        assert_eq!(
            reopened
                .repository
                .list_events(0, i64::MAX)
                .expect("events"),
            vec![event]
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn reads_what_the_writer_keeps_without_writing_itself() {
        // Arrange
        let directory =
            create_temporary_directory("reads_what_the_writer_keeps_without_writing_itself");
        let path = directory.join("history.sqlite3");
        let event = HistoryEvent {
            utc: 1_700_000_000,
            network: NetworkKey {
                gateway: MacAddress::from_octets([0x54, 0x9b, 0x49, 0x87, 0xe3, 0x49]),
                kind: ConnectionKind::WiFi,
                name: Some(generate_network_name()),
            },
            kind: EventKind::NetworkChanged,
        };
        let mut writer = SqliteMeasurementRepository::open(&path, 0)
            .expect("a history")
            .repository;
        let mut reader =
            SqliteMeasurementRepository::open_read_only(&path).expect("a second connection");

        // Act
        writer.record_event(&event).expect("an event");

        // Assert
        assert_eq!(
            reader.list_events(0, i64::MAX).expect("events"),
            vec![event.clone()]
        );
        assert!(reader.record_event(&event).is_err());
        let _ = std::fs::remove_dir_all(directory);
    }
}
