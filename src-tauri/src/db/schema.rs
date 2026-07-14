use rusqlite::{Connection, TransactionBehavior};
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: i64 = 6;
pub const DATABASE_SCHEMA_INVALID: &str = "DATABASE_SCHEMA_INVALID";
const MIGRATION_V1: &str = include_str!("../../migrations/0001_v1.sql");
const MIGRATION_V2: &str = include_str!("../../migrations/0002_allow_duplicate_project_codes.sql");
const MIGRATION_V3: &str = include_str!("../../migrations/0003_storyboard_agent.sql");
const MIGRATION_V4: &str = include_str!("../../migrations/0004_cloud_foundation.sql");
const MIGRATION_V5: &str = include_str!("../../migrations/0005_workspace_sync_foundation.sql");
const MIGRATION_V6: &str = include_str!("../../migrations/0006_daily_task_reminders.sql");
const V1_SCHEMA_FINGERPRINT: &str =
    "9efed7e5f33c46abefdd4cfe98a19a7374c2f08c81ed7f55f7eac1ced5bba1e4";
const V2_SCHEMA_FINGERPRINT: &str =
    "8417ad4553c0f710ded1daef30d5725fc5601f7aafba0580e3f8e4df6276c20e";
const V3_SCHEMA_FINGERPRINT: &str =
    "1ab5bb541a8c84e3465ead97c74b5cc2f3c7aac1290d814bd1c22a4c74a7e1a3";
const V4_SCHEMA_FINGERPRINT: &str =
    "31242f2b209ce2873b34e6be19ab019ce3437fea9e00fdb9013eb68c9becb74f";
const V5_SCHEMA_FINGERPRINT: &str =
    "36f288ab2feb0a4764f158a1ac03bf7e86eab62f7eb55fa40baa2d8c632da6c8";
const V6_SCHEMA_FINGERPRINT: &str =
    "1fc20a884699cd7529cf3c587c340f4d0fc9b85aa51f1ee36babef4c6d135570";
const REQUIRED_TABLES_PRE_V3: [&str; 15] = [
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
const REQUIRED_TABLES_PRE_V4: [&str; 16] = [
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
    "storyboard_thread_skills",
    "reminder_log",
];
const REQUIRED_TABLES_PRE_V5: [&str; 17] = [
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
    "storyboard_thread_skills",
    "reminder_log",
    "cloud_config",
];
const REQUIRED_TABLES: [&str; 21] = [
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
    "storyboard_thread_skills",
    "reminder_log",
    "cloud_config",
    "local_workspaces",
    "sync_outbox",
    "sync_cursors",
    "local_device_bindings",
];

pub fn migrate(connection: &mut Connection) -> Result<(), String> {
    let version: i64 = connection
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
    if version == SCHEMA_VERSION {
        return Ok(());
    }

    let foreign_keys_enabled: i64 = connection
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version < 2 && foreign_keys_enabled == 1 {
        connection
            .pragma_update(None, "foreign_keys", false)
            .map_err(|error| error.to_string())?;
    }

    let migration_result = (|| -> Result<(), String> {
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let mut applied_version: i64 = transaction
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if applied_version == 0 {
            transaction
                .execute_batch(MIGRATION_V1)
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                    [],
                )
                .map_err(|error| error.to_string())?;
            applied_version = 1;
        }
        if applied_version == 1 {
            transaction
                .execute_batch(MIGRATION_V2)
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (2, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                    [],
                )
                .map_err(|error| error.to_string())?;
            applied_version = 2;
        }
        if applied_version == 2 {
            transaction
                .execute_batch(MIGRATION_V3)
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (3, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                    [],
                )
                .map_err(|error| error.to_string())?;
            applied_version = 3;
        }
        if applied_version == 3 {
            transaction
                .execute_batch(MIGRATION_V4)
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (4, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                    [],
                )
                .map_err(|error| error.to_string())?;
            applied_version = 4;
        }
        if applied_version == 4 {
            transaction
                .execute_batch(MIGRATION_V5)
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (5, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                    [],
                )
                .map_err(|error| error.to_string())?;
            applied_version = 5;
        }
        if applied_version == 5 {
            transaction
                .execute_batch(MIGRATION_V6)
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (6, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
                    [],
                )
                .map_err(|error| error.to_string())?;
            applied_version = 6;
        }
        transaction
            .pragma_update(None, "user_version", applied_version)
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    })();

    let restore_foreign_keys_result = if version < 2 && foreign_keys_enabled == 1 {
        connection
            .pragma_update(None, "foreign_keys", true)
            .map_err(|error| error.to_string())
    } else {
        Ok(())
    };
    migration_result?;
    restore_foreign_keys_result
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

    validate_required_tables(connection, &REQUIRED_TABLES)?;
    validate_migration_records(connection, SCHEMA_VERSION)?;

    let fingerprint = schema_fingerprint(connection).map_err(schema_invalid)?;
    if fingerprint != V6_SCHEMA_FINGERPRINT {
        return Err(schema_invalid("schema fingerprint mismatch"));
    }
    validate_database_health(connection)
}

pub fn validate_migratable(connection: &Connection) -> Result<(), String> {
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(schema_invalid)?;
    if version == SCHEMA_VERSION {
        return validate(connection);
    }
    let (required_tables, expected_fingerprint) = match version {
        1 => (&REQUIRED_TABLES_PRE_V3[..], V1_SCHEMA_FINGERPRINT),
        2 => (&REQUIRED_TABLES_PRE_V3[..], V2_SCHEMA_FINGERPRINT),
        3 => (&REQUIRED_TABLES_PRE_V4[..], V3_SCHEMA_FINGERPRINT),
        4 => (&REQUIRED_TABLES_PRE_V5[..], V4_SCHEMA_FINGERPRINT),
        5 => (&REQUIRED_TABLES[..], V5_SCHEMA_FINGERPRINT),
        _ => {
            return Err(schema_invalid(format!(
                "expected a migratable user_version, found {version}"
            )));
        }
    };

    validate_required_tables(connection, required_tables)?;
    validate_migration_records(connection, version)?;
    let fingerprint = schema_fingerprint(connection).map_err(schema_invalid)?;
    if fingerprint != expected_fingerprint {
        return Err(schema_invalid("schema fingerprint mismatch"));
    }
    validate_database_health(connection)
}

fn validate_required_tables(
    connection: &Connection,
    required_tables: &[&str],
) -> Result<(), String> {
    for table in required_tables {
        let exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [*table],
                |row| row.get(0),
            )
            .map_err(|error| schema_invalid(error))?;
        if exists != 1 {
            return Err(schema_invalid(format!("missing table {table}")));
        }
    }

    Ok(())
}

fn validate_migration_records(connection: &Connection, latest_version: i64) -> Result<(), String> {
    for migration_version in 1..=latest_version {
        let migration_exists: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [migration_version],
                |row| row.get(0),
            )
            .map_err(|error| schema_invalid(error))?;
        if migration_exists != 1 {
            return Err(schema_invalid(format!(
                "missing schema_migrations version {migration_version}"
            )));
        }
    }

    Ok(())
}

fn validate_database_health(connection: &Connection) -> Result<(), String> {
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
        migrate, schema_fingerprint, validate, validate_migratable, DATABASE_SCHEMA_INVALID,
        MIGRATION_V1, MIGRATION_V2, MIGRATION_V3, MIGRATION_V4, MIGRATION_V5, SCHEMA_VERSION,
        V1_SCHEMA_FINGERPRINT, V2_SCHEMA_FINGERPRINT, V3_SCHEMA_FINGERPRINT,
        V4_SCHEMA_FINGERPRINT, V5_SCHEMA_FINGERPRINT, V6_SCHEMA_FINGERPRINT,
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
    fn migrate_upgrades_v1_projects_to_allow_repeated_codes() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATION_V1).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", 1_i64)
            .unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            6,
        );
        for id in ["project-1", "project-2"] {
            assert!(connection
                .execute(
                    "INSERT INTO projects (id, code, version, name, file_path, release_date, main_stage_key, archived, created_at, updated_at)
                     VALUES (?1, 'L36', 'v1', 'Test', 'C:/work/L36', '2026-07-31', 'storyboard', 0, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z')",
                    [id],
                )
                .is_ok());
        }
        assert_eq!(
            schema_fingerprint(&connection).unwrap(),
            V6_SCHEMA_FINGERPRINT
        );
        validate(&connection).unwrap();
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
        let lf_migration = MIGRATION_V1.replace("\r\n", "\n");
        let lf = schema_fingerprint_after_applying(&lf_migration);
        let crlf = schema_fingerprint_after_applying(&lf_migration.replace('\n', "\r\n"));

        assert_eq!(lf, crlf);
        assert_eq!(lf, V1_SCHEMA_FINGERPRINT);
    }

    #[test]
    fn validate_migratable_accepts_a_valid_v2_database() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATION_V1).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V2).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (2, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", 2_i64)
            .unwrap();

        assert_eq!(
            schema_fingerprint(&connection).unwrap(),
            V2_SCHEMA_FINGERPRINT
        );
        validate_migratable(&connection).unwrap();
    }

    #[test]
    fn validate_migratable_accepts_a_valid_v3_database() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATION_V1).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V2).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (2, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V3).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (3, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", 3_i64)
            .unwrap();

        assert_eq!(
            schema_fingerprint(&connection).unwrap(),
            V3_SCHEMA_FINGERPRINT
        );
        validate_migratable(&connection).unwrap();
    }

    #[test]
    fn validate_migratable_accepts_a_valid_v4_database() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATION_V1).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V2).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (2, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V3).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (3, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V4).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (4, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", 4_i64)
            .unwrap();

        assert_eq!(
            schema_fingerprint(&connection).unwrap(),
            V4_SCHEMA_FINGERPRINT
        );
        validate_migratable(&connection).unwrap();
    }

    #[test]
    fn migrate_upgrades_v5_daily_tasks_with_reminder_columns() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATION_V1).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V2).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (2, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V3).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (3, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V4).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (4, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection.execute_batch(MIGRATION_V5).unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (5, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", 5_i64)
            .unwrap();

        assert_eq!(
            schema_fingerprint(&connection).unwrap(),
            V5_SCHEMA_FINGERPRINT
        );
        validate_migratable(&connection).unwrap();

        migrate(&mut connection).unwrap();

        assert_eq!(
            connection
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            6,
        );
        let columns = table_columns(&connection, "daily_tasks");
        assert!(columns.contains(&"reminder_time".to_string()));
        assert!(columns.contains(&"reminder_content".to_string()));
        assert_eq!(
            schema_fingerprint(&connection).unwrap(),
            V6_SCHEMA_FINGERPRINT
        );
    }

    #[test]
    fn workspace_sync_foundation_schema_creates_local_cache_tables_and_columns() {
        let mut connection = Connection::open_in_memory().unwrap();

        migrate(&mut connection).unwrap();

        for required in [
            "local_workspaces",
            "sync_outbox",
            "sync_cursors",
            "local_device_bindings",
        ] {
            let exists: i64 = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [required],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing {required}");
        }

        let project_columns = table_columns(&connection, "projects");
        for required in [
            "local_workspace_id",
            "cloud_id",
            "cloud_workspace_id",
            "revision",
            "sync_state",
            "deleted_at",
        ] {
            assert!(project_columns.contains(&required.to_string()), "missing projects.{required}");
        }

        let daily_day_columns = table_columns(&connection, "daily_task_days");
        for required in [
            "local_workspace_id",
            "cloud_id",
            "cloud_workspace_id",
            "revision",
            "sync_state",
            "deleted_at",
        ] {
            assert!(
                daily_day_columns.contains(&required.to_string()),
                "missing daily_task_days.{required}"
            );
        }
    }

    #[test]
    fn production_management_schema_creates_all_tables() {
        let dir = tempfile::tempdir().unwrap();
        let db = crate::db::Database::open(dir.path().join("banana.db")).unwrap();
        let names = db
            .with_connection(|connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT name FROM sqlite_master
                         WHERE type = 'table'
                         ORDER BY name",
                    )
                    .map_err(|error| error.to_string())?;
                let rows = statement
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(|error| error.to_string())?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|error| error.to_string())
            })
            .unwrap();

        for required in [
            "projects",
            "project_stages",
            "daily_task_days",
            "daily_task_groups",
            "daily_tasks",
        ] {
            assert!(
                names.iter().any(|name| name == required),
                "missing {required}"
            );
        }
    }

    #[test]
    fn production_schema_allows_overlapping_stages_but_enforces_local_constraints() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
            .execute(
                "INSERT INTO projects (id, code, version, name, file_path, release_date, main_stage_key, archived, created_at, updated_at)
                 VALUES ('project-1', 'L36', 'v1', 'Test', 'C:/work/L36', '2026-07-31', 'storyboard', 0, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();

        assert!(connection
            .execute(
                "INSERT INTO projects (id, code, version, name, file_path, release_date, main_stage_key, archived, created_at, updated_at)
                 VALUES ('project-2', 'l36', 'v1', 'Duplicate', 'C:/work/L36b', '2026-07-31', 'storyboard', 0, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z')",
                [],
            )
            .is_ok());
        assert!(connection
            .execute(
                "INSERT INTO project_stages (id, project_id, stage_key, position, start_date, end_date, progress, updated_at)
                 VALUES ('stage-1', 'project-1', 'storyboard', 0, '2026-07-01', '2026-07-10', 100, '2026-07-12T00:00:00Z')",
                [],
            )
            .is_ok());
        assert!(connection
            .execute(
                "INSERT INTO project_stages (id, project_id, stage_key, position, start_date, end_date, progress, updated_at)
                 VALUES ('stage-2', 'project-1', 'first_cut', 1, '2026-07-05', '2026-07-15', 0, '2026-07-12T00:00:00Z')",
                [],
            )
            .is_ok());
        assert!(connection
            .execute(
                "INSERT INTO project_stages (id, project_id, stage_key, position, start_date, end_date, progress, updated_at)
                 VALUES ('stage-3', 'project-1', 'refinement', 2, '2026-07-15', '2026-07-16', 101, '2026-07-12T00:00:00Z')",
                [],
            )
            .is_err());
        assert!(connection
            .execute(
                "INSERT INTO project_stages (id, project_id, stage_key, position, start_date, end_date, progress, updated_at)
                 VALUES ('stage-4', 'project-1', 'refinement', 2, '2026-07-16', '2026-07-15', 0, '2026-07-12T00:00:00Z')",
                [],
            )
            .is_err());
    }

    fn schema_fingerprint_after_applying(migration: &str) -> String {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(migration).unwrap();
        schema_fingerprint(&connection).unwrap()
    }

    fn table_columns(connection: &Connection, table: &str) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }
}
