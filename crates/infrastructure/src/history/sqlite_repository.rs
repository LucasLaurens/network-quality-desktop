use std::path::{Path, PathBuf};

use application::{EventKind, HistoryEvent, MeasurementRepository, NetworkKey, RepositoryError};
use domain::{Aggregate, ConnectionKind, Granularity, LatencyHistogram, MacAddress, StateSeconds};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params};
use rusqlite_migration::{M, Migrations};

const MIGRATIONS: &[M<'static>] = &[M::up(include_str!("migrations/0001_create_history.sql"))];
const MINUTE: i64 = 0;
const HOUR: i64 = 1;
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
    /// Opens `path`; a file that is not a readable history is renamed next to it, kept, and a
    /// new history starts.
    ///
    /// # Errors
    ///
    /// Neither the file nor a fresh one could be opened.
    pub fn open(path: &Path, now_utc: i64) -> Result<OpenedHistory, RepositoryError> {
        match Self::open_checked(path) {
            Ok(repository) => Ok(OpenedHistory {
                repository,
                set_aside: None,
            }),
            Err(error) if path.exists() => {
                eprintln!("The history is unreadable, it is set aside: {}", error.0);
                let set_aside = path.with_file_name(format!("history.corrupt-{now_utc}.sqlite3"));
                std::fs::rename(path, &set_aside).map_err(to_error)?;
                for suffix in ["-wal", "-shm"] {
                    // Journal files of the old file are useless without it.
                    let _ = std::fs::remove_file(append_to_name(path, suffix));
                }
                Ok(OpenedHistory {
                    repository: Self::open_checked(path)?,
                    set_aside: Some(set_aside),
                })
            }
            Err(error) => Err(error),
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

    fn open_checked(path: &Path) -> Result<Self, RepositoryError> {
        let connection = Connection::open(path).map_err(to_error)?;
        let check: String = connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(to_error)?;
        if check != "ok" {
            return Err(RepositoryError(format!("integrity check: {check}")));
        }
        Self::prepare(connection)
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

    fn list_unmerged_hours(&self, before: i64) -> Result<Vec<(NetworkKey, i64)>, RepositoryError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT n.gateway_mac, n.connection_kind, n.label, \
                 a.start - a.start % 3600 AS hour FROM aggregates a \
                 JOIN networks n ON n.id = a.network_id \
                 WHERE a.granularity = 0 AND a.start < ?1 AND NOT EXISTS ( \
                     SELECT 1 FROM aggregates h WHERE h.network_id = a.network_id \
                     AND h.granularity = 1 AND h.start = a.start - a.start % 3600) \
                 ORDER BY hour",
            )
            .map_err(to_error)?;
        let rows = statement
            .query_map(params![before], |row| {
                Ok((read_network(row, 0)?, row.get(3)?))
            })
            .map_err(to_error)?;
        rows.collect::<rusqlite::Result<_>>().map_err(to_error)
    }

    /// A range delete on the primary key.
    fn purge_minutes_before(&mut self, utc: i64) -> Result<(), RepositoryError> {
        self.write(|transaction| {
            transaction.execute(
                "DELETE FROM aggregates WHERE granularity = 0 AND start < ?1",
                params![utc],
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

    fn list_events(&self, from: i64, to: i64) -> Result<Vec<HistoryEvent>, RepositoryError> {
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
        rows.collect::<rusqlite::Result<_>>().map_err(to_error)
    }
}

/// Keeps a name learnt later; a network without a readable name keeps the one it had.
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
    }
}

fn from_event_text(text: &str) -> EventKind {
    match text {
        "offline_started" => EventKind::OfflineStarted,
        "offline_ended" => EventKind::OfflineEnded,
        _ => EventKind::NetworkChanged,
    }
}

fn append_to_name(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn to_error(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use application::repository_contract::check_contract;

    use super::*;

    fn create_temporary_directory() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let directory =
            std::env::temp_dir().join(format!("history-test-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a temporary directory");
        directory
    }

    #[test]
    fn honours_the_repository_contract() {
        // Act & Assert
        check_contract(|| SqliteMeasurementRepository::open_in_memory().expect("a history"));
    }

    #[test]
    fn migrates_an_empty_database_to_the_latest_schema() {
        // Act & Assert
        assert!(Migrations::from_slice(MIGRATIONS).validate().is_ok());
    }

    #[test]
    fn sets_an_unreadable_history_aside_and_starts_a_new_one() {
        // Arrange
        let directory = create_temporary_directory();
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
        let directory = create_temporary_directory();
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
}
