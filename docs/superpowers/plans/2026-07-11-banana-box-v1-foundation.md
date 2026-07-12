# Banana Box v1 Foundation, Security, and Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the v1 storage, credential, startup, migration, import/export, and WebView security foundation without implementing the storyboard, project, task, banana-animation, or reminder feature UI.

**Architecture:** Keep the existing prompt library in a sanitized, versioned `library.json`, and place all new domains behind one Rust-owned SQLite service. Startup is a gated Rust state machine: fresh install, interrupted fresh initialization, legacy upgrade, ready v1, interrupted migration recovery, or recovery-only mode. API keys never return to the WebView; provider commands resolve them through Windows Credential Manager, while backup and legacy import operate through staged, validated files.

**Tech Stack:** Tauri 2, Rust 2021, `rusqlite` 0.40.1 with bundled SQLite and backup support, `keyring` 4.1.4, `fs2` 0.4.3, `sha2` 0.10.9, `url` 2.5.8, Vue 3, Pinia, TypeScript, Vitest, Rust unit tests.

---

## Scope And Success Criteria

This sub-plan implements milestone 0 of the approved v1 specification. It stops before the new product pages and reminder behavior.

The foundation is complete when all of the following are true:

- Existing prompt `favorite` and `order` fields round-trip through Rust.
- A fresh install creates sanitized `library.json`, `banana.db`, and two credential-less Provider records.
- A v0.2.2 install migrates through a durable sidecar without network access.
- Interrupted `preparing`, `prepared`, and `committing` states recover deterministically.
- `reverse-image` and `storyboard` use separate Provider rows and credential identities.
- The WebView can submit a new key but can never read an existing key back.
- Legacy ZIP import strips `apiKey`, rejects unsafe paths, and cannot overwrite v1 domains.
- Full backup uses a SQLite-consistent snapshot, excludes credential references, and restores through staging.
- CSP is enabled and `main`/`floatbtn` capabilities are least privilege.
- `pnpm check`, Rust tests, Rust formatting, and Rust compilation all pass.

## File Responsibility Map

### Rust files created

- `src-tauri/migrations/0001_v1.sql` — the only v1 schema source; every table, foreign key, CHECK, and index lives here.
- `src-tauri/src/db/mod.rs` — owns the single SQLite connection and exposes `open`, `with_connection`, deferred `with_transaction`, claim-safe `with_immediate_transaction`, and `online_backup`.
- `src-tauri/src/db/schema.rs` — embeds `0001_v1.sql`, applies ordered migrations, and validates `user_version`, `integrity_check`, and `foreign_key_check`.
- `src-tauri/src/fs_atomic.rs` — flushes and atomically replaces existing same-volume files with Windows `MoveFileExW` replace/write-through semantics.
- `src-tauri/src/secrets.rs` — defines `CredentialStore`, production `WindowsCredentialStore`, and test `MemoryCredentialStore`.
- `src-tauri/src/providers.rs` — owns Provider validation, persistence, host binding, and the sole secret-resolving API `resolve_for_request`.
- `src-tauri/src/provider_http.rs` — one bounded, timeout-aware HTTP client shared by reverse-image and Storyboard Providers.
- `src-tauri/src/app_state.rs` — defines `AppServices`, `StartupGate`, public startup status, and the ready-state guard.
- `src-tauri/src/startup.rs` — classifies the six startup paths and creates/recovers a fresh v1 installation.
- `src-tauri/src/migration.rs` — owns migration sidecar types, prepare/commit/recovery, hashing, locking, and same-volume file switching.
- `src-tauri/src/legacy_import.rs` — inspects and commits v0.2.2 JSON/ZIP imports without writing legacy secrets.
- `src-tauri/src/backup.rs` — creates and restores the complete v1 backup format with resource limits and staged verification.
- `src-tauri/src/backup_validation.rs` — shared typed semantic-validator registry used by startup/backup/restore boundaries.
- `src-tauri/src/safe_archive.rs` — one bounded, Windows-safe ZIP inspection/extraction engine for import and restore.
- `src-tauri/src/image_store.rs` — resolves logical `images/<name>` paths through an atomically switched generation pointer.
- `src-tauri/src/commands/provider_commands.rs` — Provider list/save/credential-clear/connection IPC; no command returns a key.
- `src-tauri/src/commands/startup_commands.rs` — startup-status IPC available in ready and recovery modes.
- `src-tauri/src/commands/backup_commands.rs` — legacy import and full backup/restore IPC.
- `src-tauri/src/command_auth.rs` — caller-window authorization shared by every custom IPC command.

### Rust files modified

- `src-tauri/Cargo.toml` and `src-tauri/Cargo.lock` — add direct foundation dependencies.
- `src-tauri/src/library.rs` — preserve `favorite`/`order`, strictly parse files, and serialize only non-sensitive settings.
- `src-tauri/src/commands.rs` — register focused command submodules and convert reverse-image inputs to `provider_id`.
- `src-tauri/src/lib.rs` — run startup before normal services are exposed, manage `StartupGate`/`AppServices`, and show recovery UI when required.

### Frontend files created

- `src/types/providers.ts` — public Provider DTOs with no key or credential reference.
- `src/lib/provider-ipc.ts` — Provider command wrappers.
- `src/lib/startup-ipc.ts` — startup status types and command wrapper.
- `src/lib/backup-ipc.ts` — distinct legacy-import and full-backup wrappers.
- `src/stores/providers.ts` — public Provider state used by settings and reverse image.
- `src/components/MainRoot.vue` — waits for startup status before mounting `App.vue`.
- `src/components/RecoveryPage.vue` — recovery-only screen that never mounts normal stores.
- `src/components/MigrationSummaryDialog.vue` — persistent post-migration summary and acknowledgement.
- `src/components/FullRestoreSummaryDialog.vue` — verified restore outcome, warnings, and cleanup acknowledgement.

### Frontend files modified

- `src/main.ts` — mount `MainRoot` for the `main` window.
- `src/types/index.ts` and `src/stores/library.ts` — remove API secrets and Provider configuration from legacy UI settings.
- `src/lib/ipc.ts` — remove secret-bearing API contracts and keep non-Provider IPC only.
- `src/components/SettingsModal.vue` — edit Provider data through `provider-ipc.ts`; password input is write-only.
- `src/components/ReverseImagePanel.vue` — send `providerId`, `model`, and `imagePath`, never a key.

### Security configuration modified

- `src-tauri/tauri.conf.json` — enable production/dev CSP.
- `src-tauri/capabilities/main.json` — main-window permissions.
- `src-tauri/capabilities/floatbtn.json` — minimal floating-window permissions.
- Delete `src-tauri/capabilities/default.json` and `src-tauri/capabilities/desktop.json` after replacements pass tests.

### Tests created or expanded

- Rust unit tests stay beside `library`, `db`, `secrets`, `providers`, `startup`, `migration`, `legacy_import`, and `backup`.
- `tests/stores/providers.test.ts`
- `tests/components/MainRoot.test.ts`
- `tests/components/RecoveryPage.test.ts`
- `tests/config/security.test.ts`
- Existing settings, reverse-image, app, and library tests are updated where their contracts change.

## Locked Interfaces For Later Storyboard Work

Later plans must use these names unchanged. The following is a signature-only contract, not paste-ready implementation code:

```text
// src-tauri/src/db/mod.rs
pub struct Database { connection: Mutex<Connection> }

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String>;
    pub fn with_connection<T>(&self, f: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String>;
    pub fn with_transaction<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T, String>) -> Result<T, String>;
    pub fn with_immediate_transaction<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T, String>) -> Result<T, String>;
    pub fn online_backup(&self, destination: impl AsRef<Path>) -> Result<(), String>;
}

// src-tauri/src/providers.rs
pub enum ProviderKind { ReverseImage, Storyboard }
pub enum StructuredMode { JsonSchema, StrictJson }
pub struct ResolvedProvider { pub provider: AiProvider, pub api_key: String }
pub enum SafeCredentialError { MissingBinding, MissingSecret, StoreUnavailable }
pub struct ProviderPreflight {
    pub provider: AiProvider,
    pub credential: Result<String, SafeCredentialError>,
}
pub struct ProviderObservation {
    pub provider_id: String,
    pub config_revision: i64,
    pub capability_revision: i64,
    pub base_url: String,
    pub models_url: String,
    pub chat_completions_url: String,
    pub probed_model: Option<String>,
}

impl ProviderService {
    pub fn list(&self, kind: ProviderKind) -> Result<Vec<AiProvider>, String>;
    pub fn get(&self, id: &str) -> Result<AiProvider, String>;
    pub fn save(&self, input: SaveProviderInput, api_key: Option<&str>) -> Result<AiProvider, String>;
    pub fn resolve_for_request(&self, id: &str) -> Result<ResolvedProvider, String>;
    pub fn with_resolved_for_request<T>(
        &self,
        id: &str,
        operation: impl FnOnce(ResolvedProvider) -> Result<T, String>,
    ) -> Result<T, String>;
    pub(crate) fn with_request_preflight<T>(
        &self,
        id: &str,
        operation: impl FnOnce(ProviderPreflight) -> Result<T, String>,
    ) -> Result<T, String>;
    pub fn clear_credential(&self, id: &str) -> Result<(), String>;
}

// src-tauri/src/app_state.rs
pub struct AppServices {
    pub db: Arc<Database>,
    pub providers: Arc<ProviderService>,
    pub provider_http: Arc<ProviderHttpClient>,
    pub operations: Arc<AppOperationGate>,
    pub images: Arc<ImageStore>,
}

pub struct StartupGate(RwLock<StartupStatus>);
impl StartupGate {
    pub fn require_ready(&self) -> Result<(), String>;
}
```

The public IPC names are also locked:

```text
get_startup_status
acknowledge_migration_summary
list_ai_providers
save_ai_provider
clear_ai_provider_credential
check_ai_provider_connection
inspect_legacy_import
commit_legacy_import
discard_legacy_import_preview
create_full_backup
inspect_full_backup
restore_full_backup
discard_full_backup_preview
acknowledge_full_restore
```

No IPC command named `get_api_key`, `read_credential`, or equivalent may be added.

Custom commands registered with `invoke_handler` are not treated as protected merely because plugin/core capabilities are narrow. Tauri deserializes ordinary typed command parameters before entering the function, so merely placing `require_caller` on line one cannot protect malformed payloads from an unauthorized window. Every protected command therefore accepts exactly one non-`Deserialize` authorized envelope that checks the originating WebView label inside `CommandArg::from_command` **before** deserializing the complete JSON payload:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IpcCaller { Main, FloatButton, Reminder }

pub(crate) fn require_caller_label(
    label: &str,
    allowed: &[IpcCaller],
) -> Result<(), String> {
    let caller = match label {
        "main" => IpcCaller::Main,
        "floatbtn" => IpcCaller::FloatButton,
        "reminder" => IpcCaller::Reminder,
        _ => return Err("FORBIDDEN_WINDOW".to_string()),
    };
    allowed.contains(&caller)
        .then_some(())
        .ok_or_else(|| "FORBIDDEN_WINDOW".to_string())
}

pub struct AuthorizedArgs<T, const MASK: u8>(pub T);
pub type MainArgs<T> = AuthorizedArgs<T, 0b001>;
pub type FloatArgs<T> = AuthorizedArgs<T, 0b010>;
pub type ReminderArgs<T> = AuthorizedArgs<T, 0b100>;
pub type MainOrFloatArgs<T> = AuthorizedArgs<T, 0b011>;

impl<'de, T, const MASK: u8, R> tauri::ipc::CommandArg<'de, R>
    for AuthorizedArgs<T, MASK>
where
    T: serde::de::DeserializeOwned,
    R: tauri::Runtime,
{
    fn from_command(
        command: tauri::ipc::CommandItem<'de, R>,
    ) -> Result<Self, tauri::ipc::InvokeError> {
        let mut allowed = Vec::with_capacity(3);
        if MASK & 0b001 != 0 { allowed.push(IpcCaller::Main); }
        if MASK & 0b010 != 0 { allowed.push(IpcCaller::FloatButton); }
        if MASK & 0b100 != 0 { allowed.push(IpcCaller::Reminder); }
        require_caller_label(command.message.webview_ref().label(), &allowed)
            .map_err(tauri::ipc::InvokeError::from)?;
        let value = match command.message.payload() {
            tauri::ipc::InvokeBody::Json(value) => value.clone(),
            tauri::ipc::InvokeBody::Raw(_) => return Err("INVALID_INPUT".into()),
        };
        serde_json::from_value(value)
            .map(Self)
            .map_err(|_| "INVALID_INPUT".into())
    }
}
```

Each command defines one `#[serde(rename_all = "camelCase", deny_unknown_fields)]` `*CommandArgs` DTO for the entire existing frontend payload; even zero-input commands use an empty deny-unknown DTO. The frontend invoke shape does not gain an `args` nesting layer because `AuthorizedArgs` intentionally deserializes the complete `InvokeBody::Json`, ignoring the macro's parameter key. No other user-controlled command parameter may implement ordinary `Deserialize`; split parameters such as `input` plus `apiKey` become fields of that single envelope. Keep `require_caller_label` as the one label policy called by the production wrapper implementation (the expanded code above shows the required order; factor the mask mapping through that helper rather than duplicate policy).

`StartupGate` and the standalone recovery-safe coordinators are managed in every startup mode, but `AppServices` exists only in Ready. Therefore no ordinary business command may declare a required `tauri::State<AppServices>` parameter: Tauri resolves command parameters before entering the function, so that signature would bypass the promised `STARTUP_NOT_READY` ordering in Recovery. The locked exported signature is `WebviewWindow + State<StartupGate> + MainArgs/FloatArgs/ReminderArgs<CommandArgs>`; authorized-envelope extraction rejects a wrong label first, then the body calls `gate.require_ready()?`, obtains `window.app_handle().try_state::<AppServices>().ok_or(STARTUP_NOT_READY)?`, acquires the operation permit, and only afterward performs business validation. Recovery-safe full-restore inspection uses a main-authorized envelope plus its always-managed standalone dependencies explicitly. Add real-handler matrices for valid and malformed/missing/raw payloads from every wrong window, correct-label malformed input, Ready/Recovery, and unknown labels. Wrong callers always receive `FORBIDDEN_WINDOW`; correct-label malformed input receives sanitized `INVALID_INPUT`; correct business callers in Recovery receive `STARTUP_NOT_READY`; no framework serde detail or repository/service adapter is exposed.

Foundation startup, library, Provider, import, backup, restore, settings, reverse-image, and other business commands are main-only. Authorization happens inside command-argument extraction before payload deserialization, so a forbidden caller cannot learn whether a record exists, trigger a dialog, distinguish validation errors, or receive raw serde details. Later plans must reuse these wrappers rather than add local label checks or ordinary typed payload parameters.

---

### Task 1: Establish The Verified Baseline And Add Direct Dependencies

**Files:**
- Modify: `src-tauri/src/lib.rs` (format-only baseline commit)
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`

- [ ] **Step 1: Put Rust on this PowerShell process PATH**

Run:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
rustc --version
cargo --version
```

Expected: both commands print versions and exit 0.

- [ ] **Step 2: Install the existing frontend lockfile**

Run:

```powershell
pnpm install --frozen-lockfile
```

Expected: exit 0 and `pnpm-lock.yaml` remains unchanged.

- [ ] **Step 3: Run the unmodified baseline**

Run:

```powershell
pnpm check
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
```

Expected: typecheck, lint, Vitest, and Rust tests all pass before foundation edits.

- [ ] **Step 4: Normalize the known Rust formatting baseline separately**

Run the check first:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri\Cargo.toml -- --check
```

Expected on commit `299dde2`: exit `1`, with rustfmt changes limited to the long assertions in `src-tauri/src/lib.rs`. Then run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri\Cargo.toml
git diff -- src-tauri/src/lib.rs
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
git add src-tauri/src/lib.rs
git commit -m "style: normalize Rust baseline formatting"
```

Expected: the diff is formatting-only, tests pass, and later v1 format gates start from a clean baseline.

- [ ] **Step 5: Add exact direct dependencies**

Run:

```powershell
Set-Location src-tauri
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add rusqlite@0.40.1 --features bundled,backup
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add keyring@4.1.4
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add fs2@0.4.3
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add sha2@0.10.9
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add url@2.5.8
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add reqwest@0.12 --no-default-features --features json,stream,rustls-tls,gzip,brotli,deflate
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add tokio@1 --features rt-multi-thread,macros,sync,time
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add tokio-util@0.7 --features rt
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add futures-util@0.3
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add walkdir@2.5.0
& "$env:USERPROFILE\.cargo\bin\cargo.exe" add windows-sys@0.61 --target 'cfg(windows)' --features Win32_Foundation,Win32_Storage_FileSystem
Set-Location ..
```

Then move `tempfile = "3"` from `[dev-dependencies]` to `[dependencies]`, because staged migration and restore use it at runtime. Keep `[dev-dependencies]` present even if it becomes empty.

- [ ] **Step 6: Verify dependency resolution**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" check --manifest-path src-tauri\Cargo.toml
git diff --check
```

Expected: Rust compilation succeeds; `git diff --check` prints nothing.

- [ ] **Step 7: Commit the dependency boundary**

Run:

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "build: add v1 foundation dependencies"
```

Expected: one commit containing only Cargo dependency files.

---

### Task 2: Preserve Prompt Favorite And Order In Rust

**Files:**
- Modify: `src-tauri/src/library.rs`
- Modify: `tests/stores/library.test.ts`

- [ ] **Step 1: Add a failing Rust round-trip test**

Add this test in `src-tauri/src/library.rs`:

```rust
#[test]
fn prompt_favorite_and_order_round_trip() {
    let dir = tempdir().unwrap();
    let mut library = Library::default();
    library.prompts.push(Prompt {
        id: "p1".into(),
        title: "镜头".into(),
        content: "内容".into(),
        category_id: None,
        tags: vec![],
        image: None,
        favorite: true,
        order: 7,
        created_at: 1,
        updated_at: 2,
    });

    save_library(dir.path(), &library).unwrap();
    let loaded = load_library_strict(dir.path()).unwrap();

    assert!(loaded.prompts[0].favorite);
    assert_eq!(loaded.prompts[0].order, 7);
}
```

- [ ] **Step 2: Run the test and confirm RED**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml library::tests::prompt_favorite_and_order_round_trip
```

Expected: compilation fails because Rust `Prompt` lacks `favorite`, `order`, and `load_library_strict`.

- [ ] **Step 3: Add the fields and strict loader**

Use this contract in `src-tauri/src/library.rs`:

```rust
pub const LIBRARY_VERSION: i32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category_id: Option<String>,
    pub tags: Vec<String>,
    pub image: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub order: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn load_library_strict(dir: &Path) -> Result<Library, String> {
    let path = library_path(dir);
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("读取 {} 失败：{error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("解析 {} 失败：{error}", path.display()))
}
```

Set new defaults to `version: LIBRARY_VERSION`. Keep a separate `Library::default()` for fresh initialization; do not silently call it from `load_library_strict`.

- [ ] **Step 4: Update every Rust `Prompt` fixture**

Add these exact fields to every existing Rust `Prompt` literal:

```text
favorite: false,
order: 0,
```

Use sequential `order` values when a test creates multiple prompts.

- [ ] **Step 5: Run Rust and frontend store tests to confirm GREEN**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml library::tests
pnpm exec vitest run tests/stores/library.test.ts
```

Expected: all targeted tests pass; serialized prompt JSON contains `favorite` and `order`.

- [ ] **Step 6: Commit the compatibility fix**

Run:

```powershell
git add src-tauri/src/library.rs tests/stores/library.test.ts
git commit -m "fix: preserve prompt favorite and order"
```

Expected: one focused compatibility commit.

---

### Task 3: Create The Versioned SQLite Schema And Database Service

**Files:**
- Create: `src-tauri/migrations/0001_v1.sql`
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/schema.rs`
- Create: `src-tauri/src/fs_atomic.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create the failing database tests**

Add in `src-tauri/src/db/mod.rs` under `#[cfg(test)]`:

```rust
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
    }).unwrap();
}
```

- [ ] **Step 2: Run the database test and confirm RED**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml db::tests::open_creates_schema_and_enforces_constraints
```

Expected: compilation fails because `db` and `Database` do not exist.

- [ ] **Step 3: Create the complete v1 migration**

Create `src-tauri/migrations/0001_v1.sql` with this schema:

```sql
CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE ai_providers (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('reverse-image', 'storyboard')),
  display_name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  models_url TEXT NOT NULL,
  chat_completions_url TEXT NOT NULL,
  default_model TEXT,
  available_models_json TEXT NOT NULL DEFAULT '[]',
  probed_model TEXT,
  structured_mode TEXT CHECK (structured_mode IS NULL OR structured_mode IN ('json_schema', 'strict_json')),
  interactive_compatible INTEGER CHECK (interactive_compatible IS NULL OR interactive_compatible IN (0, 1)),
  bound_host TEXT,
  needs_credentials INTEGER NOT NULL DEFAULT 1 CHECK (needs_credentials IN (0, 1)),
  credential_ref TEXT UNIQUE,
  config_revision INTEGER NOT NULL DEFAULT 1 CHECK (config_revision >= 1),
  capability_revision INTEGER NOT NULL DEFAULT 1 CHECK (capability_revision >= 1),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE credential_cleanup (
  credential_ref TEXT PRIMARY KEY,
  reason TEXT NOT NULL CHECK (reason IN ('candidate', 'retired')),
  created_at TEXT NOT NULL
);

CREATE TABLE projects (
  id TEXT PRIMARY KEY NOT NULL,
  code TEXT COLLATE NOCASE NOT NULL UNIQUE CHECK (length(trim(code)) > 0),
  version TEXT NOT NULL CHECK (length(trim(version)) > 0),
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  file_path TEXT NOT NULL CHECK (length(trim(file_path)) > 0),
  release_date TEXT NOT NULL CHECK (length(release_date) = 10),
  main_stage_key TEXT NOT NULL CHECK (main_stage_key IN (
    'storyboard', 'first_cut', 'refinement', 'middle_cut',
    'effects', 'art_titles', 'music', 'final_composite'
  )),
  archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE project_stages (
  id TEXT PRIMARY KEY NOT NULL,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  stage_key TEXT NOT NULL CHECK (stage_key IN (
    'storyboard', 'first_cut', 'refinement', 'middle_cut',
    'effects', 'art_titles', 'music', 'final_composite'
  )),
  position INTEGER NOT NULL CHECK (position BETWEEN 0 AND 7),
  start_date TEXT NOT NULL CHECK (length(start_date) = 10),
  end_date TEXT NOT NULL CHECK (length(end_date) = 10),
  progress INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
  updated_at TEXT NOT NULL,
  UNIQUE (project_id, stage_key),
  UNIQUE (project_id, position),
  CHECK (start_date <= end_date)
);

CREATE INDEX idx_projects_main_stage ON projects(main_stage_key, archived);
CREATE INDEX idx_projects_release_date ON projects(release_date);
CREATE INDEX idx_project_stages_project ON project_stages(project_id, position);

CREATE TABLE daily_task_days (
  id TEXT PRIMARY KEY NOT NULL,
  local_date TEXT NOT NULL UNIQUE CHECK (length(local_date) = 10),
  settled_at TEXT,
  report_snapshot TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE daily_task_groups (
  id TEXT PRIMARY KEY NOT NULL,
  day_id TEXT NOT NULL REFERENCES daily_task_days(id) ON DELETE CASCADE,
  code TEXT COLLATE NOCASE NOT NULL CHECK (length(trim(code)) > 0),
  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (day_id, code),
  UNIQUE (day_id, position)
);

CREATE TABLE daily_tasks (
  id TEXT PRIMARY KEY NOT NULL,
  group_id TEXT NOT NULL REFERENCES daily_task_groups(id) ON DELETE CASCADE,
  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
  progress INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
  note TEXT NOT NULL DEFAULT '',
  invested_minutes INTEGER NOT NULL DEFAULT 0 CHECK (invested_minutes >= 0),
  position INTEGER NOT NULL CHECK (position >= 0),
  source_task_id TEXT REFERENCES daily_tasks(id) ON DELETE SET NULL,
  carry_target_date TEXT,
  source_snapshot_hash TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (source_task_id, carry_target_date)
);

CREATE INDEX idx_daily_groups_day_position
  ON daily_task_groups(day_id, position);
CREATE INDEX idx_daily_tasks_group_position
  ON daily_tasks(group_id, position, id);
CREATE INDEX idx_daily_tasks_carry_source
  ON daily_tasks(source_task_id, carry_target_date);

CREATE TABLE skills (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('builtin', 'local')),
  current_version_id TEXT REFERENCES skill_versions(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE skill_versions (
  id TEXT PRIMARY KEY,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  display_version TEXT NOT NULL,
  protocol_version INTEGER NOT NULL,
  content_hash TEXT NOT NULL,
  manifest_json TEXT NOT NULL,
  files_json TEXT NOT NULL,
  imported_at TEXT NOT NULL,
  UNIQUE(skill_id, content_hash)
);

CREATE TABLE storyboard_threads (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  provider_id TEXT REFERENCES ai_providers(id) ON DELETE SET NULL,
  model TEXT,
  skill_id TEXT REFERENCES skills(id) ON DELETE SET NULL,
  workflow_state TEXT NOT NULL DEFAULT 'awaiting_story' CHECK (workflow_state IN ('awaiting_story','analyzing_context','collecting_settings','confirming_settings','drafting_storyboard','confirming_storyboard','generating_output','free_chat')),
  workflow_protocol_version INTEGER NOT NULL DEFAULT 1,
  workflow_revision INTEGER NOT NULL DEFAULT 0 CHECK (workflow_revision >= 0),
  request_config_revision INTEGER NOT NULL DEFAULT 0 CHECK (request_config_revision >= 0),
  request_state_revision INTEGER NOT NULL DEFAULT 0 CHECK (request_state_revision >= 0),
  workflow_context_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE agent_requests (
  id TEXT PRIMARY KEY,
  thread_id TEXT NOT NULL REFERENCES storyboard_threads(id) ON DELETE CASCADE,
  source_request_id TEXT REFERENCES agent_requests(id) ON DELETE SET NULL,
  provider_id TEXT NOT NULL REFERENCES ai_providers(id) ON DELETE RESTRICT,
  model TEXT NOT NULL,
  skill_version_id TEXT REFERENCES skill_versions(id) ON DELETE RESTRICT,
  snapshot_json TEXT NOT NULL,
  expected_workflow_revision INTEGER NOT NULL CHECK (expected_workflow_revision >= 0),
  expected_workflow_state TEXT NOT NULL CHECK (expected_workflow_state IN ('awaiting_story','analyzing_context','collecting_settings','confirming_settings','drafting_storyboard','confirming_storyboard','generating_output','free_chat')),
  expected_latest_message_position INTEGER NOT NULL,
  expected_request_config_revision INTEGER NOT NULL CHECK (expected_request_config_revision >= 0),
  last_persisted_sequence INTEGER NOT NULL DEFAULT 0 CHECK (last_persisted_sequence >= 0),
  input_start_position INTEGER NOT NULL,
  input_end_position INTEGER NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('streaming','completed','cancelled','failed','interrupted')),
  error_code TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE UNIQUE INDEX one_active_agent_request_per_thread
ON agent_requests(thread_id) WHERE status = 'streaming';

CREATE TABLE storyboard_messages (
  id TEXT PRIMARY KEY,
  thread_id TEXT NOT NULL REFERENCES storyboard_threads(id) ON DELETE CASCADE,
  request_id TEXT REFERENCES agent_requests(id) ON DELETE SET NULL,
  responds_to_message_id TEXT REFERENCES storyboard_messages(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('user','assistant')),
  message_type TEXT NOT NULL CHECK (message_type IN ('user_text','user_choices','user_confirmation','assistant_markdown','analysis_result','choice_prompt','confirmation','final_output')),
  content_markdown TEXT NOT NULL DEFAULT '',
  structured_json TEXT,
  status TEXT NOT NULL CHECK (status IN ('complete','streaming','cancelled','failed','interrupted')),
  created_at TEXT NOT NULL,
  CHECK (
    (role = 'user' AND message_type IN ('user_text','user_choices','user_confirmation')) OR
    (role = 'assistant' AND message_type IN ('assistant_markdown','analysis_result','choice_prompt','confirmation','final_output'))
  ),
  CHECK (
    (message_type IN ('user_choices','user_confirmation') AND responds_to_message_id IS NOT NULL AND structured_json IS NOT NULL) OR
    (message_type NOT IN ('user_choices','user_confirmation') AND responds_to_message_id IS NULL)
  ),
  UNIQUE(thread_id, position),
  UNIQUE(responds_to_message_id)
);

CREATE TABLE storyboard_message_blocks (
  id TEXT PRIMARY KEY,
  message_id TEXT NOT NULL REFERENCES storyboard_messages(id) ON DELETE CASCADE,
  block_key TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('storyboard','video','scene_reference','shot')),
  title TEXT NOT NULL,
  markdown TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE(message_id, position),
  UNIQUE(message_id, block_key)
);

CREATE TABLE reminder_log (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  local_date TEXT NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('initial','snooze')),
  state TEXT NOT NULL CHECK (state IN ('pending','shown','hidden','actioned','cancelled')),
  delivery_id TEXT NOT NULL,
  attempt_token TEXT,
  owner_id TEXT,
  lease_expires_at TEXT,
  attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 3),
  claimed_at TEXT,
  shown_at TEXT,
  acknowledged_at TEXT,
  snoozed_until TEXT,
  unread INTEGER NOT NULL DEFAULT 0 CHECK (unread IN (0, 1)),
  UNIQUE(kind, local_date, phase)
);
```

`reminder_log` is created only by the shared `0001_v1.sql` migration above. The desktop reminder repository must open it through `Database`; it must not execute a second runtime `CREATE TABLE` or introduce a reminder-specific migration file. An expired unacknowledged attempt keeps the same `delivery_id`, rotates `attempt_token`/`owner_id`, and increments `attempt_count`; a user-initiated unread reopen creates a new delivery. Reminder title/body are derived from current task data and are not persisted in this delivery ledger.

- [ ] **Step 4: Implement schema migration helpers**

Create `src-tauri/src/db/schema.rs`:

```rust
use rusqlite::Connection;

pub const SCHEMA_VERSION: i64 = 1;
const MIGRATION_V1: &str = include_str!("../../migrations/0001_v1.sql");

pub fn migrate(connection: &mut Connection) -> Result<(), String> {
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if version > SCHEMA_VERSION {
        return Err(format!("数据库版本 {version} 高于当前支持版本 {SCHEMA_VERSION}"));
    }
    if version == 0 {
        let transaction = connection.transaction().map_err(|error| error.to_string())?;
        transaction.execute_batch(MIGRATION_V1).map_err(|error| error.to_string())?;
        transaction.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            [],
        ).map_err(|error| error.to_string())?;
        transaction.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn validate(connection: &Connection) -> Result<(), String> {
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if integrity != "ok" { return Err(format!("SQLite integrity_check: {integrity}")); }
    let foreign_key_errors: i64 = connection
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if foreign_key_errors != 0 { return Err("SQLite foreign_key_check 失败".into()); }
    Ok(())
}
```

- [ ] **Step 5: Implement the locked Database API**

Create `src-tauri/src/db/mod.rs` with `pub mod schema;` and this implementation shape:

```rust
use rusqlite::{backup::Backup, Connection, Transaction, TransactionBehavior};
use std::{path::{Path, PathBuf}, sync::Mutex, time::Duration};

pub struct Database {
    path: PathBuf,
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let mut connection = Connection::open(&path).map_err(|error| error.to_string())?;
        connection.busy_timeout(Duration::from_secs(5)).map_err(|error| error.to_string())?;
        connection.pragma_update(None, "foreign_keys", true).map_err(|error| error.to_string())?;
        connection.pragma_update(None, "journal_mode", "WAL").map_err(|error| error.to_string())?;
        schema::migrate(&mut connection)?;
        schema::validate(&connection)?;
        Ok(Self { path, connection: Mutex::new(connection) })
    }

    pub fn with_connection<T>(&self, f: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
        let guard = self.connection.lock().map_err(|error| error.to_string())?;
        f(&guard)
    }

    pub fn with_transaction<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T, String>) -> Result<T, String> {
        let mut guard = self.connection.lock().map_err(|error| error.to_string())?;
        let transaction = guard.transaction().map_err(|error| error.to_string())?;
        let value = f(&transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(value)
    }

    pub fn with_immediate_transaction<T>(&self, f: impl FnOnce(&Transaction<'_>) -> Result<T, String>) -> Result<T, String> {
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
        backup.run_to_completion(64, Duration::from_millis(25), None)
            .map_err(|error| error.to_string())
    }
}
```

- [ ] **Step 6: Implement and test same-volume atomic replacement**

Create `src-tauri/src/fs_atomic.rs`. The source file must already be flushed and closed. Require both paths to have the same canonical parent, then use the OS replace primitive:

```rust
use std::path::Path;

pub fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    let source_parent = source.parent().ok_or_else(|| "临时文件缺少父目录".to_string())?;
    let destination_parent = destination.parent().ok_or_else(|| "目标文件缺少父目录".to_string())?;
    let source_parent = source_parent.canonicalize().map_err(|error| error.to_string())?;
    let destination_parent = destination_parent.canonicalize().map_err(|error| error.to_string())?;
    if source_parent != destination_parent {
        return Err("原子替换要求临时文件与目标文件位于同一目录/磁盘卷".into());
    }
    replace_file_platform(source, destination)
}

#[cfg(windows)]
fn replace_file_platform(source: &Path, destination: &Path) -> Result<(), String> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }
    let source = wide(source.as_os_str());
    let destination = wide(destination.as_os_str());
    let ok = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file_platform(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::rename(source, destination).map_err(|error| error.to_string())
}
```

Add tests that replace a nonexistent destination, replace the same existing destination twice, preserve the old destination when the source is missing, and reject different parents. The consecutive replacement test must run on Windows, not only through a mocked filesystem.

- [ ] **Step 7: Register the modules and run GREEN tests**

Add `mod db; mod fs_atomic;` to `src-tauri/src/lib.rs`, then run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml db::tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml fs_atomic::tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" check --manifest-path src-tauri\Cargo.toml
```

Expected: schema test passes; a two-thread claim test proves `with_immediate_transaction` serializes competing writes; no SQL migration or Rust compile errors.

- [ ] **Step 8: Commit the database foundation**

Run:

```powershell
git add src-tauri/migrations/0001_v1.sql src-tauri/src/db src-tauri/src/fs_atomic.rs src-tauri/src/lib.rs
git commit -m "feat: add v1 sqlite foundation"
```

Expected: migration and Database API land together.

---

### Task 4: Add Windows Credentials And Provider Service

**Files:**
- Create: `src-tauri/src/secrets.rs`
- Create: `src-tauri/src/providers.rs`
- Create: `src-tauri/src/provider_http.rs`
- Create: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing credential and Provider tests**

Add tests proving secrets are separate from public DTOs:

```rust
fn storyboard_input(origin: &str) -> SaveProviderInput {
    SaveProviderInput {
        id: "storyboard".into(),
        kind: ProviderKind::Storyboard,
        display_name: "故事板".into(),
        base_url: origin.into(),
        models_url: format!("{origin}/models"),
        chat_completions_url: format!("{origin}/chat/completions"),
        default_model: Some("glm-5.2".into()),
        confirm_cross_origin: false,
    }
}

#[test]
fn save_and_resolve_provider_keeps_secret_out_of_database() {
    let dir = tempfile::tempdir().unwrap();
    let db = std::sync::Arc::new(Database::open(dir.path().join("banana.db")).unwrap());
    let secrets = std::sync::Arc::new(MemoryCredentialStore::default());
    let http = std::sync::Arc::new(test_provider_http());
    let mutations = std::sync::Arc::new(CredentialMutationCoordinator::default());
    let service = ProviderService::new(db.clone(), secrets.clone(), http, mutations);

    let saved = service.save(storyboard_input("https://api.example.com"), Some("secret-key")).unwrap();
    assert!(!saved.needs_credentials);
    assert_eq!(service.resolve_for_request(&saved.id).unwrap().api_key, "secret-key");

    db.with_connection(|connection| {
        let dump: String = connection
            .query_row("SELECT quote(credential_ref) || base_url FROM ai_providers WHERE id=?1", [&saved.id], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        assert!(!dump.contains("secret-key"));
        Ok(())
    }).unwrap();
}

#[test]
fn changing_provider_host_detaches_old_credential() {
    let dir = tempfile::tempdir().unwrap();
    let db = std::sync::Arc::new(Database::open(dir.path().join("banana.db")).unwrap());
    let secrets = std::sync::Arc::new(MemoryCredentialStore::default());
    let http = std::sync::Arc::new(test_provider_http());
    let mutations = std::sync::Arc::new(CredentialMutationCoordinator::default());
    let service = ProviderService::new(db.clone(), secrets, http, mutations);

    service.save(storyboard_input("https://api.old.example"), Some("old-key")).unwrap();
    let changed = service.save(storyboard_input("https://api.new.example"), None).unwrap();

    assert!(changed.needs_credentials);
    assert!(service.resolve_for_request("storyboard").is_err());
    db.with_connection(|connection| {
        let credential_ref: Option<String> = connection
            .query_row(
                "SELECT credential_ref FROM ai_providers WHERE id='storyboard'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        assert_eq!(credential_ref, None);
        Ok(())
    }).unwrap();
}

#[test]
fn cross_origin_endpoints_require_the_explicit_confirmation_bit() {
    let mut input = storyboard_input("https://api.example.com");
    input.models_url = "https://models.example.net/v1/models".into();
    assert!(validated_host_fingerprint(&input).unwrap_err().contains("跨源"));

    input.confirm_cross_origin = true;
    let fingerprint = validated_host_fingerprint(&input).unwrap();
    assert_eq!(
        fingerprint,
        "https://api.example.com|https://models.example.net",
    );
}
```

Also write `app_state.rs` concurrency tests before implementation: user permits increment/decrement active count by RAII; maintenance prevents every later user/background permit and waits for an already-held permit; releasing an unsealed lease reopens the gate; sealing keeps it closed for restart; one `RestoreBlockerRegistry` fake participant reports a sanitized blocker without the foundation depending on a future feature type. A single restore test must complete rather than wait on its own permit.

Add a two-server redirect matrix around the shared Provider HTTP transport. For each of `301`, `302`, `307`, and `308`, cover both a same-origin `Location` and a second-origin `Location`; the call must return an explicit `PROVIDER_REDIRECT_FORBIDDEN` error, and the redirect target server must record zero requests and zero `Authorization` headers. Run the same matrix for models GET and chat POST so connection checks, reverse-image, and Storyboard cannot diverge later.

Add Provider identity tests: attempt to save seeded `reverse-image` as `Storyboard`, seeded `storyboard` as `ReverseImage`, and an arbitrary new ID. Each returns `PROVIDER_KIND_MISMATCH` before URL credential mutation, leaves the row/key byte-identical, and performs zero network calls. Provider kind is never client-updatable.

- [ ] **Step 2: Run tests and confirm RED**

Before running the filter, add `mod app_state; mod provider_http; mod providers; mod secrets;` to `src-tauri/src/lib.rs`. This intentionally makes the missing files/types compile as RED instead of letting Cargo run zero matching tests.

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml providers::tests
```

Expected: compilation fails because `CredentialStore`, `MemoryCredentialStore`, and `ProviderService` do not exist.

- [ ] **Step 3: Implement the credential abstraction**

Create `src-tauri/src/secrets.rs`:

```rust
use std::{collections::HashMap, sync::Mutex};

pub trait CredentialStore: Send + Sync {
    fn set(&self, credential_ref: &str, secret: &str) -> Result<(), String>;
    fn get(&self, credential_ref: &str) -> Result<Option<String>, String>;
    fn delete(&self, credential_ref: &str) -> Result<(), String>;
}

#[derive(Default)]
pub struct CredentialMutationCoordinator(Mutex<()>);

impl CredentialMutationCoordinator {
    pub fn acquire(&self) -> Result<std::sync::MutexGuard<'_, ()>, String> {
        self.0.lock().map_err(|error| error.to_string())
    }
}

pub struct WindowsCredentialStore;

impl CredentialStore for WindowsCredentialStore {
    fn set(&self, credential_ref: &str, secret: &str) -> Result<(), String> {
        let entry = keyring::Entry::new("com.bananabox.app", credential_ref).map_err(|error| error.to_string())?;
        entry.set_password(secret).map_err(|error| error.to_string())
    }

    fn get(&self, credential_ref: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new("com.bananabox.app", credential_ref).map_err(|error| error.to_string())?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn delete(&self, credential_ref: &str) -> Result<(), String> {
        let entry = keyring::Entry::new("com.bananabox.app", credential_ref).map_err(|error| error.to_string())?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[derive(Default)]
pub struct MemoryCredentialStore(Mutex<HashMap<String, String>>);

impl CredentialStore for MemoryCredentialStore {
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0.lock().map_err(|error| error.to_string())?.insert(key.into(), value.into());
        Ok(())
    }
    fn get(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.0.lock().map_err(|error| error.to_string())?.get(key).cloned())
    }
    fn delete(&self, key: &str) -> Result<(), String> {
        self.0.lock().map_err(|error| error.to_string())?.remove(key);
        Ok(())
    }
}
```

- [ ] **Step 4: Implement Provider public types and URL validation**

Create in `src-tauri/src/providers.rs`:

```rust
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind { ReverseImage, Storyboard }

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StructuredMode { JsonSchema, StrictJson }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProvider {
    pub id: String,
    pub kind: ProviderKind,
    pub display_name: String,
    pub base_url: String,
    pub models_url: String,
    pub chat_completions_url: String,
    pub default_model: Option<String>,
    pub available_models: Vec<String>,
    pub probed_model: Option<String>,
    pub structured_mode: Option<StructuredMode>,
    pub interactive_compatible: Option<bool>,
    pub bound_host: Option<String>,
    pub needs_credentials: bool,
    pub config_revision: i64,
    pub capability_revision: i64,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveProviderInput {
    pub id: String,
    pub kind: ProviderKind,
    pub display_name: String,
    pub base_url: String,
    pub models_url: String,
    pub chat_completions_url: String,
    pub default_model: Option<String>,
    #[serde(default)]
    pub confirm_cross_origin: bool,
}

pub struct ResolvedProvider { pub provider: AiProvider, pub api_key: String }

pub(crate) fn validated_host_fingerprint(input: &SaveProviderInput) -> Result<String, String> {
    let base = url::Url::parse(&input.base_url).map_err(|error| error.to_string())?;
    let models = url::Url::parse(&input.models_url).map_err(|error| error.to_string())?;
    let chat = url::Url::parse(&input.chat_completions_url).map_err(|error| error.to_string())?;
    for endpoint in [&base, &models, &chat] {
        if !endpoint.username().is_empty() || endpoint.password().is_some() {
            return Err("Provider URL 禁止包含用户名或密码；API Key 只能保存到 Windows 凭据库".into());
        }
        if endpoint.query().is_some() || endpoint.fragment().is_some() {
            return Err("Provider URL 禁止查询参数或片段；请填写不含密钥的固定端点".into());
        }
        let host = endpoint.host_str().ok_or_else(|| "Provider URL 缺少主机".to_string())?;
        let loopback = matches!(host, "localhost" | "127.0.0.1" | "::1");
        if endpoint.scheme() != "https" && !(loopback && endpoint.scheme() == "http") {
            return Err("Provider 只允许 HTTPS；本机 loopback 可使用 HTTP".into());
        }
    }
    let cross_origin = base.origin() != models.origin() || base.origin() != chat.origin();
    if cross_origin && !input.confirm_cross_origin {
        return Err(format!(
            "Provider 端点跨源；确认后才能保存：models={}，chat={}",
            models.origin().ascii_serialization(),
            chat.origin().ascii_serialization(),
        ));
    }
    let mut origins = [&base, &models, &chat]
        .into_iter()
        .map(|url| url.origin().ascii_serialization().to_ascii_lowercase())
        .collect::<Vec<_>>();
    origins.sort();
    origins.dedup();
    Ok(origins.join("|"))
}
```

Same-origin is the default. Cross-origin endpoints are saved only after the UI displays the concrete target origins and resubmits with `confirm_cross_origin=true`. Store the sorted origin fingerprint in `bound_host`; changing any endpoint origin therefore clears the old credential binding.

Create exactly one `ProviderHttpClient` around `reqwest::Client::builder().redirect(Policy::none()).connect_timeout(Duration::from_secs(10)).build()`. Store it in `AppServices` and route every Provider models/chat request through it; no feature constructs `ureq::Agent` or a second `reqwest::Client`. Its request methods accept only an already validated stored `Url` plus a resolved key, attach `Authorization` to that one request, reject every `300..=399` before reading a success body, and never follow `Location`. The redirect matrix above must also assert the error text/log is redacted and the second server receives no request at all.

Every request also receives a `CancellationToken`. Wrap `send()` in `tokio::select!` plus a 30-second response-header timeout, and wrap each `bytes_stream()` next-chunk poll in the same cancellation select plus a resetting 30-second idle timeout. Non-streaming models/probe/reverse-image calls additionally have a 60-second total deadline; Storyboard streams have no total wall-clock deadline but remain bounded by per-read idle timeout, size limits, and user cancellation. Map connect/header/idle/total expiration to redacted `PROVIDER_TIMEOUT`. Tests use injected timeout durations, never wall-clock 30/60-second sleeps.

The shared transport requires every caller to pass a decoded-body limit; there is no unbounded `.text()`, `.bytes()`, or `.json()` helper. Use `MAX_PROVIDER_MODELS_BODY_BYTES = 1 MiB`, `MAX_PROVIDER_MODELS = 512`, `MAX_MODEL_ID_BYTES = 256`, `MAX_REVERSE_IMAGE_RESPONSE_BYTES = 2 MiB`, and `MAX_REVERSE_IMAGE_CONTENT_BYTES = 1 MiB`. Read incrementally after gzip/brotli/deflate decoding and abort before accepting the next chunk over the cap; validate complete JSON, exact model count, non-empty UTF-8 IDs/per-ID bytes, and parsed reverse-image content bytes. Never truncate: any excess returns `PROVIDER_RESPONSE_TOO_LARGE` and writes no model metadata/partial result. Add oversized body, 513-model, 257-byte-ID, chunked reverse-image, and compressed-decoded-over-limit tests, asserting bounded memory, no partial state, and no response body in logs.

Add deterministic never-respond, headers-only, and stalled-mid-body fixtures for models discovery/check connection and reverse-image. Cancellation or each timeout must close the response stream promptly and return once; later Storyboard tests reuse the same fixture for probe, raw Markdown, and structured streams.

Add table tests that place `user:password@`, username-only userinfo, `?api_key=TEST_ONLY_DO_NOT_USE`, and `#token` independently in Base, Models, and Chat URLs. Every case must fail before SQLite/credential access, and serialized Provider rows, backups, snapshots, errors, and logs must contain none of those sentinels. Also prove ordinary HTTPS paths and explicit cross-origin paths still validate.

- [ ] **Step 5: Implement ProviderService and redacted resolution**

Define a foundation-owned `CredentialMutationCoordinator(Mutex<()>)` in `secrets.rs` with an RAII `acquire()` guard. Setup creates exactly one `Arc<CredentialMutationCoordinator>` before startup classification and injects it into both `StartupCoordinator` and the later `ProviderService`. Implement `ProviderService` with `Arc<Database>`, `Arc<dyn CredentialStore>`, the shared `Arc<ProviderHttpClient>`, and that injected coordinator; it must not construct or own a private mutex. Every migration prepare/recovery, save, clear, startup credential cleanup, and legacy-import reverse-image overwrite acquires this same lock before reading the current row/ref and holds it through candidate journaling, Keyring read/write, DB switch, and retired enqueue. Post-commit best-effort deletion may occur after the active binding is fixed. Add `Arc::ptr_eq` setup tests and barriers for migration/recovery versus Settings save/clear/cleanup; never create a command-local coordinator.

Provider IDs/kinds are seeded identities: `reverse-image` is always `ReverseImage`, `storyboard` is always `Storyboard`, and `kind` is immutable. Load the existing row before credential/network access; an unknown ID or differing submitted kind returns `PROVIDER_KIND_MISMATCH` with zero SQLite/Keyring writes.

Canonical endpoint configuration and capability metadata have separate revisions. If any canonical Base/Models/Chat URL changes, the same Provider transaction increments both `config_revision` and `capability_revision`, resets `available_models_json='[]'`, `probed_model=NULL`, `structured_mode=NULL`, `interactive_compatible=NULL`, and `default_model=NULL`; same-origin path changes retain the bound credential but never retain discovery/probe truth. Non-endpoint display-only saves increment neither. Every discovery/probe operation captures `ProviderObservation { provider_id, config_revision, capability_revision, canonical endpoint tuple, probed_model }` before HTTP and conditionally writes metadata only when all still match; each successful metadata write increments `capability_revision`, so concurrent discover/probe results cannot overwrite each other silently. Model discovery always clears `probed_model`, `structured_mode`, and `interactive_compatible` in the same metadata transaction because even an equal-looking model list is a fresh capability epoch. A probe records the exact target model, never a Provider-wide Boolean. A mismatch returns `STALE_PROVIDER_PROBE` with zero metadata changes and the caller may re-observe/retry. Add same-origin path, cross-origin, save-versus-probe, model-A-versus-model-B probe, probe-versus-probe, and discover-versus-probe barriers.

Use copy-on-write credential identities `provider/{provider_id}/{sha256(origin_fingerprint)}/{uuid}` and the non-secret `credential_cleanup` journal. `save` follows this order:

1. Validate every URL and derive the sorted endpoint-origin fingerprint; read and kind-check the current row.
2. For a non-empty new key, insert its unique candidate ref into `credential_cleanup`, write the candidate secret, and read it back. The currently referenced credential remains untouched.
3. In one DB transaction, update endpoints/fingerprint and switch `credential_ref` to the verified candidate (or detach it when the origin changes without a new key), set `needs_credentials`, remove the now-active candidate cleanup row, and enqueue the former ref as `retired` when it differs.
4. Only after DB commit, delete retired Keyring entries and then their cleanup rows. Cleanup failure returns an accepted sanitized warning and is retried at startup; it never turns committed save/clear into a rejected operation. If DB switch fails, the old binding remains active and the candidate row remains for startup GC.
5. Startup iterates `credential_cleanup`: if a ref is currently referenced, delete only the stale cleanup row; otherwise delete/read-back absence from Credential Manager and then delete the cleanup row. This makes crashes before/after the DB switch idempotent without storing a secret in SQLite/files.
6. `clear_credential` first commits `credential_ref=NULL, needs_credentials=1` and queues the old ref as retired, then performs best-effort cleanup. A DB failure leaves the old binding usable; a Keyring cleanup failure leaves it unreachable and queued, not falsely rebound.
7. Return `AiProvider`; never return either credential ref.

Add failpoints after candidate journal insert, Keyring write/readback, DB switch, retired enqueue, and Keyring delete. Assert each restart exposes either the old provider/key or complete new provider/key, never a “save failed” response with the old ref silently holding new secret bytes. Assert clear is authoritative at DB commit and every orphan is eventually removed without secret-bearing logs/backups.

Add barrier races for save A/save B and save/clear on one Provider. The final active tuple must be exactly complete A, complete B, or cleared according to serialized lock order; every non-active candidate/retired ref is present in `credential_cleanup` until deletion and is eventually absent from Keyring. Legacy import must call the ProviderService-coordinated swap API while holding this lock through its JSON commit/rollback, so it cannot restore over a concurrent Settings save.

Implement a closure-based linearizable secret boundary. `with_request_preflight` acquires the shared `CredentialMutationCoordinator`, uses one `Database::with_connection` query to read **all** public Provider columns plus `credential_ref` from the same row snapshot, copies that non-secret tuple out, and releases the single-connection Database mutex **before** any Keyring access or business closure. Still holding only the credential coordinator, it kind/URL/fingerprint-validates, reads the immutable ref, and always invokes its synchronous closure with `ProviderPreflight { provider, credential: Result<String, SafeCredentialError> }`; a missing ref/Key is data for the closure, not an early return. This ordering is mandatory because the closure may open an IMMEDIATE transaction on the same `Database`; invoking it inside `with_connection` would self-deadlock. Add a timeout-guarded reentrant test whose closure successfully calls `with_immediate_transaction`. This crate-private form lets an automatic Agent terminal commit its already-paid assistant plus a safe next-request failure even when the Key disappeared. `ProviderPreflight` must not implement Debug/Serialize/Clone.

`with_resolved_for_request` wraps that primitive, converts credential failure into the public safe error, and invokes its closure only with a complete `ResolvedProvider`. The closure is where request insertion/runtime registration captures the complete tuple; neither closure may await or perform network I/O. Save/clear cannot interleave until it returns. `resolve_for_request` is the convenience form that returns that already-complete snapshot:

```rust
pub fn resolve_for_request(&self, id: &str) -> Result<ResolvedProvider, String> {
    self.with_resolved_for_request(id, Ok)
}
```

Do not derive `Debug`, `Serialize`, or `Clone` for `ResolvedProvider`, `ProviderPreflight`, or `SafeCredentialError`. Add barriers for both closure APIs versus same/cross-origin save and clear. A resolved result is exactly complete old tuple, complete new tuple, or missing-credential error; the always-invoked preflight sees the corresponding complete public tuple plus credential result. Assert no old endpoint ever pairs with the new ref/key and vice versa. Run this through reverse-image request tests too, proving only a captured whole tuple reaches `ProviderHttpClient`. Storyboard user requests use the resolved closure; automatic terminal preflight uses the always-invoked form so missing credentials cannot strand the old request.

- [ ] **Step 6: Add AppServices and StartupGate contracts**

Create `src-tauri/src/app_state.rs`:

```rust
use crate::{db::Database, provider_http::ProviderHttpClient, providers::ProviderService};
use std::sync::{Arc, Condvar, Mutex, RwLock};

pub struct AppServices {
    pub db: Arc<Database>,
    pub providers: Arc<ProviderService>,
    pub provider_http: Arc<ProviderHttpClient>,
    pub operations: Arc<AppOperationGate>,
}

struct OperationState {
    maintenance_pending: bool,
    active_operations: usize,
}

pub struct AppOperationGate {
    state: Mutex<OperationState>,
    idle: Condvar,
}

pub struct OperationPermit {
    gate: Arc<AppOperationGate>,
    released: bool,
}

pub struct MaintenanceLease {
    gate: Arc<AppOperationGate>,
    sealed_for_restart: bool,
}

impl AppOperationGate {
    pub fn enter_user(self: &Arc<Self>) -> Result<OperationPermit, String>;
    pub fn try_enter_background(self: &Arc<Self>) -> Option<OperationPermit>;
    pub fn begin_maintenance(self: &Arc<Self>) -> Result<MaintenanceLease, String>;
}

impl MaintenanceLease {
    pub fn seal_for_restart(self) -> Result<(), String>;
}

pub trait RestoreBlocker: Send + Sync {
    fn active_blocker(&self) -> Option<RestoreBlockerInfo>;
}

pub struct RestoreBlockerInfo { pub code: &'static str, pub message: String }
pub struct RestoreBlockerRegistry(RwLock<Vec<Arc<dyn RestoreBlocker>>>);

impl RestoreBlockerRegistry {
    pub fn register(&self, participant: Arc<dyn RestoreBlocker>) -> Result<(), String>;
    pub fn first_active(&self) -> Result<Option<RestoreBlockerInfo>, String>;
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationSummary {
    pub prompts_migrated: usize,
    pub favorites_defaulted: usize,
    pub orders_rebuilt: usize,
    pub backup_path: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, serde::Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum StartupStatus {
    Ready { migration_summary: Option<MigrationSummary> },
    Recovery { message: String, backup_paths: Vec<String> },
}

pub struct StartupGate(RwLock<StartupStatus>);

impl StartupGate {
    pub fn new(status: StartupStatus) -> Self { Self(RwLock::new(status)) }
    pub fn status(&self) -> Result<StartupStatus, String> {
        self.0.read().map_err(|error| error.to_string()).map(|status| status.clone())
    }
    pub fn require_ready(&self) -> Result<(), String> {
        match self.status()? {
            StartupStatus::Ready { .. } => Ok(()),
            StartupStatus::Recovery { message, .. } => Err(message),
        }
    }
    pub fn clear_migration_summary(&self) -> Result<(), String> {
        let mut status = self.0.write().map_err(|error| error.to_string())?;
        match &mut *status {
            StartupStatus::Ready { migration_summary } => {
                *migration_summary = None;
                Ok(())
            }
            StartupStatus::Recovery { message, .. } => Err(message.clone()),
        }
    }
}
```

- [ ] **Step 7: Run tests and confirm GREEN**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml secrets::tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml providers::tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" check --manifest-path src-tauri\Cargo.toml
```

Expected: fake credential tests and Provider host-detachment tests pass.

- [ ] **Step 8: Commit Provider security boundary**

Run:

```powershell
git add src-tauri/src/secrets.rs src-tauri/src/providers.rs src-tauri/src/provider_http.rs src-tauri/src/app_state.rs src-tauri/src/lib.rs
git commit -m "feat: add provider credential boundary"
```

---

### Task 5: Implement Startup Classification And Fresh Installation

**Files:**
- Create: `src-tauri/src/startup.rs`
- Reuse: `src-tauri/src/fs_atomic.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write table-driven startup classification tests**

Create tests for all six paths:

```rust
#[test]
fn classifies_startup_matrix() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(classify(dir.path()).unwrap(), StartupPath::FreshInstall);

    std::fs::write(dir.path().join("init-v1.json"), init_sidecar_json("prepared")).unwrap();
    assert_eq!(classify(dir.path()).unwrap(), StartupPath::RecoverInitialization);
    std::fs::remove_file(dir.path().join("init-v1.json")).unwrap();

    std::fs::write(dir.path().join("library.json"), valid_legacy_json()).unwrap();
    assert_eq!(classify(dir.path()).unwrap(), StartupPath::LegacyUpgrade);

    std::fs::write(dir.path().join("migration-v1.json"), sidecar_json("preparing")).unwrap();
    assert_eq!(classify(dir.path()).unwrap(), StartupPath::RecoverMigration);
}
```

Add separate tests for ready v1 and malformed/unsupported files returning `RecoveryRequired`.

- [ ] **Step 2: Run tests and confirm RED**

Register `mod startup;` in `src-tauri/src/lib.rs` before running the filter so `startup::tests` is compiled.

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml startup::tests::classifies_startup_matrix
```

Expected: compilation fails because `startup` types do not exist.

- [ ] **Step 3: Implement the exact startup path enum and classifier**

Create `src-tauri/src/startup.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPath {
    FreshInstall,
    RecoverInitialization,
    LegacyUpgrade,
    ReadyV1,
    RecoverMigration,
    RecoveryRequired,
}

pub fn classify(data_dir: &Path) -> Result<StartupPath, String> {
    let library = data_dir.join("library.json");
    let database = data_dir.join("banana.db");
    let init_sidecar = data_dir.join("init-v1.json");
    let sidecar = data_dir.join("migration-v1.json");
    if init_sidecar.exists() && sidecar.exists() {
        return Ok(StartupPath::RecoveryRequired);
    }
    if init_sidecar.exists() {
        serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string(&init_sidecar).map_err(|e| e.to_string())?,
        ).map_err(|e| e.to_string())?;
        return Ok(StartupPath::RecoverInitialization);
    }
    if sidecar.exists() {
        let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&sidecar).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if value.get("state").and_then(|v| v.as_str()) != Some("complete") {
            return Ok(StartupPath::RecoverMigration);
        }
    }
    match (library.exists(), database.exists()) {
        (false, false) => Ok(StartupPath::FreshInstall),
        (true, false) => Ok(StartupPath::LegacyUpgrade),
        (true, true) => Ok(StartupPath::ReadyV1),
        _ => Ok(StartupPath::RecoveryRequired),
    }
}
```

- [ ] **Step 4: Implement fresh-install seeding**

Add `initialize_fresh` that creates and verifies same-directory temp files, then persists a non-secret `init-v1.json` before either live path changes. The sidecar records old-absent assertions, temp/live hashes, and exact phases `prepared`, `library_switched`, `db_switched`, `complete`; every transition uses `fs_atomic::replace_file`. Create the DB temp and seed two Providers in one transaction:

```sql
INSERT INTO ai_providers
(id, kind, display_name, base_url, models_url, chat_completions_url, default_model,
 available_models_json, structured_mode, bound_host, needs_credentials, credential_ref, created_at, updated_at)
VALUES
('reverse-image', 'reverse-image', '图片反推', 'https://ai.leihuo.netease.com',
 'https://ai.leihuo.netease.com/v1/models', 'https://ai.leihuo.netease.com/v1/chat/completions',
 'doubao-seed-1-6-vision-250815', '[]', NULL, 'https://ai.leihuo.netease.com', 1, NULL, :now, :now),
('storyboard', 'storyboard', '故事板 Agent', '', '', '', NULL, '[]', NULL, NULL, 1, NULL, :now, :now);
```

Serialize `Library::default()` after setting `version = LIBRARY_VERSION`; no API fields or keys may be written by the final library type after Task 8.

Build the reverse-image `bound_host` value with `providers::validated_host_fingerprint` from the same three seeded URLs (the literal above is its asserted result), rather than maintaining a second hostname-only algorithm. Add a test that saves the unchanged seeded Provider with a Key and proves the fingerprint remains equal and the credential is not spuriously detached.

Switch verified `library.json` first and `banana.db` second while the sidecar remains durable, then mark complete, verify both live hashes/schema, and remove the sidecar. `RecoverInitialization` resumes the remaining idempotent phases before classification can mistake a one-file state for legacy/corruption. Because there was provably no pre-existing user data, it may regenerate a missing/invalid unpublished temp from the recorded seed contract; if a live path exists with a hash not recorded by the sidecar, enter Recovery rather than overwrite it.

Add failpoints after `prepared`, after each live switch, after `complete`, and before sidecar removal. Restart each fixture twice and assert it becomes the same Ready fresh install with exactly two Provider rows, no duplicate seed, and no Recovery page. Also assert `(library absent, DB present)` and `(library present, DB absent)` are recovered only when a valid matching init sidecar explains them; without one they remain `RecoveryRequired`/`LegacyUpgrade` according to the matrix.

- [ ] **Step 5: Verify fresh install GREEN**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml startup::tests
```

Expected: all six classification tests and every initialization failpoint pass; fresh install creates readable files and both Provider IDs exactly once.

- [ ] **Step 6: Commit startup matrix and fresh seed**

Run:

```powershell
git add src-tauri/src/startup.rs src-tauri/src/lib.rs
git commit -m "feat: add v1 startup matrix"
```

---

### Task 6: Implement Two-Phase Legacy Migration And Crash Recovery

**Files:**
- Create: `src-tauri/src/migration.rs`
- Reuse: `src-tauri/src/fs_atomic.rs`
- Modify: `src-tauri/src/startup.rs`
- Modify: `src-tauri/src/library.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write sidecar serialization and phase tests**

Use these exact types:

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum MigrationState { Preparing, Prepared, Committing, Complete }

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct MigrationSidecar {
    migration: u32,
    state: MigrationState,
    original_library_hash: String,
    temp_library_hash: Option<String>,
    temp_database_hash: Option<String>,
    backup_path: Option<String>,
    candidate_credential_ref: Option<String>,
    credential_origin_fingerprint: Option<String>,
    summary: Option<crate::app_state::MigrationSummary>,
    #[serde(default)]
    summary_acknowledged: bool,
}
```

Add tests that round-trip each state, preserve an unacknowledged summary across `Complete`, round-trip the non-secret candidate ref/fingerprint, and reject `migration != 1`. The sidecar is the independent durable journal for a migration credential candidate; it never contains credential bytes.

- [ ] **Step 2: Add failure-injection tests before implementation**

Define a crate-visible test-only failpoint contract so `crate::integration_tests` can drive it without exposing production APIs:

```rust
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum MigrationFailpoint {
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
```

Implement the matching `#[cfg(test)] StartupCoordinator::run_with_failpoint` method in `migration.rs`; production builds contain neither the enum nor the harness. Keep the harness inside the crate so `crate::integration_tests` can exercise every phase without turning migration internals into a public API.

For each point, run migration with the injected stop, call recovery, and assert exactly one of these valid outcomes:

- legacy JSON remains untouched and parseable; or
- sanitized v1 JSON and valid v1 DB are both committed.

Also assert every generated backup and committed JSON excludes `"apiKey"` and the test key value. At every failpoint, including `AfterCredential`, restart twice and assert exactly one final active ref or no ref as appropriate, with every abandoned candidate deleted/read back absent from Credential Manager.

- [ ] **Step 3: Run migration tests and confirm RED**

Register `mod migration;` in `src-tauri/src/lib.rs` before running the filter; `db` and `fs_atomic` are already registered by Task 3.

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml migration::tests
```

Expected: compilation fails because migration coordinator functions do not exist.

- [ ] **Step 4: Implement legacy normalization against raw JSON**

In `library.rs`, add a migration-only parser that inspects field presence before typed deserialization:

```rust
pub struct LegacySecrets {
    pub api_base_url: String,
    pub api_key: Option<String>,
    pub reverse_model: String,
    pub available_reverse_models: Vec<String>,
}

pub fn normalize_legacy_json(raw: &str) -> Result<(Library, LegacySecrets, Vec<String>), String> {
    let mut value: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let prompts = value.get_mut("prompts").and_then(|v| v.as_array_mut())
        .ok_or_else(|| "legacy library 缺少 prompts 数组".to_string())?;
    let mut warnings = Vec::new();
    for (index, prompt) in prompts.iter_mut().enumerate() {
        let object = prompt.as_object_mut().ok_or_else(|| "prompt 不是对象".to_string())?;
        if !object.contains_key("favorite") {
            object.insert("favorite".into(), false.into());
            warnings.push(format!("prompt {index} 缺少 favorite，已迁移为 false"));
        }
        if !object.contains_key("order") {
            object.insert("order".into(), (index as i64).into());
        }
    }
    let settings = value.get_mut("settings").and_then(|v| v.as_object_mut())
        .ok_or_else(|| "legacy library 缺少 settings".to_string())?;
    let secrets = LegacySecrets {
        api_base_url: settings.remove("apiBaseUrl").and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_else(default_api_base_url),
        api_key: settings.remove("apiKey").and_then(|v| v.as_str().map(str::to_owned)).filter(|v| !v.is_empty()),
        reverse_model: settings.remove("reverseModel").and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_else(default_reverse_model),
        available_reverse_models: settings.remove("availableReverseModels").and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default(),
    };
    value["version"] = LIBRARY_VERSION.into();
    let library = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok((library, secrets, warnings))
}
```

Make the default helper functions `pub(crate)` so migration can call them. Track `favorites_defaulted` and `orders_rebuilt` as counts while inspecting field presence; the migration summary warning must explicitly say `缺失的 favorite 已按 false 迁移，历史上已经丢失的收藏无法恢复` whenever the first count is nonzero. Do not include prompt titles/content or API fields in warnings.

- [ ] **Step 5: Implement durable helpers**

In `migration.rs`, implement these signature-only helpers with the behavior below:

```text
fn sha256_file(path: &Path) -> Result<String, String>;
fn write_sidecar_atomic(path: &Path, sidecar: &MigrationSidecar) -> Result<(), String>;
fn lock_migration(data_dir: &Path) -> Result<std::fs::File, String>;
fn checkpoint_and_close(connection: rusqlite::Connection) -> Result<(), String>;
fn switch_same_volume(temp: &Path, destination: &Path, expected_hash: &str) -> Result<(), String>;
```

`lock_migration` opens `migration-v1.lock` and calls `fs2::FileExt::try_lock_exclusive`. `write_sidecar_atomic` writes a same-directory temp sidecar, calls `sync_all`, closes it, then calls `fs_atomic::replace_file`; every preparing/prepared/committing/complete transition therefore replaces an existing Windows sidecar safely. `switch_same_volume` closes all source/destination handles, calls the same helper, and accepts an already-switched destination when its hash matches, making recovery idempotent. Never implement either path as delete-then-rename or plain `std::fs::rename(temp, existing)` on Windows.

- [ ] **Step 6: Implement prepare without network access**

`prepare` must perform this exact sequence:

1. Acquire the lock and persist `Preparing` with the original JSON hash.
2. Normalize legacy JSON and retain the original file.
3. Create `banana.db.tmp`, run schema migration, and seed both Providers using `validated_host_fingerprint` for every non-empty endpoint set.
4. If a legacy key exists, acquire the setup-injected `CredentialMutationCoordinator`, generate its UUID candidate ref, persist that ref plus target origin fingerprint to the migration sidecar *before* writing the secret, then write/read it back and make the temp Provider row reference that exact candidate. Never overwrite a stable ref or leave candidate identity only inside the disposable temp DB.
5. Write sanitized `library.json.tmp`.
6. Create a sanitized timestamped backup.
7. Validate JSON, DB schema, foreign keys, Provider row, credential readback, and backup manifest.
8. Build `MigrationSummary` with migrated prompt count, both missing-field counts, backup path, and sanitized warnings; execute `PRAGMA wal_checkpoint(TRUNCATE)`, close the temp DB, hash both temp files, and persist `Prepared` with `summary_acknowledged=false`.

No `ureq` call, model listing, or connection check is permitted in this function.

- [ ] **Step 7: Implement commit and recovery**

`commit` first persists `Committing`, then switches JSON and DB separately using their sidecar hashes, reopens the final DB, validates it, persists `Complete` while retaining the summary, and removes only verified temp files. Keep the complete sidecar until the UI acknowledges the summary; acknowledgement atomically sets `summary_acknowledged=true` rather than deleting recovery evidence.

Startup processes a migration sidecar before generic `credential_cleanup` garbage collection. `recover` takes the same shared credential-mutation lock and uses this exact match:

```rust
match sidecar.state {
    MigrationState::Preparing => cleanup_candidate_then_restart_prepare_without_touching_original(),
    MigrationState::Prepared | MigrationState::Committing => finish_hash_guided_commit(),
    MigrationState::Complete => validate_committed_files(),
}
```

For `Preparing`, delete/read-back absence of any sidecar-recorded candidate before clearing its fields and rebuilding, unless the entire verified temp tuple is deliberately reused. For `Prepared/Committing`, finish the hash-guided commit with the exact candidate as the final Provider ref; after the DB switch generic cleanup recognizes it as referenced. For `Complete`, validate that the committed Provider still references it. If neither original nor committed files match recorded hashes, return recovery-only status and do not write either file or delete an uncertain referenced credential.

- [ ] **Step 8: Route StartupCoordinator through migration**

Define the following signature-only public shape, then implement `run` with the classification behavior below:

```text
pub struct StartupCoordinator {
    credentials: Arc<dyn CredentialStore>,
    credential_mutations: Arc<CredentialMutationCoordinator>,
    operations: Arc<AppOperationGate>,
}
pub enum StartupOutcome {
    Ready { services: AppServices, migration_summary: Option<MigrationSummary> },
    Recovery(RecoveryInfo),
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    pub message: String,
    pub backup_paths: Vec<String>,
}

impl StartupCoordinator {
    pub fn run(&self, data_dir: &Path) -> StartupOutcome;
}
```

`run` dispatches `FreshInstall -> initialize_fresh`, `RecoverInitialization -> recover_initialization`, `LegacyUpgrade -> prepare/commit`, `RecoverMigration -> recover`, and `ReadyV1 -> validate/open`; `RecoveryRequired` never opens services. It then opens the final `Database`, creates the one shared `ProviderHttpClient` and `ProviderService`, and returns `Ready`. A complete migration sidecar returns its summary until `summary_acknowledged=true`; completed fresh initialization and ordinary ready-v1 launches return `None`. All errors become `RecoveryInfo { message, backup_paths }`; none become an empty library.

- [ ] **Step 9: Run crash matrix and all Rust tests**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml migration::tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml startup::tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
```

Expected: every injected crash recovers to a valid pair; no generated artifact contains the test key.

- [ ] **Step 10: Commit migration as one recoverable unit**

Run:

```powershell
git add src-tauri/src/migration.rs src-tauri/src/startup.rs src-tauri/src/library.rs src-tauri/src/lib.rs
git commit -m "feat: add recoverable v1 migration"
```

---

### Task 7: Gate Frontend Startup And Expose Recovery-Only UI

**Files:**
- Create: `src-tauri/src/command_auth.rs`
- Create: `src-tauri/src/commands/startup_commands.rs`
- Create: `src/components/MainRoot.vue`
- Create: `src/components/RecoveryPage.vue`
- Create: `src/components/MigrationSummaryDialog.vue`
- Create: `src/lib/startup-ipc.ts`
- Create: `tests/components/MainRoot.test.ts`
- Create: `tests/components/RecoveryPage.test.ts`
- Create: `tests/components/MigrationSummaryDialog.test.ts`
- Modify: `src/main.ts`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing MainRoot tests**

Test both branches:

```ts
it('mounts App only after startup is ready', async () => {
  mocks.getStartupStatus.mockResolvedValue({ state: 'ready', migrationSummary: null })
  const wrapper = mount(MainRoot, { global: { plugins: [createPinia()] } })
  await flushPromises()
  expect(wrapper.findComponent(App).exists()).toBe(true)
  expect(wrapper.findComponent(RecoveryPage).exists()).toBe(false)
})

it('shows the persisted migration summary before acknowledging it', async () => {
  mocks.getStartupStatus.mockResolvedValue({
    state: 'ready',
    migrationSummary: {
      promptsMigrated: 3,
      favoritesDefaulted: 1,
      ordersRebuilt: 2,
      backupPath: 'C:/BananaBox/upgrade-backup.zip',
      warnings: ['缺失的 favorite 已按 false 迁移，历史上已经丢失的收藏无法恢复'],
    },
  })
  const wrapper = mount(MainRoot, { global: { plugins: [createPinia()] } })
  await flushPromises()
  expect(wrapper.findComponent(App).exists()).toBe(true)
  expect(wrapper.text()).toContain('历史上已经丢失的收藏无法恢复')
})

it('renders recovery without mounting App stores', async () => {
  mocks.getStartupStatus.mockResolvedValue({
    state: 'recovery', message: 'library.json 无法解析', backupPaths: ['C:/backup.zip'],
  })
  const wrapper = mount(MainRoot, { global: { plugins: [createPinia()] } })
  await flushPromises()
  expect(wrapper.findComponent(App).exists()).toBe(false)
  expect(wrapper.text()).toContain('library.json 无法解析')
})
```

- [ ] **Step 2: Run tests and confirm RED**

Run:

```powershell
pnpm exec vitest run tests/components/MainRoot.test.ts tests/components/RecoveryPage.test.ts
```

Expected: module resolution fails for the new components.

- [ ] **Step 3: Add startup IPC and Rust command**

Create `src/lib/startup-ipc.ts`:

```ts
import { invoke } from '@tauri-apps/api/core'

export type StartupStatus =
  | { state: 'ready'; migrationSummary: MigrationSummary | null }
  | { state: 'recovery'; message: string; backupPaths: string[] }

export interface MigrationSummary {
  promptsMigrated: number
  favoritesDefaulted: number
  ordersRebuilt: number
  backupPath: string
  warnings: string[]
}

export function getStartupStatus(): Promise<StartupStatus> {
  return invoke<StartupStatus>('get_startup_status')
}

export function acknowledgeMigrationSummary(): Promise<void> {
  return invoke('acknowledge_migration_summary')
}
```

Before creating the first IPC module, create/register `command_auth.rs` with the exact `IpcCaller`, `require_caller_label`, and non-`Deserialize` authorized-envelope implementation locked in the public contract. Add focused real-handler tests proving malformed payload from the wrong label returns `FORBIDDEN_WINDOW` before serde and malformed main payload returns `INVALID_INPUT`; this module exists from Task 7 onward, before any later Provider/backup/production/Storyboard command imports `MainArgs`.

Create `startup_commands.rs` with `get_startup_status` and `acknowledge_migration_summary`, each using one camelCase/deny-unknown whole-payload `MainArgs` envelope. The acknowledgement command resolves the app-data `migration-v1.json`, requires `state=complete`, atomically writes `summary_acknowledged=true`, and updates the in-memory ready status to `migration_summary=None`; it never deletes data or recovery hashes. Add `mod startup_commands; pub use startup_commands::{get_startup_status, acknowledge_migration_summary};` to `commands.rs`, then add both exported functions to the one `generate_handler!` list in `lib.rs` before running focused tests.

- [ ] **Step 4: Implement MainRoot and RecoveryPage**

`MainRoot.vue` must not render `App` while status is null:

```vue
<script setup lang="ts">
import { onMounted, ref } from 'vue'
import App from '@/App.vue'
import RecoveryPage from '@/components/RecoveryPage.vue'
import MigrationSummaryDialog from '@/components/MigrationSummaryDialog.vue'
import { getStartupStatus, type StartupStatus } from '@/lib/startup-ipc'
const status = ref<StartupStatus | null>(null)
onMounted(async () => { status.value = await getStartupStatus() })
</script>

<template>
  <div v-if="!status" class="startup-loading">正在检查本地数据...</div>
  <template v-else-if="status.state === 'ready'">
    <App />
    <MigrationSummaryDialog
      v-if="status.migrationSummary"
      :summary="status.migrationSummary"
      @acknowledged="status.migrationSummary = null"
    />
  </template>
  <RecoveryPage v-else :status="status" />
</template>
```

`RecoveryPage.vue` shows the message and each safe backup path as selectable text. It has a “重试启动检查” button that reloads the window; it does not offer destructive deletion. `MigrationSummaryDialog.vue` shows migrated prompt count, reconstructed order count, defaulted-favorite count, backup path, and every sanitized warning; its “我知道了” action awaits `acknowledgeMigrationSummary()` and emits `acknowledged` only on success.

- [ ] **Step 5: Wire Tauri setup before normal app use**

In `lib.rs` setup:

```rust
let data_dir = app.path().app_data_dir()?;
let operations = Arc::new(AppOperationGate::default());
let restore_blockers = Arc::new(RestoreBlockerRegistry::default());
let credential_mutations = Arc::new(CredentialMutationCoordinator::default());
app.manage(operations.clone());
app.manage(restore_blockers.clone());
let coordinator = StartupCoordinator::new(
    Arc::new(WindowsCredentialStore),
    credential_mutations.clone(),
    operations.clone(),
);
match coordinator.run(&data_dir) {
    StartupOutcome::Ready { services, migration_summary } => {
        app.manage(StartupGate::new(StartupStatus::Ready {
            migration_summary,
        }));
        app.manage(services);
    }
    StartupOutcome::Recovery(info) => {
        app.manage(StartupGate::new(StartupStatus::Recovery {
            message: info.message,
            backup_paths: info.backup_paths,
        }));
        if let Some(window) = app.get_webview_window("main") { window.show()?; }
    }
}
```

`StartupCoordinator` places that same `operations.clone()` into Ready `AppServices` and injects `credential_mutations.clone()` into the Ready `ProviderService`; assert both identities with `Arc::ptr_eq` in a setup test. The standalone managed gate and blocker registry therefore exist in Recovery too. No branch, command, or feature constructs a second operation gate or credential coordinator.

Only after this block should tray/hotkey registration continue.

- [ ] **Step 6: Mount MainRoot and run GREEN tests**

Change `src/main.ts` to mount `MainRoot` for non-`floatbtn` windows. Run:

```powershell
pnpm exec vitest run tests/components/MainRoot.test.ts tests/components/RecoveryPage.test.ts tests/components/MigrationSummaryDialog.test.ts tests/components/App.test.ts
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml startup
```

Expected: ready mounts `App`; an unacknowledged upgrade summary remains visible across restart until the acknowledgement IPC succeeds; recovery does not call `load_library`; all tests pass.

- [ ] **Step 7: Commit startup gate**

Run:

```powershell
git add src/main.ts src/components/MainRoot.vue src/components/RecoveryPage.vue src/components/MigrationSummaryDialog.vue src/lib/startup-ipc.ts tests/components/MainRoot.test.ts tests/components/RecoveryPage.test.ts tests/components/MigrationSummaryDialog.test.ts src-tauri/src/command_auth.rs src-tauri/src/commands/startup_commands.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat: gate app startup on data recovery"
```

---

### Task 8: Move Reverse Image And Settings To Provider IPC

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Create: `src/types/providers.ts`
- Create: `src/lib/provider-ipc.ts`
- Create: `src/stores/providers.ts`
- Create: `src-tauri/src/commands/provider_commands.rs`
- Modify: `src-tauri/src/provider_http.rs`
- Create: `tests/stores/providers.test.ts`
- Modify: `src/types/index.ts`
- Modify: `src/stores/library.ts`
- Modify: `src/lib/ipc.ts`
- Modify: `src/components/SettingsModal.vue`
- Modify: `src/components/ReverseImagePanel.vue`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/library.rs`
- Modify: `tests/components/SettingsModal.test.ts`
- Modify: `tests/components/ReverseImagePanel.test.ts`

- [ ] **Step 1: Write failing public-contract tests**

In `tests/stores/providers.test.ts`, assert loading Providers stores public data and no secret fields:

```ts
it('stores provider metadata without credential material', async () => {
  mocks.listAiProviders.mockResolvedValue([{ id: 'reverse-image', kind: 'reverse-image', displayName: '图片反推', baseUrl: 'https://api.example.com', modelsUrl: 'https://api.example.com/v1/models', chatCompletionsUrl: 'https://api.example.com/v1/chat/completions', defaultModel: 'vision', availableModels: ['vision'], structuredMode: null, interactiveCompatible: null, boundHost: 'https://api.example.com', needsCredentials: true, configRevision: 1, capabilityRevision: 1 }])
  const store = useProviderStore()
  await store.load('reverse-image')
  expect(store.byId('reverse-image')?.needsCredentials).toBe(true)
  expect(JSON.stringify(store.providers)).not.toContain('apiKey')
  expect(JSON.stringify(store.providers)).not.toContain('credentialRef')
})
```

Update `ReverseImagePanel.test.ts` first to expect:

```ts
expect(mocks.reverseImagePrompt).toHaveBeenCalledWith({
  providerId: 'reverse-image',
  model: 'vision-model',
  imagePath: 'images/source.png',
})
```

Add Rust mock-server tests for `check_provider_connection` and `reverse_image_prompt`. Exercise same-host and cross-host `301`, `302`, `307`, and `308` responses. Every call must return `PROVIDER_REDIRECT_FORBIDDEN`; the `Location` target records zero requests and never observes `Authorization`. Cover never-returning headers, headers with no body chunks, stalled chunked bodies, 2 MiB + 1 decoded chat JSON, compressed expansion over the limit, and parsed content over 1 MiB; expect `PROVIDER_TIMEOUT` or `PROVIDER_RESPONSE_TOO_LARGE` with no partial result/body log. Also assert both commands use the `ProviderHttpClient` instance from `AppServices`, not a command-local client.

- [ ] **Step 2: Run tests and confirm RED**

Run:

```powershell
pnpm exec vitest run tests/stores/providers.test.ts tests/components/ReverseImagePanel.test.ts
```

Expected: missing Provider store and old secret-bearing payload mismatch.

- [ ] **Step 3: Add exact public TypeScript DTOs and IPC**

Create `src/types/providers.ts`:

```ts
export type ProviderKind = 'reverse-image' | 'storyboard'
export type StructuredMode = 'json_schema' | 'strict_json'
export interface AiProvider {
  id: string; kind: ProviderKind; displayName: string; baseUrl: string
  modelsUrl: string; chatCompletionsUrl: string; defaultModel: string | null
  availableModels: string[]; probedModel: string | null; structuredMode: StructuredMode | null
  interactiveCompatible: boolean | null
  boundHost: string | null; needsCredentials: boolean; configRevision: number
  capabilityRevision: number
}
export interface SaveAiProviderInput {
  provider: Omit<AiProvider, 'availableModels' | 'probedModel' | 'structuredMode' | 'interactiveCompatible' | 'boundHost' | 'needsCredentials' | 'configRevision' | 'capabilityRevision'> & {
    confirmCrossOrigin?: boolean
  }
  apiKey?: string
}
```

Create `provider-ipc.ts` wrappers for the locked commands. `saveAiProvider` sends exactly `invoke('save_ai_provider', { input: { ...value.provider, confirmCrossOrigin: value.provider.confirmCrossOrigin ?? false }, apiKey: value.apiKey })`; it may send the newly typed password once, and no wrapper reads a password. Add a fixture proving `confirmCrossOrigin: true` reaches Rust as `SaveProviderInput.confirm_cross_origin=true`, while `probedModel`, `configRevision`, `capabilityRevision`, and every server-owned field are rejected by TypeScript/Rust deny-unknown validation.

- [ ] **Step 4: Add Provider Pinia store**

Implement `load(kind)`, `byId(id)`, `save(input)`, and `clearCredential(id)`. After save, set the local password input to an empty string; never persist it in Pinia.

- [ ] **Step 5: Add Rust Provider commands**

Each command accepts `tauri::WebviewWindow`, `State<StartupGate>`, and one authorized whole-payload envelope; the wrapper authorizes before deserialization, then the body calls `require_ready()`, obtains Ready-only services with `window.app_handle().try_state::<AppServices>()`, acquires `let _permit = services.operations.enter_user()?`, and only then touches state or input-dependent services. The save signature is exactly `save_ai_provider(window, gate, args: MainArgs<SaveAiProviderCommandArgs>)`, where the deny-unknown args struct contains `input: SaveProviderInput` and `api_key: Option<String>` and preserves the frontend `{ input, apiKey }` shape; it passes `api_key.as_deref()` and returns `AiProvider` only. Convert connection checking and reverse-image chat to async commands over `services.provider_http`; remove the old `ureq` call path. After `rg -n "ureq" src-tauri/src` returns no production match, remove `ureq` from `Cargo.toml` and regenerate `Cargo.lock`.

Add `mod provider_commands; pub use provider_commands::{list_ai_providers, save_ai_provider, clear_ai_provider_credential, check_ai_provider_connection};` to `commands.rs` (plus the existing reverse-image command if it moves into this module), and register each export in the single `generate_handler!` list. The focused/full command tests must invoke the registered handler surface, not call an otherwise unreferenced file directly.

Change reverse-image Rust input to:

```rust
pub struct ReverseImagePromptInput {
    pub provider_id: String,
    pub model: String,
    pub image_path: String,
}
```

Inside the command call `services.providers.resolve_for_request(&input.provider_id)?`, require `resolved.provider.kind == ProviderKind::ReverseImage` before reading the image or issuing HTTP, then use its URLs/key locally. A Storyboard Provider ID returns `PROVIDER_KIND_MISMATCH` with zero file/network access. Remove `base_url` and `api_key` from frontend and command DTOs.

- [ ] **Step 6: Remove API data from Library settings**

At the end of the foundation stage, `Settings` on both Rust and TypeScript contains only the existing non-sensitive fields below. The Storyboard plan later adds only its non-sensitive disclosure timestamp with a Rust round-trip test:

```ts
export interface Settings {
  hotkey: string
  theme: 'auto' | 'light' | 'dark'
}
```

Legacy API fields remain readable only through `normalize_legacy_json`. Update defaults and all fixtures accordingly.

- [ ] **Step 7: Update SettingsModal write-only key flow**

On open, load `reverse-image`. Show Base URL, explicit endpoint URLs, model, and a blank password input whose visible hint is “留空表示不修改”。After save, clear the password ref. Connection checking sends only `providerId` and displays returned model IDs.

- [ ] **Step 8: Run frontend and Rust GREEN tests**

Run:

```powershell
pnpm exec vitest run tests/stores/providers.test.ts tests/components/SettingsModal.test.ts tests/components/ReverseImagePanel.test.ts tests/stores/library.test.ts
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
pnpm typecheck
```

Expected: no TS interface or serialized DB/JSON includes `apiKey`; reverse-image tests pass with `providerId`; all redirect cases fail closed without contacting their targets; no production `ureq` dependency remains.

- [ ] **Step 9: Commit Provider IPC conversion**

Run:

```powershell
git add src/types src/lib src/stores src/components/SettingsModal.vue src/components/ReverseImagePanel.vue tests src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/provider_http.rs src-tauri/src/commands src-tauri/src/commands.rs src-tauri/src/library.rs
git commit -m "refactor: move api credentials behind providers"
```

---

### Task 9: Harden Legacy Prompt-Library Import

**Files:**
- Create: `src-tauri/src/legacy_import.rs`
- Create: `src-tauri/src/safe_archive.rs`
- Create: `src-tauri/src/commands/backup_commands.rs`
- Modify: `src-tauri/src/library.rs`
- Modify: `src-tauri/src/startup.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src/lib/backup-ipc.ts`
- Modify: `src/components/SettingsModal.vue`

- [ ] **Step 1: Write unsafe archive and key-stripping tests**

Tests must construct ZIPs in memory and cover these exact assertions:

- `inspect_rejects_parent_path_entry` writes `images/../../escape.txt` and asserts that `inspect` returns an unsafe-path error without creating staged or live files.
- `inspect_strips_legacy_key_before_commit` writes `library.json` with `apiKey="legacy-secret"`, runs inspect and commit with `MemoryCredentialStore`, then asserts that committed JSON contains neither `apiKey` nor `legacy-secret`, the `reverse-image` provider resolves to `legacy-secret`, and the `banana.db` project/task/storyboard row counts are unchanged.

Add cases for absolute paths, symlink Unix mode, duplicate `library.json`, and an image entry larger than the legacy import limit. Run the same ZIP-bomb/Windows-alias matrix later used by full restore: 10,001 entries, 101:1 declared/actual ratio, forged central sizes, cumulative overflow, `Foo`/`foo`, trailing dot/space, reserved device names, and colon ADS. Every case must remove staging and leave live JSON/images/credentials unchanged.

Add a collision fixture with one imported category ID and one prompt ID already present. Assert an explicit `old_category_id -> committed_category_id` map rewrites every imported prompt to the imported category, imported categories/prompts keep source relative order after existing rows, and no prompt points to the pre-existing colliding category. Add crash/failpoint tests after candidate-ref journaling, after candidate secret readback, after Provider-ref switch, after every generated image copy, immediately before JSON switch, and immediately after JSON switch, plus barriers against a concurrent Settings save and clear.

- [ ] **Step 2: Run tests and confirm RED**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml legacy_import::tests
```

Expected: module does not exist.

Register `mod legacy_import; mod safe_archive;` in `lib.rs` before rerunning the focused test; keep both modules crate-private.

- [ ] **Step 3: Implement inspect-only staging**

Define:

```rust
pub struct LegacyImportPreview {
    pub token: String,
    pub prompt_count: usize,
    pub category_count: usize,
    pub has_api_key: bool,
    pub credential_conflict: bool,
    pub warnings: Vec<String>,
}
```

Create `safe_archive.rs` as the only ZIP extraction engine. It accepts an `ArchiveLimits` value plus an entry-policy callback, applies archive/file/count/actual-byte/ratio limits and Windows-normalized collision checks before opening each destination, and returns a verified staged file manifest. `inspect` accepts JSON directly or calls this engine for ZIP with a policy allowing only one exact `library.json` and flat `images/<safe-name>` regular files. The raw `library.json` entry is streamed into a bounded in-memory buffer, parsed, stripped of `apiKey`, normalized, and only then written as sanitized staged JSON; the raw entry and Key bytes are never written to the token, sidecar, log, error, or preview. Images may stream to verified staging. The token metadata records only a canonical source path, source file length/full SHA-256, sanitized manifest/hashes, non-secret preview fields, creation time, and claimed state. No code in `legacy_import.rs` or `backup.rs` may call `ZipArchive::by_index` and write files independently of this engine.

Task 9 already defines and uses the shared v1 limits later listed in Task 10: 2 GiB archive bytes, 10,000 entries, 512 MiB per file, 4 GiB total actual uncompressed bytes, and 100:1 maximum compression ratio. Task 10 reuses the same constant rather than redefining a looser full-restore path.

Add one always-managed `BackupStagingCoordinator` shared by legacy/full preview paths. Tokens are backend-generated UUID v4 values serialized only as the canonical lowercase hyphenated 36-character form; every IPC string must parse and round-trip to that exact form, then look up an internal coordinator record rather than becoming an unchecked path component. Directory/sidecar creation and deletion resolves final Windows handles under fixed staging/sidecar roots, rejects reparse escapes, and never interpolates an unparsed token. Test `..`, drive/UNC/absolute paths, `/`, `\`, `:`, ADS, trailing dot/space, uppercase/noncanonical UUID, alias collision, and a swapped reparse point; every case performs zero deletion/live write.

The coordinator permits at most two unclaimed tokens and at most 4 GiB of **actual extracted bytes in aggregate**, reserves/releases bytes through the same counting writer, gives unclaimed tokens a 30-minute in-process lifetime, and issues an RAII `PreviewClaimGuard` through an atomic single-consumer CAS before commit/restore. A concurrent/replayed claim returns `STALE_PREVIEW_TOKEN`. Before a durable operation sidecar reaches `prepared`, every validation/conflict/maintenance/blocker/I/O error drops the guard and atomically restores `unclaimed` with its original expiry/quota so the user can retry or discard. Once the sidecar is durable, `adopt_by_sidecar()` transfers ownership and only recovery/ack cleanup may release the token. No credential/image/live tuple write is allowed before the sidecar owns the guard. On every startup before classification, remove every unclaimed token because no prior WebView can still own it, and remove a `claimed` token with no referencing sidecar as a crash orphan because the pre-sidecar rule proves it changed no live state; retain tokens referenced by any active legacy-import/full-restore sidecar regardless of age. Failed extraction removes its reservation/directory. `discard_legacy_import_preview(token)` is main-only Ready, operation-permit protected, idempotent for absent/unclaimed tokens, refuses a concurrently claimed or sidecar-owned token with `PREVIEW_IN_USE`, and removes the directory plus quota reservation. Settings calls it on Cancel, close, and replacement by a new preview. Test every pre-sidecar failure, two concurrent commits with one guard winner, crash exactly after claim/before sidecar, and sidecar-owned retention.

- [ ] **Step 4: Implement transactional commit semantics**

`commit(token, overwrite_credential)` uses `library.json` as its commit boundary and is recoverable across process death:

1. Revalidate the sanitized staged manifest/hashes, then open the recorded canonical source once with the same Windows no-follow/final-path rules used by secure Skill import and retain that handle through verification. From that one handle, require exact length/full SHA-256, stream the raw `library.json` into the bounded in-memory parser, recover the legacy Key only in memory, and prove its sanitized normalization hash equals the staged value. Only after all checks pass atomically CAS the still-unclaimed token to `claimed` while retaining the open verified handle/key buffer; then proceed. A missing/path-swapped/changed source returns stable `SOURCE_CHANGED` **before** token claim, import sidecar, candidate ref, Keyring, DB, image, or live JSON writes, leaving the preview discardable/re-inspectable. Build a mapping for every imported category ID: retain a valid unused ID, otherwise generate a UUID. Rewrite every imported prompt `category_id` through that map and reject missing source categories. Apply the same collision rule to prompt IDs. Append imported categories after existing categories and imported prompts after existing prompts in each target category, preserving each imported collection's source `(order, id)` relative order and then normalizing positions.
2. Persist a non-secret `legacy-import-<token>.json` sidecar through `fs_atomic::replace_file`. It records old/new sanitized JSON hashes, exact created image paths, target origin fingerprint, `old_credential_ref`, UUID `candidate_credential_ref`, and phases `prepared`, `candidate_journaled`, `candidate_verified`, `provider_ref_switched`, `images_copied`, `json_switched`, `complete`; it never records or copies either secret value.
3. If a legacy key exists, require explicit overwrite when the target credential already exists. Enter the ProviderService credential-mutation coordinator before reading that binding and keep its shared lock through candidate creation, DB-ref switch, JSON commit/rollback decision, and cleanup journaling. Use the same copy-on-write protocol as `ProviderService::save`: persist the candidate ref in the import sidecar before writing/readback, keep the old referenced credential untouched, then atomically switch the Provider row to the verified candidate. Do not use a rollback credential identity and never access a command-local CredentialStore/lock.
4. Copy images to newly generated safe UUID paths only, track each path in the durable sidecar before it can become live, rewrite prompt paths, and atomically switch the sanitized merged `library.json` last.
5. Startup recovers every legacy-import sidecar under the shared credential lock before normal Provider/library services and before generic `credential_cleanup` GC. If the live JSON hash is still the recorded old hash, delete only this token's created images, atomically restore the Provider row to `old_credential_ref` (or null), enqueue the candidate as retired, delete/read-back absence, and keep the original JSON. If the live JSON hash is the recorded new hash, treat the import as committed, keep the candidate active, enqueue the old ref as retired only now, and finish idempotent bookkeeping. Any other hash enters Recovery instead of guessing or deleting either uncertain ref.
6. After either verified outcome, remove staging and sidecar only after queued cleanup is durable. Cleanup failure after a committed JSON switch returns a warning and is retried from `credential_cleanup` on next startup; it never turns committed success into a rejected IPC that invites a duplicate import.

Failpoint restarts must end in exactly one of two tuples: byte-identical old JSON + old/absent active ref + no token-created images, or committed new JSON + candidate active ref + every rewritten image. In both outcomes every non-active ref is durably queued then absent after cleanup. Concurrent import versus Settings save/clear is serialized by the same lock and ends as one whole ordered tuple, never a mixed ref/endpoint/JSON. After a successful inspect but before commit, recursively scan the complete token tree plus sidecars/log/error captures and assert no legacy/new/old secret sentinel exists. Cover source deletion/change, cancel/discard, two-token/byte quota, 30-minute expiry, crash-startup orphan GC, and an active sidecar token that startup never deletes.

- [ ] **Step 5: Add distinct IPC and UI labels**

Create `backup-ipc.ts` functions `inspectLegacyImport(path)`, `commitLegacyImport(token, overwriteCredential)`, and `discardLegacyImportPreview(token)`. In Settings use the visible label “导入旧版提示词库”; do not call it full restore. Cancel/close awaits best-effort discard before clearing local preview state.

Add `mod backup_commands; pub use backup_commands::{inspect_legacy_import, commit_legacy_import, discard_legacy_import_preview};` to `commands.rs` in Task 9 and register all three exports in `generate_handler!`. Task 10 extends the same module/re-export and the one `generate_handler!` list with `create_full_backup`, `inspect_full_backup`, `restore_full_backup`, `discard_full_backup_preview`, and `acknowledge_full_restore`; it does not create a second command module or handler list.

- [ ] **Step 6: Run legacy import tests GREEN**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml legacy_import::tests
pnpm exec vitest run tests/components/SettingsModal.test.ts
```

Expected: all malicious archives are rejected; committed JSON contains no secret; v1 tables remain unchanged.

- [ ] **Step 7: Commit legacy import hardening**

Run:

```powershell
git add src-tauri/src/legacy_import.rs src-tauri/src/safe_archive.rs src-tauri/src/commands/backup_commands.rs src-tauri/src/startup.rs src-tauri/src/commands.rs src-tauri/src/library.rs src-tauri/src/lib.rs src/lib/backup-ipc.ts src/components/SettingsModal.vue tests/components/SettingsModal.test.ts
git commit -m "fix: harden legacy prompt import"
```

---

### Task 10: Add Full Backup And Staged Restore

**Files:**
- Create: `src-tauri/src/backup.rs`
- Create: `src-tauri/src/backup_validation.rs`
- Create: `src-tauri/src/image_store.rs`
- Modify: `src-tauri/src/safe_archive.rs`
- Modify: `src-tauri/src/app_state.rs`
- Modify: `src-tauri/src/commands/backup_commands.rs`
- Modify: `src-tauri/src/startup.rs`
- Modify: `src-tauri/src/library.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/legacy_import.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/backup-ipc.ts`
- Modify: `src/lib/startup-ipc.ts`
- Modify: `src/components/MainRoot.vue`
- Modify: `src/components/SettingsModal.vue`
- Modify: `src/components/RecoveryPage.vue`
- Create: `src/components/FullRestoreSummaryDialog.vue`
- Modify: `tests/components/RecoveryPage.test.ts`
- Create: `tests/components/FullRestoreSummaryDialog.test.ts`
- Modify: `tests/components/MainRoot.test.ts`

Before the first focused test, add crate-private `mod backup_validation;` in `lib.rs`; re-export only the crate-private `BackupDomainValidator`, `BackupDomainValidatorRegistry`, limits, and safe error needed by later domain modules. Setup constructs/manages exactly one registry before `StartupCoordinator::run`, registers `foundation-v1`, and passes the same Arc to backup commands/startup validation. Add a module/`Arc::ptr_eq` test; no backup path may construct a private registry.

- [ ] **Step 1: Write manifest, snapshot, and resource-limit tests**

Use this manifest contract:

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    format: String,             // "banana-box-v1"
    format_version: u32,        // 1
    schema_version: i64,        // db::schema::SCHEMA_VERSION
    created_at: String,
    files: std::collections::BTreeMap<String, String>, // path -> sha256
}
```

Tests must prove:

- the DB snapshot passes `integrity_check` and contains committed WAL rows;
- snapshot `ai_providers` rows have `credential_ref=NULL` and `needs_credentials=1`;
- after sanitizing or migrating a WAL-mode staged DB, delete/reject any staged `-wal`/`-shm`, reopen the archived main file alone, and still observe the redacted Provider rows and current schema;
- the DB snapshot retains project/task/thread rows plus every `skill_versions.files_json` canonical Markdown body;
- no ZIP entry or manifest text contains a test API key;
- an archive length exactly 2 GiB is accepted by the size gate while 2 GiB + 1 byte is rejected before ZIP parsing; use an injected length reader or sparse-file metadata rather than allocating those bytes;
- 10,001 entries, a declared or actually written 101:1 compression ratio, future schema, checksum mismatch, and DB foreign-key failure are rejected before live-file switching;
- forged central-directory sizes cannot write more than the per-file or cumulative extraction limits; `Foo`/`foo`, trailing dot/space aliases, `CON`/`NUL`/`COM1`, colon ADS names, and any two entries with the same Windows-normalized key are rejected before a staging file is opened;
- adding and deleting an image at every generation failpoint leaves the pointer on a complete old or complete new generation; barrier races around add/delete prove a backup contains either the complete old `(library.json, generation)` tuple or complete new tuple, never a JSON/reference set from one side with image bytes from the other;
- a legacy library with one missing image reference and a restored backup with the same missing reference both remain Ready, preserve that logical reference, and expose a sanitized warning; a file present in a generation/backup manifest with a bad hash is still fatal.
- backup destinations equal to or aliased through a junction/symlink into `library.json`, `banana.db`/WAL/SHM, any startup/restore/import sidecar, `images-current.json`, active/old generations, or internal staging are rejected before opening/truncating the destination; every source byte remains unchanged.

- [ ] **Step 2: Run backup tests and confirm RED**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml backup::tests
```

Expected: module does not exist.

Register `mod backup; mod image_store;` in `lib.rs` before rerunning the focused test; `safe_archive` was registered by Task 9. Add restore-sidecar detection to `startup.rs`; it must run before the live `Database` is opened.

- [ ] **Step 3: Implement backup creation through Database::online_backup**

Before backup/restore code, introduce `ImageStore`. Keep library values as logical `images/<safe-name>` paths, but resolve every read/write/delete/export through an atomically replaced `images-current.json` pointer of shape `{ "version": 1, "generationId": "<uuid>", "manifestSha256": "<sha256>" }`. Physical files live under `image-generations/<generationId>/<safe-name>` and each generation has a canonical sorted filename/SHA-256 manifest whose hash is in the pointer. On first v1 startup without a pointer, first persist `image-adoption-v1.json` with the verified final-path legacy directory, canonical file/hash manifest, target generation, and phases `prepared | pointer_switched | ready_once | gc_complete`; then copy the verified legacy `images/` contents into one generation, verify/write its manifest, atomically write the pointer, and re-resolve every library image. Retain the old directory until either the user acknowledges the related migration/full-restore summary or a second consecutive Ready startup has revalidated the current pointer/manifest/every Library reference. Under the ImageStore write guard, that confirmation atomically marks the journal GC-eligible, deletes only the exact still-matching old manifest directory, read-verifies absence, and then removes the journal; failure keeps both for retry. An unknown/changed/reparse-swapped old directory is never deleted and produces a sanitized retained-backup warning. Add adoption crash at every phase, first/second Ready, summary ACK, GC failure/retry, and same logical filename with different old/new bytes. A missing file referenced by a prompt is a persisted sanitized warning and uses the existing broken-image UI state; it does not invalidate otherwise readable prompt data. Traversal, a bad manifest hash, or bad bytes for a file the manifest claims exists remains fatal.

Treat every published generation as immutable. `ImageStore` owns one snapshot `RwLock` shared by all `library.json` reads/writes, image mutations, and backup capture. A mutation takes the write guard, captures the current pointer, constructs a sibling temporary generation from the captured one (hard-link/reflink unchanged immutable files when supported, otherwise copy), closes/flushes all files, writes/verifies the canonical manifest, and atomically replaces the pointer only if it still names the captured generation. On a compare mismatch, discard the unpublished generation and retry from the new pointer. Never modify, overwrite, or delete a file inside the active generation.

Use a reference-safe publication order under that one write guard. Adding/replacing an image publishes the generation containing its new UUID first, then atomically writes the Library JSON that begins referencing it; a failed JSON write leaves only an unreferenced image and returns failure. Removing a reference atomically writes Library JSON first and treats physical removal as later generation garbage collection, so a post-commit cleanup failure is only a warning and never a fake rejected delete. Non-image Library edits still take the same write guard. On process death at any phase, every JSON reference therefore resolves or was already absent; at worst an unreferenced file remains for verified GC.

Backup takes the snapshot read guard, reads and sanitizes one stable Library value, captures one pointer/manifest, and retains the generation lease through copying and hash verification before releasing the guard. It may take the independent SQLite online snapshot afterward because SQLite rows do not reference Library images. Garbage collection skips all leased/current/rollback generations and removes only files proven unreferenced by the current locked Library plus verified orphan temporaries on a later successful startup.

Before creating an archive, resolve the destination parent with a Windows handle/final path (including reparse points), require the resolved target to be outside the entire app-data tree and all internal staging paths, and reject the live input archive itself. Write with create-new semantics to a random sibling temp in the approved destination directory; stream/close/fsync it, reopen and verify its manifest/hashes, then atomically replace the chosen destination. Failure deletes only that random temp and leaves any previous destination and every live app-data file untouched.

Reuse the Task 4 process-wide `AppOperationGate`: `enter_user()` rejects with `RESTORE_PENDING` when maintenance is set; background scheduler work uses `try_enter_background()` and skips; otherwise each command/internal write holds a permit through its last DB/ImageStore/credential commit. `begin_maintenance()` atomically blocks new permits and waits for the active count to reach zero. A maintenance lease releases the flag on any failure before restore `prepared`; `seal_for_restart()` leaves it set after `prepared` so only relaunch/exit/recovery-safe commands remain.

After startup constructs the verified store, extend the Task 4 shared service contract only with `pub images: Arc<ImageStore>`; its existing `operations` field must remain `Arc::ptr_eq` to the standalone gate managed before startup classification. Commands receive these instances; they never construct another gate/store or resolve physical generation paths themselves. Retrofit every remaining foundation business command to authorize caller, require ready, then acquire an operation permit before state access. Later production, desktop, and Storyboard plans follow the same order.

Add resolver tests for traversal/absolute path rejection, pointer/hash mismatch, first-start adoption with a missing logical reference warning, read/write/delete failpoints, concurrent mutation retry, backup read leases, and two generations with the same logical filename returning different bytes. Add barrier tests pausing add between pointer/JSON and delete between JSON/GC while backup starts; after release, the archive must be exactly the old or new reference-resolving tuple. Update legacy import, every Library save, and every existing image command to use the shared `ImageStore` snapshot guard; the frontend contract remains the stable logical path.

Use a staging directory containing:

```text
manifest.json
library.json
banana.db
images/<files>
```

Call `db.online_backup(staging/banana.db)`, open only the snapshot, and run:

```sql
UPDATE ai_providers
SET credential_ref = NULL, needs_credentials = 1;
DELETE FROM credential_cleanup;
```

Write sanitized `library.json`, copy only regular image files, compute SHA-256 after every file is closed, then write `manifest.json` last. ZIP only files listed in the manifest.

Treat the staged SQLite main file as incomplete until all WAL state is folded into it. After the sanitizing transaction commits, run `PRAGMA wal_checkpoint(TRUNCATE)`, switch the staged copy to `PRAGMA journal_mode=DELETE`, finalize every statement, and close every connection. Require `banana.db-wal` and `banana.db-shm` to be absent (remove only empty checkpoint artifacts); then compute the DB SHA-256. Reopen `banana.db` alone in read-only/offline mode and recheck current schema, `integrity_check`, `foreign_key_check`, and that every Provider has `credential_ref IS NULL AND needs_credentials=1`. Only this post-checkpoint hash may enter `manifest.json`; never package WAL/SHM files.

- [ ] **Step 4: Implement streaming archive limits**

Define/reuse these exact `safe_archive::ArchiveLimits` constants for both legacy ZIP import and full-backup restore:

```rust
const MAX_ARCHIVE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 10_000;
const MAX_SINGLE_FILE: u64 = 512 * 1024 * 1024;
const MAX_TOTAL_UNCOMPRESSED: u64 = 4 * 1024 * 1024 * 1024;
const MAX_COMPRESSION_RATIO: u64 = 100;
const MAX_BACKUP_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const MAX_LIBRARY_JSON_BYTES: u64 = 64 * 1024 * 1024;
const MAX_JSON_DEPTH: usize = 64;
const MAX_LIBRARY_CATEGORIES: usize = 10_000;
const MAX_LIBRARY_PROMPTS: usize = 100_000;
const MAX_STRUCTURED_STRING_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 8192;
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
const MAX_ANIMATED_IMAGE_FRAMES: u32 = 240;
const MAX_ANIMATED_DECODE_PIXELS: u64 = 80_000_000;
```

The 512 MiB generic file cap is not a JSON allocation budget. Before `read_to_string`/serde, stream `manifest.json` and raw/sanitized `library.json` through their smaller byte caps and a depth-counting parser; direct JSON legacy import first checks file metadata and still stops at `MAX_LIBRARY_JSON_BYTES+1` while streaming, so a racy/lying length cannot bypass it. Manifest entries are unique, path-bounded, exact-hash records and may not exceed `MAX_FILES`; Library categories/prompts, every string, nesting depth, and aggregate bytes obey the dedicated limits above. Add limit-minus-one/at-limit/plus-one, deeply nested objects, oversized arrays/strings, duplicate aliases, and high-compression expansion fixtures for direct JSON and ZIP paths. Every rejection has bounded peak memory, removes/resets staging reservation, and changes no live file/row/credential.

Treat image decoding as a separate resource boundary while preserving the real v0.2.2 compatibility surface. Accept PNG, JPEG, WebP, and GIF whose actual magic matches the normalized extension; reject SVG/XML/HTML, unknown/dual magic, truncated metadata, and extension spoofing. Static images enforce each dimension <= `MAX_IMAGE_DIMENSION` and checked width*height <= `MAX_IMAGE_PIXELS`. Animated GIF/WebP remain valid legacy/current assets only when a bounded streaming container parser proves frame count <= `MAX_ANIMATED_IMAGE_FRAMES`, every canvas/frame dimension is bounded, checked sum of composited frame pixels <= `MAX_ANIMATED_DECODE_PIXELS`, loop/chunk tables are structurally valid, and file-byte limits hold; add a reviewed decoder/parser feature/dependency if the existing `image` feature set cannot inspect animated WebP safely. Do not silently turn a v0.2.2-supported GIF/WebP into a broken reference merely because it has multiple frames.

Apply the identical validator to legacy import staging, ImageStore mutations, first adoption, backup creation, full-backup inspect/pre-switch/startup/ack, and image-generation assets. A restore manifest that claims an invalid image is fatal. During first legacy adoption only, an invalid/malicious **referenced** old file is excluded from the new manifest and becomes the established broken-image warning while the unchanged old directory remains under its adoption journal; unreferenced invalid files are ignored/preserved until safe GC. Add huge-dimension/pixel-overflow headers, GIF/WebP frame/decoded-pixel bombs, fake extensions, SVG/HTML, truncated files, and valid static/animated boundary fixtures without fully decoding rejected bytes.

Before opening/parsing the ZIP, read the archive file metadata (or injected reader length) and reject `length > MAX_ARCHIVE_BYTES`. Before extracting each entry, validate `enclosed_name`, Unix mode, file count, declared uncompressed size, cumulative size, and ratio with checked multiplication: `size > max(compressed_size, 1) * MAX_COMPRESSION_RATIO` is rejected, avoiding integer-division truncation.

Canonicalize every entry to a Windows collision key before opening its destination: normalize separators, reject empty/dot components, `:`, ADS, invalid Windows characters, trailing dot/space, and reserved basenames `CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`; then Unicode-case-fold each component and reject a duplicate key. Extraction uses a counting writer/manual copy loop that checks actual bytes before each write against the entry limit, cumulative limit, and actual compression-ratio limit, rather than trusting central-directory sizes. Abort, close handles, and remove the token staging directory on the first violation.

Keep entry policies separate but the extractor identical: full restore permits only its exact manifest-declared tree, while legacy import permits its one JSON plus flat images. Add one table test that feeds every malicious archive to both entry points and asserts the same early error class, bounded actual bytes, complete staging cleanup, and zero live writes.

- [ ] **Step 5: Implement inspect and staged validation**

`inspect_full_backup` reserves space through the Task 9 `BackupStagingCoordinator`, extracts into a random token directory, parses the manifest, rejects `schema_version > SCHEMA_VERSION`, verifies exact file set and hashes, opens staged SQLite, and migrates older supported schema only inside staging. Any migration/sanitization write must use the same checkpoint-close-no-sidecar/offline-reopen routine from backup creation; recompute the post-migration DB hash and store that hash in the restore sidecar, never reuse the archive's pre-migration hash. Then run `integrity_check`, `foreign_key_check`, schema checks, and Provider redaction against the main file with WAL/SHM removed. The token uses the same maximum-two/4-GiB aggregate quota, 30-minute lifetime, single-consumer claim, startup orphan GC, and active-sidecar retention rules as legacy import.

Create an always-managed foundation `BackupDomainValidatorRegistry` before startup classification, parallel to `RestoreBlockerRegistry`. A registered `BackupDomainValidator` has a stable unique domain name and a pure read-only `validate(&rusqlite::Connection, BackupValidationLimits) -> Result<(), SafeBackupValidationError>` method; it may return only domain/table/safe row ID/stable code, never TEXT/JSON bodies. Reject duplicate registrations and run domains in stable-name order. Validators stream rows with bounded SQL columns and enforce the **same per-row/per-JSON** UTF-8, depth, array/map, and document-byte caps as every corresponding write path before allocation; they impose no global row-count or total-live-DB cap, so long-term legitimate use cannot make the next startup fail merely by accumulating rows. The archive/DB file limits bound untrusted backup total size; backup creation that exceeds them returns `BACKUP_TOO_LARGE` before producing an archive, without making normal Ready startup fail. Foundation registers built-ins for every foundation-owned structured TEXT field, including typed/unique/bounded `ai_providers.available_models_json`, Provider model/probe consistency, credential redaction, and core schema versions. Later modules register their own typed validators without making Foundation import future Storyboard types. Add unlimited-small-row streaming, per-row limit ±1, restart, and oversized backup-creation tests.

The same registry is mandatory at four boundaries: `inspect_full_backup`; the final revalidation under maintenance immediately before persisting/switching `restore-v1.json`; startup validation of whichever old/new tuple would become Ready; and acknowledgement's current-reachable-closure validation. It reads only a closed offline staged/snapshot DB except acknowledgement's transactionally stable current DB connection. Any missing required domain registration in the final v1 registry, malformed JSON, unknown union/schema/protocol version, over-limit collection/text, hash/path mismatch, or column/JSON fence mismatch returns a sanitized blocker and performs zero live writes. Foundation fake-domain tests inject malformed/oversized rows; the Storyboard plan later adds concrete workflow/message/request/Skill fixtures, and Integration asserts the final registry contains every expected domain before any backup/restore handler is exposed.

Pair the DB registry with one bounded foundation `validate_library` used by ordinary startup, legacy normalization before staging, backup creation, full-backup inspect, pre-switch revalidation, startup selected-tuple validation, and acknowledgement. It validates the actual typed Library schema/version; unique category/prompt IDs; every prompt's category ownership; finite/non-negative, deterministic order values; category/prompt/string/count/aggregate-byte limits; and Windows-safe normalized logical `images/<safe-name>` references with no traversal, absolute path, alias collision, or direct physical path. It then verifies referenced files only through the selected ImageStore manifest/hash closure. A logical reference absent from an otherwise valid manifest remains the designed sanitized warning; a duplicate alias, unsafe path, or manifest-claimed file with wrong bytes is fatal. Add cross-boundary fixtures for duplicate IDs, missing category, invalid/duplicate order, oversized values, traversal/alias paths, allowed missing image, and claimed bad hash; no boundary may use a looser one-off JSON parse.

Implement/register `discard_full_backup_preview(token)` beside the legacy discard command. It is main-only and allowed in Ready or Recovery because the preview UI exists in both; it uses the always-managed staging coordinator/operation gate, is idempotent for an absent unclaimed token, rejects `PREVIEW_IN_USE` for a claimed or `restore-v1.json`-referenced token, and removes staging plus its quota reservation. Settings and RecoveryPage invoke it on Cancel, close, replacing a preview, and component unmount. Tests repeat inspect/cancel until quotas are reused, crash and restart with orphan tokens, and prove active restore/legacy sidecar tokens are never garbage-collected.

Return:

```rust
pub struct FullBackupPreview {
    pub token: String,
    pub created_at: String,
    pub prompt_count: i64,
    pub project_count: i64,
    pub task_count: i64,
    pub thread_count: i64,
    pub skill_version_count: i64,
    pub providers_need_credentials: bool,
    pub supersede_recovery_required: bool,
    pub recovery_set_fingerprint: Option<String>,
    pub warnings: Vec<String>,
}
```

Define deny-unknown `RestoreFullBackupCommandArgs { token, confirm_forward_only, live_tuple_fingerprint, confirm_supersede_recovery, recovery_set_fingerprint }` inside the main-authorized envelope. Supersede confirmation is accepted only when preview returned the matching recovery-set requirement/fingerprint. Forward-only confirmation uses the separate two-step result below because inspect cannot know whether a maintenance-exclusive live rollback capture will succeed; the backend never trusts a client-synthesized fingerprint.

Return a typed union from restore: `RestoreFullBackupResult::RequiresForwardOnly { live_tuple_fingerprint, recovery_set_fingerprint, warnings }` or `RestartScheduled`. On the first restore attempt, if maintenance-exclusive online capture/validation cannot produce a complete old DB/Library/image tuple, perform zero sidecar/live/credential writes, drop the preview claim guard back to unclaimed, release maintenance, and return `RequiresForwardOnly`. Only a second explicit call with `confirm_forward_only=true` and both exact fingerprints may proceed; it reacquires maintenance, re-captures/revalidates the same failure state and recovery set, and returns `STALE_LIVE_TUPLE`/`STALE_RECOVERY_SET` without writes if either changed or became independently repairable. Add writer/repair barriers between calls, failed capture, invalid confirmation, and successful forward-only preparation.

- [ ] **Step 6: Implement restore sidecar and atomic image-generation switching**

Lock one strategy; do not implement per-image replacement. Extract and verify all backup images into a new immutable `image-generations/<new-generation-id>` directory, write its sorted manifest, and prepare a new `images-current.json.restore.tmp` that points to that generation. The current generation remains untouched.

Lock startup sidecar arbitration before staging. **Any existing `restore-v1.json`** has absolute classifier priority over initialization, migration, legacy-import, and image-adoption journals, including unreadable/corrupt bytes and `complete` with `cleanup_acknowledged=false`. Startup processes only that restore until a verified acknowledgement cleanup atomically removes the marker; complete-unacknowledged enters Ready only to show its summary/ack and never resumes an older journal first. Preview computes one `RecoverySetFingerprint` from the stable sorted typed path/hash/phase records for every current recovery journal plus the captured live-tuple fingerprint. If a restore marker exists, normal `restore_full_backup` returns `RESTORE_ALREADY_PENDING`; if only another unfinished/unacknowledged journal exists, it returns `RECOVERY_CONFLICT`. Offer a separate visibly high-risk “使用新备份强制救援” confirmation only after preview; its input requires `confirmSupersedeRecovery=true` and the exact complete `RecoverySetFingerprint`. Under maintenance, re-enumerate and require byte-for-byte fingerprint equality before writing a new marker; a newly added/removed/changed journal or live tuple returns `STALE_RECOVERY_SET` with no sidecar/live write. A false/missing/stale confirmation changes nothing. Test complete-unacknowledged plus each older sidecar across restart and prove only the restore summary/ack path runs.

Under maintenance plus the credential coordinator, confirmed supersede uses one typed `RecoverySidecarPaths` registry shared by classifier, inventory, and GC; its exact patterns are `init-v1.json`, `migration-v1.json`, `legacy-import-<canonical-token>.json`, `image-adoption-v1.json`, and `restore-v1.json`. It enumerates every active match before any new switch and has a filename-contract test so no recovery phase is silently omitted. Before replacing a prior fixed restore marker, copy its exact bytes with create-new semantics to a random immutable internal quarantine path, fsync/close/reopen/hash it, and include that quarantine path/hash plus its artifact closure in `SupersededRecoveryInventory`. Build the complete new marker at a sibling temp, fsync/close/reopen/validate its inventory, and only then atomically replace the fixed `restore-v1.json`; at every crash boundary the fixed marker is the complete old or complete new file, never absent or half-written. The new inventory also contains each older sidecar path/hash/phase, verified old/new artifact hashes, created images/generations/legacy-image directory manifests, and every non-secret old/candidate/retired/current credential ref known only to those journals. Older sidecars/quarantined marker are retained and never executed while superseded. If an older/corrupt sidecar cannot be semantically parsed, retain its entire path/hash/artifact tree as `untrusted`, make the new restore forward-only, warn that unknown credentials/storage will be preserved, and never guess-delete them. Add failpoints after quarantine create/write/fsync/readback, inventory persist, new-marker fsync, and atomic replace. This allows emergency rescue without losing prior marker bytes or candidate refs that never reached the old DB.

After the selected backup tuple is Ready, acknowledgement preserves every currently active credential, current/leased/rollback/adoption generation, and any unknown superseded artifact. It durably retires only known unreferenced candidate/old refs, deletes only proven superseded created artifacts, then removes superseded sidecars/inventory entries after read-back. Add each init/migration/import/adoption phase -> Recovery full restore -> two restarts, candidate-only Keyring ref, partial/corrupt prior restore, default second-restore refusal, stale/valid high-risk confirmation, and crash immediately after new highest-priority sidecar. Every result is the complete chosen backup tuple or Recovery, never execution of an older journal over it.

Before persisting `prepared`, capture an internally consistent rollback tuple rather than copying the live SQLite main file. `restore_full_backup` is the sole exception to the ordinary user-permit template: it must not hold an `OperationPermit` and then wait for itself. It first validates only its immutable staged token, then calls the standalone managed `AppOperationGate::begin_maintenance()` in both Ready and Recovery to reject new business IPC/background writes and wait for every in-flight DB/ImageStore/credential permit. Under the exclusive lease it revalidates the token, manifest, staged DB, and every staged hash before reading live state. It then asks the foundation-owned `RestoreBlockerRegistry`; any active participant releases maintenance and returns that sanitized blocker. Foundation tests use a fake participant, so this task has no compile-time dependency on the later `AgentRuntime`; the Storyboard plan registers that concrete blocker and its tests later.

Then in Ready obtain `app.try_state::<AppServices>().map(|services| (services.db.clone(), services.images.clone()))`; `Database`/`ImageStore` are not separately managed states. Call `Database::online_backup` to `restore-old/banana.db`, checkpoint/close/no-sidecar/offline-verify/hash that snapshot, and capture old Library JSON plus image pointer/generation under the ImageStore snapshot read lock. This preserves committed rows that still live in WAL. Under Recovery, no `AppServices` is required and normal services are already absent: the standalone validator may open the live SQLite set including WAL/SHM only to perform the same online-backup capture. If DB/JSON/pointer validation cannot produce a complete old tuple, use the `RequiresForwardOnly` two-step fingerprint handshake above; only its successful second call records `old_tuple_available=false`. Never infer confirmation during inspect or claim rollback availability from a raw main-file copy.

Create `restore-v1.json` with `old_tuple_available`, source hashes, verified old rollback paths/hashes for `library.json`, offline `banana.db`, and `images-current.json` when available, and the exact non-secret credential-ref inventory from that verified old offline DB (or `null` when untrusted). Build the inventory as the union of non-null active `ai_providers.credential_ref` and every `credential_cleanup.credential_ref`, retaining each ref's source/reason (`active`, `candidate`, or `retired`) so a pre-existing cleanup failure is not lost when the restored backup DB clears its journal. Also record the new generation ID/manifest hash, new temp hashes, selected outcome, `cleanup_acknowledged=false`, and exact phases `prepared`, `json_switched`, `db_switched`, `images_pointer_switched`, `complete`. Persist every transition through `fs_atomic::replace_file`. `restore_full_backup` only stages/validates, captures the rollback metadata, persists `prepared`, seals maintenance for restart, and requests restart. Before `prepared`, any error releases the lease and normal work may resume; after `prepared`, relaunch failure leaves the app visibly restore-pending and every normal command returns `RESTORE_PENDING` until restart. Before opening the live DB, startup revalidates all old/new artifacts, then switches the same-directory JSON temp, DB temp, and finally image pointer, persisting the phase after each successful switch.

Recovery never exposes a mixed tuple to normal startup. If every remaining new artifact is valid, resume forward to the fully new JSON/DB/image generation. If forward completion is impossible and `old_tuple_available=true` with all rollback artifacts valid, atomically restore all three old members in reverse order using the offline backup DB. When old tuple is unavailable, recovery is forward-only; if any verified new artifact cannot complete, remain on RecoveryPage and never guess from the raw live main/WAL. If neither permitted full tuple validates, do not open `Database` or resolve images. Only after the selected tuple passes JSON parse, SQLite integrity/foreign keys, pointer/manifest hashes, every manifest-listed image, and cross-file row/count checks may startup persist `complete` and become ready with a durable `FullRestoreSummary`. Logical prompt references absent from that verified manifest remain unchanged and produce `FullBackupPreview.warnings` plus the summary warning; they are not treated as corrupt archive bytes. Delete neither generation, rollback DB, sidecar, nor any recorded credential ref until the user acknowledges that summary.

`acknowledge_full_restore` is a main-only Ready command and holds a normal user-operation permit plus the shared credential coordinator. Successful restore startup publishes a managed `FullRestoreLineage { token, selected_outcome, selected_base_hashes }` only after it has verified and opened the chosen tuple; acknowledgement requires that lineage token/outcome to match the still-complete sidecar and strictly re-hashes the immutable rollback evidence, but it does **not** require the live DB/JSON/image-pointer bytes to remain equal to their switch-time hashes. Ready startup may already have registered a bundled Skill, marked requests interrupted, written reminders, accepted Provider credentials, or produced a descendant ImageStore generation before the user acknowledges.

Under the credential coordinator and the ImageStore snapshot/write guard, acknowledgement validates the **current reachable closure** at its own linearization point: current SQLite schema/integrity/foreign keys are valid; current Library JSON parses; its current pointer, immutable manifest/hash, every referenced image, and generation lease are valid; and current active Provider refs are read from one DB snapshot. For a forward outcome with a verified old-ref inventory, compare the entire old active+candidate+retired union against those current active refs; preserve every current ref regardless of old reason, and durably insert every other old ref into the current DB `credential_cleanup` as `reason='retired'` before delete/read-back (the sidecar retains its original source classification; the cleanup table's CHECK remains only candidate/retired). Retry failures from that journal on startup; only after every queued ref is absent may it garbage-collect the immutable old offline DB and generations proven unreachable from the **current** locked pointer/Library, then mark/remove the sidecar. For rollback, retain currently referenced old refs and current/leased generations and garbage-collect only verified unreachable new artifacts. If `old_tuple_available=false`/old refs are unknown, never guess-delete credentials; display the retained-cleanup limitation and clean only proven unreachable new artifacts. A failure leaves `cleanup_acknowledged=false` and all rollback evidence/journal entries for idempotent retry. Add an old-DB fixture containing one active, one candidate, and one retired cleanup ref, plus restore -> startup Skill/interrupted repair -> reminder write -> Library/image COW edit -> Provider re-entry -> acknowledgement. Race acknowledgement against each Provider/ImageStore/business mutation and assert existing coordinator/store/DB locks produce one whole before/after state; cleanup never deletes a ref or generation selected by the winning current snapshot. Manage `FullRestoreLineage` as a separate Ready-only Tauri state and obtain it with `AppHandle::try_state::<FullRestoreLineage>`; do not add it to the locked five-field `AppServices` or require it in Recovery. Extend Ready `StartupStatus`/MainRoot with `FullRestoreSummaryDialog` and this command.

Export all five Task 10 commands from `commands::backup_commands` and register them in the application's single `tauri::generate_handler!` list. Add an IPC registration test that invokes `acknowledge_full_restore` through the real handler table, plus authorization tests proving `main` may call it only in Ready while `floatbtn`, `reminder`, unknown labels, and Recovery receive the stable sanitized error before sidecar, database, Keyring, or filesystem adapters are touched. This registration is part of the locked public IPC contract above; a summary-dialog button may never depend on an unregistered test-only function.

Add failpoints after `prepared`, each of the three switches, before `complete`, and through acknowledgement cleanup (journal insert, Keyring delete/readback, old DB/image GC, sidecar removal), plus corrupt-new, corrupt-old, and old-tuple-unavailable cases. Seed committed live rows that exist in WAL before capture; after every rollback-capable failpoint the old offline snapshot must retain them. Restart after every failpoint and assert the ready app sees either the complete verified old tuple or fully verified new tuple, never new JSON/DB with old/missing images. Forward-only fixtures must finish new or remain Recovery, never report a lossy rollback. Test forward cleanup, rollback retention, cleanup failure, unknown-old-ref no-delete, and repeated restart/ack idempotence; every known orphan credential is eventually absent and every current ref remains. This avoids replacing/copying an open WAL main file and makes the image collection switch atomic from the application's observable startup boundary.

Add foundation maintenance barrier races for Library/image add/delete, Provider save/clear, legacy import, and a fake long-lived restore blocker. Either the writer acquires a permit first and its committed bytes/ref are included in the old rollback tuple, or maintenance wins and the user command returns `RESTORE_PENDING`/background tick skips. No successful post-snapshot write may later disappear during rollback. Assert one restore call never self-deadlocks, a fake blocker releases maintenance with its stable error, and restore-versus-writer has one linearization order. Production, reminder, and Agent-specific barriers belong to their later plans and the final integration suite.

- [ ] **Step 7: Add full-backup IPC to Settings and recovery-only UI**

Expose separate labels “创建完整备份” and “恢复 v1 完整备份”。After restore preview, show counts, every sanitized missing-image warning, and “API Key 不在备份中，恢复后需要重新填写”。When `supersedeRecoveryRequired=true`, Settings/RecoveryPage show a separate high-risk recovery-conflict confirmation and return the exact preview `recoverySetFingerprint`; ordinary restore remains disabled until it is accepted. If the first restore IPC returns `RequiresForwardOnly`, retain the same preview/token, show the explicit “旧数据无法验证，本次恢复不可回滚” warning, and only a second confirmation sends both returned `liveTupleFingerprint` and `recoverySetFingerprint` plus any already-required supersede confirmation. `STALE_LIVE_TUPLE`, `STALE_RECOVERY_SET`, `RECOVERY_CONFLICT`, `RESTORE_ALREADY_PENDING`, or write/relaunch failure returns to a re-inspect/continue state and never shows success. Only `RestartScheduled` invokes Tauri relaunch. Preview Cancel/close/unmount calls `discard_full_backup_preview` before clearing an unclaimed token; the intermediate confirmation paths never discard it. After successful startup, `FullRestoreSummaryDialog` shows forward/rollback outcome, retained rollback evidence, warnings, and a “确认新数据可用并清理回滚副本” action wired to `acknowledge_full_restore`; it closes only after cleanup/journaling succeeds.

Add identical Settings/RecoveryPage component tests for reject/accept supersede, first-call forward-only requirement, combined supersede+forward-only confirmations, stale fingerprints, default second-restore refusal, cancel/discard, persistence/relaunch failure, and exact-one relaunch only on `RestartScheduled`.

`RecoveryPage` must also offer “检查并恢复 v1 完整备份” because Settings is not mounted in recovery. Split the handlers so `inspect_full_backup`, `restore_full_backup`, and `discard_full_backup_preview` inject only their always-managed operation/staging/blocker/validator dependencies, `StartupGate`, and app-data paths; they must not require a Ready `AppServices`. In Ready, restore may use `AppHandle::try_state::<AppServices>()` only after maintenance is exclusive; in Recovery it uses the standalone offline capture path, while discard touches staging only. Authorize exactly these three main-window commands in recovery, while create-backup, legacy merge/discard, Provider, and all other business commands remain `STARTUP_NOT_READY`. The recovery flow inspects, shows the same preview/warnings, allows cancel/discard, asks for explicit confirmation, stages `restore-v1.json`, and relaunches; it never mutates live library/DB/image members inside the preview IPC calls.

- [ ] **Step 8: Run backup and Settings tests GREEN**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml backup::tests
pnpm exec vitest run tests/components/SettingsModal.test.ts tests/components/RecoveryPage.test.ts tests/components/FullRestoreSummaryDialog.test.ts tests/components/MainRoot.test.ts
```

Expected: snapshot/limits/restore-sidecar, copy-on-write generation, Windows archive collision, missing-reference warning, and recovery-safe authorization tests pass; UI keeps legacy import distinct from full restore and can stage a full restore while startup is in Recovery.

- [ ] **Step 9: Commit full backup as a separate feature**

Run:

```powershell
git add src-tauri/src/backup.rs src-tauri/src/backup_validation.rs src-tauri/src/image_store.rs src-tauri/src/safe_archive.rs src-tauri/src/app_state.rs src-tauri/src/commands/backup_commands.rs src-tauri/src/startup.rs src-tauri/src/library.rs src-tauri/src/commands.rs src-tauri/src/legacy_import.rs src-tauri/src/lib.rs src/lib/backup-ipc.ts src/lib/startup-ipc.ts src/components/MainRoot.vue src/components/SettingsModal.vue src/components/RecoveryPage.vue src/components/FullRestoreSummaryDialog.vue tests/components/MainRoot.test.ts tests/components/SettingsModal.test.ts tests/components/RecoveryPage.test.ts tests/components/FullRestoreSummaryDialog.test.ts
git commit -m "feat: add staged v1 backup recovery"
```

---

### Task 11: Enable CSP And Split Window Capabilities

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/command_auth.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/commands/*.rs`
- Create: `src-tauri/capabilities/main.json`
- Create: `src-tauri/capabilities/floatbtn.json`
- Delete: `src-tauri/capabilities/default.json`
- Delete: `src-tauri/capabilities/desktop.json`
- Create: `tests/config/security.test.ts`

- [ ] **Step 1: Write failing configuration tests**

Create `tests/config/security.test.ts`:

```ts
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

const readJson = (path: string) => JSON.parse(readFileSync(resolve(process.cwd(), path), 'utf8'))

describe('Tauri security configuration', () => {
  it('enables a non-null CSP', () => {
    const config = readJson('src-tauri/tauri.conf.json')
    expect(config.app.security.csp).toContain("default-src 'self'")
    expect(config.app.security.csp).toContain("object-src 'none'")
  })

  it('does not grant filesystem, dialog, updater, or process access to floatbtn', () => {
    const capability = readJson('src-tauri/capabilities/floatbtn.json')
    expect(capability.windows).toEqual(['floatbtn'])
    expect(capability.permissions).toEqual([
      'core:app:allow-register-listener',
      'core:app:allow-remove-listener',
      'core:event:allow-listen',
      'core:event:allow-unlisten',
      'core:event:allow-emit-to',
      'core:window:allow-start-dragging',
    ])
  })
})
```

Add a table-driven Rust authorization test for labels `main`, `floatbtn`, `reminder`, and `unknown`. Call representative sensitive command wrappers with forbidden labels: Provider save, full restore, and legacy import commit must return exactly `FORBIDDEN_WINDOW` before repository/dialog mocks are touched. With caller `main`, assert `inspect_full_backup`/`restore_full_backup` are permitted under Ready or Recovery without constructing `AppServices`, while Provider save, create backup, and legacy import still return `STARTUP_NOT_READY` under Recovery before service access. The integration plan later repeats this against project deletion, settlement, and Skill activation after those commands exist.

- [ ] **Step 2: Run test and confirm RED**

Run:

```powershell
pnpm exec vitest run tests/config/security.test.ts
```

Expected: CSP is null and `floatbtn.json` does not exist.

- [ ] **Step 3: Add production and development CSP**

Replace the security block with:

```json
"security": {
  "csp": "default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost blob: data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
  "devCsp": "default-src 'self' http://127.0.0.1:1422; connect-src 'self' ipc: http://ipc.localhost ws://127.0.0.1:1422 http://127.0.0.1:1422; img-src 'self' asset: http://asset.localhost blob: data:; style-src 'self' 'unsafe-inline'; font-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"
}
```

Model network requests remain in Rust, so arbitrary Provider hosts do not belong in WebView `connect-src`.

- [ ] **Step 4: Create least-privilege capability files**

Create `main.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-start-dragging",
    "core:window:allow-start-resize-dragging",
    "opener:default",
    "clipboard-manager:default",
    "dialog:default",
    "process:default",
    "autostart:default",
    "updater:default"
  ]
}
```

Create `floatbtn.json` with only the six permissions asserted by the test. The app-listener pair is required by the existing native drag/drop listener, listen/unlisten by Rust-to-floatbtn events, emit-to by the retained `floating-file-dropped` event to main, and start-dragging by pointer drag. Do not grant broad `core:default`, unrestricted emit, `fs`, `dialog`, `process`, `updater`, `autostart`, or `clipboard-manager` to `floatbtn`.

Audit and, if needed, modify the Task 7 `command_auth.rs`; do not recreate or replace it. Migrate every remaining foundation-owned custom command to exactly one `MainArgs<WholeCommandArgs>`/surface-appropriate authorized envelope and remove every ordinary deserializable user payload parameter plus the now-unused old `require_caller(WebviewWindow, ...)` helper/imports; retain only `require_caller_label` used by `AuthorizedArgs`. Capability JSON and backend pre-deserialization caller authorization are separate mandatory layers. After envelope extraction, all normal commands call `require_ready()`; only the explicitly named standalone `inspect_full_backup`, `restore_full_backup`, and `discard_full_backup_preview` call `require_ready_or_recovery()` and operate without live services. The real-handler security table includes malformed/missing/raw payloads from every wrong surface and asserts `FORBIDDEN_WINDOW`, not a framework serde error.

- [ ] **Step 5: Delete broad capabilities and run GREEN tests**

Delete `default.json` and `desktop.json`, then run:

```powershell
pnpm exec vitest run tests/config/security.test.ts
& "$env:USERPROFILE\.cargo\bin\cargo.exe" check --manifest-path src-tauri\Cargo.toml
pnpm tauri build --debug
```

Expected: configuration test passes; Tauri accepts CSP/capability schema; debug build succeeds.

- [ ] **Step 6: Commit security configuration**

Run:

```powershell
git add src-tauri/tauri.conf.json src-tauri/capabilities src-tauri/src/command_auth.rs src-tauri/src/commands.rs src-tauri/src/commands src-tauri/src/lib.rs tests/config/security.test.ts
git commit -m "security: enable csp and split capabilities"
```

---

### Task 12: Run Foundation Regression And Security Audit

**Files:**
- Modify only if a verification command exposes a defect in files already named by this plan.

- [ ] **Step 1: Scan for secret-bearing frontend contracts**

Run:

```powershell
rg -n "apiKey|api_key|credentialRef|credential_ref" src tests src-tauri/src
```

Expected: frontend matches occur only in write-only save input/tests; Rust matches occur only in migration, Provider service, credential service, and redaction tests. No serialized public Provider response includes these fields.

- [ ] **Step 2: Scan plans and implementation for unsafe fallback**

Run:

```powershell
rg -n 'unwrap_or_else\(\|_\| Library::default|csp"\s*:\s*null|\.\.\\|\.\./' src-tauri/src src-tauri/tauri.conf.json
```

Expected: no silent malformed-library fallback, null CSP, or archive path concatenation appears.

- [ ] **Step 3: Run the complete automated suite**

Run:

```powershell
pnpm check
& "$env:USERPROFILE\.cargo\bin\cargo.exe" fmt --manifest-path src-tauri\Cargo.toml -- --check
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
& "$env:USERPROFILE\.cargo\bin\cargo.exe" check --manifest-path src-tauri\Cargo.toml
```

Expected: every command exits 0.

- [ ] **Step 4: Exercise the six startup fixtures**

Use Rust integration fixtures, not the real user data directory:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml startup::tests -- --nocapture
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml migration::tests -- --nocapture
```

Expected output identifies and passes: fresh install, interrupted fresh-initialization recovery, legacy upgrade, ready v1, interrupted migration recovery, and recovery-only corrupt data.

- [ ] **Step 5: Verify the final diff and commit any test-only audit additions**

Run:

```powershell
git diff --check
git status --short
git log --oneline -12
```

Expected: no whitespace errors; only intentional foundation files remain uncommitted. If Task 12 added new audit tests, commit them with:

```powershell
git add tests src-tauri/src
git commit -m "test: verify v1 foundation recovery"
```

## Execution Notes

- Execute tasks in order. Tasks 7–10 depend on the `AppServices`, `ProviderService`, and migration contracts established earlier.
- Never test migration against `%APPDATA%/banana-box`; use `tempfile::tempdir()` fixtures containing copies.
- Never place a real API key in a test fixture, command line, snapshot, log, or commit.
- A network outage must not fail fresh initialization, migration commit, backup creation, or restore validation.
- Before each commit, run the targeted RED/GREEN command named in that task; before handing off the branch, run all Task 12 commands.
