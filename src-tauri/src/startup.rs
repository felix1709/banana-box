use crate::{
    db::{schema, Database},
    fs_atomic,
    library::{library_path, Library, LIBRARY_VERSION},
    migration::{self, MigrationState, MIGRATION_SIDECAR_FILE},
    providers::{self, ProviderKind, SaveProviderInput},
};
use fs2::FileExt;
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};
use uuid::Uuid;

const LIBRARY_FILE: &str = "library.json";
const DATABASE_FILE: &str = "banana.db";
const INIT_SIDECAR_FILE: &str = "init-v1.json";
const STARTUP_LOCK_FILE: &str = "startup-v1.lock";
const INIT_FORMAT: u32 = 1;
const REVERSE_BASE_URL: &str = "https://ai.leihuo.netease.com";
const REVERSE_MODELS_URL: &str = "https://ai.leihuo.netease.com/v1/models";
const REVERSE_CHAT_URL: &str = "https://ai.leihuo.netease.com/v1/chat/completions";
const REVERSE_MODEL: &str = "doubao-seed-1-6-vision-250815";

pub(crate) const STARTUP_CLASSIFICATION_UNAVAILABLE: &str = "STARTUP_CLASSIFICATION_UNAVAILABLE";
pub(crate) const STARTUP_INITIALIZATION_UNAVAILABLE: &str = "STARTUP_INITIALIZATION_UNAVAILABLE";
pub(crate) const STARTUP_INITIALIZATION_RECOVERY_REQUIRED: &str =
    "STARTUP_INITIALIZATION_RECOVERY_REQUIRED";
pub(crate) const STARTUP_LOCK_UNAVAILABLE: &str = "STARTUP_LOCK_UNAVAILABLE";
#[cfg(test)]
const STARTUP_INITIALIZATION_INTERRUPTED: &str = "STARTUP_INITIALIZATION_INTERRUPTED";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPath {
    FreshInstall,
    RecoverInitialization,
    LegacyUpgrade,
    ReadyV1,
    RecoverMigration,
    RecoveryRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryPresence {
    Missing,
    File,
    Other,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum InitializationPhase {
    Prepared,
    LibrarySwitched,
    DbSwitched,
    Complete,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct InitializationFile {
    live_file: String,
    temp_file: String,
    sha256: String,
    old_absent: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct InitializationSidecar {
    format: u32,
    phase: InitializationPhase,
    library: InitializationFile,
    database: InitializationFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArtifactState {
    Staged,
    Switched,
}

struct StartupLock(File);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitializationFailpoint {
    AfterPrepared,
    AfterLibrarySwitched,
    AfterDatabaseSwitched,
    AfterComplete,
}

impl InitializationFailpoint {
    #[cfg(test)]
    const ALL: &'static [Self] = &[
        Self::AfterPrepared,
        Self::AfterLibrarySwitched,
        Self::AfterDatabaseSwitched,
        Self::AfterComplete,
    ];
}

pub fn classify(data_dir: &Path) -> Result<StartupPath, String> {
    let init_sidecar = data_dir.join(INIT_SIDECAR_FILE);
    let migration_sidecar = data_dir.join(MIGRATION_SIDECAR_FILE);
    let init_presence = entry_presence(&init_sidecar)?;
    let migration_presence = entry_presence(&migration_sidecar)?;

    if init_presence != EntryPresence::Missing && migration_presence != EntryPresence::Missing {
        return Ok(StartupPath::RecoveryRequired);
    }
    if init_presence == EntryPresence::Other || migration_presence == EntryPresence::Other {
        return Ok(StartupPath::RecoveryRequired);
    }
    if init_presence == EntryPresence::File {
        return Ok(match read_init_sidecar(data_dir) {
            Ok(_) => StartupPath::RecoverInitialization,
            Err(_) => StartupPath::RecoveryRequired,
        });
    }
    if migration_presence == EntryPresence::File {
        return match migration::migration_sidecar_state(data_dir) {
            Ok(MigrationState::Complete) => match live_file_matrix(data_dir)? {
                (EntryPresence::File, EntryPresence::File) => Ok(classify_ready_v1(data_dir)),
                _ => Ok(StartupPath::RecoveryRequired),
            },
            Ok(_) => Ok(StartupPath::RecoverMigration),
            Err(_) => Ok(StartupPath::RecoveryRequired),
        };
    }

    match live_file_matrix(data_dir)? {
        (EntryPresence::Missing, EntryPresence::Missing) => Ok(StartupPath::FreshInstall),
        (EntryPresence::File, EntryPresence::Missing) => Ok(StartupPath::LegacyUpgrade),
        (EntryPresence::File, EntryPresence::File) => Ok(classify_ready_v1(data_dir)),
        _ => Ok(StartupPath::RecoveryRequired),
    }
}

fn classify_ready_v1(data_dir: &Path) -> StartupPath {
    let library = library_path(data_dir);
    let database = data_dir.join(DATABASE_FILE);
    if verify_ready_library_file(&library, STARTUP_CLASSIFICATION_UNAVAILABLE).is_ok()
        && verify_ready_database_file(&database, STARTUP_CLASSIFICATION_UNAVAILABLE).is_ok()
    {
        StartupPath::ReadyV1
    } else {
        StartupPath::RecoveryRequired
    }
}

pub(crate) fn initialize_fresh(data_dir: &Path) -> Result<(), String> {
    initialize_fresh_inner(data_dir, None)
}

#[cfg(test)]
fn initialize_fresh_with_failpoint(
    data_dir: &Path,
    failpoint: InitializationFailpoint,
) -> Result<(), String> {
    initialize_fresh_inner(data_dir, Some(failpoint))
}

fn initialize_fresh_inner(
    data_dir: &Path,
    failpoint: Option<InitializationFailpoint>,
) -> Result<(), String> {
    let _lock = acquire_startup_lock(data_dir)?;
    if classify(data_dir)? != StartupPath::FreshInstall {
        return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
    }

    let library_temp_name = temporary_name("init-library-");
    let database_temp_name = temporary_name("init-database-");
    let library_temp = data_dir.join(&library_temp_name);
    let database_temp = data_dir.join(&database_temp_name);
    let library_hash = create_library_temp(&library_temp)?;
    let database_hash = create_database_temp(&database_temp)?;
    let sidecar = InitializationSidecar {
        format: INIT_FORMAT,
        phase: InitializationPhase::Prepared,
        library: InitializationFile {
            live_file: LIBRARY_FILE.into(),
            temp_file: library_temp_name,
            sha256: library_hash,
            old_absent: true,
        },
        database: InitializationFile {
            live_file: DATABASE_FILE.into(),
            temp_file: database_temp_name,
            sha256: database_hash,
            old_absent: true,
        },
    };
    write_init_sidecar(data_dir, &sidecar)?;
    interrupt_after(failpoint, InitializationFailpoint::AfterPrepared)?;
    finish_initialization(data_dir, sidecar, failpoint)
}

pub(crate) fn recover_initialization(data_dir: &Path) -> Result<(), String> {
    let _lock = acquire_startup_lock(data_dir)?;
    let sidecar = read_init_sidecar(data_dir)?;
    finish_initialization(data_dir, sidecar, None)
}

fn acquire_startup_lock(data_dir: &Path) -> Result<StartupLock, String> {
    fs::create_dir_all(data_dir).map_err(|_| STARTUP_LOCK_UNAVAILABLE.to_string())?;
    let lock_path = data_dir.join(STARTUP_LOCK_FILE);
    if entry_presence(&lock_path).map_err(|_| STARTUP_LOCK_UNAVAILABLE.to_string())?
        == EntryPresence::Other
    {
        return Err(STARTUP_LOCK_UNAVAILABLE.into());
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(lock_path)
        .map_err(|_| STARTUP_LOCK_UNAVAILABLE.to_string())?;
    file.try_lock_exclusive()
        .map_err(|_| STARTUP_LOCK_UNAVAILABLE.to_string())?;
    Ok(StartupLock(file))
}

impl Drop for StartupLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn finish_initialization(
    data_dir: &Path,
    mut sidecar: InitializationSidecar,
    failpoint: Option<InitializationFailpoint>,
) -> Result<(), String> {
    validate_init_sidecar(&sidecar)?;
    let minimum_progress = required_live_progress(&sidecar.phase);
    let mut progress = observed_progress(data_dir, &sidecar)?;
    if progress < minimum_progress {
        return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
    }

    if progress == 0 {
        switch_artifact(data_dir, &sidecar.library)?;
        progress = 1;
    }
    if phase_progress(&sidecar.phase) < 1 {
        sidecar.phase = InitializationPhase::LibrarySwitched;
        write_init_sidecar(data_dir, &sidecar)?;
        interrupt_after(failpoint, InitializationFailpoint::AfterLibrarySwitched)?;
    }

    if progress == 1 {
        switch_artifact(data_dir, &sidecar.database)?;
        progress = 2;
    }
    if phase_progress(&sidecar.phase) < 2 {
        sidecar.phase = InitializationPhase::DbSwitched;
        write_init_sidecar(data_dir, &sidecar)?;
        interrupt_after(failpoint, InitializationFailpoint::AfterDatabaseSwitched)?;
    }

    if progress != 2 {
        return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
    }
    verify_live_tuple(data_dir, &sidecar)?;
    if phase_progress(&sidecar.phase) < 3 {
        sidecar.phase = InitializationPhase::Complete;
        write_init_sidecar(data_dir, &sidecar)?;
        interrupt_after(failpoint, InitializationFailpoint::AfterComplete)?;
    }

    verify_live_tuple(data_dir, &sidecar)?;
    remove_sqlite_sidecars(
        &data_dir.join(&sidecar.database.live_file),
        STARTUP_INITIALIZATION_RECOVERY_REQUIRED,
    )?;
    remove_verified_temp_files(data_dir, &sidecar)?;
    remove_verified_sidecar(data_dir, &sidecar)
}

fn entry_presence(path: &Path) -> Result<EntryPresence, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(EntryPresence::File),
        Ok(_) => Ok(EntryPresence::Other),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(EntryPresence::Missing),
        Err(_) => Err(STARTUP_CLASSIFICATION_UNAVAILABLE.into()),
    }
}

fn live_file_matrix(data_dir: &Path) -> Result<(EntryPresence, EntryPresence), String> {
    Ok((
        entry_presence(&library_path(data_dir))?,
        entry_presence(&data_dir.join(DATABASE_FILE))?,
    ))
}

fn read_init_sidecar(data_dir: &Path) -> Result<InitializationSidecar, String> {
    let bytes = fs::read(data_dir.join(INIT_SIDECAR_FILE))
        .map_err(|_| STARTUP_INITIALIZATION_RECOVERY_REQUIRED.to_string())?;
    let sidecar: InitializationSidecar = serde_json::from_slice(&bytes)
        .map_err(|_| STARTUP_INITIALIZATION_RECOVERY_REQUIRED.to_string())?;
    validate_init_sidecar(&sidecar)?;
    Ok(sidecar)
}

fn validate_init_sidecar(sidecar: &InitializationSidecar) -> Result<(), String> {
    if sidecar.format != INIT_FORMAT
        || !sidecar.library.old_absent
        || !sidecar.database.old_absent
        || sidecar.library.live_file != LIBRARY_FILE
        || sidecar.database.live_file != DATABASE_FILE
        || !is_init_temp_name(&sidecar.library.temp_file, "init-library-")
        || !is_init_temp_name(&sidecar.database.temp_file, "init-database-")
        || !is_lower_sha256(&sidecar.library.sha256)
        || !is_lower_sha256(&sidecar.database.sha256)
    {
        return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
    }
    Ok(())
}

fn is_init_temp_name(value: &str, prefix: &str) -> bool {
    let path = Path::new(value);
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if file_name != value {
        return false;
    }
    let Some(identifier) = value
        .strip_prefix(prefix)
        .and_then(|value| value.strip_suffix(".tmp"))
    else {
        return false;
    };
    identifier.len() == 32
        && identifier
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
}

fn temporary_name(prefix: &str) -> String {
    format!("{prefix}{}.tmp", Uuid::new_v4().simple())
}

fn create_library_temp(path: &Path) -> Result<String, String> {
    let library = Library::default();
    if library.version != LIBRARY_VERSION {
        return Err(STARTUP_INITIALIZATION_UNAVAILABLE.into());
    }
    let bytes = serde_json::to_vec_pretty(&library)
        .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
    write_new_synced(path, &bytes)?;
    verify_library_file(path, STARTUP_INITIALIZATION_UNAVAILABLE)?;
    sha256_file(path, STARTUP_INITIALIZATION_UNAVAILABLE)
}

fn create_database_temp(path: &Path) -> Result<String, String> {
    write_new_synced(path, b"")?;
    let reverse_input = seeded_reverse_provider_input();
    let bound_host = providers::validated_host_fingerprint(&reverse_input)
        .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
    if bound_host != REVERSE_BASE_URL {
        return Err(STARTUP_INITIALIZATION_UNAVAILABLE.into());
    }

    {
        let database =
            Database::open(path).map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
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
                            ?1, 'reverse-image', '图片反推', ?2, ?3, ?4,
                            ?5, '[]', NULL, NULL, NULL, ?6, 1, NULL,
                            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                         )",
                        params![
                            "reverse-image",
                            REVERSE_BASE_URL,
                            REVERSE_MODELS_URL,
                            REVERSE_CHAT_URL,
                            REVERSE_MODEL,
                            bound_host,
                        ],
                    )
                    .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
                let storyboard_rows = transaction
                    .execute(
                        "INSERT INTO ai_providers (
                            id, kind, display_name, base_url, models_url, chat_completions_url,
                            default_model, available_models_json, probed_model, structured_mode,
                            interactive_compatible, bound_host, needs_credentials, credential_ref,
                            created_at, updated_at
                         ) VALUES (
                            'storyboard', 'storyboard', '故事板 Agent', '', '', '',
                            NULL, '[]', NULL, NULL, NULL, NULL, 1, NULL,
                            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                         )",
                        [],
                    )
                    .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
                if reverse_rows != 1 || storyboard_rows != 1 {
                    return Err(STARTUP_INITIALIZATION_UNAVAILABLE.into());
                }
                Ok(())
            })
            .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
        verify_seeded_database(&database, STARTUP_INITIALIZATION_UNAVAILABLE)?;
        checkpoint_database(&database, STARTUP_INITIALIZATION_UNAVAILABLE)?;
    }
    remove_sqlite_sidecars(path, STARTUP_INITIALIZATION_UNAVAILABLE)?;
    sync_existing_file(path, STARTUP_INITIALIZATION_UNAVAILABLE)?;
    sha256_file(path, STARTUP_INITIALIZATION_UNAVAILABLE)
}

fn seeded_reverse_provider_input() -> SaveProviderInput {
    SaveProviderInput {
        id: "reverse-image".into(),
        kind: ProviderKind::ReverseImage,
        display_name: "图片反推".into(),
        base_url: REVERSE_BASE_URL.into(),
        models_url: REVERSE_MODELS_URL.into(),
        chat_completions_url: REVERSE_CHAT_URL.into(),
        default_model: Some(REVERSE_MODEL.into()),
        temperature: None,
        context_window_tokens: None,
        confirm_cross_origin: false,
    }
}

fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
    file.write_all(bytes)
        .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
    file.sync_all()
        .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())
}

fn write_init_sidecar(data_dir: &Path, sidecar: &InitializationSidecar) -> Result<(), String> {
    validate_init_sidecar(sidecar)?;
    let sidecar_path = data_dir.join(INIT_SIDECAR_FILE);
    let temporary_path = data_dir.join(temporary_name("init-sidecar-"));
    let bytes =
        serde_json::to_vec(sidecar).map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
    write_new_synced(&temporary_path, &bytes)?;
    fs_atomic::replace_file(&temporary_path, &sidecar_path)
        .map_err(|_| STARTUP_INITIALIZATION_UNAVAILABLE.to_string())?;
    sync_existing_file(&sidecar_path, STARTUP_INITIALIZATION_UNAVAILABLE)
}

fn observed_progress(data_dir: &Path, sidecar: &InitializationSidecar) -> Result<u8, String> {
    let library = artifact_state(data_dir, &sidecar.library)?;
    let database = artifact_state(data_dir, &sidecar.database)?;
    match (library, database) {
        (ArtifactState::Staged, ArtifactState::Staged) => Ok(0),
        (ArtifactState::Switched, ArtifactState::Staged) => Ok(1),
        (ArtifactState::Switched, ArtifactState::Switched) => Ok(2),
        (ArtifactState::Staged, ArtifactState::Switched) => {
            Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into())
        }
    }
}

fn artifact_state(data_dir: &Path, artifact: &InitializationFile) -> Result<ArtifactState, String> {
    let live = data_dir.join(&artifact.live_file);
    let temporary = data_dir.join(&artifact.temp_file);
    let live_matches = hash_matches(&live, &artifact.sha256)?;
    let temp_matches = hash_matches(&temporary, &artifact.sha256)?;
    match (live_matches, temp_matches) {
        (false, true) => Ok(ArtifactState::Staged),
        (true, false) | (true, true) => Ok(ArtifactState::Switched),
        (false, false) => Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into()),
    }
}

fn hash_matches(path: &Path, expected: &str) -> Result<bool, String> {
    match entry_presence(path).map_err(|_| STARTUP_INITIALIZATION_RECOVERY_REQUIRED.to_string())? {
        EntryPresence::Missing => Ok(false),
        EntryPresence::Other => Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into()),
        EntryPresence::File => {
            if sha256_file(path, STARTUP_INITIALIZATION_RECOVERY_REQUIRED)? == expected {
                Ok(true)
            } else {
                Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into())
            }
        }
    }
}

fn switch_artifact(data_dir: &Path, artifact: &InitializationFile) -> Result<(), String> {
    let live = data_dir.join(&artifact.live_file);
    let temporary = data_dir.join(&artifact.temp_file);
    match artifact_state(data_dir, artifact)? {
        ArtifactState::Switched => return Ok(()),
        ArtifactState::Staged => {}
    }
    if entry_presence(&live).map_err(|_| STARTUP_INITIALIZATION_RECOVERY_REQUIRED.to_string())?
        != EntryPresence::Missing
    {
        return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
    }
    fs_atomic::replace_file(&temporary, &live)
        .map_err(|_| STARTUP_INITIALIZATION_RECOVERY_REQUIRED.to_string())?;
    sync_existing_file(&live, STARTUP_INITIALIZATION_RECOVERY_REQUIRED)?;
    if !hash_matches(&live, &artifact.sha256)? {
        return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
    }
    Ok(())
}

fn phase_progress(phase: &InitializationPhase) -> u8 {
    match phase {
        InitializationPhase::Prepared => 0,
        InitializationPhase::LibrarySwitched => 1,
        InitializationPhase::DbSwitched => 2,
        InitializationPhase::Complete => 3,
    }
}

fn required_live_progress(phase: &InitializationPhase) -> u8 {
    match phase {
        InitializationPhase::Prepared => 0,
        InitializationPhase::LibrarySwitched => 1,
        InitializationPhase::DbSwitched | InitializationPhase::Complete => 2,
    }
}

fn verify_live_tuple(data_dir: &Path, sidecar: &InitializationSidecar) -> Result<(), String> {
    let library_path = data_dir.join(&sidecar.library.live_file);
    let database_path = data_dir.join(&sidecar.database.live_file);
    if !hash_matches(&library_path, &sidecar.library.sha256)?
        || !hash_matches(&database_path, &sidecar.database.sha256)?
    {
        return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
    }
    verify_library_file(&library_path, STARTUP_INITIALIZATION_RECOVERY_REQUIRED)?;
    verify_database_file(&database_path, STARTUP_INITIALIZATION_RECOVERY_REQUIRED)
}

fn verify_library_file(path: &Path, error_code: &str) -> Result<(), String> {
    let library = read_ready_library_file(path, error_code)?;
    if library != Library::default() {
        return Err(error_code.into());
    }
    Ok(())
}

fn verify_ready_library_file(path: &Path, error_code: &str) -> Result<(), String> {
    read_ready_library_file(path, error_code).map(|_| ())
}

fn read_ready_library_file(path: &Path, error_code: &str) -> Result<Library, String> {
    let bytes = fs::read(path).map_err(|_| error_code.to_string())?;
    let library: Library = serde_json::from_slice(&bytes).map_err(|_| error_code.to_string())?;
    if library.version != LIBRARY_VERSION {
        return Err(error_code.into());
    }
    Ok(library)
}

fn verify_database_file(path: &Path, error_code: &str) -> Result<(), String> {
    let connection = open_readonly_database(path, error_code)?;
    verify_seeded_connection(&connection, error_code)
}

fn verify_ready_database_file(path: &Path, error_code: &str) -> Result<(), String> {
    let connection = open_readonly_database(path, error_code)?;
    verify_ready_connection(&connection, error_code)
}

fn open_readonly_database(path: &Path, error_code: &str) -> Result<Connection, String> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| error_code.to_string())
}

fn verify_seeded_database(database: &Database, error_code: &str) -> Result<(), String> {
    database
        .with_connection(|connection| verify_seeded_connection(connection, error_code))
        .map_err(|_| error_code.to_string())
}

fn verify_seeded_connection(connection: &Connection, error_code: &str) -> Result<(), String> {
    verify_ready_connection(connection, error_code)?;
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM ai_providers", [], |row| row.get(0))
        .map_err(|_| error_code.to_string())?;
    if count != 2 {
        return Err(error_code.into());
    }
    Ok(())
}

fn verify_ready_connection(connection: &Connection, error_code: &str) -> Result<(), String> {
    schema::validate_migratable(connection).map_err(|_| error_code.to_string())?;
    let reverse_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_providers WHERE id = 'reverse-image' AND kind = 'reverse-image'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| error_code.to_string())?;
    let storyboard_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM ai_providers WHERE id = 'storyboard' AND kind = 'storyboard'",
            [],
            |row| row.get(0),
        )
        .map_err(|_| error_code.to_string())?;
    if reverse_count != 1 || storyboard_count != 1 {
        return Err(error_code.into());
    }
    Ok(())
}

fn checkpoint_database(database: &Database, error_code: &str) -> Result<(), String> {
    database
        .with_connection(|connection| {
            connection
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|_| error_code.to_string())
        })
        .map_err(|_| error_code.to_string())
}

fn remove_sqlite_sidecars(path: &Path, error_code: &str) -> Result<(), String> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| error_code.to_string())?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = path.with_file_name(format!("{file_name}{suffix}"));
        match entry_presence(&sidecar).map_err(|_| error_code.to_string())? {
            EntryPresence::Missing => {}
            EntryPresence::Other => return Err(error_code.into()),
            EntryPresence::File => {
                if suffix == "-wal"
                    && fs::metadata(&sidecar)
                        .map_err(|_| error_code.to_string())?
                        .len()
                        != 0
                {
                    return Err(error_code.into());
                }
                fs::remove_file(&sidecar).map_err(|_| error_code.to_string())?;
            }
        }
    }
    Ok(())
}

fn sha256_file(path: &Path, error_code: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|_| error_code.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn sync_existing_file(path: &Path, error_code: &str) -> Result<(), String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| error_code.to_string())
}

fn remove_verified_temp_files(
    data_dir: &Path,
    sidecar: &InitializationSidecar,
) -> Result<(), String> {
    for artifact in [&sidecar.library, &sidecar.database] {
        let temporary = data_dir.join(&artifact.temp_file);
        match entry_presence(&temporary)
            .map_err(|_| STARTUP_INITIALIZATION_RECOVERY_REQUIRED.to_string())?
        {
            EntryPresence::Missing => {}
            EntryPresence::Other => return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into()),
            EntryPresence::File => {
                if !hash_matches(&temporary, &artifact.sha256)? {
                    return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
                }
                fs::remove_file(&temporary)
                    .map_err(|_| STARTUP_INITIALIZATION_RECOVERY_REQUIRED.to_string())?;
            }
        }
    }
    Ok(())
}

fn remove_verified_sidecar(
    data_dir: &Path,
    expected: &InitializationSidecar,
) -> Result<(), String> {
    let current = read_init_sidecar(data_dir)?;
    if current != *expected || current.phase != InitializationPhase::Complete {
        return Err(STARTUP_INITIALIZATION_RECOVERY_REQUIRED.into());
    }
    fs::remove_file(data_dir.join(INIT_SIDECAR_FILE))
        .map_err(|_| STARTUP_INITIALIZATION_RECOVERY_REQUIRED.to_string())
}

#[cfg(test)]
fn interrupt_after(
    failpoint: Option<InitializationFailpoint>,
    expected: InitializationFailpoint,
) -> Result<(), String> {
    if failpoint == Some(expected) {
        return Err(STARTUP_INITIALIZATION_INTERRUPTED.into());
    }
    Ok(())
}

#[cfg(not(test))]
fn interrupt_after(
    _failpoint: Option<InitializationFailpoint>,
    _expected: InitializationFailpoint,
) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::Database,
        library::{library_path, load_library_strict, Library},
        provider_http::ProviderHttpClient,
        providers::{ProviderKind, ProviderService, SaveProviderInput},
        secrets::{CredentialMutationCoordinator, MemoryCredentialStore},
    };
    use rusqlite::Connection;
    use std::{fs, sync::Arc};
    use tempfile::tempdir;

    const REVERSE_BASE_URL: &str = "https://ai.leihuo.netease.com";
    const REVERSE_MODELS_URL: &str = "https://ai.leihuo.netease.com/v1/models";
    const REVERSE_CHAT_URL: &str = "https://ai.leihuo.netease.com/v1/chat/completions";
    const REVERSE_MODEL: &str = "doubao-seed-1-6-vision-250815";

    #[test]
    fn classifies_the_complete_startup_matrix() {
        let fresh = tempdir().unwrap();
        assert_eq!(classify(fresh.path()).unwrap(), StartupPath::FreshInstall);

        let initializing = tempdir().unwrap();
        fs::write(
            initializing.path().join("init-v1.json"),
            valid_init_sidecar_json("prepared"),
        )
        .unwrap();
        assert_eq!(
            classify(initializing.path()).unwrap(),
            StartupPath::RecoverInitialization
        );

        let legacy = tempdir().unwrap();
        fs::write(
            library_path(legacy.path()),
            serde_json::to_vec(&Library::default()).unwrap(),
        )
        .unwrap();
        assert_eq!(classify(legacy.path()).unwrap(), StartupPath::LegacyUpgrade);

        let ready = tempdir().unwrap();
        initialize_fresh(ready.path()).unwrap();
        assert_eq!(classify(ready.path()).unwrap(), StartupPath::ReadyV1);

        let migrating = tempdir().unwrap();
        fs::write(
            migrating.path().join("migration-v1.json"),
            valid_migration_sidecar_json("preparing"),
        )
        .unwrap();
        assert_eq!(
            classify(migrating.path()).unwrap(),
            StartupPath::RecoverMigration
        );

        let completed_migration = tempdir().unwrap();
        initialize_fresh(completed_migration.path()).unwrap();
        fs::write(
            completed_migration.path().join("migration-v1.json"),
            valid_migration_sidecar_json("complete"),
        )
        .unwrap();
        assert_eq!(
            classify(completed_migration.path()).unwrap(),
            StartupPath::ReadyV1
        );

        let orphan_database = tempdir().unwrap();
        fs::write(orphan_database.path().join("banana.db"), b"not a database").unwrap();
        assert_eq!(
            classify(orphan_database.path()).unwrap(),
            StartupPath::RecoveryRequired
        );

        let malformed_sidecar = tempdir().unwrap();
        fs::write(malformed_sidecar.path().join("init-v1.json"), b"{not JSON").unwrap();
        assert_eq!(
            classify(malformed_sidecar.path()).unwrap(),
            StartupPath::RecoveryRequired
        );

        let conflicting_sidecars = tempdir().unwrap();
        fs::write(
            conflicting_sidecars.path().join("init-v1.json"),
            valid_init_sidecar_json("prepared"),
        )
        .unwrap();
        fs::write(
            conflicting_sidecars.path().join("migration-v1.json"),
            valid_migration_sidecar_json("preparing"),
        )
        .unwrap();
        assert_eq!(
            classify(conflicting_sidecars.path()).unwrap(),
            StartupPath::RecoveryRequired
        );
    }

    #[test]
    fn valid_v1_database_remains_eligible_for_schema_migration() {
        let directory = tempdir().unwrap();
        fs::write(
            library_path(directory.path()),
            serde_json::to_vec(&Library::default()).unwrap(),
        )
        .unwrap();

        let database_path = directory.path().join(DATABASE_FILE);
        let connection = Connection::open(&database_path).unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_v1.sql"))
            .unwrap();
        connection
            .execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (1, '2026-07-12T00:00:00Z')",
                [],
            )
            .unwrap();
        connection
            .pragma_update(None, "user_version", 1_i64)
            .unwrap();
        for (id, kind) in [
            ("reverse-image", "reverse-image"),
            ("storyboard", "storyboard"),
        ] {
            connection
                .execute(
                    "INSERT INTO ai_providers
                     (id, kind, display_name, base_url, models_url, chat_completions_url,
                      available_models_json, created_at, updated_at)
                     VALUES (?1, ?2, 'Provider', 'https://example.test', 'https://example.test/models',
                             'https://example.test/chat', '[]', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z')",
                    [id, kind],
                )
                .unwrap();
        }
        drop(connection);

        assert_eq!(classify(directory.path()).unwrap(), StartupPath::ReadyV1);
        Database::open(&database_path).unwrap();
    }

    #[test]
    fn invalid_or_unsupported_sidecars_fail_closed_without_raw_errors() {
        let directory = tempdir().unwrap();
        fs::write(
            directory.path().join("init-v1.json"),
            valid_init_sidecar_json("unexpected_phase"),
        )
        .unwrap();

        assert_eq!(
            classify(directory.path()).unwrap(),
            StartupPath::RecoveryRequired
        );

        fs::remove_file(directory.path().join("init-v1.json")).unwrap();
        fs::write(
            directory.path().join("migration-v1.json"),
            r#"{"migration":99,"state":"prepared"}"#,
        )
        .unwrap();
        assert_eq!(
            classify(directory.path()).unwrap(),
            StartupPath::RecoveryRequired
        );

        let forged_complete = tempdir().unwrap();
        initialize_fresh(forged_complete.path()).unwrap();
        fs::write(
            forged_complete.path().join("migration-v1.json"),
            r#"{"migration":1,"state":"complete"}"#,
        )
        .unwrap();
        assert_eq!(
            classify(forged_complete.path()).unwrap(),
            StartupPath::RecoveryRequired
        );
    }

    #[test]
    fn startup_lock_blocks_initialization_and_recovery_without_file_changes() {
        let fresh = tempdir().unwrap();
        let _fresh_lock = acquire_startup_lock(fresh.path()).unwrap();
        let fresh_before = file_snapshot(fresh.path());

        assert_eq!(
            initialize_fresh(fresh.path()).unwrap_err(),
            STARTUP_LOCK_UNAVAILABLE
        );
        assert_eq!(file_snapshot(fresh.path()), fresh_before);

        let recovering = tempdir().unwrap();
        assert_eq!(
            initialize_fresh_with_failpoint(
                recovering.path(),
                InitializationFailpoint::AfterPrepared,
            )
            .unwrap_err(),
            STARTUP_INITIALIZATION_INTERRUPTED
        );
        let _recovering_lock = acquire_startup_lock(recovering.path()).unwrap();
        let recovering_before = file_snapshot(recovering.path());
        let sidecar_before = fs::read(recovering.path().join(INIT_SIDECAR_FILE)).unwrap();

        assert_eq!(
            recover_initialization(recovering.path()).unwrap_err(),
            STARTUP_LOCK_UNAVAILABLE
        );
        assert_eq!(file_snapshot(recovering.path()), recovering_before);
        assert_eq!(
            fs::read(recovering.path().join(INIT_SIDECAR_FILE)).unwrap(),
            sidecar_before
        );
    }

    #[test]
    fn initialization_sidecar_rejects_unknown_fields_and_free_paths() {
        let directory = tempdir().unwrap();
        let unknown_field = format!(
            "{},\"api_key\":\"not-a-secret\"}}",
            valid_init_sidecar_json("prepared").trim_end_matches('}')
        );
        fs::write(directory.path().join("init-v1.json"), unknown_field).unwrap();
        assert_eq!(
            classify(directory.path()).unwrap(),
            StartupPath::RecoveryRequired
        );

        let unsafe_path = valid_init_sidecar_json("prepared").replace(
            "init-library-0123456789abcdef0123456789abcdef.tmp",
            "../library.json",
        );
        fs::write(directory.path().join("init-v1.json"), unsafe_path).unwrap();
        assert_eq!(
            classify(directory.path()).unwrap(),
            StartupPath::RecoveryRequired
        );
    }

    #[test]
    fn ready_pair_with_invalid_or_unsupported_data_requires_recovery() {
        let malformed_library = tempdir().unwrap();
        initialize_fresh(malformed_library.path()).unwrap();
        fs::write(library_path(malformed_library.path()), b"{not JSON").unwrap();
        assert_eq!(
            classify(malformed_library.path()).unwrap(),
            StartupPath::RecoveryRequired
        );

        let unsupported_library = tempdir().unwrap();
        initialize_fresh(unsupported_library.path()).unwrap();
        let mut library: serde_json::Value =
            serde_json::from_slice(&fs::read(library_path(unsupported_library.path())).unwrap())
                .unwrap();
        library["version"] = serde_json::Value::from(999);
        fs::write(
            library_path(unsupported_library.path()),
            serde_json::to_vec(&library).unwrap(),
        )
        .unwrap();
        assert_eq!(
            classify(unsupported_library.path()).unwrap(),
            StartupPath::RecoveryRequired
        );

        let malformed_database = tempdir().unwrap();
        initialize_fresh(malformed_database.path()).unwrap();
        fs::write(
            malformed_database.path().join("banana.db"),
            b"not a database",
        )
        .unwrap();
        assert_eq!(
            classify(malformed_database.path()).unwrap(),
            StartupPath::RecoveryRequired
        );
    }

    #[test]
    fn fresh_initialization_writes_readable_library_and_exact_provider_seeds() {
        let directory = tempdir().unwrap();

        initialize_fresh(directory.path()).unwrap();

        assert_eq!(
            load_library_strict(directory.path()).unwrap(),
            Library::default()
        );
        assert!(!directory.path().join("banana.db-wal").exists());
        assert!(!directory.path().join("banana.db-shm").exists());
        let database = Database::open(directory.path().join("banana.db")).unwrap();
        database
            .with_connection(|connection| {
                crate::db::schema::validate(connection).map_err(|_| "TEST_SCHEMA".to_string())?;
                let count: i64 = connection
                    .query_row("SELECT COUNT(*) FROM ai_providers", [], |row| row.get(0))
                    .map_err(|_| "TEST_QUERY".to_string())?;
                assert_eq!(count, 2);

                let reverse: (
                    String,
                    String,
                    String,
                    String,
                    String,
                    Option<String>,
                    String,
                    Option<String>,
                    i64,
                    Option<String>,
                ) = connection
                    .query_row(
                        "SELECT id, kind, display_name, base_url, models_url, chat_completions_url, default_model, bound_host, needs_credentials, credential_ref FROM ai_providers WHERE id = 'reverse-image'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
                    )
                    .map_err(|_| "TEST_QUERY".to_string())?;
                assert_eq!(reverse.0, "reverse-image");
                assert_eq!(reverse.1, "reverse-image");
                assert_eq!(reverse.2, "图片反推");
                assert_eq!(reverse.3, REVERSE_BASE_URL);
                assert_eq!(reverse.4, REVERSE_MODELS_URL);
                assert_eq!(reverse.5.as_deref(), Some(REVERSE_CHAT_URL));
                assert_eq!(reverse.6, REVERSE_MODEL);
                assert_eq!(reverse.7.as_deref(), Some(REVERSE_BASE_URL));
                assert_eq!(reverse.8, 1);
                assert_eq!(reverse.9, None);

                let reverse_metadata: (
                    String,
                    Option<String>,
                    Option<String>,
                    Option<i64>,
                    i64,
                    i64,
                ) = connection
                    .query_row(
                        "SELECT available_models_json, probed_model, structured_mode, interactive_compatible, config_revision, capability_revision FROM ai_providers WHERE id = 'reverse-image'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                    )
                    .map_err(|_| "TEST_QUERY".to_string())?;
                assert_eq!(reverse_metadata.0, "[]");
                assert_eq!(reverse_metadata.1, None);
                assert_eq!(reverse_metadata.2, None);
                assert_eq!(reverse_metadata.3, None);
                assert_eq!(reverse_metadata.4, 1);
                assert_eq!(reverse_metadata.5, 1);

                let storyboard: (String, String, String, String, String, String, Option<String>, Option<String>, i64, Option<String>) = connection
                    .query_row(
                        "SELECT id, kind, display_name, base_url, models_url, chat_completions_url, default_model, bound_host, needs_credentials, credential_ref FROM ai_providers WHERE id = 'storyboard'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
                    )
                    .map_err(|_| "TEST_QUERY".to_string())?;
                assert_eq!(storyboard.0, "storyboard");
                assert_eq!(storyboard.1, "storyboard");
                assert_eq!(storyboard.2, "故事板 Agent");
                assert_eq!(storyboard.3, "");
                assert_eq!(storyboard.4, "");
                assert_eq!(storyboard.5, "");
                assert_eq!(storyboard.6, None);
                assert_eq!(storyboard.7, None);
                assert_eq!(storyboard.8, 1);
                assert_eq!(storyboard.9, None);

                let storyboard_metadata: (
                    String,
                    Option<String>,
                    Option<String>,
                    Option<i64>,
                    i64,
                    i64,
                ) = connection
                    .query_row(
                        "SELECT available_models_json, probed_model, structured_mode, interactive_compatible, config_revision, capability_revision FROM ai_providers WHERE id = 'storyboard'",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
                    )
                    .map_err(|_| "TEST_QUERY".to_string())?;
                assert_eq!(storyboard_metadata.0, "[]");
                assert_eq!(storyboard_metadata.1, None);
                assert_eq!(storyboard_metadata.2, None);
                assert_eq!(storyboard_metadata.3, None);
                assert_eq!(storyboard_metadata.4, 1);
                assert_eq!(storyboard_metadata.5, 1);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn saving_an_unchanged_reverse_provider_preserves_the_bound_credential() {
        let directory = tempdir().unwrap();
        initialize_fresh(directory.path()).unwrap();
        let database = Arc::new(Database::open(directory.path().join("banana.db")).unwrap());
        let service = ProviderService::new(
            Arc::clone(&database),
            Arc::new(MemoryCredentialStore::default()),
            Arc::new(ProviderHttpClient::new().unwrap()),
            Arc::new(CredentialMutationCoordinator::default()),
        );
        let input = reverse_provider_input();

        service.save(input.clone(), Some("test-key")).unwrap();
        let before = provider_binding(&database, "reverse-image");
        service.save(input, None).unwrap();
        let after = provider_binding(&database, "reverse-image");

        assert_eq!(after.0, REVERSE_BASE_URL);
        assert_eq!(after.0, before.0);
        assert_eq!(after.1, before.1);
        assert_eq!(after.2, before.2);
        assert_eq!(after.3, 0);
        assert!(after.1.is_some());
    }

    #[test]
    fn initialization_recovers_every_persisted_phase_idempotently() {
        for failpoint in InitializationFailpoint::ALL {
            let directory = tempdir().unwrap();
            let error = initialize_fresh_with_failpoint(directory.path(), *failpoint).unwrap_err();
            assert_eq!(error, STARTUP_INITIALIZATION_INTERRUPTED);
            assert_eq!(
                classify(directory.path()).unwrap(),
                StartupPath::RecoverInitialization
            );

            recover_initialization(directory.path()).unwrap();
            assert_eq!(classify(directory.path()).unwrap(), StartupPath::ReadyV1);
            assert_eq!(classify(directory.path()).unwrap(), StartupPath::ReadyV1);
            assert_seed_count(directory.path(), 2);
        }
    }

    #[test]
    fn recovery_accepts_a_verified_switch_before_its_phase_is_persisted() {
        let library_window = tempdir().unwrap();
        initialize_fresh_with_failpoint(
            library_window.path(),
            InitializationFailpoint::AfterPrepared,
        )
        .unwrap_err();
        let prepared = read_init_sidecar(library_window.path()).unwrap();
        let library_temp = library_window.path().join(&prepared.library.temp_file);
        let library_live = library_window.path().join(&prepared.library.live_file);
        crate::fs_atomic::replace_file(&library_temp, &library_live).unwrap();
        sync_existing_file(&library_live, STARTUP_INITIALIZATION_RECOVERY_REQUIRED).unwrap();
        recover_initialization(library_window.path()).unwrap();
        assert_eq!(
            classify(library_window.path()).unwrap(),
            StartupPath::ReadyV1
        );

        let database_window = tempdir().unwrap();
        initialize_fresh_with_failpoint(
            database_window.path(),
            InitializationFailpoint::AfterLibrarySwitched,
        )
        .unwrap_err();
        let library_switched = read_init_sidecar(database_window.path()).unwrap();
        let database_temp = database_window
            .path()
            .join(&library_switched.database.temp_file);
        let database_live = database_window
            .path()
            .join(&library_switched.database.live_file);
        crate::fs_atomic::replace_file(&database_temp, &database_live).unwrap();
        sync_existing_file(&database_live, STARTUP_INITIALIZATION_RECOVERY_REQUIRED).unwrap();
        recover_initialization(database_window.path()).unwrap();
        assert_eq!(
            classify(database_window.path()).unwrap(),
            StartupPath::ReadyV1
        );
    }

    #[test]
    fn recovery_rejects_a_hash_mismatch_without_overwriting_live_paths() {
        let directory = tempdir().unwrap();
        let error = initialize_fresh_with_failpoint(
            directory.path(),
            InitializationFailpoint::AfterPrepared,
        )
        .unwrap_err();
        assert_eq!(error, STARTUP_INITIALIZATION_INTERRUPTED);

        let record = read_init_sidecar(directory.path()).unwrap();
        fs::write(directory.path().join(record.library.temp_file), b"tampered").unwrap();

        assert_eq!(
            recover_initialization(directory.path()).unwrap_err(),
            STARTUP_INITIALIZATION_RECOVERY_REQUIRED
        );
        assert!(!library_path(directory.path()).exists());
        assert!(!directory.path().join("banana.db").exists());
        assert!(directory.path().join("init-v1.json").exists());
    }

    #[test]
    fn live_database_verification_preserves_the_recorded_main_file_hash() {
        let directory = tempdir().unwrap();
        initialize_fresh_with_failpoint(
            directory.path(),
            InitializationFailpoint::AfterDatabaseSwitched,
        )
        .unwrap_err();
        let record = read_init_sidecar(directory.path()).unwrap();
        let database = directory.path().join(&record.database.live_file);
        let before = sha256_file(&database, STARTUP_INITIALIZATION_RECOVERY_REQUIRED).unwrap();

        verify_database_file(&database, STARTUP_INITIALIZATION_RECOVERY_REQUIRED).unwrap();

        assert_eq!(
            sha256_file(&database, STARTUP_INITIALIZATION_RECOVERY_REQUIRED).unwrap(),
            before
        );
    }

    fn valid_init_sidecar_json(phase: &str) -> String {
        format!(
            r#"{{"format":1,"phase":"{phase}","library":{{"live_file":"library.json","temp_file":"init-library-0123456789abcdef0123456789abcdef.tmp","sha256":"{}","old_absent":true}},"database":{{"live_file":"banana.db","temp_file":"init-database-fedcba9876543210fedcba9876543210.tmp","sha256":"{}","old_absent":true}}}}"#,
            "a".repeat(64),
            "b".repeat(64),
        )
    }

    fn valid_migration_sidecar_json(state: &str) -> String {
        let original_hash = "a".repeat(64);
        let original_backup = format!(".migration-original-library-{}.tmp", "c".repeat(32));
        if state == "preparing" {
            return format!(
                r#"{{"migration":1,"state":"preparing","original_library_hash":"{original_hash}","temp_library_hash":null,"temp_database_hash":null,"backup_path":null,"original_library_backup_path":"{original_backup}","candidate_credential_ref":null,"credential_origin_fingerprint":null,"summary":null}}"#
            );
        }
        format!(
            r#"{{"migration":1,"state":"{state}","original_library_hash":"{original_hash}","temp_library_hash":"{}","temp_database_hash":"{}","backup_path":"migration-backup-{}.json","original_library_backup_path":"{original_backup}","candidate_credential_ref":null,"credential_origin_fingerprint":null,"summary":{{"promptsMigrated":1,"favoritesDefaulted":0,"ordersRebuilt":0,"backupPath":"C:/safe/migration-backup.json","warnings":[]}},"summary_acknowledged":false}}"#,
            "b".repeat(64),
            "c".repeat(64),
            "d".repeat(32),
        )
    }

    fn reverse_provider_input() -> SaveProviderInput {
        SaveProviderInput {
            id: "reverse-image".into(),
            kind: ProviderKind::ReverseImage,
            display_name: "图片反推".into(),
            base_url: REVERSE_BASE_URL.into(),
            models_url: REVERSE_MODELS_URL.into(),
            chat_completions_url: REVERSE_CHAT_URL.into(),
            default_model: Some(REVERSE_MODEL.into()),
            temperature: None,
            context_window_tokens: None,
            confirm_cross_origin: false,
        }
    }

    fn provider_binding(database: &Database, id: &str) -> (String, Option<String>, i64, i64) {
        database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT bound_host, credential_ref, config_revision, needs_credentials FROM ai_providers WHERE id = ?1",
                        [id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )
                    .map_err(|_| "TEST_QUERY".to_string())
            })
            .unwrap()
    }

    fn assert_seed_count(path: &std::path::Path, expected: i64) {
        let database = Database::open(path.join("banana.db")).unwrap();
        database
            .with_connection(|connection| {
                let count: i64 = connection
                    .query_row("SELECT COUNT(*) FROM ai_providers", [], |row| row.get(0))
                    .map_err(|_| "TEST_QUERY".to_string())?;
                assert_eq!(count, expected);
                Ok(())
            })
            .unwrap();
    }

    fn file_snapshot(path: &std::path::Path) -> Vec<(String, Vec<u8>)> {
        let mut files = fs::read_dir(path)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.unwrap();
                let name = entry.file_name().to_string_lossy().into_owned();
                (name != STARTUP_LOCK_FILE).then(|| (name, fs::read(entry.path()).unwrap()))
            })
            .collect::<Vec<_>>();
        files.sort_by(|left, right| left.0.cmp(&right.0));
        files
    }
}
