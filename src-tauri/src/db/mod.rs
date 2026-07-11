#![allow(dead_code)]

pub mod schema;

use rusqlite::{backup::Backup, Connection, Transaction, TransactionBehavior};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

pub struct Database {
    path: PathBuf,
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let mut connection = Connection::open(&path).map_err(|error| error.to_string())?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| error.to_string())?;
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|error| error.to_string())?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|error| error.to_string())?;
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
        backup
            .run_to_completion(64, Duration::from_millis(25), None)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use std::sync::{Arc, Barrier};
    use std::thread;

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
    fn immediate_transactions_allow_only_one_pending_reminder_claim() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(Database::open(dir.path().join("banana.db")).unwrap());
        db.with_connection(|connection| {
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

        let start = Arc::new(Barrier::new(3));
        let workers = (0..2)
            .map(|_| {
                let db = Arc::clone(&db);
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    db.with_immediate_transaction(|transaction| {
                        transaction
                            .execute(
                                "UPDATE reminder_log SET state = 'shown'
                                 WHERE id = 'r1' AND state = 'pending'",
                                [],
                            )
                            .map_err(|error| error.to_string())
                    })
                    .unwrap()
                })
            })
            .collect::<Vec<_>>();

        start.wait();
        let claimed = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .sum::<usize>();
        assert_eq!(claimed, 1);
        db.with_connection(|connection| {
            let state: String = connection
                .query_row(
                    "SELECT state FROM reminder_log WHERE id = 'r1'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            assert_eq!(state, "shown");
            Ok(())
        })
        .unwrap();
    }
}
