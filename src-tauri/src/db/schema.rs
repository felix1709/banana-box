use rusqlite::{Connection, TransactionBehavior};

pub const SCHEMA_VERSION: i64 = 1;
pub const DATABASE_SCHEMA_INVALID: &str = "DATABASE_SCHEMA_INVALID";
const MIGRATION_V1: &str = include_str!("../../migrations/0001_v1.sql");
const REQUIRED_TABLES: [&str; 15] = [
    "schema_migrations",
    "ai_providers",
    "credential_cleanup",
    "projects",
    "project_stages",
    "daily_task_days",
    "daily_task_groups",
    "daily_tasks",
    "skills",
    "skill_versions",
    "storyboard_threads",
    "agent_requests",
    "storyboard_messages",
    "storyboard_message_blocks",
    "reminder_log",
];

pub fn migrate(connection: &mut Connection) -> Result<(), String> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| error.to_string())?;
    let version: i64 = transaction
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version < 0 {
        return Err(schema_invalid(format!("negative user_version {version}")));
    }
    if version > SCHEMA_VERSION {
        return Err(schema_invalid(format!(
            "user_version {version} is newer than supported version {SCHEMA_VERSION}"
        )));
    }
    if version == 0 {
        transaction
            .execute_batch(MIGRATION_V1)
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(())
}

pub fn validate(connection: &Connection) -> Result<(), String> {
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| schema_invalid(error))?;
    if version != SCHEMA_VERSION {
        return Err(schema_invalid(format!(
            "expected user_version {SCHEMA_VERSION}, found {version}"
        )));
    }

    for table in REQUIRED_TABLES {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get(0),
            )
            .map_err(|error| schema_invalid(error))?;
        if exists != 1 {
            return Err(schema_invalid(format!("missing table {table}")));
        }
    }

    let migration_exists: i64 = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [SCHEMA_VERSION],
            |row| row.get(0),
        )
        .map_err(|error| schema_invalid(error))?;
    if migration_exists != 1 {
        return Err(schema_invalid("missing schema_migrations version 1"));
    }

    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| schema_invalid(error))?;
    if integrity != "ok" {
        return Err(schema_invalid(format!(
            "SQLite integrity_check: {integrity}"
        )));
    }
    let foreign_key_errors: i64 = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(|error| schema_invalid(error))?;
    if foreign_key_errors != 0 {
        return Err(schema_invalid("SQLite foreign_key_check failed"));
    }
    Ok(())
}

fn schema_invalid(message: impl std::fmt::Display) -> String {
    format!("{DATABASE_SCHEMA_INVALID}: {message}")
}

#[cfg(test)]
mod tests {
    use super::{migrate, validate, SCHEMA_VERSION};
    use rusqlite::Connection;

    #[test]
    fn migrate_rejects_a_negative_user_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "user_version", -1_i64)
            .unwrap();

        let error = migrate(&mut connection).unwrap_err();

        assert!(error.starts_with("DATABASE_SCHEMA_INVALID"));
    }

    #[test]
    fn validate_rejects_a_forged_v1_user_version_without_the_schema() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .unwrap();

        let error = validate(&connection).unwrap_err();

        assert!(error.starts_with("DATABASE_SCHEMA_INVALID"));
    }
}
