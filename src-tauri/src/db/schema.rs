use rusqlite::{Connection, TransactionBehavior};
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: i64 = 1;
pub const DATABASE_SCHEMA_INVALID: &str = "DATABASE_SCHEMA_INVALID";
const MIGRATION_V1: &str = include_str!("../../migrations/0001_v1.sql");
const V1_SCHEMA_FINGERPRINT: &str =
    "9efed7e5f33c46abefdd4cfe98a19a7374c2f08c81ed7f55f7eac1ced5bba1e4";
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

    let fingerprint = schema_fingerprint(connection).map_err(schema_invalid)?;
    if fingerprint != V1_SCHEMA_FINGERPRINT {
        return Err(schema_invalid("schema fingerprint mismatch"));
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

fn schema_fingerprint(connection: &Connection) -> Result<String, rusqlite::Error> {
    let mut statement = connection.prepare(
        "
        SELECT type, name, tbl_name, sql
        FROM sqlite_master
        WHERE type IN ('table', 'index', 'trigger', 'view') AND name NOT GLOB 'sqlite_*'
        ORDER BY type, name, tbl_name, sql
        ",
    )?;
    let mut rows = statement.query([])?;
    let mut hasher = Sha256::new();

    while let Some(row) = rows.next()? {
        let object_type: String = row.get(0)?;
        let name: String = row.get(1)?;
        let table_name: String = row.get(2)?;
        let sql: Option<String> = row.get(3)?;
        let normalized_sql = normalize_schema_sql(sql.as_deref().unwrap_or_default());
        for field in [object_type.as_str(), name.as_str(), table_name.as_str()] {
            update_schema_fingerprint_field(&mut hasher, field);
        }
        update_schema_fingerprint_field(&mut hasher, &normalized_sql);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn update_schema_fingerprint_field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn normalize_schema_sql(sql: &str) -> String {
    sql.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::{
        migrate, schema_fingerprint, validate, DATABASE_SCHEMA_INVALID, MIGRATION_V1,
        SCHEMA_VERSION, V1_SCHEMA_FINGERPRINT,
    };
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

    #[test]
    fn validate_rejects_a_v1_database_with_a_replaced_projects_table() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection
            .execute_batch(
                "
                DROP TABLE projects;
                CREATE TABLE projects (id TEXT PRIMARY KEY);
                ",
            )
            .unwrap();

        let error = validate(&connection).unwrap_err();

        assert_eq!(
            error,
            format!("{DATABASE_SCHEMA_INVALID}: schema fingerprint mismatch")
        );
    }

    #[test]
    fn validate_rejects_a_v1_database_with_an_unexpected_trigger() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
            .execute_batch(
                "
                CREATE TRIGGER unexpected_projects_trigger
                AFTER INSERT ON projects
                BEGIN
                    SELECT 1;
                END;
                ",
            )
            .unwrap();

        let error = validate(&connection).unwrap_err();

        assert_eq!(
            error,
            format!("{DATABASE_SCHEMA_INVALID}: schema fingerprint mismatch")
        );
    }

    #[test]
    fn schema_fingerprint_normalizes_lf_and_crlf_v1_migrations() {
        let lf = schema_fingerprint_after_applying(MIGRATION_V1);
        let crlf = schema_fingerprint_after_applying(&MIGRATION_V1.replace('\n', "\r\n"));

        assert_eq!(lf, crlf);
        assert_eq!(lf, V1_SCHEMA_FINGERPRINT);
    }

    fn schema_fingerprint_after_applying(migration: &str) -> String {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(migration).unwrap();
        schema_fingerprint(&connection).unwrap()
    }
}
