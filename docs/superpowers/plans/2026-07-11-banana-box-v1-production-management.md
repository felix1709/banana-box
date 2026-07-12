# Banana Box v1 Production Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the local project board, eight-stage overlapping timeline, daily task ledger, exact daily-report copy workflow, settlement snapshots, idempotent carry-forward, and the page-entry contract used by the 18:00 reminder.

**Architecture:** Keep the legacy prompt library in `library.json` and persist production-management data in the shared SQLite `banana.db`. Rust owns validation, transactions, stable ordering, report snapshots, and carry conflict resolution; Vue/Pinia owns filters, editing state, accessible presentation, and navigation. This plan reuses the storage foundation's `db/mod.rs` API (`open`, `with_connection`, `with_transaction`, `online_backup`) and verifies the production-table contract already created by the single v1 migration.

**Tech Stack:** Tauri 2, Rust 2021, rusqlite bundled SQLite, serde, chrono, uuid, sha2, Vue 3, TypeScript, Pinia, Vitest, Vue Test Utils, lucide-vue-next, CSS Grid.

---

## Scope And Foundation Contract

Run the v1 storage-foundation plan before this plan. It must provide this signature-only API contract:

```text
impl Database {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, String>;
    pub fn with_connection<T>(
        &self,
        operation: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
    ) -> Result<T, String>;
    pub fn with_transaction<T>(
        &self,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String>;
    pub fn with_immediate_transaction<T>(
        &self,
        operation: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String>;
    pub fn online_backup(&self, destination: &std::path::Path) -> Result<(), String>;
}
```

The single migration remains `src-tauri/migrations/0001_v1.sql` and is loaded by `src-tauri/src/db/schema.rs` with `include_str!`. Do not create `src-tauri/src/db.rs` or a second version-1 migration.

Every production Tauri command receives `tauri::WebviewWindow`, `tauri::State<StartupGate>`, and one foundation `MainArgs<WholeCommandArgs>` envelope, but never an ordinary deserializable payload parameter or required `State<AppServices>`. It strictly performs envelope caller authorization before payload deserialization → `gate.require_ready()?` → `window.app_handle().try_state::<AppServices>().ok_or(STARTUP_NOT_READY)?` → `let _permit = services.operations.enter_user()?` → business input/state access, then clones `services.db` before `spawn_blocking`; the RAII permit remains in the async command scope across `.await` until the final repository/navigation result. The foundation manages one Ready `AppServices`, not a separate `Arc<Database>` or operation-gate state; wrong-window malformed input, recovery, and maintenance must reject with the stable errors rather than serde/state framework details.

The reminder scheduler and reminder-window lease/ACK logic are implemented by the desktop-reminder plan. This plan supplies the stable integration contract:

```text
Rust helper: async daily_tasks::navigation::navigate_to_daily_tasks(app, local_date)
Tauri event: open-daily-tasks
Payload: { "localDate": "YYYY-MM-DD" }
Frontend target: ui.openDailyTasks(localDate)
```

## File Map

**Shared storage and registration**

- Verify: `src-tauri/migrations/0001_v1.sql` - confirm the storage foundation already creates the production-management tables exactly once.
- Modify: `src-tauri/src/db/schema.rs` - assert that the foundation migration creates and constrains these tables.
- Modify: `src-tauri/src/lib.rs` - register project/daily modules and IPC handlers against the foundation-owned Ready services.
- Modify: `src-tauri/Cargo.toml` - ensure the dependency list contains exactly one `sha2 = "0.10"` entry for carry snapshot hashes.
- Create: `src-tauri/src/production_backup_validator.rs` - read-only `production-v1` backup/startup semantic invariants.

**Project backend**

- Create: `src-tauri/src/projects/mod.rs` - Tauri commands and public module exports.
- Create: `src-tauri/src/projects/model.rs` - DTOs, eight stage keys, input validation, and derived stage status.
- Create: `src-tauri/src/projects/repository.rs` - transactional project/stage queries.
- Create: `src-tauri/src/projects/tests.rs` - repository and validation tests.

**Daily-task backend**

- Create: `src-tauri/src/daily_tasks/mod.rs` - Tauri commands and exports.
- Create: `src-tauri/src/daily_tasks/model.rs` - day/group/task DTOs and settlement inputs.
- Create: `src-tauri/src/daily_tasks/repository.rs` - CRUD, stable ordering, historical-date reads, and locking.
- Create: `src-tauri/src/daily_tasks/report.rs` - pure exact-string report formatting.
- Create: `src-tauri/src/daily_tasks/carry.rs` - settlement, snapshots, idempotent carry, and conflict resolution.
- Create: `src-tauri/src/daily_tasks/navigation.rs` - common page-entry helper for the reminder action.
- Create: `src-tauri/src/daily_tasks/tests.rs` - CRUD, report, settlement, carry, and navigation-domain tests.

**Frontend domain and state**

- Create: `src/domain/production.ts` - shared project/daily types, fixed accessible palette, derived stage status, and date helpers.
- Create: `src/lib/productionIpc.ts` - typed IPC wrappers for every production-management command.
- Create: `src/stores/projects.ts` - project loading, filtering, editing, and stage-column projection.
- Create: `src/stores/dailyTasks.ts` - selected date, stable groups, edits, copies, settlement, conflicts, and reopen state.
- Modify: `src/stores/ui.ts` - add `projects` and `daily-tasks` tools plus the dated daily-task navigation action.

**Frontend project UI**

- Create: `src/components/projects/ProjectBoardPage.vue` - fixed eight-column board, filters, archive visibility, and horizontal overflow.
- Create: `src/components/projects/ProjectEditorDialog.vue` - project fields, main stage, independent stage dates/progress, and file relinking.
- Create: `src/components/projects/ProjectTimeline.vue` - overlapping stage rows, today line, and separate actual-progress markers.

**Frontend daily UI**

- Create: `src/components/daily/DailyTasksPage.vue` - date navigation, task creation, history, whole-report copy, settlement, and reopen controls.
- Create: `src/components/daily/DailyTaskGroup.vue` - stable group/task ordering, per-group copy, progress, note, invested time, and updated time.
- Create: `src/components/daily/DailySettlementDialog.vue` - incomplete-task carry selection and explicit conflict resolution.
- Modify: `src/components/AppSidebar.vue` - add Project Management and Daily Tasks entries.
- Modify: `src/App.vue` - render both pages and consume `open-daily-tasks`.

**Tests**

- Create: `tests/domain/production.test.ts`
- Create: `tests/stores/projects.test.ts`
- Create: `tests/stores/dailyTasks.test.ts`
- Create: `tests/components/ProjectBoardPage.test.ts`
- Create: `tests/components/ProjectTimeline.test.ts`
- Create: `tests/components/DailyTasksPage.test.ts`
- Create: `tests/components/DailySettlementDialog.test.ts`
- Modify: `tests/components/AppSidebar.test.ts`
- Modify: `tests/components/App.test.ts`
- Modify: `tests/stores/ui.test.ts`

## IPC Contract

Every production-management command is main-window-only. Inject `tauri::WebviewWindow` plus one foundation `MainArgs<WholeCommandArgs>` that authorizes the source label before deserializing the complete payload; the function body then performs `StartupGate`, Ready-service lookup, operation permit, business validation, and `spawn_blocking` in that order. Tests call project deletion and settlement with malformed as well as valid payloads from `floatbtn`/`reminder` and expect `FORBIDDEN_WINDOW` with no serde detail or repository invocation. A second table covers correct-label `INVALID_INPUT`, Recovery `STARTUP_NOT_READY`, and restore-maintenance `RESTORE_PENDING`, all with no validation/repository/navigation call.

Use exactly these command names so the reminder and integration plans can depend on them:

```text
list_projects
create_project
update_project
save_project_with_stages
set_project_stage
archive_project
delete_project
load_daily_task_day
create_daily_task
update_daily_task
delete_daily_task
reorder_daily_groups
reorder_daily_tasks
get_daily_report
settle_daily_task_day
reopen_daily_task_day
open_daily_tasks_page
```

Use exactly these Pinia names:

```text
useProjectsStore       store id: projects
useDailyTasksStore     store id: dailyTasks
```

### Task 1: Verify The Unified V1 Production Schema Contract

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Verify: `src-tauri/migrations/0001_v1.sql`

- [ ] **Step 1: Add a schema contract test for all production tables**

Append this test to the existing `db::schema` test module:

```rust
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
        assert!(names.iter().any(|name| name == required), "missing {required}");
    }
}
```

- [ ] **Step 2: Run the schema contract test against the completed foundation**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml db::schema::tests::production_management_schema_creates_all_tables -- --exact
```

Expected: PASS. A missing table means the storage-foundation implementation does not satisfy its locked v1 contract; stop this plan and correct the foundation migration rather than adding a second `CREATE TABLE` here.

- [ ] **Step 3: Confirm the existing migration matches the exact project DDL contract**

Compare the existing table definitions in `0001_v1.sql` with this contract reference. Each table must be created once by the foundation migration; do not paste a duplicate definition into the file:

```sql
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
```

- [ ] **Step 4: Confirm the existing migration matches the exact daily-task DDL contract**

```sql
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
```

- [ ] **Step 5: Add constraint assertions for code uniqueness, progress, and date order**

Add a second test that inserts a project, then verifies `l36` conflicts with `L36`, progress `101` fails, overlapping stages succeed, and only a stage whose own start exceeds its end fails. Use `rusqlite::params!` and assert `is_err()` or `is_ok()` for each statement. Every assertion must pass against the foundation schema; a failure is fixed in the foundation migration before any repository work begins.

- [ ] **Step 6: Run all schema tests**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml db::schema::tests
```

Expected: all `db::schema::tests` PASS; no duplicate-table, `foreign key mismatch`, or missing-table error.

- [ ] **Step 7: Commit the unified schema contribution**

```powershell
git add src-tauri/src/db/schema.rs
git commit -m "test(db): verify production schema contract"
```

### Task 2: Implement The Project Domain And Repository

**Files:**
- Create: `src-tauri/src/projects/model.rs`
- Create: `src-tauri/src/projects/repository.rs`
- Create: `src-tauri/src/projects/tests.rs`
- Create: `src-tauri/src/projects/mod.rs`
- Modify: `src-tauri/src/lib.rs`

Create `projects/mod.rs` with `pub mod model; pub mod repository; #[cfg(test)] mod tests;`, and add crate-private `mod projects;` in `lib.rs` before the first focused test is expected to compile.

- [ ] **Step 1: Write failing tests for the eight stages and the required middle-cut stage**

Create `src-tauri/src/projects/tests.rs` with these assertions:

```rust
#[test]
fn creating_project_seeds_fixed_eight_stages_in_order() {
    let db = test_database();
    let project = repository::create_project(&db, create_input("L36")).unwrap();
    assert_eq!(project.stages.len(), 8);
    assert_eq!(project.stages[0].stage_key, StageKey::Storyboard);
    assert_eq!(project.stages[3].stage_key, StageKey::MiddleCut);
    assert_eq!(project.stages[7].stage_key, StageKey::FinalComposite);
    assert!(project.stages.iter().all(|stage| stage.progress == 0));
}

#[test]
fn project_codes_are_unique_ignoring_ascii_case() {
    let db = test_database();
    repository::create_project(&db, create_input("L36")).unwrap();
    let error = repository::create_project(&db, create_input("l36")).unwrap_err();
    assert_eq!(error, "项目编号已存在");
}
```

The local helpers are concrete:

```rust
fn test_database() -> crate::db::Database {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.keep().join("banana.db");
    crate::db::Database::open(path).unwrap()
}

fn create_input(code: &str) -> CreateProjectInput {
    CreateProjectInput {
        code: code.to_string(),
        version: "v1".to_string(),
        name: "三丽鸥短片".to_string(),
        file_path: r"C:\work\L36".to_string(),
        release_date: "2026-07-31".to_string(),
        main_stage_key: StageKey::Storyboard,
        stages: STAGE_KEYS.iter().enumerate().map(|(position, &stage_key)| {
            SaveProjectStageInput {
                stage_key,
                start_date: format!("2026-07-{:02}", position + 1),
                end_date: format!("2026-07-{:02}", position + 8),
                progress: 0,
            }
        }).collect(),
    }
}

fn set_range(
    db: &crate::db::Database,
    project_id: &str,
    stage_key: StageKey,
    start_date: &str,
    end_date: &str,
    progress: i64,
) -> Result<ProjectDto, String> {
    repository::set_project_stage(
        db,
        SetProjectStageInput {
            project_id: project_id.to_string(),
            stage_key,
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            progress,
        },
    )
}

fn stage(project: &ProjectDto, stage_key: StageKey) -> &ProjectStageDto {
    project
        .stages
        .iter()
        .find(|stage| stage.stage_key == stage_key)
        .unwrap()
}

fn update_main_stage(project: &ProjectDto, main_stage_key: StageKey) -> UpdateProjectInput {
    UpdateProjectInput {
        project_id: project.id.clone(),
        code: project.code.clone(),
        version: project.version.clone(),
        name: project.name.clone(),
        file_path: project.file_path.clone(),
        release_date: project.release_date.clone(),
        main_stage_key,
    }
}
```

- [ ] **Step 2: Run the project tests and confirm the module is missing**

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml projects::tests
```

Expected: compilation FAIL because `projects`, `StageKey`, and repository functions do not exist.

- [ ] **Step 3: Define the exact stage enum and project DTOs**

Create `model.rs` with `#[serde(rename_all = "snake_case")]` stage values and `#[serde(rename_all = "camelCase")]` DTO fields:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKey {
    Storyboard,
    FirstCut,
    Refinement,
    MiddleCut,
    Effects,
    ArtTitles,
    Music,
    FinalComposite,
}

pub const STAGE_KEYS: [StageKey; 8] = [
    StageKey::Storyboard,
    StageKey::FirstCut,
    StageKey::Refinement,
    StageKey::MiddleCut,
    StageKey::Effects,
    StageKey::ArtTitles,
    StageKey::Music,
    StageKey::FinalComposite,
];

impl StageKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Storyboard => "storyboard",
            Self::FirstCut => "first_cut",
            Self::Refinement => "refinement",
            Self::MiddleCut => "middle_cut",
            Self::Effects => "effects",
            Self::ArtTitles => "art_titles",
            Self::Music => "music",
            Self::FinalComposite => "final_composite",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStageDto {
    pub id: String,
    pub stage_key: StageKey,
    pub position: i64,
    pub start_date: String,
    pub end_date: String,
    pub progress: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub code: String,
    pub version: String,
    pub name: String,
    pub file_path: String,
    pub file_exists: bool,
    pub release_date: String,
    pub main_stage_key: StageKey,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub stages: Vec<ProjectStageDto>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectInput {
    pub code: String,
    pub version: String,
    pub name: String,
    pub file_path: String,
    pub release_date: String,
    pub main_stage_key: StageKey,
    pub stages: Vec<SaveProjectStageInput>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectInput {
    pub project_id: String,
    pub code: String,
    pub version: String,
    pub name: String,
    pub file_path: String,
    pub release_date: String,
    pub main_stage_key: StageKey,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetProjectStageInput {
    pub project_id: String,
    pub stage_key: StageKey,
    pub start_date: String,
    pub end_date: String,
    pub progress: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectStageInput {
    pub stage_key: StageKey,
    pub start_date: String,
    pub end_date: String,
    pub progress: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectWithStagesInput {
    pub project_id: String,
    pub code: String,
    pub version: String,
    pub name: String,
    pub file_path: String,
    pub release_date: String,
    pub main_stage_key: StageKey,
    pub archived: bool,
    pub stages: Vec<SaveProjectStageInput>,
}
```

Also define `CreateProjectInput`, `UpdateProjectInput`, `SetProjectStageInput`, and the atomic editor inputs above. Lock shared write/backup-validator constants in `projects/model.rs`: `MAX_PROJECT_CODE_BYTES=32`, `MAX_PROJECT_VERSION_BYTES=64`, `MAX_PROJECT_NAME_BYTES=800`, and `MAX_PROJECT_FILE_PATH_BYTES=32*1024`. Count UTF-8 bytes after trimming (code remains ASCII), reject NUL/C0 controls, and validate `project_id` as UUID, required strings, every required date with `chrono::NaiveDate`, progress `0..=100`, and only `start_date > end_date` inside the same stage. Both `CreateProjectInput.stages` and `SaveProjectWithStagesInput.stages` must contain every `STAGE_KEYS` value exactly once and no unknown/duplicate key; sort by the fixed definition order server-side and never trust client order. No command or repository helper accepts a missing/blank/null stage date. Do not compare one stage's dates with another, so cross-stage overlap remains valid. Add every limit at -1/exact/+1 for create/update/atomic save and assert zero rows on rejection.

- [ ] **Step 4: Implement transactional project creation and stage seeding**

In `repository.rs`, make `create_project` call `db.with_transaction`. Validate and sort the input's complete eight-stage array, insert the project, and then insert exactly those eight stage rows. Map SQLite unique-constraint errors on `projects.code` to `项目编号已存在`.

The stage insert must use this value order:

```rust
for (position, stage) in validated_stages.iter().enumerate() {
    transaction.execute(
        "INSERT INTO project_stages
         (id, project_id, stage_key, position, start_date, end_date, progress, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            uuid::Uuid::new_v4().to_string(),
            project_id,
            stage.stage_key.as_str(),
            position as i64,
            stage.start_date,
            stage.end_date,
            stage.progress,
            now,
        ],
    ).map_err(map_sql_error)?;
}
```

- [ ] **Step 5: Implement list, update, stage update, archive, and delete repository operations**

Use this signature-only repository contract and implement each body in this step:

```text
pub fn list_projects(db: &Database) -> Result<Vec<ProjectDto>, String>;
pub fn create_project(db: &Database, input: CreateProjectInput) -> Result<ProjectDto, String>;
pub fn update_project(db: &Database, input: UpdateProjectInput) -> Result<ProjectDto, String>;
pub fn save_project_with_stages(db: &Database, input: SaveProjectWithStagesInput) -> Result<ProjectDto, String>;
pub fn set_project_stage(db: &Database, input: SetProjectStageInput) -> Result<ProjectDto, String>;
pub fn archive_project(db: &Database, project_id: &str, archived: bool) -> Result<ProjectDto, String>;
pub fn delete_project(db: &Database, project_id: &str) -> Result<(), String>;
```

Always read stages with `ORDER BY position`. Return `file_exists` from `std::path::Path::new(&file_path).exists()`. Updating `main_stage_key` changes only `projects.main_stage_key`; updating a stage requires and changes that row's two dates/progress.

`save_project_with_stages` is the only ProjectEditor save path. In one `with_immediate_transaction`, it validates the complete input, upserts the project by the frontend-generated stable `project_id`, then inserts/updates all eight stage rows and returns the reloaded project before commit. A retry with the same ID after a lost response is idempotent; the same code on a different ID still returns `项目编号已存在`. No dialog code may call create/update followed by eight `set_project_stage` requests.

- [ ] **Step 6: Add tests for overlap, invalid self-range, and independent main-stage progress**

Add atomic-editor failpoint tests: inject a failure after project upsert and after each of the eight stage writes. For a new UUID, every failure leaves no project/stage row and retrying the identical input succeeds once without code collision. For an existing project, every failure leaves all prior fields/stages unchanged. Simulate a successful commit with a lost response, retry the same UUID, and assert one project with exactly eight stages.

```rust
#[test]
fn stages_may_overlap_but_each_stage_must_have_a_valid_range() {
    let db = test_database();
    let project = repository::create_project(&db, create_input("L36")).unwrap();
    set_range(&db, &project.id, StageKey::Storyboard, "2026-07-01", "2026-07-10", 80).unwrap();
    set_range(&db, &project.id, StageKey::FirstCut, "2026-07-05", "2026-07-14", 30).unwrap();
    let error = set_range(&db, &project.id, StageKey::Refinement, "2026-07-20", "2026-07-19", 0).unwrap_err();
    assert_eq!(error, "阶段开始日期不能晚于结束日期");
}

#[test]
fn changing_main_stage_does_not_change_any_stage_progress() {
    let db = test_database();
    let project = repository::create_project(&db, create_input("L36")).unwrap();
    set_range(&db, &project.id, StageKey::Storyboard, "2026-07-01", "2026-07-10", 65).unwrap();
    let changed = repository::update_project(&db, update_main_stage(&project, StageKey::Effects)).unwrap();
    assert_eq!(changed.main_stage_key, StageKey::Effects);
    assert_eq!(stage(&changed, StageKey::Storyboard).progress, 65);
    assert_eq!(stage(&changed, StageKey::Effects).progress, 0);
}
```

- [ ] **Step 7: Expose typed async Tauri project commands**

Create `projects/mod.rs` and wrap repository calls with `tauri::async_runtime::spawn_blocking`, authorizing, checking the startup gate, acquiring the operation permit, and cloning `services.db` before entering the blocking closure. Export these command functions with exact names:

```rust
async fn run_db<T, F>(db: std::sync::Arc<Database>, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&Database) -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || operation(&db))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn list_projects(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<ListProjectsCommandArgs>,
) -> Result<Vec<ProjectDto>, String> {
    let ListProjectsCommandArgs {} = args.0;
    gate.require_ready()?;
    let services = window.app_handle().try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    run_db(services.db.clone(), repository::list_projects).await
}

#[tauri::command]
pub async fn create_project(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<CreateProjectCommandArgs>,
) -> Result<ProjectDto, String> {
    gate.require_ready()?;
    let services = window.app_handle().try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    let input = args.0.input;
    run_db(services.db.clone(), move |db| repository::create_project(db, input)).await
}

#[tauri::command]
pub async fn update_project(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<UpdateProjectCommandArgs>,
) -> Result<ProjectDto, String> {
    gate.require_ready()?;
    let services = window.app_handle().try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    let input = args.0.input;
    run_db(services.db.clone(), move |db| repository::update_project(db, input)).await
}

#[tauri::command]
pub async fn save_project_with_stages(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<SaveProjectWithStagesCommandArgs>,
) -> Result<ProjectDto, String> {
    gate.require_ready()?;
    let services = window.app_handle().try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    let input = args.0.input;
    run_db(services.db.clone(), move |db| repository::save_project_with_stages(db, input)).await
}

#[tauri::command]
pub async fn set_project_stage(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<SetProjectStageCommandArgs>,
) -> Result<ProjectDto, String> {
    gate.require_ready()?;
    let services = window.app_handle().try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    let input = args.0.input;
    run_db(services.db.clone(), move |db| repository::set_project_stage(db, input)).await
}

#[tauri::command]
pub async fn archive_project(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<ArchiveProjectCommandArgs>,
) -> Result<ProjectDto, String> {
    gate.require_ready()?;
    let services = window.app_handle().try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    let ArchiveProjectCommandArgs { project_id, archived } = args.0;
    run_db(services.db.clone(), move |db| {
        repository::archive_project(db, &project_id, archived)
    }).await
}

#[tauri::command]
pub async fn delete_project(
    window: tauri::WebviewWindow,
    gate: tauri::State<'_, StartupGate>,
    args: MainArgs<DeleteProjectCommandArgs>,
) -> Result<(), String> {
    gate.require_ready()?;
    let services = window.app_handle().try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    let project_id = args.0.project_id;
    run_db(services.db.clone(), move |db| repository::delete_project(db, &project_id)).await
}
```

Define every `*CommandArgs` above with camelCase/deny-unknown and the complete existing invoke object (`{}` for list; `{ input }`; or `{ projectId, archived }`); the wrapper keeps those frontend shapes unchanged. Import `crate::command_auth::MainArgs`; for all seven handlers, caller authorization occurs inside envelope extraction before malformed input can deserialize, then the body reaches the startup gate/permit. Register the module and all seven handlers in `src-tauri/src/lib.rs`. Add a real-handler table invoking valid, missing-field, wrong-type, unknown-field, and raw payloads from `floatbtn`, `reminder`, and unknown labels; every wrong caller returns `FORBIDDEN_WINDOW` before gate/database mocks, while malformed main input returns `INVALID_INPUT`. Add a pause inside `spawn_blocking`: restore must wait until that command commits and drops its permit, while a command started after maintenance returns `RESTORE_PENDING` with zero repository calls.

- [ ] **Step 8: Run project tests and the complete Rust suite**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml projects::tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
```

Expected: project tests PASS; complete Rust suite exits with code `0`.

- [ ] **Step 9: Commit the project backend**

```powershell
git add src-tauri/src/projects src-tauri/src/lib.rs
git commit -m "feat(projects): add project and stage repository"
```

### Task 3: Define Frontend Production Types And Fixed Accessible Stages

**Files:**
- Create: `src/domain/production.ts`
- Create: `tests/domain/production.test.ts`

- [ ] **Step 1: Write failing tests for labels, colors, contrast, and derived status**

```ts
import { describe, expect, it } from 'vitest'
import { STAGE_DEFINITIONS, stageStatus } from '@/domain/production'

describe('production stage definitions', () => {
  it('keeps the confirmed eight stages including 中版', () => {
    expect(STAGE_DEFINITIONS.map((stage) => stage.label)).toEqual([
      '分镜', '初版', '精修', '中版', '特效', '美术字', '音乐', '合成终版',
    ])
  })

  it('keeps every fixed foreground/background pair at 4.5:1 or higher', () => {
    for (const stage of STAGE_DEFINITIONS) {
      expect(stage.contrastRatio).toBeGreaterThanOrEqual(4.5)
    }
  })

  it('derives status only from independent progress', () => {
    expect(stageStatus(0)).toBe('not_started')
    expect(stageStatus(1)).toBe('in_progress')
    expect(stageStatus(99)).toBe('in_progress')
    expect(stageStatus(100)).toBe('completed')
  })
})
```

- [ ] **Step 2: Run the domain test and confirm the import failure**

```powershell
pnpm test -- tests/domain/production.test.ts
```

Expected: FAIL with `Failed to resolve import "@/domain/production"`.

- [ ] **Step 3: Define the exact fixed stage mapping**

```ts
export const STAGE_DEFINITIONS = [
  { key: 'storyboard', label: '分镜', color: '#F4C430', textColor: '#17212B', contrastRatio: 9.92 },
  { key: 'first_cut', label: '初版', color: '#1D4ED8', textColor: '#FFFFFF', contrastRatio: 6.70 },
  { key: 'refinement', label: '精修', color: '#0F766E', textColor: '#FFFFFF', contrastRatio: 5.47 },
  { key: 'middle_cut', label: '中版', color: '#15803D', textColor: '#FFFFFF', contrastRatio: 5.02 },
  { key: 'effects', label: '特效', color: '#C2410C', textColor: '#FFFFFF', contrastRatio: 5.18 },
  { key: 'art_titles', label: '美术字', color: '#BE123C', textColor: '#FFFFFF', contrastRatio: 6.29 },
  { key: 'music', label: '音乐', color: '#6D28D9', textColor: '#FFFFFF', contrastRatio: 7.10 },
  { key: 'final_composite', label: '合成终版', color: '#334155', textColor: '#FFFFFF', contrastRatio: 10.35 },
] as const

export type StageKey = (typeof STAGE_DEFINITIONS)[number]['key']
export type StageStatus = 'not_started' | 'in_progress' | 'completed'

export function stageStatus(progress: number): StageStatus {
  if (progress === 0) return 'not_started'
  if (progress === 100) return 'completed'
  return 'in_progress'
}
```

- [ ] **Step 4: Add the exact project and daily DTO types to the same file**

Define `Project`, `ProjectStage`, `ProjectFilter`, `DailyTaskDay`, `DailyTaskGroup`, `DailyTask`, `DailyReportResult`, `CarrySelection`, `CarryConflict`, and `SettlementResult` using the Rust camelCase payload names. Use `number` for integer progress/minutes/positions; project-stage dates are required `string`, while nullable timestamps, project links, source IDs, hashes, and snapshots use `string | null`.

The required project shape is:

```ts
export interface Project {
  id: string
  code: string
  version: string
  name: string
  filePath: string
  fileExists: boolean
  releaseDate: string
  mainStageKey: StageKey
  archived: boolean
  createdAt: string
  updatedAt: string
  stages: ProjectStage[]
}

export interface ProjectStage {
  id: string
  stageKey: StageKey
  position: number
  startDate: string
  endDate: string
  progress: number
  updatedAt: string
}

export interface CreateProjectInput {
  code: string
  version: string
  name: string
  filePath: string
  releaseDate: string
  mainStageKey: StageKey
  stages: Array<{
    stageKey: StageKey
    startDate: string
    endDate: string
    progress: number
  }>
}

export interface UpdateProjectInput extends Omit<CreateProjectInput, 'stages'> {
  projectId: string
}

export interface SetProjectStageInput {
  projectId: string
  stageKey: StageKey
  startDate: string
  endDate: string
  progress: number
}

export interface SaveProjectWithStagesInput extends Omit<CreateProjectInput, 'stages'> {
  projectId: string
  archived: boolean
  stages: Array<{
    stageKey: StageKey
    startDate: string
    endDate: string
    progress: number
  }>
}

export interface ProjectFilter {
  query: string
  stageKey: StageKey | 'all'
  releaseDate: string
  archived: boolean | 'all'
}

export interface DailyTask {
  id: string
  title: string
  progress: number
  note: string
  investedMinutes: number
  position: number
  sourceTaskId: string | null
  sourceSnapshotHash: string | null
  createdAt: string
  updatedAt: string
}

export interface DailyTaskGroup {
  id: string
  code: string
  projectId: string | null
  position: number
  tasks: DailyTask[]
}

export interface DailyTaskDay {
  id: string
  localDate: string
  settledAt: string | null
  reportSnapshot: string | null
  groups: DailyTaskGroup[]
}

export interface DailyReportResult { text: string; taskCount: number }
export type CarryConflictResolution = 'keep_target' | 'overwrite_target'
export interface CarrySelection {
  sourceTaskId: string
  carry: boolean
  resolution: CarryConflictResolution | null
}
export interface CarryConflict {
  sourceTaskId: string
  targetTaskId: string
  targetDate: string
}
export interface SettlementResult {
  settled: boolean
  reportSnapshot: string
  settledAt: string | null
  conflicts: CarryConflict[]
  day: DailyTaskDay
}
```

- [ ] **Step 5: Run the domain test and typecheck**

```powershell
pnpm test -- tests/domain/production.test.ts
pnpm typecheck
```

Expected: domain tests PASS and `vue-tsc --noEmit` exits with code `0`.

- [ ] **Step 6: Commit the production domain contract**

```powershell
git add src/domain/production.ts tests/domain/production.test.ts
git commit -m "feat(ui): define production domain types"
```

### Task 4: Add Project IPC, Pinia State, And Filters

**Files:**
- Create: `src/lib/productionIpc.ts`
- Create: `src/stores/projects.ts`
- Create: `tests/stores/projects.test.ts`

- [ ] **Step 1: Write failing store tests for filtering and single-column placement**

Mock `@/lib/productionIpc`, hydrate projects in two stages, and assert:

```ts
it('places a project only in its selected main-stage column', () => {
  const store = useProjectsStore()
  store.hydrate([project({ id: 'p1', mainStageKey: 'effects' })])
  expect(store.projectsByStage.effects.map((item) => item.id)).toEqual(['p1'])
  expect(Object.values(store.projectsByStage).flat().filter((item) => item.id === 'p1')).toHaveLength(1)
})

it('filters by code or name, stage, release date, and archive state', () => {
  const store = useProjectsStore()
  store.hydrate([
    project({ id: 'p1', code: 'L36', name: '三丽鸥', releaseDate: '2026-07-31', mainStageKey: 'storyboard' }),
    project({ id: 'p2', code: 'L50', name: '录像带', releaseDate: '2026-08-02', mainStageKey: 'effects', archived: true }),
  ])
  store.filters = { query: '三丽', stageKey: 'storyboard', releaseDate: '2026-07-31', archived: false }
  expect(store.filteredProjects.map((item) => item.id)).toEqual(['p1'])
})

function project(overrides: Partial<Project> = {}): Project {
  const base: Project = {
    id: 'p1',
    code: 'L36',
    version: 'v1',
    name: '三丽鸥',
    filePath: 'C:\\work\\L36',
    fileExists: true,
    releaseDate: '2026-07-31',
    mainStageKey: 'storyboard',
    archived: false,
    createdAt: '2026-07-11T08:00:00Z',
    updatedAt: '2026-07-11T08:00:00Z',
    stages: STAGE_DEFINITIONS.map((stage, position) => ({
      id: `${stage.key}-id`,
      stageKey: stage.key,
      position,
      startDate: `2026-07-${String(position + 1).padStart(2, '0')}`,
      endDate: `2026-07-${String(position + 8).padStart(2, '0')}`,
      progress: 0,
      updatedAt: '2026-07-11T08:00:00Z',
    })),
  }
  return Object.assign(base, overrides)
}
```

- [ ] **Step 2: Run the store test and confirm the missing-store failure**

```powershell
pnpm test -- tests/stores/projects.test.ts
```

Expected: FAIL resolving `@/stores/projects`.

- [ ] **Step 3: Add typed IPC wrappers with exact command names**

In `productionIpc.ts`, call `invoke` with camelCase arguments:

```ts
export const listProjects = () => invoke<Project[]>('list_projects')
export const createProject = (input: CreateProjectInput) =>
  invoke<Project>('create_project', { input })
export const updateProject = (input: UpdateProjectInput) =>
  invoke<Project>('update_project', { input })
export const saveProjectWithStages = (input: SaveProjectWithStagesInput) =>
  invoke<Project>('save_project_with_stages', { input })
export const setProjectStage = (input: SetProjectStageInput) =>
  invoke<Project>('set_project_stage', { input })
export const archiveProject = (projectId: string, archived: boolean) =>
  invoke<Project>('archive_project', { projectId, archived })
export const deleteProject = (projectId: string) =>
  invoke<void>('delete_project', { projectId })
```

- [ ] **Step 4: Implement `useProjectsStore` with deterministic filters**

Use store id `projects`. State contains `projects`, `filters`, `loading`, `error`, `editorProjectId`. Getters contain `filteredProjects`, `projectsByStage`, and `editingProject`. Initialize every stage key in `projectsByStage` even when its list is empty. Sort cards by `releaseDate`, then case-insensitive `code`, then `id`.

Actions `load`, `hydrate`, `saveEditor`, `setStage`, `setArchived`, and `remove` must replace the returned project atomically in the local array. `saveEditor` calls only `saveProjectWithStages` with the dialog's stable UUID and all eight rows; `setStage` is reserved for an explicit single-stage quick edit outside the full dialog and never modifies `mainStageKey`.

- [ ] **Step 5: Add a regression test for main-stage/progress independence**

Mock `saveProjectWithStages` to return a project whose main stage is `effects` while its storyboard progress remains `65`. Assert the store changes columns without altering the returned stages array.

- [ ] **Step 6: Run project store tests and typecheck**

```powershell
pnpm test -- tests/stores/projects.test.ts
pnpm typecheck
```

Expected: project store tests PASS and typecheck exits `0`.

- [ ] **Step 7: Commit IPC and project state**

```powershell
git add src/lib/productionIpc.ts src/stores/projects.ts tests/stores/projects.test.ts
git commit -m "feat(projects): add project frontend state"
```

### Task 5: Build The Eight-Column Board, Editor, And Overlapping Timeline

**Files:**
- Modify: `package.json`
- Modify: `pnpm-lock.yaml`
- Create: `src/components/projects/ProjectBoardPage.vue`
- Create: `src/components/projects/ProjectEditorDialog.vue`
- Create: `src/components/projects/ProjectTimeline.vue`
- Create: `tests/components/ProjectBoardPage.test.ts`
- Create: `tests/components/ProjectEditorDialog.test.ts`
- Create: `tests/components/ProjectTimeline.test.ts`

- [ ] **Step 0: Add the shared icon dependency**

Run `pnpm add lucide-vue-next`. Project controls introduced below use its `File`, `FolderOpen`, `Link`, archive, delete, and close icons with accessible names/tooltips; do not draw custom SVG controls. The Storyboard plan reuses this installed dependency.

- [ ] **Step 1: Write a failing board test for eight columns and one card**

Mount `ProjectBoardPage` with a hydrated store and assert:

```ts
expect(wrapper.findAll('[data-stage-column]')).toHaveLength(8)
expect(wrapper.get('[data-stage-column="middle_cut"]').text()).toContain('中版')
expect(wrapper.findAll('[data-project-id="p1"]')).toHaveLength(1)
expect(wrapper.get('[data-project-id="p1"]').text()).toContain('65%')
expect(wrapper.get('[data-stage-column="storyboard"] header').attributes('style')).toContain('#F4C430')
```

- [ ] **Step 2: Write a failing timeline test for overlapping bars and separate markers**

Give storyboard `2026-07-01..10` at 80% and first cut `2026-07-05..14` at 30%. Assert both bars render, neither receives an overlap error, `[data-today-line]` exists, and each bar owns one `[data-progress-marker]` with a different `left` style.

- [ ] **Step 2a: Write rejected-write editor tests before implementation**

Mock the single `saveProjectWithStages` IPC to reject. Assert the editor remains open, its stable draft UUID and all project/stage dates/progress values remain unchanged, the card/store does not optimistically move, a retryable inline error is visible, and no success toast is emitted. Retry the exact same payload after success and assert the dialog closes only after the one committed `Project` replaces local state. Separately test quick `setProjectStage` rejection where that narrow control is exposed.

- [ ] **Step 3: Run both component tests and confirm missing components**

```powershell
pnpm test -- tests/components/ProjectBoardPage.test.ts tests/components/ProjectEditorDialog.test.ts tests/components/ProjectTimeline.test.ts
```

Expected: FAIL because the three project components do not exist.

- [ ] **Step 4: Implement the fixed board structure and overflow behavior**

`ProjectBoardPage.vue` must render `STAGE_DEFINITIONS` in order with this stable layout:

```css
.project-board-scroll {
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-gutter: stable;
}
.project-board {
  display: grid;
  grid-template-columns: repeat(8, minmax(220px, 1fr));
  gap: 8px;
  min-width: 1816px;
}
.stage-column {
  min-width: 220px;
}
```

Each header uses exactly `stage.color` and `stage.textColor`. Each card shows code, name, version, release date, the selected main-stage label, and only that stage's progress. Do not compute or display a total project percentage.

- [ ] **Step 5: Implement filters and archive controls**

Add a query input, stage select, release-date input, and archive select (`active`, `archived`, `all`) above the board. Inputs write to `store.filters`. Provide “新建项目” and “显示归档” commands without placing the toolbar inside a decorative card.

- [ ] **Step 6: Implement the editor with independent stage rows**

`ProjectEditorDialog.vue` must provide required code/version/name/file path/release date, a main-stage select, archive toggle, and eight stage rows. Beside the editable path field, provide separate icon buttons for “选择文件” and “选择文件夹” using `@tauri-apps/plugin-dialog` with `multiple:false`; a successful string result replaces the draft path, while cancel/error leaves the original value and all other fields untouched. Each stage row contains required start date, required end date, numeric `0..100` input, range slider, and derived Chinese status. The row uses its fixed background/text swatch. Disable submit and show an inline row error for either blank/invalid date or when that row's start date exceeds its end date; permit overlaps across rows.

Submit the dialog exactly once with its UUID and all eight normalized rows:

```ts
await projects.saveEditor({
  projectId: draft.projectId,
  code: draft.code,
  version: draft.version,
  name: draft.name,
  filePath: draft.filePath,
  releaseDate: draft.releaseDate,
  mainStageKey: draft.mainStageKey,
  archived: draft.archived,
  stages: rows.map((row) => ({
    stageKey: row.stageKey,
    startDate: row.startDate,
    endDate: row.endDate,
    progress: Math.trunc(row.progress),
  })),
})
```

Generate `draft.projectId` once when opening a new dialog and retain it across rejected saves. Never loop over stage IPC calls from this dialog.

- [ ] **Step 7: Implement the timeline coordinate model**

Collect all sixteen required stage boundaries plus today, set `domainStart` to the minimum calendar date and `domainEndExclusive` to one calendar day after the maximum. Compute all positions from integer calendar-day offsets over `max(1, domainEndExclusive - domainStart)`; clamp every normalized start/end/today/progress coordinate to `[0, 1]` before CSS conversion, so a same-day stage still has one visible day of width and no `NaN`/Infinity is possible. A corrupt/incomplete persisted fixture is rejected by the production backup/startup validator rather than rendered as a normal project.

Render one fixed-height row and one bar per stage, so overlapping dates occupy different rows. Treat the stage end as inclusive by positioning its right edge at `end + 1 day`. Position the white actual-progress marker within that clamped bar using `clamp(0, progress, 100)`. Render today as a full-height dashed line. Add a `1px #17212B` outline around white markers on the yellow stage for visibility.

Add pure-coordinate and component tests for all eight same-day stages, heavily overlapping stages, today earlier than every stage, today later than every stage, and invalid/out-of-range progress fixtures. Assert every emitted percentage is finite and within `0..=100` and bars have non-negative width. Backend/model tests separately prove missing/null/blank dates cannot be written or restored.

- [ ] **Step 8: Add long-path, missing-path, and deletion behavior**

Show the full path in a wrapping `overflow-wrap:anywhere` field. When `fileExists` is false, show `需要重新关联` beside a `Link` icon action that opens the same file/folder choices and saves only after a successful selection; cancel/failure preserves the stored path and warning. Test file selection, directory selection, cancel, dialog failure, and missing-path relinking. Archive is the primary removal action; permanent delete requires the existing `ConfirmDialog` and then calls `projects.remove(id)`.

- [ ] **Step 9: Run component tests, typecheck, and lint**

```powershell
pnpm test -- tests/components/ProjectBoardPage.test.ts tests/components/ProjectEditorDialog.test.ts tests/components/ProjectTimeline.test.ts
pnpm typecheck
pnpm lint
```

Expected: component tests PASS; typecheck and lint exit `0`.

- [ ] **Step 10: Commit the project UI**

```powershell
git add package.json pnpm-lock.yaml src/components/projects tests/components/ProjectBoardPage.test.ts tests/components/ProjectEditorDialog.test.ts tests/components/ProjectTimeline.test.ts
git commit -m "feat(projects): add board and overlapping timeline"
```

### Task 6: Implement Daily Task CRUD, Stable Grouping, And Historical Dates

**Files:**
- Create: `src-tauri/src/daily_tasks/model.rs`
- Create: `src-tauri/src/daily_tasks/repository.rs`
- Create: `src-tauri/src/daily_tasks/tests.rs`
- Create: `src-tauri/src/daily_tasks/mod.rs`
- Modify: `src-tauri/src/lib.rs`

Create `daily_tasks/mod.rs` with `pub mod model; pub mod repository; #[cfg(test)] mod tests;`, and add crate-private `mod daily_tasks;` in `lib.rs` before the focused suite is expected to compile. Later tasks append `report`, `carry`, and `navigation` from this same module root.

- [ ] **Step 1: Write a failing stable-order and persisted-field test**

```rust
#[test]
fn daily_groups_keep_explicit_order_and_tasks_keep_work_metadata() {
    let db = test_database();
    repository::create_task(&db, input("2026-07-11", "L50", "录像带", 50, 95)).unwrap();
    repository::create_task(&db, input("2026-07-11", "L36", "三丽鸥", 100, 40)).unwrap();
    repository::reorder_groups(&db, "2026-07-11", vec!["L36".into(), "L50".into()]).unwrap();

    let day = repository::load_day(&db, "2026-07-11").unwrap();
    assert_eq!(day.groups.iter().map(|group| group.code.as_str()).collect::<Vec<_>>(), ["L36", "L50"]);
    assert_eq!(day.groups[1].tasks[0].invested_minutes, 95);
    assert!(!day.groups[1].tasks[0].updated_at.is_empty());
}

#[test]
fn loading_a_history_date_never_merges_tasks_from_another_day() {
    let db = test_database();
    repository::create_task(&db, input("2026-07-10", "L36", "昨天", 100, 20)).unwrap();
    repository::create_task(&db, input("2026-07-11", "L36", "今天", 0, 0)).unwrap();
    let history = repository::load_day(&db, "2026-07-10").unwrap();
    assert_eq!(history.groups[0].tasks[0].title, "昨天");
}

fn input(
    local_date: &str,
    code: &str,
    title: &str,
    progress: i64,
    invested_minutes: i64,
) -> CreateDailyTaskInput {
    CreateDailyTaskInput {
        local_date: local_date.to_string(),
        code: code.to_string(),
        project_id: None,
        title: title.to_string(),
        progress,
        note: String::new(),
        invested_minutes,
    }
}
```

Add rejected-input tests for empty code/title, code longer than 32 ASCII characters, `#L36`, `L36\n#L50`, colon/space/non-ASCII code characters, and titles containing CR, LF, NUL, other C0/DEL controls, `【`, or `】`. Assert no day/group/task row is created or updated and an existing edit remains byte-identical. Keep a positive Unicode title fixture such as `三丽鸥跟进 #角色` to prove ordinary Chinese text and an inline `#` remain valid.

- [ ] **Step 2: Run the daily tests and confirm missing functions**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml daily_tasks::tests
```

Expected: compilation FAIL because daily models/repository are absent.

- [ ] **Step 3: Define exact day, group, and task DTOs**

In `model.rs`, define camelCase serializable DTOs with these fields:

```rust
pub struct DailyTaskDayDto {
    pub id: String,
    pub local_date: String,
    pub settled_at: Option<String>,
    pub report_snapshot: Option<String>,
    pub groups: Vec<DailyTaskGroupDto>,
}

pub struct DailyTaskGroupDto {
    pub id: String,
    pub code: String,
    pub project_id: Option<String>,
    pub position: i64,
    pub tasks: Vec<DailyTaskDto>,
}

pub struct DailyTaskDto {
    pub id: String,
    pub title: String,
    pub progress: i64,
    pub note: String,
    pub invested_minutes: i64,
    pub position: i64,
    pub source_task_id: Option<String>,
    pub source_snapshot_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDailyTaskInput {
    pub local_date: String,
    pub code: String,
    pub project_id: Option<String>,
    pub title: String,
    pub progress: i64,
    pub note: String,
    pub invested_minutes: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDailyTaskInput {
    pub task_id: String,
    pub title: String,
    pub progress: i64,
    pub note: String,
    pub invested_minutes: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderDailyGroupsInput {
    pub local_date: String,
    pub group_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderDailyTasksInput {
    pub local_date: String,
    pub group_id: String,
    pub task_ids: Vec<String>,
}
```

Define `CreateDailyTaskInput`, `UpdateDailyTaskInput`, `ReorderDailyGroupsInput`, and `ReorderDailyTasksInput`. Parse every local date with `NaiveDate::parse_from_str(value, "%Y-%m-%d")`. Trim and ASCII-uppercase group codes, then require `^[A-Z0-9][A-Z0-9_-]{0,31}$`; this excludes Markdown heading/delimiter injection while retaining values such as `L36`. Lock shared constants in `daily_tasks/model.rs`: `MAX_TASK_TITLE_SCALARS=200`, `MAX_TASK_TITLE_BYTES=800`, `MAX_TASK_NOTE_BYTES=64*1024`, and in `report.rs` `MAX_DAILY_REPORT_BYTES=8*1024*1024`. Trim titles, enforce both scalar/UTF-8 caps, and reject CR, LF, NUL, every C0/DEL control, `【`, and `】`. Validate the same title contract on update, progress `0..=100`, invested minutes `>=0`, and note UTF-8 bytes before carry hashing/persistence. Notes may be multiline because they never enter the report, but reject NUL/other disallowed controls. Report generation counts bytes incrementally and returns `REPORT_TOO_LARGE` before snapshot/clipboard allocation or settlement writes. Add -1/exact/+1 fixtures for Unicode title, note, and report.

- [ ] **Step 4: Implement create and load with stable explicit ordering**

Define a production-owned transaction hook now so this checkpoint does not depend on the later reminder module:

```rust
pub trait DailyTaskMutationHook: Send + Sync {
    fn reconcile_in_transaction(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        local_date: &str,
    ) -> Result<(), String>;
}

pub struct NoopDailyTaskMutationHook;
```

Repository create/update/delete/reorder and reopen/settle entry points accept `&dyn DailyTaskMutationHook` and call it exactly once after their own successful row changes but before the outer transaction commits; any hook error rolls back all task/day/report/carry changes. Focused production commands/tests inject `NoopDailyTaskMutationHook`, keeping this plan independently compilable. Desktop Task 10 supplies the reminder implementation and rewires the final command assembly; Integration must prove no shipped production command still holds the no-op hook.

`create_task` must create the day if absent, reject edits if `settled_at` is non-null, reuse the case-insensitive group if present, otherwise append a group at `MAX(position)+1`, then append the task at its group's `MAX(position)+1`.

`load_day` must query groups with `ORDER BY position, id` and tasks with `ORDER BY position, id`. If no day row exists, return an unsaved empty DTO for the requested date rather than tasks from today.

- [ ] **Step 5: Implement updates, deletes, and collision-safe reorder transactions**

Update `updated_at` on every user edit, including progress, note, invested minutes, title, and position. Before applying group reorder, verify the input IDs are exactly the day's group IDs once each. Shift positions by `+1000000`, then assign normalized `0..n-1` positions inside one transaction so `(day_id, position)` never collides.

For task reorder, verify the IDs belong to one group and normalize their positions in the supplied order. Reject every mutation on a settled day with `当天已结算，请先重新打开结算`.

- [ ] **Step 6: Expose the daily CRUD and reorder commands**

Use `spawn_blocking` and register exact command names:

```text
load_daily_task_day
create_daily_task
update_daily_task
delete_daily_task
reorder_daily_groups
reorder_daily_tasks
```

Their return values are the freshly loaded `DailyTaskDayDto`, except `delete_daily_task`, which also returns the freshly loaded day so Pinia can replace state atomically.

Every command uses the `WebviewWindow + State<StartupGate> + MainArgs<WholeCommandArgs> + AppHandle::try_state<AppServices>` pattern from this plan's foundation contract: the envelope authorizes before deserializing, then the body calls `require_ready()`, resolves Ready services, acquires `services.operations.enter_user()`, clones `services.db`, and only then enters `spawn_blocking`; the permit remains held across `.await`, and none receives ordinary typed payloads, required `State<AppServices>`, or `State<Arc<Database>>`. Add malformed wrong-window, Recovery `STARTUP_NOT_READY`, and maintenance `RESTORE_PENDING` zero-repository tables for load/create/update/delete/reorder.

- [ ] **Step 7: Run daily tests and the complete Rust suite**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml daily_tasks::tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
```

Expected: daily tests PASS and the complete Rust suite exits `0`.

- [ ] **Step 8: Commit daily CRUD and stable ordering**

```powershell
git add src-tauri/src/daily_tasks src-tauri/src/lib.rs
git commit -m "feat(tasks): add daily task ledger"
```

### Task 7: Add Exact Daily-Report Formatting And Copy Endpoints

**Files:**
- Create: `src-tauri/src/daily_tasks/report.rs`
- Modify: `src-tauri/src/daily_tasks/tests.rs`
- Modify: `src-tauri/src/daily_tasks/mod.rs`
- Modify: `src/lib/productionIpc.ts`

Use these exact pure formatter inputs in implementation and tests:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportTask {
    pub title: String,
    pub progress: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportGroup {
    pub code: String,
    pub tasks: Vec<ReportTask>,
}
```

- [ ] **Step 1: Write the exact full-report failing test**

```rust
#[test]
fn full_report_matches_the_confirmed_string_exactly() {
    let groups = vec![
        report_group("L36", vec![("三丽鸥跟进", 100), ("412漫画发型跟进", 100)]),
        report_group("L50", vec![("混厄录像带切片制作", 50)]),
    ];
    assert_eq!(
        report::format_full_report(&groups),
        "@日报\n#L36\n1.【L36】【三丽鸥跟进】【100%】\n2.【L36】【412漫画发型跟进】【100%】\n#L50\n1.【L50】【混厄录像带切片制作】【50%】"
    );
}
```

- [ ] **Step 2: Add failing tests for group copy and inclusion of 0% tasks**

```rust
#[test]
fn group_report_omits_daily_header_but_keeps_every_task() {
    let group = report_group("L36", vec![("未开始", 0), ("制作中", 35), ("完成", 100)]);
    assert_eq!(
        report::format_group_report(&group),
        "#L36\n1.【L36】【未开始】【0%】\n2.【L36】【制作中】【35%】\n3.【L36】【完成】【100%】"
    );
}
```

Also call create/update validation with the malicious code/title matrix from Task 6, then format the unchanged rows and compare the complete string exactly. This proves user input cannot inject a second `#` group line or a fake `】【...】` report field.

- [ ] **Step 3: Run the formatter tests and confirm the missing-function failure**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml report_
```

Expected: compilation FAIL because `format_full_report` and `format_group_report` do not exist.

- [ ] **Step 4: Implement the formatter as pure functions**

```rust
pub fn format_group_report(group: &ReportGroup) -> String {
    let mut lines = vec![format!("#{}", group.code)];
    lines.extend(group.tasks.iter().enumerate().map(|(index, task)| {
        format!(
            "{}.【{}】【{}】【{}%】",
            index + 1,
            group.code,
            task.title,
            task.progress
        )
    }));
    lines.join("\n")
}

pub fn format_full_report(groups: &[ReportGroup]) -> String {
    let mut sections = vec!["@日报".to_string()];
    sections.extend(
        groups
            .iter()
            .filter(|group| !group.tasks.is_empty())
            .map(format_group_report),
    );
    sections.join("\n")
}
```

The report query must order groups by `daily_task_groups.position` and tasks by `daily_tasks.position`; it must not filter by progress. Notes, invested minutes, and timestamps never enter `ReportTask`.

- [ ] **Step 5: Add `get_daily_report` with full/group scope**

Define this request contract:

```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDailyReportInput {
    pub local_date: String,
    pub group_id: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyReportResult {
    pub text: String,
    pub task_count: usize,
}
```

When `group_id` is null, return the `@日报` version. When present, verify that group belongs to the requested date and return only `#编号` plus tasks.

The report command uses `WebviewWindow + State<StartupGate> + MainArgs<GetDailyReportCommandArgs> + AppHandle::try_state<AppServices>` and performs the ordered query in `spawn_blocking` only after envelope authorization/deserialization, `require_ready()`, Ready-service resolution, and `enter_user()` succeed; retain the permit across `.await`. Wrong-window malformed payload returns `FORBIDDEN_WINDOW`; Recovery returns `STARTUP_NOT_READY`; maintenance returns `RESTORE_PENDING`, all before formatting/querying.

- [ ] **Step 6: Add the frontend wrapper and run tests**

```ts
export const getDailyReport = (localDate: string, groupId: string | null) =>
  invoke<DailyReportResult>('get_daily_report', {
    input: { localDate, groupId },
  })
```

Run:

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml report_
pnpm typecheck
```

Expected: formatter tests PASS and typecheck exits `0`.

- [ ] **Step 7: Commit exact report formatting**

```powershell
git add src-tauri/src/daily_tasks src/lib/productionIpc.ts
git commit -m "feat(tasks): add exact daily report formatting"
```

### Task 8: Implement Settlement Snapshots And Idempotent Carry-Forward

**Files:**
- Create: `src-tauri/src/daily_tasks/carry.rs`
- Create: `src-tauri/src/production_backup_validator.rs`
- Modify: `src-tauri/src/daily_tasks/model.rs`
- Modify: `src-tauri/src/daily_tasks/repository.rs`
- Modify: `src-tauri/src/daily_tasks/tests.rs`
- Modify: `src-tauri/src/daily_tasks/mod.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `sha2` and write a failing first-settlement test**

Add `sha2 = "0.10"` to Rust dependencies, then test that settlement copies only selected incomplete tasks, leaves the source unchanged, and persists the exact report:

```rust
#[test]
fn settlement_snapshots_today_and_copies_selected_incomplete_tasks() {
    let db = test_database();
    let source = repository::create_task(&db, input("2026-07-11", "L36", "制作中", 40, 75)).unwrap();
    let source_task = source.groups[0].tasks[0].clone();
    let result = carry::settle_day(&db, settle("2026-07-11", &source_task.id, true)).unwrap();

    assert!(result.conflicts.is_empty());
    assert_eq!(result.report_snapshot, "@日报\n#L36\n1.【L36】【制作中】【40%】");
    assert_eq!(repository::load_day(&db, "2026-07-11").unwrap().groups[0].tasks[0], source_task);
    assert_eq!(repository::load_day(&db, "2026-07-12").unwrap().groups[0].tasks.len(), 1);
}
```

- [ ] **Step 2: Write failing idempotency and conflict tests**

Cover these exact branches:

```text
still selected + target unchanged -> update the existing copy, never insert a second copy
not selected + target unchanged -> delete the carry copy
target user-edited + no resolution -> return conflict and do not settle
target user-edited + keep_target -> preserve target
target user-edited + overwrite_target -> replace with latest source values
```

Assert the target count remains `1` after reopening and settling repeatedly. Assert every branch leaves the historical source row and its progress unchanged.

Add a selection-set matrix: one omitted incomplete ID, one duplicated incomplete ID (with equal and conflicting `carry` values), one unknown/other-date ID, and one 100% task ID. Every case returns `INVALID_CARRY_SELECTION_SET`, calls the settlement hook zero times, and leaves source day, target day, conflicts, report snapshot, and reminder rows byte-for-byte unchanged. The positive fixture contains every 0–99% source task ID exactly once, including explicit `carry=false` rows.

- [ ] **Step 3: Run carry tests and confirm they fail**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml carry_
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml settlement_
```

Expected: compilation FAIL because settlement/carry functions are absent.

- [ ] **Step 4: Define the exact settlement and conflict contracts**

```rust
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarryConflictResolution {
    KeepTarget,
    OverwriteTarget,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarrySelection {
    pub source_task_id: String,
    pub carry: bool,
    pub resolution: Option<CarryConflictResolution>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettleDailyTaskDayInput {
    pub local_date: String,
    pub selections: Vec<CarrySelection>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarryConflict {
    pub source_task_id: String,
    pub target_task_id: String,
    pub target_date: String,
}
```

`SettlementResult` contains `settled: bool`, `report_snapshot: String`, `settled_at: Option<String>`, `conflicts`, and the freshly loaded source day.

- [ ] **Step 5: Implement deterministic carry snapshot hashing**

Hash the copied values with length-delimited UTF-8 fields so concatenation cannot collide:

```rust
fn carry_hash(code: &str, title: &str, progress: i64, note: &str, invested_minutes: i64) -> String {
    use sha2::{Digest, Sha256};
    let canonical = format!(
        "{}:{}|{}:{}|{}|{}:{}|{}",
        code.len(), code,
        title.len(), title,
        progress,
        note.len(), note,
        invested_minutes,
    );
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}
```

Store this in `source_snapshot_hash` on the target copy. A target is user-edited when hashing its current copied fields differs from its stored hash.

- [ ] **Step 6: Implement conflict detection before mutation**

Inside one `with_transaction`, load all incomplete source tasks and existing targets for `(source_task_id, tomorrow)`. Before conflict detection or any write, require `selections` to be a one-to-one set match for all and only the source date's `progress BETWEEN 0 AND 99` task IDs: every incomplete ID appears exactly once, no duplicate/omission/foreign ID exists, and no 100% task appears. A mismatch returns `INVALID_CARRY_SELECTION_SET` and rolls back with zero hook calls. Then detect unresolved edited targets before changing any task, group, day, or snapshot. If any conflict lacks a resolution, return `settled=false` with all conflicts and roll back the transaction.

Default UI selections are all incomplete tasks, but the backend requires an explicit true/false row for each one and never accepts or carries a 100% task.

- [ ] **Step 7: Implement idempotent apply and settlement snapshot**

For unchanged targets, update or delete by existing target ID. For new selected targets, insert once with `source_task_id` and `carry_target_date`; reuse or append the target date's group. Apply explicit conflict resolutions. Then generate the full source-date report from the transaction and update `daily_task_days.report_snapshot` and `settled_at`.

`settle_day_in_transaction(transaction, input, mutation_hook)` calls `DailyTaskMutationHook::reconcile_in_transaction(transaction, local_date)` exactly once only after there are no unresolved conflicts and all carry/report/settled writes have succeeded, but before the outer transaction commits. A normal `SettlementResult { settled: false, conflicts }` path returns before any carry/report/reminder mutation and never calls the hook. If the hook returns `Err`, propagate it from the transaction closure so carry targets, snapshot, settled timestamp, and reminder rows all roll back. The focused production wrapper supplies `NoopDailyTaskMutationHook`; Desktop/Integration replaces it with reminder eligibility reconciliation.

Add tests with a counting hook for: unresolved conflict returns `settled=false` and count `0`; successful settlement count `1`; injected hook failure leaves source day unsettled, snapshot unchanged, target tasks unchanged, and allows a clean retry.

Reopening a settled day sets only `settled_at = NULL` and updates `updated_at`; keep the last snapshot visible until the next successful settlement replaces it. Mutations are allowed only after reopen.

- [ ] **Step 8: Expose settle and reopen commands**

Register:

```text
settle_daily_task_day
reopen_daily_task_day
```

Both use `WebviewWindow + State<StartupGate> + MainArgs<WholeCommandArgs> + AppHandle::try_state<AppServices>`, envelope main authorization, Ready, service resolution, `enter_user()`, and `spawn_blocking`, holding the permit through the settlement/hook transaction and `.await`. `reopen_daily_task_day` returns the unlocked source day. Repeating settlement without reopening returns `当天已结算` and does not touch tomorrow. The integration plan later calls the reminder settlement helper inside this same database transaction; it does not introduce a second managed database state. Add malformed wrong-window and settle/reopen-versus-Recovery/maintenance barriers: either the complete carry/snapshot/reminder hook commits before restore capture, or the command returns `FORBIDDEN_WINDOW`/`STARTUP_NOT_READY`/`RESTORE_PENDING` with all rows unchanged.

- [ ] **Step 9: Register production backup semantics**

Implement `ProductionBackupDomainValidator` with stable registry name `production-v1` and register exactly once in the foundation `BackupDomainValidatorRegistry` before `StartupCoordinator::run` in every mode. It imports the exact project/task/report constants above rather than defining stricter restore-only limits, streams unbounded legitimate row counts, and enforces states that repository transactions normally guarantee but SQLite FK/CHECK alone cannot: each project has exactly the eight unique fixed stage keys at positions 0..7; its `main_stage_key` names one of them; stage start/end are valid ISO dates with start <= end while cross-stage overlap remains allowed; progress is 0..100; colors are never trusted/read from DB and remain keyed frontend constants; group/task positions are locally unique and contiguous; day/group/task ownership and local-date formats agree; carry fields are all-null for originals or a complete `(source_task_id, carry_target_date, 64-hex source_snapshot_hash)` tuple for a target whose day equals the target date; and a non-null `settled_at` requires the exact bounded persisted report snapshot while reopened `settled_at=null` may retain it. Recompute the canonical report/carry hash with the production functions and reject mismatch without rewriting the backup. A pre-v1/corrupt oversized row cannot be silently truncated; startup remains Recovery with safe row ID and recovery/full-restore access, while every v1 write path is proven unable to create such a row.

Before its focused test, add crate-private `mod production_backup_validator;` in `lib.rs` and call its one registration function during the shared pre-startup registry assembly. Assert the registered Arc is the same instance used by Foundation backup/startup; do not create a module-local registry.

Add safe-ID-only fixtures for missing/duplicate/extra stage, invalid main stage/date/progress/position, cross-day group/task, partial/foreign/incorrect carry tuple/hash, malformed or oversized report, and illegal settlement pairing. Missing or duplicate `production-v1` registration blocks inspect/pre-switch/startup/ack; valid overlapping stages and a reopened historical day pass. The validator never probes `file_path` existence or changes user data.

- [ ] **Step 10: Run carry, settlement, and full Rust tests**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml carry_
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml settlement_
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
```

Expected: all carry/settlement branches PASS and the complete Rust suite exits `0`.

- [ ] **Step 11: Commit settlement and carry behavior**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/daily_tasks src-tauri/src/production_backup_validator.rs src-tauri/src/lib.rs
git commit -m "feat(tasks): add settlement and idempotent carry"
```

### Task 9: Build Daily Pinia State, History, Copies, And Settlement UI

**Files:**
- Create: `src/stores/dailyTasks.ts`
- Create: `src/components/daily/DailyTasksPage.vue`
- Create: `src/components/daily/DailyTaskGroup.vue`
- Create: `src/components/daily/DailySettlementDialog.vue`
- Create: `tests/stores/dailyTasks.test.ts`
- Create: `tests/components/DailyTasksPage.test.ts`
- Create: `tests/components/DailySettlementDialog.test.ts`
- Modify: `src/lib/productionIpc.ts`

- [ ] **Step 1: Add every daily IPC wrapper**

Use exact command names and return types:

```ts
export const loadDailyTaskDay = (localDate: string) =>
  invoke<DailyTaskDay>('load_daily_task_day', { localDate })
export const createDailyTask = (input: CreateDailyTaskInput) =>
  invoke<DailyTaskDay>('create_daily_task', { input })
export const updateDailyTask = (input: UpdateDailyTaskInput) =>
  invoke<DailyTaskDay>('update_daily_task', { input })
export const deleteDailyTask = (taskId: string, localDate: string) =>
  invoke<DailyTaskDay>('delete_daily_task', { taskId, localDate })
export const reorderDailyGroups = (localDate: string, groupIds: string[]) =>
  invoke<DailyTaskDay>('reorder_daily_groups', { input: { localDate, groupIds } })
export const reorderDailyTasks = (localDate: string, groupId: string, taskIds: string[]) =>
  invoke<DailyTaskDay>('reorder_daily_tasks', { input: { localDate, groupId, taskIds } })
export const settleDailyTaskDay = (input: SettleDailyTaskDayInput) =>
  invoke<SettlementResult>('settle_daily_task_day', { input })
export const reopenDailyTaskDay = (localDate: string) =>
  invoke<DailyTaskDay>('reopen_daily_task_day', { localDate })
```

- [ ] **Step 2: Write failing store tests for history, stable reorder, and copy scope**

Assert `selectDate('2026-07-10')` calls only that date; reorder replaces state with backend order; `copyGroup('g36')` copies text beginning `#L36` without `@日报`; `copyWholeReport()` copies text beginning `@日报`; and every task, including 0%, remains in the mocked report response. For a settled day, set `reportSnapshot` to a byte-distinct historical string, call `copySettledSnapshot()`, and assert that exact stored string reaches the clipboard without calling `getDailyReport`.

Also reject `createDailyTask`, `updateDailyTask`, delete/reorder, settle, and reopen one at a time. Assert `day`, the current edit draft, `carrySelections`, and conflict choices remain unchanged; `error` becomes retryable; and no settled/success state is fabricated.

- [ ] **Step 3: Implement `useDailyTasksStore` with exact state ownership**

Use store id `dailyTasks`. State contains `selectedDate`, `day`, `loading`, `error`, `settlementOpen`, `carrySelections`, and `conflicts`. Actions contain `selectDate`, `load`, `createTask`, `updateTask`, `deleteTask`, `reorderGroups`, `reorderTasks`, `copyGroup`, `copyWholeReport`, `copySettledSnapshot`, `openSettlement`, `settle`, `resolveConflict`, and `reopen`.

Every successful mutation replaces `day` with the backend response. Current-draft group/whole copy actions call `getDailyReport`, then existing `copyToClipboard`; they never construct report strings in Vue. `copySettledSnapshot` is the deliberate exception: it requires non-null `day.reportSnapshot` and copies those stored bytes directly, so later formatter changes cannot rewrite historical reports.

- [ ] **Step 4: Add failing page tests for task fields and historical navigation**

Mount `DailyTasksPage` and assert visible controls for date, code, name, integer/range progress, note, invested hours, invested minutes, updated time, group copy, whole-report copy, settle, and previous/next date. Set a historical date and assert the heading contains it.

Mock add and edit IPC rejection. Assert the add row or edit control stays open with code/title/progress/note/time intact, focus remains in the relevant workflow, and neither the task list nor success feedback changes until a retry succeeds.

- [ ] **Step 5: Implement the page and stable group controls**

`DailyTasksPage.vue` owns the date toolbar, add-task row, whole-report command, settlement/reopen command, and internal scrolling. `DailyTaskGroup.vue` renders groups in backend position order. Use drag handles or explicit up/down icon controls, but always send the complete ordered ID list to the store.

Display invested time as two numeric inputs and save:

```ts
const investedMinutes = Math.max(0, Math.trunc(hours * 60 + minutes))
await daily.updateTask({ taskId, title, progress, note, investedMinutes })
```

Display `updatedAt` converted to local time. Do not include note, time, or updated time in copied report text.

- [ ] **Step 6: Lock settled history and expose reopen**

When `day.settledAt` is non-null, disable add/edit/delete/reorder and show the immutable `reportSnapshot` with a “复制本次结算快照” action wired to `copySettledSnapshot`. Show “重新打开结算”; after confirmation, call `daily.reopen()` and restore editing without deleting the existing snapshot from view. In reopened state, label it “上次结算快照” and keep its byte-for-byte copy action separate from “复制当前草稿日报”, which calls the current formatter. Add a component/store test that changes the mocked current formatter output after reopening and proves each button copies its own distinct source.

- [ ] **Step 7: Write failing conflict-dialog tests**

Assert all incomplete tasks default to checked, completed tasks are absent, a returned conflict shows both “保留明日版本” and “用今日版本覆盖”, and pressing confirm again supplies `keep_target` or `overwrite_target` for each conflict.

Reject both the first settlement call and the resolved-conflict retry. Assert the dialog remains open, task selections and every conflict resolution stay selected, the report snapshot is not replaced, and no “已结算” feedback appears.

- [ ] **Step 8: Implement settlement and conflict UI**

`DailySettlementDialog.vue` lists tasks below 100%, checked by default. The first submit sends all explicit selections. When the backend returns `settled=false`, keep the dialog open and require one resolution per conflict before enabling the next submit. On `settled=true`, close the dialog and show the saved snapshot.

- [ ] **Step 9: Run daily frontend tests, typecheck, and lint**

```powershell
pnpm test -- tests/stores/dailyTasks.test.ts tests/components/DailyTasksPage.test.ts tests/components/DailySettlementDialog.test.ts
pnpm typecheck
pnpm lint
```

Expected: all daily frontend tests PASS; typecheck and lint exit `0`.

- [ ] **Step 10: Commit the daily frontend**

```powershell
git add src/lib/productionIpc.ts src/stores/dailyTasks.ts src/components/daily tests/stores/dailyTasks.test.ts tests/components/DailyTasksPage.test.ts tests/components/DailySettlementDialog.test.ts
git commit -m "feat(tasks): add daily workflow and settlement UI"
```

### Task 10: Integrate Sidebar Navigation And The 18:00 Page Entry

**Files:**
- Create: `src-tauri/src/daily_tasks/navigation.rs`
- Modify: `src-tauri/src/daily_tasks/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/stores/ui.ts`
- Modify: `src/components/AppSidebar.vue`
- Modify: `src/App.vue`
- Modify: `tests/stores/ui.test.ts`
- Modify: `tests/components/AppSidebar.test.ts`
- Modify: `tests/components/App.test.ts`

- [ ] **Step 1: Add failing UI-store and sidebar tests**

Assert `ActiveTool` accepts `projects` and `daily-tasks`; `ui.openDailyTasks('2026-07-11')` sets both `activeTool='daily-tasks'` and `dailyTasksDate='2026-07-11'`; the sidebar renders `项目管理` and `当日任务`; and clicking either switches tools.

- [ ] **Step 2: Add a failing App event-navigation test**

Mock `@tauri-apps/api/event.listen`, capture the `open-daily-tasks` callback, invoke it with `{ payload: { localDate: '2026-07-11' } }`, and assert `DailyTasksPage` renders while `ProjectBoardPage` does not.

- [ ] **Step 3: Run the navigation tests and confirm failure**

```powershell
pnpm test -- tests/stores/ui.test.ts tests/components/AppSidebar.test.ts tests/components/App.test.ts
```

Expected: FAIL because the new tool IDs, pages, and event listener are absent.

- [ ] **Step 4: Extend the frontend tool contract**

Use this exact type and state field:

```text
export type ActiveTool = 'prompts' | 'reverse-image' | 'compression' | 'projects' | 'daily-tasks'

dailyTasksDate: '' as string,

openDailyTasks(localDate: string) {
  this.dailyTasksDate = localDate
  this.activeTool = 'daily-tasks'
  this.showPanel()
}
```

Add sidebar entries in this order after the current tools: `项目管理`, then `当日任务`.

- [ ] **Step 5: Render the two pages and consume the event once**

In `App.vue`, import both pages, add explicit `v-else-if` branches, and register:

```ts
interface OpenDailyTasksPayload { localDate: string }

unlistenOpenDailyTasks = await listen<OpenDailyTasksPayload>('open-daily-tasks', (event) => {
  ui.openDailyTasks(event.payload.localDate)
})
```

Dispose this listener in `onUnmounted`, alongside the existing floating-drop listener. Watch `ui.dailyTasksDate`; when it changes, call `dailyTasks.selectDate(value)` once.

- [ ] **Step 6: Implement the common Rust page-entry helper**

```rust
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDailyTasksPayload {
    pub local_date: String,
}

pub async fn navigate_to_daily_tasks(app: &tauri::AppHandle, local_date: String) -> Result<(), String> {
    chrono::NaiveDate::parse_from_str(&local_date, "%Y-%m-%d")
        .map_err(|_| "日期格式必须为 YYYY-MM-DD".to_string())?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    app.emit_to("main", "open-daily-tasks", OpenDailyTasksPayload { local_date })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn open_daily_tasks_page(
    window: tauri::WebviewWindow,
    gate: tauri::State<StartupGate>,
    args: MainArgs<OpenDailyTasksPageCommandArgs>,
) -> Result<(), String> {
    gate.require_ready()?;
    let app = window.app_handle();
    let services = app.try_state::<AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    navigate_to_daily_tasks(app, args.0.local_date).await
}
```

`OpenDailyTasksPageCommandArgs` is camelCase/deny-unknown and contains only `local_date`; its `MainArgs` envelope performs authorization before deserialization exactly like all other production commands.

Import `tauri::{Emitter, Manager}` plus `crate::command_auth::MainArgs`, and register `open_daily_tasks_page`. Add authorized-envelope tests with malformed and valid payloads from `floatbtn`, `reminder`, and unknown labels; each returns `FORBIDDEN_WINDOW` before gate/window/event access, plus maintenance returning `RESTORE_PENDING` before show/focus/event. The reminder plan calls the internal `navigate_to_daily_tasks` helper directly while holding its own fenced user-operation permit, not this main-only IPC and not duplicate window/event logic.

- [ ] **Step 7: Test invalid date and event payload serialization in Rust**

Extract date validation into a pure helper and assert `2026-07-11` passes while `2026-7-11` and `2026-02-30` fail. Serialize `OpenDailyTasksPayload` and assert the JSON key is exactly `localDate`.

- [ ] **Step 8: Run navigation, frontend, and Rust tests**

```powershell
pnpm test -- tests/stores/ui.test.ts tests/components/AppSidebar.test.ts tests/components/App.test.ts
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml daily_tasks::navigation
```

Expected: navigation tests PASS in both runtimes.

- [ ] **Step 9: Commit navigation and reminder entry contract**

```powershell
git add src-tauri/src/daily_tasks src-tauri/src/lib.rs src/stores/ui.ts src/components/AppSidebar.vue src/App.vue tests/stores/ui.test.ts tests/components/AppSidebar.test.ts tests/components/App.test.ts
git commit -m "feat(tasks): connect daily task reminder entry"
```

### Task 11: Full Verification And Production-Management QA

**Files:**
- Verify all files in this plan.
- Modify only files that fail the checks below.

- [ ] **Step 1: Run focused Rust tests**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml projects::
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml daily_tasks::
```

Expected: every focused project/daily test PASS.

- [ ] **Step 2: Run focused frontend tests**

```powershell
pnpm test -- tests/domain/production.test.ts tests/stores/projects.test.ts tests/stores/dailyTasks.test.ts tests/components/ProjectBoardPage.test.ts tests/components/ProjectTimeline.test.ts tests/components/DailyTasksPage.test.ts tests/components/DailySettlementDialog.test.ts tests/components/AppSidebar.test.ts tests/components/App.test.ts
```

Expected: all listed test files PASS with zero failed assertions.

- [ ] **Step 3: Run the complete automated checks**

```powershell
pnpm check
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --manifest-path src-tauri\Cargo.toml
```

Expected: typecheck, ESLint, Vitest, and all Rust tests exit with code `0`.

- [ ] **Step 4: Run the desktop app and execute the project acceptance path**

```powershell
pnpm tauri dev
```

Verify in order: create `L36`; reject duplicate `l36`; confirm eight columns include `中版`; set storyboard `07-01..07-10` to 80% and first cut `07-05..07-14` to 30%; confirm overlap renders; change main stage to 特效; confirm storyboard remains 80%; archive and find the project using archive filter; break its file path and confirm `需要重新关联` appears.

- [ ] **Step 5: Execute the daily acceptance path**

On one date, create the exact three sample tasks under `L36` and `L50`, including the 50% item. Reorder groups to L36 then L50; enter note, 1 hour 35 minutes, and progress; confirm updated time changes. Copy the L36 group and the full report; compare clipboard contents byte-for-byte with the strings in Task 7.

- [ ] **Step 6: Execute settlement, carry, history, and conflict acceptance**

Settle with the 50% task selected for tomorrow; confirm today is locked and its snapshot remains unchanged. Reopen and settle again; confirm tomorrow has one copy. Edit tomorrow's copy, reopen today, change the source, and settle; confirm conflict choices appear. Test both keep and overwrite. Navigate back to the historical date and confirm no task from another date appears.

- [ ] **Step 7: Verify the 18:00 entry contract without waiting for clock time**

From the Tauri devtools console invoke `open_daily_tasks_page` with today's `YYYY-MM-DD`. Confirm the main window appears, the active page is 当日任务, and the selected date matches the payload. This validates the action target used by the reminder scheduler without duplicating scheduler QA here.

- [ ] **Step 8: Run Gstack visual and interaction review**

Run Gstack `browse`, `qa`, and `design-review` against the project board, editor/timeline, daily history, settled snapshot, and conflict dialog. Required results: no clipped content at `1080×720` or `760×560`; board scrolls horizontally; long paths wrap; all eight fixed color/text pairs remain unchanged; keyboard focus is visible; date bars, today line, and progress markers do not overlap labels.

- [ ] **Step 9: Inspect the final diff for accidental scope**

```powershell
git status --short
git diff --stat
git diff --check
```

Expected: only files mapped by this plan are changed; `git diff --check` prints no whitespace errors.

- [ ] **Step 10: Commit verification fixes, if any checks required changes**

```powershell
git add src src-tauri tests
git commit -m "test: verify production management workflows"
```

If `git status --short` is empty after verification, skip this commit; the feature commits already contain the verified implementation.

## Completion Checklist

- [ ] All eight fixed stages exist in the confirmed order and include `中版`.
- [ ] Fixed stage background/text colors match the specification and remain at least 4.5:1.
- [ ] Overlapping stage dates are accepted; only an invalid range inside one stage is rejected.
- [ ] Main-stage selection never overwrites any independent stage progress.
- [ ] The UI never shows a combined eight-stage project percentage.
- [ ] Daily groups and tasks retain explicit stable order after restart and historical navigation.
- [ ] Every day's task, including 0%, enters the report in exact confirmed syntax.
- [ ] Group copy excludes `@日报`; whole-report copy includes it.
- [ ] Notes, invested minutes, and updated timestamps persist but never enter report text.
- [ ] Settlement stores an immutable report snapshot and locks the day until explicit reopen.
- [ ] Carry-forward is unique by `(source_task_id, target_date)` and never mutates the source day.
- [ ] Edited target copies require an explicit keep/overwrite conflict decision.
- [ ] Historical dates load independently and never merge with today.
- [ ] `open_daily_tasks_page` and `open-daily-tasks` provide the single 18:00 reminder action target.
- [ ] Focused and full Rust/Vue checks pass, followed by Gstack browse/qa/design-review.
