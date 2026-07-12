use crate::{
    app_state::{AppOperationGate, AppServices, MigrationSummary},
    db::{schema, Database},
    fs_atomic,
    library::{
        library_path, normalize_legacy_json_with_counts, serialize_sanitized_library,
        LegacySecrets, Library, LIBRARY_VERSION,
    },
    provider_http::ProviderHttpClient,
    providers::{
        credential_ref_matches_binding, credential_reference, validated_host_fingerprint,
        ProviderKind, ProviderService, SaveProviderInput,
    },
    secrets::{CredentialMutationCoordinator, CredentialStore},
    startup::{classify, initialize_fresh, recover_initialization, StartupPath},
};
use fs2::FileExt;
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Arc,
};
use uuid::Uuid;

pub(crate) const MIGRATION_SIDECAR_FILE: &str = "migration-v1.json";
const MIGRATION_LOCK_FILE: &str = "migration-v1.lock";
const LIBRARY_FILE: &str = "library.json";
const DATABASE_FILE: &str = "banana.db";
const LIBRARY_TEMP_FILE: &str = "library.json.tmp";
const DATABASE_TEMP_FILE: &str = "banana.db.tmp";
const BACKUP_PREFIX: &str = "migration-backup-";
const ORIGINAL_LIBRARY_BACKUP_PREFIX: &str = ".migration-original-library-";
const MIGRATION_VERSION: u32 = 1;
const MAX_LIBRARY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SIDECAR_BYTES: u64 = 128 * 1024;
const MIGRATION_RECOVERY_REQUIRED: &str = "MIGRATION_RECOVERY_REQUIRED";
const MIGRATION_UNAVAILABLE: &str = "MIGRATION_UNAVAILABLE";
const MIGRATION_LOCK_UNAVAILABLE: &str = "MIGRATION_LOCK_UNAVAILABLE";
const REVERSE_DISPLAY_NAME: &str = "图片反推";
const STORYBOARD_DISPLAY_NAME: &str = "故事板 Agent";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MigrationState {
    Preparing,
    Prepared,
    Committing,
    Complete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MigrationSidecar {
    migration: u32,
    state: MigrationState,
    original_library_hash: String,
    temp_library_hash: Option<String>,
    temp_database_hash: Option<String>,
    backup_path: Option<String>,
    original_library_backup_path: Option<String>,
    candidate_credential_ref: Option<String>,
    credential_origin_fingerprint: Option<String>,
    summary: Option<MigrationSummary>,
    #[serde(default)]
    summary_acknowledged: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryInfo {
    pub message: String,
    pub backup_paths: Vec<String>,
}

pub(crate) enum StartupOutcome {
    Ready {
        services: AppServices,
        migration_summary: Option<MigrationSummary>,
    },
    Recovery(RecoveryInfo),
}

pub(crate) struct StartupCoordinator {
    credentials: Arc<dyn CredentialStore>,
    credential_mutations: Arc<CredentialMutationCoordinator>,
    operations: Arc<AppOperationGate>,
}

impl StartupCoordinator {
    pub(crate) fn new(
        credentials: Arc<dyn CredentialStore>,
        credential_mutations: Arc<CredentialMutationCoordinator>,
        operations: Arc<AppOperationGate>,
    ) -> Self {
        Self {
            credentials,
            credential_mutations,
            operations,
        }
    }

    pub(crate) fn run(&self, data_dir: &Path) -> StartupOutcome {
        match self.run_inner(data_dir, None) {
            Ok((services, migration_summary)) => StartupOutcome::Ready {
                services,
                migration_summary,
            },
            Err(_) => StartupOutcome::Recovery(RecoveryInfo {
                message: "本地数据需要恢复，已保留原始文件和可用备份。".into(),
                backup_paths: recovery_backup_paths(data_dir),
            }),
        }
    }

    fn run_inner(
        &self,
        data_dir: &Path,
        #[allow(unused_variables)] failpoint: Option<MigrationFailpoint>,
    ) -> Result<(AppServices, Option<MigrationSummary>), String> {
        let migration_summary = match classify(data_dir).map_err(|_| MIGRATION_RECOVERY_REQUIRED)? {
            StartupPath::FreshInstall => {
                initialize_fresh(data_dir).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
                None
            }
            StartupPath::RecoverInitialization => {
                recover_initialization(data_dir).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
                None
            }
            StartupPath::LegacyUpgrade => {
                let _lock = lock_migration(data_dir)?;
                let _credential_guard = self
                    .credential_mutations
                    .acquire()
                    .map_err(|_| MIGRATION_UNAVAILABLE)?;
                let mut source_lock = lock_legacy_source(data_dir)?;
                let sidecar = self.prepare_locked(data_dir, Some(&mut source_lock), failpoint)?;
                Some(self.finish_commit_locked(
                    data_dir,
                    sidecar,
                    Some(&mut source_lock),
                    failpoint,
                )?)
            }
            StartupPath::RecoverMigration => {
                let _lock = lock_migration(data_dir)?;
                let _credential_guard = self
                    .credential_mutations
                    .acquire()
                    .map_err(|_| MIGRATION_UNAVAILABLE)?;
                self.recover_locked(data_dir)?
            }
            StartupPath::ReadyV1 => self.complete_summary_if_present(data_dir)?,
            StartupPath::RecoveryRequired => return Err(MIGRATION_RECOVERY_REQUIRED.into()),
        };

        let database = Arc::new(
            Database::open(data_dir.join(DATABASE_FILE))
                .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?,
        );
        self.drain_credential_cleanup(&database);
        let provider_http = Arc::new(ProviderHttpClient::new().map_err(|_| MIGRATION_UNAVAILABLE)?);
        let providers = Arc::new(ProviderService::new(
            database.clone(),
            self.credentials.clone(),
            provider_http.clone(),
            self.credential_mutations.clone(),
        ));
        Ok((
            AppServices {
                database,
                provider_http,
                providers,
                operations: self.operations.clone(),
            },
            migration_summary,
        ))
    }

    fn prepare_locked(
        &self,
        data_dir: &Path,
        source: Option<&mut File>,
        #[allow(unused_variables)] failpoint: Option<MigrationFailpoint>,
    ) -> Result<MigrationSidecar, String> {
        let original_path = library_path(data_dir);
        let raw = match source {
            Some(source) => read_locked_regular_file(source, MAX_LIBRARY_BYTES)?,
            None => read_regular_file(&original_path, MAX_LIBRARY_BYTES)?,
        };
        let original_hash = sha256_bytes(&raw);
        let (library, secrets, warnings, favorites_defaulted, orders_rebuilt) =
            normalize_legacy_json_with_counts(
                std::str::from_utf8(&raw).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?,
            )
            .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
        let reverse_input = legacy_reverse_provider_input(&secrets)?;
        let origin_fingerprint =
            validated_host_fingerprint(&reverse_input).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
        let sanitized_library =
            serialize_sanitized_library(&library).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;

        ensure_missing_regular_file(&data_dir.join(LIBRARY_TEMP_FILE))?;
        ensure_missing_regular_file(&data_dir.join(DATABASE_TEMP_FILE))?;
        let original_library_backup_path = format!(
            "{ORIGINAL_LIBRARY_BACKUP_PREFIX}{}.tmp",
            Uuid::new_v4().simple()
        );
        ensure_missing_regular_file(&data_dir.join(&original_library_backup_path))?;
        let mut sidecar = MigrationSidecar {
            migration: MIGRATION_VERSION,
            state: MigrationState::Preparing,
            original_library_hash: original_hash,
            temp_library_hash: None,
            temp_database_hash: None,
            backup_path: None,
            original_library_backup_path: Some(original_library_backup_path),
            candidate_credential_ref: None,
            credential_origin_fingerprint: None,
            summary: None,
            summary_acknowledged: false,
        };
        write_sidecar_atomic(data_dir, &sidecar)?;
        interrupt_after(failpoint, MigrationFailpoint::AfterSidecar)?;

        let library_temp = data_dir.join(LIBRARY_TEMP_FILE);
        write_new_synced(&library_temp, &sanitized_library)?;
        verify_sanitized_library(&library_temp)?;

        let database_temp = data_dir.join(DATABASE_TEMP_FILE);
        let database = create_temp_database(
            &database_temp,
            &reverse_input,
            &secrets.available_reverse_models,
            &origin_fingerprint,
        )?;
        interrupt_after(failpoint, MigrationFailpoint::AfterDatabase)?;

        if let Some(legacy_key) = secrets.api_key.as_deref() {
            let credential_ref = credential_reference("reverse-image", &origin_fingerprint);
            if !credential_ref_matches_binding(
                &credential_ref,
                "reverse-image",
                &origin_fingerprint,
            ) {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
            sidecar.candidate_credential_ref = Some(credential_ref.clone());
            sidecar.credential_origin_fingerprint = Some(origin_fingerprint.clone());
            write_sidecar_atomic(data_dir, &sidecar)?;
            self.credentials
                .set(&credential_ref, legacy_key)
                .map_err(|_| MIGRATION_UNAVAILABLE)?;
            match self.credentials.get(&credential_ref) {
                Ok(Some(stored)) if stored == legacy_key => {}
                _ => return Err(MIGRATION_UNAVAILABLE.into()),
            }
            bind_temp_credential(&database, &credential_ref)?;
            interrupt_after(failpoint, MigrationFailpoint::AfterCredential)?;
        }
        drop(database);
        checkpoint_and_close_database(&database_temp)?;

        let backup_name = format!("{BACKUP_PREFIX}{}.json", Uuid::new_v4().simple());
        let backup_path = data_dir.join(&backup_name);
        write_new_synced(&backup_path, &sanitized_library)?;
        verify_sanitized_library(&backup_path)?;

        let temp_library_hash = sha256_file(&library_temp)?;
        let temp_database_hash = sha256_file(&database_temp)?;
        verify_staged_database(
            &database_temp,
            sidecar.candidate_credential_ref.as_deref(),
            sidecar.credential_origin_fingerprint.as_deref(),
        )?;
        if let Some(credential_ref) = sidecar.candidate_credential_ref.as_deref() {
            if self
                .credentials
                .get(credential_ref)
                .map_err(|_| MIGRATION_UNAVAILABLE)?
                .is_none()
            {
                return Err(MIGRATION_UNAVAILABLE.into());
            }
        }

        sidecar.temp_library_hash = Some(temp_library_hash);
        sidecar.temp_database_hash = Some(temp_database_hash);
        sidecar.backup_path = Some(backup_name);
        sidecar.summary = Some(MigrationSummary {
            prompts_migrated: library.prompts.len(),
            favorites_defaulted,
            orders_rebuilt,
            backup_path: backup_path.to_string_lossy().to_string(),
            warnings,
        });
        sidecar.state = MigrationState::Prepared;
        write_sidecar_atomic(data_dir, &sidecar)?;
        interrupt_after(failpoint, MigrationFailpoint::AfterPrepared)?;
        Ok(sidecar)
    }

    fn finish_commit_locked(
        &self,
        data_dir: &Path,
        mut sidecar: MigrationSidecar,
        mut source: Option<&mut File>,
        #[allow(unused_variables)] failpoint: Option<MigrationFailpoint>,
    ) -> Result<MigrationSummary, String> {
        validate_sidecar_for_data_dir(data_dir, &sidecar)?;
        if sidecar.state == MigrationState::Complete {
            validate_current_v1_pair(data_dir)?;
            return completed_summary(&sidecar)?.ok_or_else(|| MIGRATION_RECOVERY_REQUIRED.into());
        }
        if !matches!(
            sidecar.state,
            MigrationState::Prepared | MigrationState::Committing
        ) {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
        let temp_library_hash = sidecar
            .temp_library_hash
            .as_deref()
            .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
        let temp_database_hash = sidecar
            .temp_database_hash
            .as_deref()
            .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
        let original_library_backup_path = data_dir.join(
            sidecar
                .original_library_backup_path
                .as_deref()
                .ok_or(MIGRATION_RECOVERY_REQUIRED)?,
        );
        verify_commit_sources(data_dir, &sidecar, source.as_deref_mut())?;

        if sidecar.state == MigrationState::Prepared {
            sidecar.state = MigrationState::Committing;
            write_sidecar_atomic(data_dir, &sidecar)?;
        }
        let library_switched = switch_same_volume(
            &data_dir.join(LIBRARY_TEMP_FILE),
            &data_dir.join(LIBRARY_FILE),
            temp_library_hash,
            Some(&sidecar.original_library_hash),
            Some(&original_library_backup_path),
            source.as_deref_mut(),
        )?;
        remove_verified_original_library_backup(
            &original_library_backup_path,
            &sidecar.original_library_hash,
            source.as_deref_mut(),
            library_switched && cfg!(windows),
        )?;
        if let Some(source) = source {
            source.unlock().map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
        }
        interrupt_after(failpoint, MigrationFailpoint::AfterLibrarySwitch)?;
        switch_same_volume(
            &data_dir.join(DATABASE_TEMP_FILE),
            &data_dir.join(DATABASE_FILE),
            temp_database_hash,
            None,
            None,
            None,
        )?;
        interrupt_after(failpoint, MigrationFailpoint::AfterDatabaseSwitch)?;

        validate_committed(data_dir, &sidecar, self.credentials.as_ref())?;
        sidecar.state = MigrationState::Complete;
        write_sidecar_atomic(data_dir, &sidecar)?;
        remove_verified_staged_file(&data_dir.join(LIBRARY_TEMP_FILE), temp_library_hash)?;
        remove_verified_staged_file(&data_dir.join(DATABASE_TEMP_FILE), temp_database_hash)?;
        completed_summary(&sidecar)?.ok_or_else(|| MIGRATION_RECOVERY_REQUIRED.into())
    }

    fn recover_locked(&self, data_dir: &Path) -> Result<Option<MigrationSummary>, String> {
        let sidecar = read_sidecar(data_dir)?;
        validate_sidecar_for_data_dir(data_dir, &sidecar)?;
        match sidecar.state {
            MigrationState::Preparing => {
                let mut source_lock = lock_legacy_source(data_dir)?;
                validate_preparing_source_layout(data_dir, &sidecar, &mut source_lock)?;
                self.cleanup_preparing_candidate(data_dir, &sidecar)?;
                remove_preparing_temp(&data_dir.join(LIBRARY_TEMP_FILE))?;
                remove_preparing_database_temp(&data_dir.join(DATABASE_TEMP_FILE))?;
                fs::remove_file(data_dir.join(MIGRATION_SIDECAR_FILE))
                    .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
                let prepared = self.prepare_locked(data_dir, Some(&mut source_lock), None)?;
                Ok(Some(self.finish_commit_locked(
                    data_dir,
                    prepared,
                    Some(&mut source_lock),
                    None,
                )?))
            }
            MigrationState::Prepared | MigrationState::Committing => {
                recover_partial_library_replacement(data_dir, &sidecar)?;
                let mut source_lock = lock_legacy_source(data_dir)?;
                Ok(Some(self.finish_commit_locked(
                    data_dir,
                    sidecar,
                    Some(&mut source_lock),
                    None,
                )?))
            }
            MigrationState::Complete => {
                validate_current_v1_pair(data_dir)?;
                completed_summary(&sidecar)
            }
        }
    }

    fn cleanup_preparing_candidate(
        &self,
        data_dir: &Path,
        sidecar: &MigrationSidecar,
    ) -> Result<(), String> {
        let Some(credential_ref) = sidecar.candidate_credential_ref.as_deref() else {
            return Ok(());
        };
        if credential_reference_is_active(data_dir, credential_ref)? {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
        self.credentials
            .delete(credential_ref)
            .map_err(|_| MIGRATION_UNAVAILABLE)?;
        if self
            .credentials
            .get(credential_ref)
            .map_err(|_| MIGRATION_UNAVAILABLE)?
            .is_some()
        {
            return Err(MIGRATION_UNAVAILABLE.into());
        }
        Ok(())
    }

    fn complete_summary_if_present(
        &self,
        data_dir: &Path,
    ) -> Result<Option<MigrationSummary>, String> {
        let sidecar_path = data_dir.join(MIGRATION_SIDECAR_FILE);
        if !sidecar_path.exists() {
            return Ok(None);
        }
        let sidecar = read_sidecar(data_dir)?;
        if sidecar.state != MigrationState::Complete {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
        validate_sidecar_for_data_dir(data_dir, &sidecar)?;
        validate_current_v1_pair(data_dir)?;
        completed_summary(&sidecar)
    }

    fn drain_credential_cleanup(&self, database: &Database) {
        let Ok(_credential_guard) = self.credential_mutations.acquire() else {
            return;
        };
        let references = database.with_connection(|connection| {
            let mut statement = connection
                .prepare("SELECT credential_ref FROM credential_cleanup ORDER BY credential_ref")
                .map_err(|_| MIGRATION_UNAVAILABLE.to_string())?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|_| MIGRATION_UNAVAILABLE.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| MIGRATION_UNAVAILABLE.to_string())
        });
        let Ok(references) = references else {
            return;
        };

        for credential_ref in references {
            let active = database
                .with_connection(|connection| {
                    connection
                        .query_row(
                            "SELECT EXISTS(SELECT 1 FROM ai_providers WHERE credential_ref = ?1)",
                            [&credential_ref],
                            |row| row.get::<_, i64>(0),
                        )
                        .map(|value| value != 0)
                        .map_err(|_| MIGRATION_UNAVAILABLE.to_string())
                })
                .unwrap_or(true);
            if active {
                let _ = delete_cleanup_reference(database, &credential_ref);
                continue;
            }
            if !is_managed_credential_reference(&credential_ref) {
                continue;
            }

            // Failed deletions remain journaled for a later startup attempt.
            let removed = self
                .credentials
                .delete(&credential_ref)
                .and_then(|()| self.credentials.get(&credential_ref))
                .map(|remaining| remaining.is_none())
                .unwrap_or(false);
            if removed {
                let _ = delete_cleanup_reference(database, &credential_ref);
            }
        }
    }
}

fn completed_summary(sidecar: &MigrationSidecar) -> Result<Option<MigrationSummary>, String> {
    if sidecar.summary_acknowledged {
        return Ok(None);
    }
    sidecar
        .summary
        .clone()
        .map(Some)
        .ok_or_else(|| MIGRATION_RECOVERY_REQUIRED.into())
}

fn recovery_backup_paths(data_dir: &Path) -> Vec<String> {
    read_sidecar(data_dir)
        .ok()
        .and_then(|sidecar| sidecar.backup_path)
        .filter(|name| is_backup_file_name(name))
        .map(|name| data_dir.join(name).to_string_lossy().to_string())
        .into_iter()
        .collect()
}

pub(crate) fn acknowledge_migration_summary(data_dir: &Path) -> Result<(), String> {
    let _lock = lock_migration(data_dir)?;
    let mut sidecar = read_sidecar(data_dir)?;
    validate_sidecar_for_data_dir(data_dir, &sidecar)?;
    if sidecar.state != MigrationState::Complete {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    validate_current_v1_pair(data_dir)?;
    if !sidecar.summary_acknowledged {
        sidecar.summary_acknowledged = true;
        write_sidecar_atomic(data_dir, &sidecar)?;
    }
    Ok(())
}

fn delete_cleanup_reference(database: &Database, credential_ref: &str) -> Result<(), String> {
    database.with_immediate_transaction(|transaction| {
        transaction
            .execute(
                "DELETE FROM credential_cleanup WHERE credential_ref = ?1",
                [credential_ref],
            )
            .map_err(|_| MIGRATION_UNAVAILABLE.to_string())?;
        Ok(())
    })
}

fn is_managed_credential_reference(credential_ref: &str) -> bool {
    let mut parts = credential_ref.split('/');
    let (Some(prefix), Some(provider_id), Some(origin_hash), Some(identifier)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    parts.next().is_none()
        && prefix == "provider"
        && matches!(provider_id, "reverse-image" | "storyboard")
        && is_lower_sha256(origin_hash)
        && Uuid::parse_str(identifier)
            .map(|value| value.hyphenated().to_string() == identifier)
            .unwrap_or(false)
}

fn legacy_reverse_provider_input(secrets: &LegacySecrets) -> Result<SaveProviderInput, String> {
    let base_url = secrets.api_base_url.trim().trim_end_matches('/');
    if base_url.is_empty() {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    let version_base = if base_url.ends_with("/v1") {
        base_url.to_string()
    } else {
        format!("{base_url}/v1")
    };
    Ok(SaveProviderInput {
        id: "reverse-image".into(),
        kind: ProviderKind::ReverseImage,
        display_name: REVERSE_DISPLAY_NAME.into(),
        base_url: base_url.into(),
        models_url: format!("{version_base}/models"),
        chat_completions_url: format!("{version_base}/chat/completions"),
        default_model: Some(secrets.reverse_model.clone()),
        confirm_cross_origin: false,
    })
}

fn create_temp_database(
    path: &Path,
    reverse: &SaveProviderInput,
    available_models: &[String],
    origin_fingerprint: &str,
) -> Result<Database, String> {
    let available_models_json =
        serde_json::to_string(available_models).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    let database = Database::open(path).map_err(|_| MIGRATION_UNAVAILABLE)?;
    database
        .with_immediate_transaction(|transaction| {
            let reverse_rows = transaction
                .execute(
                    "INSERT INTO ai_providers (
                        id, kind, display_name, base_url, models_url, chat_completions_url,
                        default_model, available_models_json, probed_model, structured_mode,
                        interactive_compatible, bound_host, needs_credentials, credential_ref,
                        created_at, updated_at
                     ) VALUES (
                        'reverse-image', 'reverse-image', ?1, ?2, ?3, ?4,
                        ?5, ?6, NULL, NULL, NULL, ?7, 1, NULL,
                        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     )",
                    params![
                        reverse.display_name,
                        reverse.base_url,
                        reverse.models_url,
                        reverse.chat_completions_url,
                        reverse.default_model,
                        available_models_json,
                        origin_fingerprint,
                    ],
                )
                .map_err(|_| MIGRATION_UNAVAILABLE.to_string())?;
            let storyboard_rows = transaction
                .execute(
                    "INSERT INTO ai_providers (
                        id, kind, display_name, base_url, models_url, chat_completions_url,
                        default_model, available_models_json, probed_model, structured_mode,
                        interactive_compatible, bound_host, needs_credentials, credential_ref,
                        created_at, updated_at
                     ) VALUES (
                        'storyboard', 'storyboard', ?1, '', '', '',
                        NULL, '[]', NULL, NULL, NULL, NULL, 1, NULL,
                        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     )",
                    [STORYBOARD_DISPLAY_NAME],
                )
                .map_err(|_| MIGRATION_UNAVAILABLE.to_string())?;
            if reverse_rows != 1 || storyboard_rows != 1 {
                return Err(MIGRATION_UNAVAILABLE.into());
            }
            Ok(())
        })
        .map_err(|_| MIGRATION_UNAVAILABLE)?;
    Ok(database)
}

fn bind_temp_credential(database: &Database, credential_ref: &str) -> Result<(), String> {
    database
        .with_immediate_transaction(|transaction| {
            let changed = transaction
                .execute(
                    "UPDATE ai_providers
                     SET credential_ref = ?1, needs_credentials = 0,
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = 'reverse-image'",
                    [credential_ref],
                )
                .map_err(|_| MIGRATION_UNAVAILABLE.to_string())?;
            if changed != 1 {
                return Err(MIGRATION_UNAVAILABLE.into());
            }
            Ok(())
        })
        .map_err(|_| MIGRATION_UNAVAILABLE.to_string())
}

fn verify_sanitized_library(path: &Path) -> Result<(), String> {
    let raw = read_regular_file(path, MAX_LIBRARY_BYTES)?;
    let value: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    let settings = value
        .get("settings")
        .and_then(serde_json::Value::as_object)
        .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
    if [
        "apiBaseUrl",
        "apiKey",
        "reverseModel",
        "availableReverseModels",
    ]
    .iter()
    .any(|key| settings.contains_key(*key))
    {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    let library: Library =
        serde_json::from_value(value).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    if library.version != LIBRARY_VERSION {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    Ok(())
}

fn verify_staged_database(
    path: &Path,
    credential_ref: Option<&str>,
    origin_fingerprint: Option<&str>,
) -> Result<(), String> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    schema::validate(&connection).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    let expected_needs_credentials = i64::from(credential_ref.is_none());
    let (kind, bound_host, needs_credentials, row_credential_ref): (
        String,
        Option<String>,
        i64,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT kind, bound_host, needs_credentials, credential_ref
             FROM ai_providers WHERE id = 'reverse-image'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    let storyboard_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_providers WHERE id = 'storyboard' AND kind = 'storyboard'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    if kind != "reverse-image"
        || bound_host.as_deref() != origin_fingerprint
        || needs_credentials != expected_needs_credentials
        || row_credential_ref.as_deref() != credential_ref
        || storyboard_count != 1
    {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    if let (Some(reference), Some(origin)) = (credential_ref, origin_fingerprint) {
        if !credential_ref_matches_binding(reference, "reverse-image", origin) {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
    }
    Ok(())
}

fn checkpoint_and_close_database(path: &Path) -> Result<(), String> {
    let connection = Connection::open(path).map_err(|_| MIGRATION_UNAVAILABLE)?;
    checkpoint_and_close(connection)?;
    remove_sqlite_sidecars(path)
}

fn checkpoint_and_close(connection: Connection) -> Result<(), String> {
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|_| MIGRATION_UNAVAILABLE)?;
    connection
        .close()
        .map_err(|_| MIGRATION_UNAVAILABLE.to_string())
}

fn remove_sqlite_sidecars(path: &Path) -> Result<(), String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = path.with_file_name(format!("{file_name}{suffix}"));
        match regular_file_state(&sidecar)? {
            FileState::Missing => {}
            FileState::Other => return Err(MIGRATION_RECOVERY_REQUIRED.into()),
            FileState::File => {
                if suffix == "-wal"
                    && fs::metadata(&sidecar)
                        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?
                        .len()
                        != 0
                {
                    return Err(MIGRATION_RECOVERY_REQUIRED.into());
                }
                fs::remove_file(sidecar).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
            }
        }
    }
    Ok(())
}

fn verify_commit_sources(
    data_dir: &Path,
    sidecar: &MigrationSidecar,
    source: Option<&mut File>,
) -> Result<(), String> {
    let new_library_hash = sidecar
        .temp_library_hash
        .as_deref()
        .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
    let new_database_hash = sidecar
        .temp_database_hash
        .as_deref()
        .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
    let library_hash = match source {
        Some(source) => {
            if !path_matches_locked_file(&data_dir.join(LIBRARY_FILE), source)? {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
            Some(sha256_locked_file(source)?)
        }
        None => file_hash_if_regular(&data_dir.join(LIBRARY_FILE))?,
    };
    match library_hash {
        Some(hash) if hash == sidecar.original_library_hash || hash == new_library_hash => {}
        _ => return Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
    match file_hash_if_regular(&data_dir.join(DATABASE_FILE))? {
        None => {}
        Some(hash) if hash == new_database_hash => {}
        Some(_) => return Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
    for (path, expected) in [
        (data_dir.join(LIBRARY_TEMP_FILE), new_library_hash),
        (data_dir.join(DATABASE_TEMP_FILE), new_database_hash),
    ] {
        if let Some(hash) = file_hash_if_regular(&path)? {
            if hash != expected {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
        }
    }
    Ok(())
}

fn switch_same_volume(
    temp: &Path,
    destination: &Path,
    expected_hash: &str,
    expected_previous_hash: Option<&str>,
    replacement_backup: Option<&Path>,
    locked_destination: Option<&mut File>,
) -> Result<bool, String> {
    let destination_hash = match locked_destination {
        Some(locked_file) => {
            if !path_matches_locked_file(destination, locked_file)? {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
            Some(sha256_locked_file(locked_file)?)
        }
        None => file_hash_if_regular(destination)?,
    };
    if destination_hash.as_deref() == Some(expected_hash) {
        return Ok(false);
    }
    if destination_hash.as_deref() != expected_previous_hash {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    if file_hash_if_regular(temp)?.as_deref() != Some(expected_hash) {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    match (expected_previous_hash, replacement_backup) {
        (Some(previous_hash), Some(backup)) => {
            match file_hash_if_regular(backup)? {
                None => {}
                Some(hash) if hash == previous_hash => {
                    fs::remove_file(backup).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
                }
                Some(_) => return Err(MIGRATION_RECOVERY_REQUIRED.into()),
            }
            fs_atomic::replace_existing_file_with_backup(temp, destination, backup)
        }
        (Some(_), None) => fs_atomic::replace_existing_file(temp, destination),
        (None, None) => fs_atomic::replace_file(temp, destination),
        (None, Some(_)) => return Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
    .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    sync_existing_file(destination)?;
    if file_hash_if_regular(destination)?.as_deref() != Some(expected_hash) {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    Ok(true)
}

fn remove_verified_original_library_backup(
    backup: &Path,
    expected_hash: &str,
    locked_backup: Option<&mut File>,
    require_present: bool,
) -> Result<(), String> {
    let backup_hash = match locked_backup {
        Some(file) if path_matches_locked_file(backup, file)? => Some(sha256_locked_file(file)?),
        _ => file_hash_if_regular(backup)?,
    };
    match backup_hash {
        Some(hash) if hash == expected_hash => {
            fs::remove_file(backup).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
            if regular_file_state(backup)? != FileState::Missing {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
            Ok(())
        }
        None if !require_present => Ok(()),
        None | Some(_) => Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
}

fn recover_partial_library_replacement(
    data_dir: &Path,
    sidecar: &MigrationSidecar,
) -> Result<(), String> {
    let library = data_dir.join(LIBRARY_FILE);
    match regular_file_state(&library)? {
        FileState::File => Ok(()),
        FileState::Other => Err(MIGRATION_RECOVERY_REQUIRED.into()),
        FileState::Missing => {
            let backup = data_dir.join(
                sidecar
                    .original_library_backup_path
                    .as_deref()
                    .ok_or(MIGRATION_RECOVERY_REQUIRED)?,
            );
            let temp = data_dir.join(LIBRARY_TEMP_FILE);
            let expected_temp_hash = sidecar
                .temp_library_hash
                .as_deref()
                .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
            if file_hash_if_regular(&backup)?.as_deref() != Some(&sidecar.original_library_hash)
                || file_hash_if_regular(&temp)?.as_deref() != Some(expected_temp_hash)
            {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
            fs_atomic::replace_file(&temp, &library).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
            sync_existing_file(&library)?;
            if file_hash_if_regular(&library)?.as_deref() != Some(expected_temp_hash) {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
            Ok(())
        }
    }
}

fn validate_committed(
    data_dir: &Path,
    sidecar: &MigrationSidecar,
    credentials: &dyn CredentialStore,
) -> Result<(), String> {
    let library_hash = sidecar
        .temp_library_hash
        .as_deref()
        .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
    let database_hash = sidecar
        .temp_database_hash
        .as_deref()
        .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
    if file_hash_if_regular(&data_dir.join(LIBRARY_FILE))?.as_deref() != Some(library_hash)
        || file_hash_if_regular(&data_dir.join(DATABASE_FILE))?.as_deref() != Some(database_hash)
    {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    verify_sanitized_library(&data_dir.join(LIBRARY_FILE))?;
    verify_staged_database(
        &data_dir.join(DATABASE_FILE),
        sidecar.candidate_credential_ref.as_deref(),
        sidecar.credential_origin_fingerprint.as_deref(),
    )?;
    if let Some(credential_ref) = sidecar.candidate_credential_ref.as_deref() {
        if credentials
            .get(credential_ref)
            .map_err(|_| MIGRATION_UNAVAILABLE)?
            .is_none()
        {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
    }
    Ok(())
}

fn validate_current_v1_pair(data_dir: &Path) -> Result<(), String> {
    verify_sanitized_library(&data_dir.join(LIBRARY_FILE))?;

    let database_path = data_dir.join(DATABASE_FILE);
    if regular_file_state(&database_path)? != FileState::File {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    let connection = Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    schema::validate(&connection).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    for (id, kind) in [
        ("reverse-image", "reverse-image"),
        ("storyboard", "storyboard"),
    ] {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM ai_providers WHERE id = ?1 AND kind = ?2",
                params![id, kind],
                |row| row.get(0),
            )
            .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
        if count != 1 {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
    }
    Ok(())
}

fn validate_preparing_source_layout(
    data_dir: &Path,
    sidecar: &MigrationSidecar,
    source: &mut File,
) -> Result<(), String> {
    if !path_matches_locked_file(&data_dir.join(LIBRARY_FILE), source)?
        || sha256_locked_file(source)? != sidecar.original_library_hash
        || regular_file_state(&data_dir.join(DATABASE_FILE))? != FileState::Missing
    {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    Ok(())
}

fn credential_reference_is_active(data_dir: &Path, credential_ref: &str) -> Result<bool, String> {
    let database_path = data_dir.join(DATABASE_FILE);
    match regular_file_state(&database_path)? {
        FileState::Missing => Ok(false),
        FileState::Other => Err(MIGRATION_RECOVERY_REQUIRED.into()),
        FileState::File => {
            let connection =
                Connection::open_with_flags(&database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
            schema::validate(&connection).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM ai_providers WHERE credential_ref = ?1)",
                    [credential_ref],
                    |row| row.get::<_, i64>(0),
                )
                .map(|active| active != 0)
                .map_err(|_| MIGRATION_RECOVERY_REQUIRED.into())
        }
    }
}

fn validate_sidecar_for_data_dir(
    data_dir: &Path,
    sidecar: &MigrationSidecar,
) -> Result<(), String> {
    validate_sidecar(sidecar)?;
    if let Some(backup_name) = sidecar.backup_path.as_deref() {
        let summary = sidecar
            .summary
            .as_ref()
            .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
        if summary.backup_path != data_dir.join(backup_name).to_string_lossy() {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
    }
    Ok(())
}

fn validate_sidecar(sidecar: &MigrationSidecar) -> Result<(), String> {
    if sidecar.migration != MIGRATION_VERSION || !is_lower_sha256(&sidecar.original_library_hash) {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    if !sidecar
        .original_library_backup_path
        .as_deref()
        .is_some_and(is_original_library_backup_file_name)
    {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    let has_candidate = sidecar.candidate_credential_ref.is_some();
    if has_candidate != sidecar.credential_origin_fingerprint.is_some() {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    if let (Some(reference), Some(origin)) = (
        sidecar.candidate_credential_ref.as_deref(),
        sidecar.credential_origin_fingerprint.as_deref(),
    ) {
        if !credential_ref_matches_binding(reference, "reverse-image", origin) {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
    }
    match sidecar.state {
        MigrationState::Preparing => {
            if sidecar.temp_library_hash.is_some()
                || sidecar.temp_database_hash.is_some()
                || sidecar.backup_path.is_some()
                || sidecar.summary.is_some()
                || sidecar.summary_acknowledged
            {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
        }
        MigrationState::Prepared | MigrationState::Committing | MigrationState::Complete => {
            if !sidecar
                .temp_library_hash
                .as_deref()
                .is_some_and(is_lower_sha256)
                || !sidecar
                    .temp_database_hash
                    .as_deref()
                    .is_some_and(is_lower_sha256)
                || !sidecar
                    .backup_path
                    .as_deref()
                    .is_some_and(is_backup_file_name)
                || sidecar.summary.is_none()
            {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
            if sidecar.state != MigrationState::Complete && sidecar.summary_acknowledged {
                return Err(MIGRATION_RECOVERY_REQUIRED.into());
            }
        }
    }
    Ok(())
}

pub(crate) fn read_sidecar(data_dir: &Path) -> Result<MigrationSidecar, String> {
    let raw = read_regular_file(&data_dir.join(MIGRATION_SIDECAR_FILE), MAX_SIDECAR_BYTES)?;
    let sidecar: MigrationSidecar =
        serde_json::from_slice(&raw).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    validate_sidecar(&sidecar)?;
    Ok(sidecar)
}

pub(crate) fn migration_sidecar_state(data_dir: &Path) -> Result<MigrationState, String> {
    read_sidecar(data_dir).map(|sidecar| sidecar.state)
}

fn write_sidecar_atomic(data_dir: &Path, sidecar: &MigrationSidecar) -> Result<(), String> {
    validate_sidecar(sidecar)?;
    let bytes = serde_json::to_vec(sidecar).map_err(|_| MIGRATION_UNAVAILABLE)?;
    let temporary = data_dir.join(format!(
        ".migration-sidecar-{}.tmp",
        Uuid::new_v4().simple()
    ));
    write_new_synced(&temporary, &bytes)?;
    fs_atomic::replace_file(&temporary, &data_dir.join(MIGRATION_SIDECAR_FILE))
        .map_err(|_| MIGRATION_UNAVAILABLE)?;
    sync_existing_file(&data_dir.join(MIGRATION_SIDECAR_FILE))
}

fn lock_migration(data_dir: &Path) -> Result<File, String> {
    fs::create_dir_all(data_dir).map_err(|_| MIGRATION_LOCK_UNAVAILABLE)?;
    let lock_path = data_dir.join(MIGRATION_LOCK_FILE);
    if regular_file_state(&lock_path)? == FileState::Other {
        return Err(MIGRATION_LOCK_UNAVAILABLE.into());
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(lock_path)
        .map_err(|_| MIGRATION_LOCK_UNAVAILABLE)?;
    lock.try_lock_exclusive()
        .map_err(|_| MIGRATION_LOCK_UNAVAILABLE)?;
    Ok(lock)
}

fn lock_legacy_source(data_dir: &Path) -> Result<File, String> {
    let source_path = library_path(data_dir);
    if regular_file_state(&source_path)? != FileState::File {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }

    let mut options = OpenOptions::new();
    options.read(true).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ};

        options.share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE);
    }
    let source = options
        .open(&source_path)
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;

    source
        .try_lock_exclusive()
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    Ok(source)
}

fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| MIGRATION_UNAVAILABLE)?;
    file.write_all(bytes).map_err(|_| MIGRATION_UNAVAILABLE)?;
    file.sync_all()
        .map_err(|_| MIGRATION_UNAVAILABLE.to_string())
}

fn sync_existing_file(path: &Path) -> Result<(), String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED.into())
}

fn read_regular_file(path: &Path, max_bytes: u64) -> Result<Vec<u8>, String> {
    if regular_file_state(path)? != FileState::File {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    let mut file = File::open(path).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    read_bounded_file(&mut file, max_bytes)
}

fn read_locked_regular_file(file: &mut File, max_bytes: u64) -> Result<Vec<u8>, String> {
    if !file
        .metadata()
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?
        .file_type()
        .is_file()
    {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    read_bounded_file(file, max_bytes)
}

fn read_bounded_file(file: &mut File, max_bytes: u64) -> Result<Vec<u8>, String> {
    let metadata = file.metadata().map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    if metadata.len() > max_bytes {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }

    file.seek(SeekFrom::Start(0))
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
        if read == 0 {
            break;
        }
        let next_len = (bytes.len() as u64)
            .checked_add(read as u64)
            .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
        if next_len > max_bytes {
            return Err(MIGRATION_RECOVERY_REQUIRED.into());
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    if regular_file_state(path)? != FileState::File {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    let mut file = File::open(path).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    sha256_open_file(&mut file)
}

fn sha256_locked_file(file: &mut File) -> Result<String, String> {
    if !file
        .metadata()
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?
        .file_type()
        .is_file()
    {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    sha256_open_file(file)
}

fn sha256_open_file(file: &mut File) -> Result<String, String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn path_matches_locked_file(path: &Path, file: &File) -> Result<bool, String> {
    if regular_file_state(path)? != FileState::File {
        return Ok(false);
    }
    let path_file = File::open(path).map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    Ok(file_identity(file)? == file_identity(&path_file)?)
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    volume_serial_number: u32,
    file_index: u64,
}

#[cfg(windows)]
fn file_identity(file: &File) -> Result<FileIdentity, String> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
    };

    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    let ok = unsafe {
        GetFileInformationByHandle(
            file.as_raw_handle() as HANDLE,
            &mut information as *mut BY_HANDLE_FILE_INFORMATION,
        )
    };
    if ok == 0 {
        return Err(MIGRATION_RECOVERY_REQUIRED.into());
    }
    Ok(FileIdentity {
        volume_serial_number: information.dwVolumeSerialNumber,
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
    })
}

#[cfg(not(windows))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(not(windows))]
fn file_identity(file: &File) -> Result<FileIdentity, String> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata().map_err(|_| MIGRATION_RECOVERY_REQUIRED)?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileState {
    Missing,
    File,
    Other,
}

fn regular_file_state(path: &Path) -> Result<FileState, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(FileState::File),
        Ok(_) => Ok(FileState::Other),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(FileState::Missing),
        Err(_) => Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
}

fn file_hash_if_regular(path: &Path) -> Result<Option<String>, String> {
    match regular_file_state(path)? {
        FileState::Missing => Ok(None),
        FileState::File => sha256_file(path).map(Some),
        FileState::Other => Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
}

fn ensure_missing_regular_file(path: &Path) -> Result<(), String> {
    match regular_file_state(path)? {
        FileState::Missing => Ok(()),
        FileState::File | FileState::Other => Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
}

fn remove_preparing_temp(path: &Path) -> Result<(), String> {
    match regular_file_state(path)? {
        FileState::Missing => Ok(()),
        FileState::File => fs::remove_file(path).map_err(|_| MIGRATION_RECOVERY_REQUIRED.into()),
        FileState::Other => Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
}

fn remove_preparing_database_temp(path: &Path) -> Result<(), String> {
    remove_preparing_temp(path)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(MIGRATION_RECOVERY_REQUIRED)?;
    for suffix in ["-wal", "-shm"] {
        remove_preparing_temp(&path.with_file_name(format!("{file_name}{suffix}")))?;
    }
    Ok(())
}

fn remove_verified_staged_file(path: &Path, expected_hash: &str) -> Result<(), String> {
    match file_hash_if_regular(path)? {
        None => Ok(()),
        Some(hash) if hash == expected_hash => {
            fs::remove_file(path).map_err(|_| MIGRATION_RECOVERY_REQUIRED.into())
        }
        Some(_) => Err(MIGRATION_RECOVERY_REQUIRED.into()),
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
}

fn is_backup_file_name(value: &str) -> bool {
    let path = Path::new(value);
    path.file_name().and_then(|name| name.to_str()) == Some(value)
        && value.starts_with(BACKUP_PREFIX)
        && value.ends_with(".json")
        && value.len() == BACKUP_PREFIX.len() + 32 + ".json".len()
        && value[BACKUP_PREFIX.len()..value.len() - ".json".len()]
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

fn is_original_library_backup_file_name(value: &str) -> bool {
    let path = Path::new(value);
    path.file_name().and_then(|name| name.to_str()) == Some(value)
        && value.starts_with(ORIGINAL_LIBRARY_BACKUP_PREFIX)
        && value.ends_with(".tmp")
        && value.len() == ORIGINAL_LIBRARY_BACKUP_PREFIX.len() + 32 + ".tmp".len()
        && value[ORIGINAL_LIBRARY_BACKUP_PREFIX.len()..value.len() - ".tmp".len()]
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MigrationFailpoint {
    AfterSidecar,
    AfterDatabase,
    AfterCredential,
    AfterPrepared,
    AfterLibrarySwitch,
    AfterDatabaseSwitch,
}

#[cfg(not(test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MigrationFailpoint {
    AfterSidecar,
    AfterDatabase,
    AfterCredential,
    AfterPrepared,
    AfterLibrarySwitch,
    AfterDatabaseSwitch,
}

#[cfg(test)]
impl MigrationFailpoint {
    pub(crate) const ALL: &'static [Self] = &[
        Self::AfterSidecar,
        Self::AfterDatabase,
        Self::AfterCredential,
        Self::AfterPrepared,
        Self::AfterLibrarySwitch,
        Self::AfterDatabaseSwitch,
    ];
}

#[cfg(test)]
pub(crate) fn run_with_failpoint(
    coordinator: &StartupCoordinator,
    data_dir: &Path,
    failpoint: MigrationFailpoint,
) -> Result<(), String> {
    coordinator.run_with_failpoint(data_dir, failpoint)
}

#[cfg(test)]
impl StartupCoordinator {
    pub(crate) fn run_with_failpoint(
        &self,
        data_dir: &Path,
        failpoint: MigrationFailpoint,
    ) -> Result<(), String> {
        self.run_inner(data_dir, Some(failpoint)).map(|_| ())
    }
}

#[cfg(test)]
fn interrupt_after(
    failpoint: Option<MigrationFailpoint>,
    expected: MigrationFailpoint,
) -> Result<(), String> {
    if failpoint == Some(expected) {
        return Err("MIGRATION_INTERRUPTED".into());
    }
    Ok(())
}

#[cfg(not(test))]
fn interrupt_after(
    _failpoint: Option<MigrationFailpoint>,
    _expected: MigrationFailpoint,
) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app_state::AppOperationGate,
        library::library_path,
        secrets::{CredentialMutationCoordinator, CredentialStore, MemoryCredentialStore},
    };
    use rusqlite::Connection;
    use std::{fs, sync::Arc};
    use tempfile::tempdir;

    const LEGACY_KEY: &str = "test-only-migration-key";

    fn sidecar(state: MigrationState) -> MigrationSidecar {
        let complete_fields = !matches!(state, MigrationState::Preparing);
        MigrationSidecar {
            migration: 1,
            state,
            original_library_hash: "a".repeat(64),
            temp_library_hash: complete_fields.then(|| "b".repeat(64)),
            temp_database_hash: complete_fields.then(|| "c".repeat(64)),
            backup_path: complete_fields.then(|| format!("{BACKUP_PREFIX}{}.json", "d".repeat(32))),
            original_library_backup_path: Some(format!(
                "{ORIGINAL_LIBRARY_BACKUP_PREFIX}{}.tmp",
                "e".repeat(32)
            )),
            candidate_credential_ref: Some(credential_reference(
                "reverse-image",
                "https://legacy.example.test",
            )),
            credential_origin_fingerprint: Some("https://legacy.example.test".into()),
            summary: complete_fields.then(|| crate::app_state::MigrationSummary {
                prompts_migrated: 1,
                favorites_defaulted: 1,
                orders_rebuilt: 1,
                backup_path: "C:/safe/migration-backup.json".into(),
                warnings: vec![
                    "缺失的 favorite 已按 false 迁移，历史上已经丢失的收藏无法恢复".into(),
                ],
            }),
            summary_acknowledged: false,
        }
    }

    #[test]
    fn sidecar_round_trips_every_state_and_rejects_an_unsupported_version() {
        let directory = tempdir().unwrap();
        for state in [
            MigrationState::Preparing,
            MigrationState::Prepared,
            MigrationState::Committing,
            MigrationState::Complete,
        ] {
            let expected = sidecar(state);
            write_sidecar_atomic(directory.path(), &expected).unwrap();
            assert_eq!(read_sidecar(directory.path()).unwrap(), expected);
        }

        let mut invalid = serde_json::to_value(sidecar(MigrationState::Prepared)).unwrap();
        invalid["migration"] = serde_json::Value::from(2);
        fs::write(
            directory.path().join(MIGRATION_SIDECAR_FILE),
            serde_json::to_vec(&invalid).unwrap(),
        )
        .unwrap();
        assert!(read_sidecar(directory.path()).is_err());
    }

    #[test]
    fn legacy_upgrade_sanitizes_library_and_moves_the_key_to_credential_storage() {
        let directory = tempdir().unwrap();
        fs::write(library_path(directory.path()), legacy_library_json()).unwrap();
        let credentials = Arc::new(MemoryCredentialStore::default());
        let coordinator = StartupCoordinator::new(
            credentials.clone(),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        let _lock = lock_migration(directory.path()).unwrap();
        let prepared = match coordinator.prepare_locked(directory.path(), None, None) {
            Ok(sidecar) => sidecar,
            Err(error) => panic!("迁移准备不应失败：{error}"),
        };
        let summary = match coordinator.finish_commit_locked(directory.path(), prepared, None, None)
        {
            Ok(summary) => summary,
            Err(error) => panic!("迁移提交不应失败：{error}"),
        };

        let sanitized = fs::read_to_string(library_path(directory.path())).unwrap();
        assert!(!sanitized.contains(LEGACY_KEY));
        assert!(!sanitized.contains("apiKey"));
        assert!(!sanitized.contains("apiBaseUrl"));
        assert_eq!(summary.prompts_migrated, 1);
        assert_eq!(summary.favorites_defaulted, 1);
        assert_eq!(summary.orders_rebuilt, 1);

        let connection = Connection::open(directory.path().join("banana.db")).unwrap();
        let credential_ref: String = connection
            .query_row(
                "SELECT credential_ref FROM ai_providers WHERE id = 'reverse-image'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            credentials.get(&credential_ref).unwrap().as_deref(),
            Some(LEGACY_KEY)
        );

        let sidecar = fs::read_to_string(directory.path().join(MIGRATION_SIDECAR_FILE)).unwrap();
        assert!(!sidecar.contains(LEGACY_KEY));
        let backup = fs::read_to_string(&summary.backup_path).unwrap();
        assert!(!backup.contains(LEGACY_KEY));
        let sidecar = read_sidecar(directory.path()).unwrap();
        assert!(!directory
            .path()
            .join(sidecar.original_library_backup_path.unwrap())
            .exists());
    }

    #[test]
    fn recovers_a_known_partial_library_replacement_without_leaving_the_raw_backup() {
        let directory = tempdir().unwrap();
        fs::write(library_path(directory.path()), legacy_library_json()).unwrap();
        let credentials = Arc::new(MemoryCredentialStore::default());
        let coordinator = StartupCoordinator::new(
            credentials,
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        let _migration_lock = lock_migration(directory.path()).unwrap();
        let mut prepared = coordinator
            .prepare_locked(directory.path(), None, None)
            .unwrap();
        prepared.state = MigrationState::Committing;
        let raw_backup = directory
            .path()
            .join(prepared.original_library_backup_path.as_deref().unwrap());
        fs::rename(library_path(directory.path()), &raw_backup).unwrap();
        assert!(fs::read_to_string(&raw_backup)
            .unwrap()
            .contains(LEGACY_KEY));
        write_sidecar_atomic(directory.path(), &prepared).unwrap();
        drop(_migration_lock);

        match coordinator.run(directory.path()) {
            StartupOutcome::Ready { .. } => {}
            StartupOutcome::Recovery(info) => {
                panic!("已知的部分替换状态应自动恢复：{}", info.message)
            }
        }

        let library = fs::read_to_string(library_path(directory.path())).unwrap();
        assert!(!library.contains(LEGACY_KEY));
        assert!(!library.contains("apiKey"));
        assert!(!raw_backup.exists());
        assert!(Connection::open(directory.path().join(DATABASE_FILE)).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn legacy_source_lock_rejects_writes_until_the_commit_finishes() {
        let directory = tempdir().unwrap();
        let legacy_library = legacy_library_json();
        fs::write(library_path(directory.path()), &legacy_library).unwrap();
        let coordinator = StartupCoordinator::new(
            Arc::new(MemoryCredentialStore::default()),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        let _migration_lock = lock_migration(directory.path()).unwrap();
        let mut source_lock = lock_legacy_source(directory.path()).unwrap();
        let prepared = coordinator
            .prepare_locked(directory.path(), Some(&mut source_lock), None)
            .unwrap();

        assert!(fs::write(library_path(directory.path()), "external update").is_err());
        assert_eq!(
            sha256_locked_file(&mut source_lock).unwrap(),
            sha256_bytes(legacy_library.as_bytes())
        );
        coordinator
            .finish_commit_locked(directory.path(), prepared, Some(&mut source_lock), None)
            .unwrap();

        drop(source_lock);
        fs::write(library_path(directory.path()), "external update").unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn commit_refuses_a_library_path_replaced_after_the_source_lock() {
        let directory = tempdir().unwrap();
        fs::write(library_path(directory.path()), legacy_library_json()).unwrap();
        let coordinator = StartupCoordinator::new(
            Arc::new(MemoryCredentialStore::default()),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        let _migration_lock = lock_migration(directory.path()).unwrap();
        let mut source_lock = lock_legacy_source(directory.path()).unwrap();
        let prepared = coordinator
            .prepare_locked(directory.path(), Some(&mut source_lock), None)
            .unwrap();
        let external_replacement = directory.path().join("external-library.json");
        fs::write(&external_replacement, "external update").unwrap();
        fs_atomic::replace_existing_file(&external_replacement, &library_path(directory.path()))
            .unwrap();

        assert!(coordinator
            .finish_commit_locked(directory.path(), prepared, Some(&mut source_lock), None)
            .is_err());

        drop(source_lock);
        assert_eq!(
            fs::read_to_string(library_path(directory.path())).unwrap(),
            "external update"
        );
    }

    #[test]
    fn startup_rechecks_a_complete_sidecar_and_hides_an_acknowledged_summary() {
        let directory = tempdir().unwrap();
        fs::write(library_path(directory.path()), legacy_library_json()).unwrap();
        let coordinator = StartupCoordinator::new(
            Arc::new(MemoryCredentialStore::default()),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        match coordinator.run(directory.path()) {
            StartupOutcome::Ready {
                migration_summary: Some(_),
                ..
            } => {}
            StartupOutcome::Ready { .. } => panic!("新完成的迁移应显示摘要"),
            StartupOutcome::Recovery(info) => panic!("迁移不应进入恢复模式：{}", info.message),
        }

        acknowledge_migration_summary(directory.path()).unwrap();

        match coordinator.run(directory.path()) {
            StartupOutcome::Ready {
                migration_summary: None,
                ..
            } => {}
            StartupOutcome::Ready { .. } => panic!("确认后的摘要不应再次显示"),
            StartupOutcome::Recovery(info) => {
                panic!("确认摘要后不应进入恢复模式：{}", info.message)
            }
        }
    }

    #[test]
    fn complete_migration_keeps_running_after_normal_v1_edits() {
        let directory = tempdir().unwrap();
        fs::write(library_path(directory.path()), legacy_library_json()).unwrap();
        let coordinator = StartupCoordinator::new(
            Arc::new(MemoryCredentialStore::default()),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        let services = match coordinator.run(directory.path()) {
            StartupOutcome::Ready { services, .. } => services,
            StartupOutcome::Recovery(info) => panic!("迁移不应进入恢复模式：{}", info.message),
        };
        acknowledge_migration_summary(directory.path()).unwrap();

        let mut library: serde_json::Value =
            serde_json::from_slice(&fs::read(library_path(directory.path())).unwrap()).unwrap();
        library["settings"]["theme"] = serde_json::Value::String("light".into());
        fs::write(
            library_path(directory.path()),
            serde_json::to_vec_pretty(&library).unwrap(),
        )
        .unwrap();
        services
            .providers
            .clear_credential("reverse-image")
            .unwrap();
        drop(services);

        match coordinator.run(directory.path()) {
            StartupOutcome::Ready {
                migration_summary: None,
                ..
            } => {}
            StartupOutcome::Ready { .. } => panic!("确认后的摘要不应再次显示"),
            StartupOutcome::Recovery(info) => {
                panic!("正常编辑后的 v1 数据不应进入恢复模式：{}", info.message)
            }
        }
    }

    #[test]
    fn complete_migration_rejects_a_v1_library_that_reintroduces_an_api_key() {
        let directory = tempdir().unwrap();
        fs::write(library_path(directory.path()), legacy_library_json()).unwrap();
        let coordinator = StartupCoordinator::new(
            Arc::new(MemoryCredentialStore::default()),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        match coordinator.run(directory.path()) {
            StartupOutcome::Ready { .. } => {}
            StartupOutcome::Recovery(info) => panic!("迁移不应进入恢复模式：{}", info.message),
        }

        let mut library: serde_json::Value =
            serde_json::from_slice(&fs::read(library_path(directory.path())).unwrap()).unwrap();
        library["settings"]["apiKey"] = serde_json::Value::String("reintroduced-key".into());
        fs::write(
            library_path(directory.path()),
            serde_json::to_vec_pretty(&library).unwrap(),
        )
        .unwrap();

        assert!(matches!(
            coordinator.run(directory.path()),
            StartupOutcome::Recovery(_)
        ));
    }

    #[test]
    fn preparing_sidecar_never_deletes_an_active_credential() {
        let directory = tempdir().unwrap();
        initialize_fresh(directory.path()).unwrap();
        let credentials = Arc::new(MemoryCredentialStore::default());
        let origin = "https://ai.leihuo.netease.com";
        let active_ref = credential_reference("reverse-image", origin);
        credentials.set(&active_ref, "active-key").unwrap();

        let database = Database::open(directory.path().join(DATABASE_FILE)).unwrap();
        database
            .with_immediate_transaction(|transaction| {
                transaction
                    .execute(
                        "UPDATE ai_providers
                         SET credential_ref = ?1, needs_credentials = 0
                         WHERE id = 'reverse-image'",
                        [&active_ref],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .unwrap();
        drop(database);

        write_sidecar_atomic(
            directory.path(),
            &MigrationSidecar {
                migration: MIGRATION_VERSION,
                state: MigrationState::Preparing,
                original_library_hash: sha256_file(&library_path(directory.path())).unwrap(),
                temp_library_hash: None,
                temp_database_hash: None,
                backup_path: None,
                original_library_backup_path: Some(format!(
                    "{ORIGINAL_LIBRARY_BACKUP_PREFIX}{}.tmp",
                    "a".repeat(32)
                )),
                candidate_credential_ref: Some(active_ref.clone()),
                credential_origin_fingerprint: Some(origin.into()),
                summary: None,
                summary_acknowledged: false,
            },
        )
        .unwrap();
        let coordinator = StartupCoordinator::new(
            credentials.clone(),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        assert!(matches!(
            coordinator.run(directory.path()),
            StartupOutcome::Recovery(_)
        ));
        assert_eq!(
            credentials.get(&active_ref).unwrap().as_deref(),
            Some("active-key")
        );
    }

    #[test]
    fn every_interruption_recovers_to_one_complete_sanitized_state() {
        for &failpoint in MigrationFailpoint::ALL {
            let directory = tempdir().unwrap();
            fs::write(library_path(directory.path()), legacy_library_json()).unwrap();
            let credentials = Arc::new(MemoryCredentialStore::default());
            let coordinator = StartupCoordinator::new(
                credentials.clone(),
                Arc::new(CredentialMutationCoordinator::default()),
                Arc::new(AppOperationGate::default()),
            );

            assert!(run_with_failpoint(&coordinator, directory.path(), failpoint).is_err());
            let interrupted_candidate = read_sidecar(directory.path())
                .ok()
                .and_then(|sidecar| sidecar.candidate_credential_ref);

            match coordinator.run(directory.path()) {
                StartupOutcome::Ready { .. } => {}
                StartupOutcome::Recovery(info) => {
                    let detail = match coordinator.run_inner(directory.path(), None) {
                        Ok(_) => "recovery retry unexpectedly succeeded".into(),
                        Err(error) => error,
                    };
                    panic!(
                        "{failpoint:?} 后重启不应进入恢复模式：{} ({detail})",
                        info.message
                    )
                }
            }
            let connection = Connection::open(directory.path().join(DATABASE_FILE)).unwrap();
            let final_ref: String = connection
                .query_row(
                    "SELECT credential_ref FROM ai_providers WHERE id = 'reverse-image'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(
                credentials.get(&final_ref).unwrap().as_deref(),
                Some(LEGACY_KEY)
            );
            if let Some(interrupted_candidate) = interrupted_candidate {
                if interrupted_candidate != final_ref {
                    assert_eq!(credentials.get(&interrupted_candidate).unwrap(), None);
                }
            }

            match coordinator.run(directory.path()) {
                StartupOutcome::Ready { .. } => {}
                StartupOutcome::Recovery(info) => {
                    panic!("{failpoint:?} 第二次重启不应进入恢复模式：{}", info.message)
                }
            }
            let restarted_connection =
                Connection::open(directory.path().join(DATABASE_FILE)).unwrap();
            let restarted_ref: String = restarted_connection
                .query_row(
                    "SELECT credential_ref FROM ai_providers WHERE id = 'reverse-image'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(restarted_ref, final_ref);

            for entry in fs::read_dir(directory.path()).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    let contents = fs::read(entry.path()).unwrap();
                    assert!(!contents
                        .windows(LEGACY_KEY.len())
                        .any(|window| window == LEGACY_KEY.as_bytes()));
                    assert!(!contents
                        .windows(b"apiKey".len())
                        .any(|window| window == b"apiKey"));
                }
            }
        }
    }

    #[test]
    fn startup_cleanup_removes_only_an_unreferenced_credential_journal_entry() {
        let directory = tempdir().unwrap();
        let database = Database::open(directory.path().join(DATABASE_FILE)).unwrap();
        let credentials = Arc::new(MemoryCredentialStore::default());
        let stale_ref = credential_reference("reverse-image", "https://stale.example.test");
        credentials.set(&stale_ref, "stale-key").unwrap();
        database
            .with_immediate_transaction(|transaction| {
                transaction
                    .execute(
                        "INSERT INTO credential_cleanup (credential_ref, reason, created_at)
                         VALUES (?1, 'candidate', '2026-07-12T00:00:00Z')",
                        [&stale_ref],
                    )
                    .map_err(|error| error.to_string())?;
                Ok(())
            })
            .unwrap();
        let coordinator = StartupCoordinator::new(
            credentials.clone(),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        coordinator.drain_credential_cleanup(&database);

        assert_eq!(credentials.get(&stale_ref).unwrap(), None);
        let remaining: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM credential_cleanup", [], |row| {
                        row.get(0)
                    })
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn invalid_legacy_endpoint_leaves_the_legacy_file_untouched() {
        let directory = tempdir().unwrap();
        let legacy = legacy_library_json().replace(
            "https://legacy.example.test",
            "https://user:password@legacy.example.test?api_key=not-allowed",
        );
        fs::write(library_path(directory.path()), &legacy).unwrap();
        let coordinator = StartupCoordinator::new(
            Arc::new(MemoryCredentialStore::default()),
            Arc::new(CredentialMutationCoordinator::default()),
            Arc::new(AppOperationGate::default()),
        );

        assert!(matches!(
            coordinator.run(directory.path()),
            StartupOutcome::Recovery(_)
        ));
        assert_eq!(
            fs::read_to_string(library_path(directory.path())).unwrap(),
            legacy
        );
        assert!(!directory.path().join(DATABASE_FILE).exists());
        assert!(!directory.path().join(MIGRATION_SIDECAR_FILE).exists());
    }

    fn legacy_library_json() -> String {
        format!(
            r#"{{
              "version": 1,
              "categories": [],
              "prompts": [{{
                "id": "p1", "title": "旧提示词", "content": "内容",
                "categoryId": null, "tags": [], "image": null,
                "createdAt": 1, "updatedAt": 1
              }}],
              "settings": {{
                "hotkey": "Ctrl+Shift+B", "theme": "auto",
                "apiBaseUrl": "https://legacy.example.test",
                "apiKey": "{LEGACY_KEY}",
                "reverseModel": "legacy-model",
                "availableReverseModels": ["legacy-model"]
              }}
            }}"#
        )
    }
}
