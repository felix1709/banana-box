#![allow(dead_code)]

pub mod schema;

use rusqlite::{
    backup::{Backup, StepResult},
    Connection, ErrorCode, Transaction, TransactionBehavior,
};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

const DATABASE_FOREIGN_KEYS_REQUIRED: &str = "DATABASE_FOREIGN_KEYS_REQUIRED";
const DATABASE_WAL_REQUIRED: &str = "DATABASE_WAL_REQUIRED";
const ONLINE_BACKUP_TIMEOUT: &str = "ONLINE_BACKUP_TIMEOUT";
const DATABASE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const DATABASE_OPEN_RETRY_DEADLINE: Duration = Duration::from_secs(5);
const DATABASE_OPEN_RETRY_PAUSE: Duration = Duration::from_millis(25);
const ONLINE_BACKUP_DEADLINE: Duration = Duration::from_secs(30);
const ONLINE_BACKUP_RETRY_PAUSE: Duration = Duration::from_millis(25);

pub struct Database {
    path: PathBuf,
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let mut connection = Connection::open(&path).map_err(|error| error.to_string())?;
        connection
            .busy_timeout(DATABASE_BUSY_TIMEOUT)
            .map_err(|error| error.to_string())?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|error| error.to_string())?;
        let foreign_keys: i64 = connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if foreign_keys != 1 {
            return Err(DATABASE_FOREIGN_KEYS_REQUIRED.into());
        }
        let journal_mode = configure_wal_mode(&connection)?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            return Err(format!(
                "{DATABASE_WAL_REQUIRED}: journal_mode={journal_mode}"
            ));
        }
        schema::migrate(&mut connection)?;
        schema::validate(&connection)?;
        Ok(Self {
            path,
            connection: Mutex::new(connection),
        })
    }

    pub fn with_connection<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let guard = self.connection.lock().map_err(|error| error.to_string())?;
        f(&guard)
    }

    pub fn with_transaction<T>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self.connection.lock().map_err(|error| error.to_string())?;
        let transaction = guard.transaction().map_err(|error| error.to_string())?;
        let value = f(&transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(value)
    }

    pub fn with_immediate_transaction<T>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self.connection.lock().map_err(|error| error.to_string())?;
        let transaction = guard
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let value = f(&transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(value)
    }

    pub fn online_backup(&self, destination: impl AsRef<Path>) -> Result<(), String> {
        let source = self.connection.lock().map_err(|error| error.to_string())?;
        let mut destination = Connection::open(destination).map_err(|error| error.to_string())?;
        let backup = Backup::new(&source, &mut destination).map_err(|error| error.to_string())?;
        let started = Instant::now();
        run_backup_step_loop(
            || backup.step(64),
            ONLINE_BACKUP_DEADLINE,
            || started.elapsed(),
            thread::sleep,
        )
    }
}

fn configure_wal_mode(connection: &Connection) -> Result<String, String> {
    connection
        .busy_timeout(Duration::ZERO)
        .map_err(|error| format!("{DATABASE_WAL_REQUIRED}: {error}"))?;
    let started = Instant::now();
    let result = run_wal_mode_retry_loop(
        || connection.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0)),
        DATABASE_OPEN_RETRY_DEADLINE,
        || started.elapsed(),
        thread::sleep,
    );
    connection
        .busy_timeout(DATABASE_BUSY_TIMEOUT)
        .map_err(|error| {
            format!("{DATABASE_WAL_REQUIRED}: failed to restore busy timeout: {error}")
        })?;

    result
}

fn run_wal_mode_retry_loop<Attempt, Elapsed, Pause>(
    mut attempt: Attempt,
    deadline: Duration,
    mut elapsed: Elapsed,
    mut pause: Pause,
) -> Result<String, String>
where
    Attempt: FnMut() -> rusqlite::Result<String>,
    Elapsed: FnMut() -> Duration,
    Pause: FnMut(Duration),
{
    loop {
        if elapsed() >= deadline {
            return Err(format!("{DATABASE_WAL_REQUIRED}: timed out enabling WAL"));
        }

        match attempt() {
            Ok(journal_mode) => return Ok(journal_mode),
            Err(error)
                if matches!(
                    error.sqlite_error_code(),
                    Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
                ) =>
            {
                pause(DATABASE_OPEN_RETRY_PAUSE);
            }
            Err(error) => return Err(format!("{DATABASE_WAL_REQUIRED}: {error}")),
        }
    }
}

fn run_backup_step_loop<Step, Elapsed, Pause>(
    mut step: Step,
    deadline: Duration,
    mut elapsed: Elapsed,
    mut pause: Pause,
) -> Result<(), String>
where
    Step: FnMut() -> rusqlite::Result<StepResult>,
    Elapsed: FnMut() -> Duration,
    Pause: FnMut(Duration),
{
    loop {
        if elapsed() >= deadline {
            return Err(ONLINE_BACKUP_TIMEOUT.into());
        }

        let result = step().map_err(|error| error.to_string())?;
        if elapsed() >= deadline {
            return Err(ONLINE_BACKUP_TIMEOUT.into());
        }

        match result {
            StepResult::Done => return Ok(()),
            StepResult::More => {}
            StepResult::Busy | StepResult::Locked => {
                pause(ONLINE_BACKUP_RETRY_PAUSE);
            }
            _ => return Err("ONLINE_BACKUP_UNEXPECTED_STEP_RESULT".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{configure_wal_mode, run_backup_step_loop, run_wal_mode_retry_loop, Database};
    use rusqlite::{backup::StepResult, ffi, Connection, Error};
    use std::{
        cell::Cell,
        sync::{Arc, Barrier},
        thread,
        time::Duration,
    };

    #[test]
    fn open_creates_schema_and_enforces_constraints() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path().join("banana.db")).unwrap();

        db.with_connection(|connection| {
            let version: i64 = connection
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .map_err(|error| error.to_string())?;
            let table_count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='ai_providers'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            assert_eq!(version, 1);
            assert_eq!(table_count, 1);
            assert!(connection.execute(
                "INSERT INTO projects (id, code, version, name, file_path, release_date, main_stage_key, archived, created_at, updated_at)
                 VALUES ('p1', 'L36', '1', 'A', 'C:/work/L36', '2026-07-31', 'storyboard', 0, '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z')",
                [],
            ).is_ok());
            assert!(connection.execute(
                "INSERT INTO projects (id, code, version, name, file_path, release_date, main_stage_key, archived, created_at, updated_at)
                 VALUES ('p2', 'l36', '1', 'B', 'C:/work/l36', '2026-08-01', 'storyboard', 0, '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z')",
                [],
            ).is_err());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn open_rejects_memory_databases_when_wal_is_unavailable() {
        let first = Database::open(":memory:")
            .err()
            .expect("in-memory SQLite cannot enable WAL");
        let second = Database::open(":memory:")
            .err()
            .expect("in-memory SQLite cannot enable WAL");

        assert!(first.starts_with("DATABASE_WAL_REQUIRED"));
        assert_eq!(first, second);
    }

    #[test]
    fn immediate_transaction_excludes_another_database_instance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("banana.db");
        let first = Database::open(&path).unwrap();
        let second = Database::open(&path).unwrap();

        first
            .with_immediate_transaction(|_| {
                second.with_connection(|connection| {
                    connection
                        .busy_timeout(Duration::ZERO)
                        .map_err(|error| error.to_string())
                })?;

                let attempt: Result<(), String> = second.with_immediate_transaction(|_| Ok(()));
                assert!(attempt.is_err());
                Ok(())
            })
            .unwrap();

        second
            .with_connection(|connection| {
                connection
                    .busy_timeout(Duration::from_secs(5))
                    .map_err(|error| error.to_string())
            })
            .unwrap();
    }

    #[test]
    fn simultaneous_fresh_opens_apply_the_migration_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("banana.db");
        let start = Arc::new(Barrier::new(3));
        let workers = (0..2)
            .map(|_| {
                let path = path.clone();
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    Database::open(path)
                })
            })
            .collect::<Vec<_>>();

        start.wait();
        let databases = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Result<Vec<_>, _>>()
            .expect("both opens should succeed");
        drop(databases);

        let connection = Connection::open(&path).unwrap();
        let migration_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migration_count, 1);
    }

    #[test]
    fn backup_step_loop_times_out_when_every_step_is_busy() {
        let clock = Cell::new(Duration::ZERO);
        let error = run_backup_step_loop(
            || Ok::<_, rusqlite::Error>(StepResult::Busy),
            Duration::from_millis(50),
            || clock.get(),
            |pause| clock.set(clock.get() + pause),
        )
        .unwrap_err();

        assert_eq!(error, "ONLINE_BACKUP_TIMEOUT");
    }

    #[test]
    fn backup_step_loop_counts_step_time_against_the_deadline() {
        let clock = Cell::new(Duration::ZERO);
        let error = run_backup_step_loop(
            || {
                clock.set(Duration::from_millis(50));
                Ok::<_, rusqlite::Error>(StepResult::Done)
            },
            Duration::from_millis(50),
            || clock.get(),
            |_| {},
        )
        .unwrap_err();

        assert_eq!(error, "ONLINE_BACKUP_TIMEOUT");
    }

    #[test]
    fn wal_mode_retry_loop_returns_a_stable_error_after_the_busy_deadline() {
        let clock = Cell::new(Duration::ZERO);
        let pause_count = Cell::new(0);
        let error = run_wal_mode_retry_loop(
            || {
                Err::<String, _>(Error::SqliteFailure(
                    ffi::Error::new(ffi::SQLITE_BUSY),
                    None,
                ))
            },
            Duration::from_millis(50),
            || clock.get(),
            |pause| {
                pause_count.set(pause_count.get() + 1);
                clock.set(clock.get() + pause);
            },
        )
        .unwrap_err();

        assert_eq!(error, "DATABASE_WAL_REQUIRED: timed out enabling WAL");
        assert_eq!(pause_count.get(), 2);
    }

    #[test]
    fn configure_wal_mode_restores_the_standard_busy_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let connection = Connection::open(dir.path().join("banana.db")).unwrap();
        connection.busy_timeout(Duration::from_millis(1)).unwrap();

        assert_eq!(
            configure_wal_mode(&connection)
                .unwrap()
                .to_ascii_lowercase(),
            "wal"
        );
        let busy_timeout: i64 = connection
            .pragma_query_value(None, "busy_timeout", |row| row.get(0))
            .unwrap();

        assert_eq!(busy_timeout, 5_000);
    }

    #[test]
    fn backup_step_loop_continues_after_more_without_pausing() {
        let step_count = Cell::new(0);
        let pause_count = Cell::new(0);

        run_backup_step_loop(
            || {
                let next_step = step_count.get() + 1;
                step_count.set(next_step);
                Ok::<_, rusqlite::Error>(if next_step < 4 {
                    StepResult::More
                } else {
                    StepResult::Done
                })
            },
            Duration::from_secs(1),
            || Duration::ZERO,
            |_| pause_count.set(pause_count.get() + 1),
        )
        .unwrap();

        assert_eq!(step_count.get(), 4);
        assert_eq!(pause_count.get(), 0);
    }

    #[test]
    fn online_backup_wal_snapshot_is_readable_from_the_destination_main_file() {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("source.db");
        let destination_path = dir.path().join("snapshot.db");
        let source = Database::open(&source_path).unwrap();

        source
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO reminder_log (id, kind, local_date, phase, state, delivery_id)
                         VALUES ('r1', 'daily-task', '2026-07-11', 'initial', 'pending', 'delivery-1')",
                        [],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .unwrap();
        source.online_backup(&destination_path).unwrap();

        let destination = Connection::open(&destination_path).unwrap();
        let row_count: i64 = destination
            .query_row(
                "SELECT COUNT(*) FROM reminder_log WHERE id = 'r1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(row_count, 1);
    }
}
