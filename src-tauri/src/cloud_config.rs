use crate::db::Database;
use chrono::Utc;
use rusqlite::params;

const DEFAULT_SUPABASE_URL: &str = "https://erovhwtwlrmxusyrwzbc.supabase.co";
const DEFAULT_SUPABASE_ANON_KEY: &str = "sb_publishable_p3TIyl4W0Fxy6wVv74zLwg_8hTMEkIo";

#[derive(Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudConfigDto {
    pub supabase_url: String,
    pub has_anon_key: bool,
    pub cloud_enabled: bool,
    pub updated_at: Option<String>,
}

#[derive(Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudRuntimeConfigDto {
    pub supabase_url: String,
    pub anon_key: String,
    pub cloud_enabled: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveCloudConfigInput {
    pub supabase_url: String,
    pub anon_key: String,
    pub cloud_enabled: bool,
}

pub fn load_cloud_config(db: &Database) -> Result<CloudConfigDto, String> {
    db.with_connection(|connection| {
        let result = connection.query_row(
            "SELECT supabase_url, anon_key <> '', cloud_enabled, updated_at
             FROM cloud_config WHERE id = 'default'",
            [],
            |row| {
                Ok(CloudConfigDto {
                    supabase_url: row.get(0)?,
                    has_anon_key: row.get::<_, i64>(1)? != 0,
                    cloud_enabled: row.get::<_, i64>(2)? != 0,
                    updated_at: Some(row.get(3)?),
                })
            },
        );

        match result {
            Ok(config) => Ok(config),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(default_cloud_config()),
            Err(error) => Err(error.to_string()),
        }
    })
}

pub fn load_cloud_runtime_config(db: &Database) -> Result<CloudRuntimeConfigDto, String> {
    db.with_connection(|connection| {
        let result = connection.query_row(
            "SELECT supabase_url, anon_key, cloud_enabled
             FROM cloud_config WHERE id = 'default'",
            [],
            |row| {
                let supabase_url = row.get::<_, String>(0)?;
                let anon_key = row.get::<_, String>(1)?;
                let cloud_enabled = row.get::<_, i64>(2)? != 0;
                if cloud_enabled && !anon_key.trim().is_empty() {
                    Ok(CloudRuntimeConfigDto {
                        supabase_url,
                        anon_key,
                        cloud_enabled: true,
                    })
                } else {
                    Ok(CloudRuntimeConfigDto {
                        supabase_url: String::new(),
                        anon_key: String::new(),
                        cloud_enabled: false,
                    })
                }
            },
        );

        match result {
            Ok(config) => Ok(config),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(default_cloud_runtime_config()),
            Err(error) => Err(error.to_string()),
        }
    })
}

pub fn save_cloud_config(
    db: &Database,
    input: SaveCloudConfigInput,
) -> Result<CloudConfigDto, String> {
    let existing_anon_key = load_existing_anon_key(db)?;
    let supabase_url = input.supabase_url.trim().trim_end_matches('/').to_string();
    let can_use_default_key = supabase_url == DEFAULT_SUPABASE_URL;
    validate_cloud_config(&input, existing_anon_key.is_some() || can_use_default_key)?;

    let now = Utc::now().to_rfc3339();
    let anon_key = {
        let trimmed = input.anon_key.trim();
        if trimmed.is_empty() {
            existing_anon_key.unwrap_or_else(|| DEFAULT_SUPABASE_ANON_KEY.to_string())
        } else {
            trimmed.to_string()
        }
    };

    db.with_immediate_transaction(|transaction| {
        transaction
            .execute(
                "INSERT INTO cloud_config
                 (id, supabase_url, anon_key, cloud_enabled, created_at, updated_at)
                 VALUES ('default', ?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(id) DO UPDATE SET
                   supabase_url = excluded.supabase_url,
                   anon_key = excluded.anon_key,
                   cloud_enabled = excluded.cloud_enabled,
                   updated_at = excluded.updated_at",
                params![supabase_url, anon_key, i64::from(input.cloud_enabled), now],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    })?;

    load_cloud_config(db)
}

fn default_cloud_config() -> CloudConfigDto {
    CloudConfigDto {
        supabase_url: DEFAULT_SUPABASE_URL.to_string(),
        has_anon_key: true,
        cloud_enabled: true,
        updated_at: None,
    }
}

fn default_cloud_runtime_config() -> CloudRuntimeConfigDto {
    CloudRuntimeConfigDto {
        supabase_url: DEFAULT_SUPABASE_URL.to_string(),
        anon_key: DEFAULT_SUPABASE_ANON_KEY.to_string(),
        cloud_enabled: true,
    }
}

fn load_existing_anon_key(db: &Database) -> Result<Option<String>, String> {
    db.with_connection(|connection| {
        let result = connection.query_row(
            "SELECT anon_key FROM cloud_config WHERE id = 'default'",
            [],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(anon_key) if anon_key.trim().is_empty() => Ok(None),
            Ok(anon_key) => Ok(Some(anon_key)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    })
}

fn validate_cloud_config(input: &SaveCloudConfigInput, has_existing_key: bool) -> Result<(), String> {
    let supabase_url = input.supabase_url.trim().trim_end_matches('/');
    if supabase_url.is_empty() {
        return Err("CLOUD_URL_REQUIRED".into());
    }
    let parsed = url::Url::parse(supabase_url).map_err(|_| "CLOUD_URL_INVALID".to_string())?;
    let is_loopback_http = parsed.scheme() == "http"
        && matches!(
            parsed.host_str(),
            Some("localhost" | "127.0.0.1" | "::1")
        );
    if parsed.scheme() != "https" && !is_loopback_http {
        return Err("CLOUD_URL_INSECURE".into());
    }

    let anon_key = input.anon_key.trim();
    if anon_key.is_empty() && !has_existing_key {
        return Err("CLOUD_ANON_KEY_REQUIRED".into());
    }
    if anon_key.to_ascii_lowercase().contains("service_role") {
        return Err("CLOUD_SERVICE_ROLE_KEY_BLOCKED".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    fn test_db() -> (TempDir, Database) {
        let dir = tempdir().unwrap();
        let db = Database::open(dir.path().join("banana.db")).unwrap();
        (dir, db)
    }

    #[test]
    fn missing_config_loads_default_cloud_for_first_install() {
        let (_dir, db) = test_db();
        let config = load_cloud_config(&db).unwrap();

        assert_eq!(config.supabase_url, "https://erovhwtwlrmxusyrwzbc.supabase.co");
        assert!(config.has_anon_key);
        assert!(config.cloud_enabled);
        assert_eq!(config.updated_at, None);
    }

    #[test]
    fn missing_runtime_config_returns_default_cloud_for_first_install() {
        let (_dir, db) = test_db();
        let runtime = load_cloud_runtime_config(&db).unwrap();

        assert_eq!(runtime.supabase_url, "https://erovhwtwlrmxusyrwzbc.supabase.co");
        assert!(!runtime.anon_key.is_empty());
        assert!(runtime.cloud_enabled);
    }

    #[test]
    fn saving_config_upserts_the_default_row() {
        let (_dir, db) = test_db();

        let saved = save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co".into(),
                anon_key: "anon-test-key".into(),
                cloud_enabled: true,
            },
        )
        .unwrap();
        let loaded = load_cloud_config(&db).unwrap();

        assert_eq!(saved.supabase_url, "https://example.supabase.co");
        assert_eq!(loaded.supabase_url, "https://example.supabase.co");
        assert!(loaded.has_anon_key);
        assert!(loaded.cloud_enabled);
        assert!(loaded.updated_at.is_some());
    }

    #[test]
    fn service_role_key_is_rejected_without_persisting() {
        let (_dir, db) = test_db();

        let error = match save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co".into(),
                anon_key: "service_role.secret".into(),
                cloud_enabled: true,
            },
        ) {
            Ok(_) => panic!("service role key should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error, "CLOUD_SERVICE_ROLE_KEY_BLOCKED");
        let persisted_rows = db
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM cloud_config", [], |row| row.get::<_, i64>(0))
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        assert_eq!(persisted_rows, 0);
    }

    #[test]
    fn saving_without_a_new_key_preserves_the_existing_key() {
        let (_dir, db) = test_db();
        save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co".into(),
                anon_key: "anon-test-key".into(),
                cloud_enabled: true,
            },
        )
        .unwrap();

        let saved = save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co/".into(),
                anon_key: "".into(),
                cloud_enabled: false,
            },
        )
        .unwrap();

        assert!(saved.has_anon_key);
        assert_eq!(saved.supabase_url, "https://example.supabase.co");
        assert!(!saved.cloud_enabled);
    }

    #[test]
    fn runtime_config_returns_key_only_when_cloud_is_enabled() {
        let (_dir, db) = test_db();
        save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co".into(),
                anon_key: "anon-test-key".into(),
                cloud_enabled: true,
            },
        )
        .unwrap();

        let runtime = load_cloud_runtime_config(&db).unwrap();

        assert_eq!(runtime.supabase_url, "https://example.supabase.co");
        assert_eq!(runtime.anon_key, "anon-test-key");
        assert!(runtime.cloud_enabled);
    }

    #[test]
    fn runtime_config_does_not_return_disabled_key() {
        let (_dir, db) = test_db();
        save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co".into(),
                anon_key: "anon-test-key".into(),
                cloud_enabled: false,
            },
        )
        .unwrap();

        let runtime = load_cloud_runtime_config(&db).unwrap();

        assert_eq!(runtime.supabase_url, "");
        assert_eq!(runtime.anon_key, "");
        assert!(!runtime.cloud_enabled);
    }

    #[test]
    fn backend_rejects_invalid_cloud_config_shape() {
        let (_dir, db) = test_db();

        let missing_key = match save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co".into(),
                anon_key: "".into(),
                cloud_enabled: true,
            },
        ) {
            Ok(_) => panic!("missing anon key should be rejected"),
            Err(error) => error,
        };
        assert_eq!(missing_key, "CLOUD_ANON_KEY_REQUIRED");

        let insecure_url = match save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "http://example.supabase.co".into(),
                anon_key: "anon-test-key".into(),
                cloud_enabled: true,
            },
        ) {
            Ok(_) => panic!("insecure remote url should be rejected"),
            Err(error) => error,
        };
        assert_eq!(insecure_url, "CLOUD_URL_INSECURE");
    }
}
