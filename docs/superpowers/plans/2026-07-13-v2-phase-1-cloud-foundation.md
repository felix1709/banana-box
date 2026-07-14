# Banana Box v2 Phase 1 Cloud Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add safe Supabase cloud configuration and a visible cloud readiness state without changing existing local-only prompt, project, or daily task behavior.

**Architecture:** Store Supabase URL and anon key in a dedicated local SQLite table managed by Tauri commands. Add a small frontend cloud config module, Pinia store, and Settings UI section that validates configuration shape, saves it locally, and clearly shows local-only, cloud-configured, and invalid states. Do not add login, data upload, sync, invitations, comments, or realtime subscriptions in this phase.

**Tech Stack:** Tauri 2, Rust, SQLite, Vue 3, Pinia, TypeScript, Vitest, Cargo tests.

---

## Phase Boundary

This phase intentionally does not authenticate users. It only prepares the app to know whether Supabase configuration exists and looks safe enough to be used by Phase 2.

Do not install or initialize `@supabase/supabase-js` in Phase 1. Phase 2 owns login and the Supabase client.

Do not store a Supabase service role key. The desktop app may store only the public Supabase URL and anon key.

## Files

Create:

- `src/types/cloud.ts`
- `src/lib/cloud-config.ts`
- `src/stores/cloudSession.ts`
- `tests/lib/cloud-config.test.ts`
- `tests/stores/cloudSession.test.ts`
- `src-tauri/migrations/0004_cloud_foundation.sql`
- `src-tauri/src/cloud_config.rs`

Modify:

- `src/types/index.ts`
- `src/lib/ipc.ts`
- `src/components/SettingsModal.vue`
- `tests/components/SettingsModal.test.ts`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/db/schema.rs`

## Data Model

Local SQLite table:

```sql
CREATE TABLE cloud_config (
  id TEXT PRIMARY KEY NOT NULL CHECK (id = 'default'),
  supabase_url TEXT NOT NULL,
  anon_key TEXT NOT NULL,
  cloud_enabled INTEGER NOT NULL DEFAULT 0 CHECK (cloud_enabled IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

Only one config row exists. Use `id = 'default'`.

Frontend types:

```ts
export interface CloudConfig {
  supabaseUrl: string
  anonKey: string
  cloudEnabled: boolean
  updatedAt: string | null
}

export interface SaveCloudConfigInput {
  supabaseUrl: string
  anonKey: string
  cloudEnabled: boolean
}

export type CloudReadiness = 'local_only' | 'configured' | 'invalid'

export interface CloudConfigValidationResult {
  ok: boolean
  code: 'OK' | 'URL_REQUIRED' | 'URL_INVALID' | 'URL_INSECURE' | 'ANON_KEY_REQUIRED' | 'SERVICE_ROLE_KEY_BLOCKED'
}
```

Validation rules:

- URL is required.
- URL must parse as a URL.
- URL must be `https://` unless it is `http://localhost`, `http://127.0.0.1`, or `http://[::1]`.
- anon key is required.
- anon key must not contain `service_role`.
- validation must never print the key.

## Task 1: Frontend Cloud Config Validation

**Files:**

- Create: `src/types/cloud.ts`
- Create: `src/lib/cloud-config.ts`
- Create: `tests/lib/cloud-config.test.ts`
- Modify: `src/types/index.ts`

- [ ] **Step 1: Write the failing validation tests**

Create `tests/lib/cloud-config.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import {
  cloudReadiness,
  normalizeCloudConfigInput,
  validateCloudConfigInput,
} from '@/lib/cloud-config'

describe('cloud config validation', () => {
  it('normalizes a valid Supabase URL without logging or changing the anon key', () => {
    const normalized = normalizeCloudConfigInput({
      supabaseUrl: 'https://example.supabase.co/',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    expect(normalized).toEqual({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })
    expect(validateCloudConfigInput(normalized)).toEqual({ ok: true, code: 'OK' })
    expect(cloudReadiness({ ...normalized, updatedAt: '2026-07-13T00:00:00Z' })).toBe('configured')
  })

  it('allows localhost http for local Supabase development', () => {
    expect(validateCloudConfigInput({
      supabaseUrl: 'http://localhost:54321',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })).toEqual({ ok: true, code: 'OK' })
  })

  it('rejects insecure remote http URLs', () => {
    expect(validateCloudConfigInput({
      supabaseUrl: 'http://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'URL_INSECURE' })
  })

  it('rejects blank and malformed inputs with stable codes', () => {
    expect(validateCloudConfigInput({
      supabaseUrl: '',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'URL_REQUIRED' })
    expect(validateCloudConfigInput({
      supabaseUrl: 'not a url',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'URL_INVALID' })
    expect(validateCloudConfigInput({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: '',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'ANON_KEY_REQUIRED' })
  })

  it('blocks service role keys from being stored in the desktop app', () => {
    expect(validateCloudConfigInput({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'service_role.secret.must.not.ship',
      cloudEnabled: true,
    })).toEqual({ ok: false, code: 'SERVICE_ROLE_KEY_BLOCKED' })
  })

  it('reports local only when cloud is disabled', () => {
    expect(cloudReadiness({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: false,
      updatedAt: '2026-07-13T00:00:00Z',
    })).toBe('local_only')
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```powershell
pnpm vitest run tests/lib/cloud-config.test.ts
```

Expected: FAIL because `src/lib/cloud-config.ts` does not exist.

- [ ] **Step 3: Add cloud types**

Create `src/types/cloud.ts`:

```ts
export interface CloudConfig {
  supabaseUrl: string
  anonKey: string
  cloudEnabled: boolean
  updatedAt: string | null
}

export interface SaveCloudConfigInput {
  supabaseUrl: string
  anonKey: string
  cloudEnabled: boolean
}

export type CloudReadiness = 'local_only' | 'configured' | 'invalid'

export interface CloudConfigValidationResult {
  ok: boolean
  code: 'OK' | 'URL_REQUIRED' | 'URL_INVALID' | 'URL_INSECURE' | 'ANON_KEY_REQUIRED' | 'SERVICE_ROLE_KEY_BLOCKED'
}
```

Modify `src/types/index.ts` to export the new type file:

```ts
export * from './cloud'
```

- [ ] **Step 4: Add validation implementation**

Create `src/lib/cloud-config.ts`:

```ts
import type {
  CloudConfig,
  CloudConfigValidationResult,
  CloudReadiness,
  SaveCloudConfigInput,
} from '@/types/cloud'

function trimTrailingSlashes(value: string) {
  return value.trim().replace(/\/+$/, '')
}

function isLoopbackHttp(url: URL) {
  return (
    url.protocol === 'http:'
    && ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)
  )
}

export function normalizeCloudConfigInput(input: SaveCloudConfigInput): SaveCloudConfigInput {
  return {
    supabaseUrl: trimTrailingSlashes(input.supabaseUrl),
    anonKey: input.anonKey.trim(),
    cloudEnabled: input.cloudEnabled,
  }
}

export function validateCloudConfigInput(input: SaveCloudConfigInput): CloudConfigValidationResult {
  const normalized = normalizeCloudConfigInput(input)
  if (!normalized.supabaseUrl) return { ok: false, code: 'URL_REQUIRED' }
  let parsed: URL
  try {
    parsed = new URL(normalized.supabaseUrl)
  } catch {
    return { ok: false, code: 'URL_INVALID' }
  }
  if (parsed.protocol !== 'https:' && !isLoopbackHttp(parsed)) {
    return { ok: false, code: 'URL_INSECURE' }
  }
  if (!normalized.anonKey) return { ok: false, code: 'ANON_KEY_REQUIRED' }
  if (normalized.anonKey.toLocaleLowerCase().includes('service_role')) {
    return { ok: false, code: 'SERVICE_ROLE_KEY_BLOCKED' }
  }
  return { ok: true, code: 'OK' }
}

export function cloudReadiness(config: CloudConfig | null): CloudReadiness {
  if (!config?.cloudEnabled) return 'local_only'
  return validateCloudConfigInput(config).ok ? 'configured' : 'invalid'
}
```

- [ ] **Step 5: Run validation tests**

Run:

```powershell
pnpm vitest run tests/lib/cloud-config.test.ts
```

Expected: PASS.

## Task 2: Backend Cloud Config Storage

**Files:**

- Create: `src-tauri/migrations/0004_cloud_foundation.sql`
- Create: `src-tauri/src/cloud_config.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/db/schema.rs`
- Test: `src-tauri/src/cloud_config.rs`

- [ ] **Step 1: Write failing Rust tests**

Create `src-tauri/src/cloud_config.rs` with only the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::tempdir;

    fn test_db() -> Database {
        let dir = tempdir().unwrap();
        Database::open(dir.path().join("banana.db")).unwrap()
    }

    #[test]
    fn missing_config_loads_as_disabled_local_only() {
        let db = test_db();
        let config = load_cloud_config(&db).unwrap();

        assert_eq!(config.supabase_url, "");
        assert_eq!(config.anon_key, "");
        assert!(!config.cloud_enabled);
        assert_eq!(config.updated_at, None);
    }

    #[test]
    fn saving_config_upserts_the_default_row() {
        let db = test_db();

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
        assert_eq!(loaded.anon_key, "anon-test-key");
        assert!(loaded.cloud_enabled);
        assert!(loaded.updated_at.is_some());
    }

    #[test]
    fn service_role_key_is_rejected_without_persisting() {
        let db = test_db();

        let error = save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co".into(),
                anon_key: "service_role.secret".into(),
                cloud_enabled: true,
            },
        )
        .unwrap_err();

        assert_eq!(error, "CLOUD_SERVICE_ROLE_KEY_BLOCKED");
        assert_eq!(load_cloud_config(&db).unwrap().anon_key, "");
    }
}
```

- [ ] **Step 2: Run Rust test to verify it fails**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml cloud_config::
```

Expected: FAIL because `load_cloud_config`, `save_cloud_config`, and types are not defined.

- [ ] **Step 3: Add migration SQL**

Create `src-tauri/migrations/0004_cloud_foundation.sql`:

```sql
CREATE TABLE cloud_config (
  id TEXT PRIMARY KEY NOT NULL CHECK (id = 'default'),
  supabase_url TEXT NOT NULL,
  anon_key TEXT NOT NULL,
  cloud_enabled INTEGER NOT NULL DEFAULT 0 CHECK (cloud_enabled IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

Modify `src-tauri/src/db/schema.rs` to include the new migration in the migration registry using the existing pattern for `0001_v1.sql`, `0002_allow_duplicate_project_codes.sql`, and `0003_storyboard_agent.sql`.

- [ ] **Step 4: Implement Rust storage module**

Replace `src-tauri/src/cloud_config.rs` with:

```rust
use crate::db::Database;
use chrono::Utc;
use rusqlite::params;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudConfigDto {
    pub supabase_url: String,
    pub anon_key: String,
    pub cloud_enabled: bool,
    pub updated_at: Option<String>,
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
            "SELECT supabase_url, anon_key, cloud_enabled, updated_at
             FROM cloud_config WHERE id = 'default'",
            [],
            |row| {
                Ok(CloudConfigDto {
                    supabase_url: row.get(0)?,
                    anon_key: row.get(1)?,
                    cloud_enabled: row.get::<_, i64>(2)? != 0,
                    updated_at: Some(row.get(3)?),
                })
            },
        );

        match result {
            Ok(config) => Ok(config),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(CloudConfigDto {
                supabase_url: String::new(),
                anon_key: String::new(),
                cloud_enabled: false,
                updated_at: None,
            }),
            Err(error) => Err(error.to_string()),
        }
    })
}

pub fn save_cloud_config(
    db: &Database,
    input: SaveCloudConfigInput,
) -> Result<CloudConfigDto, String> {
    validate_cloud_config(&input)?;
    let now = Utc::now().to_rfc3339();
    let supabase_url = input.supabase_url.trim().trim_end_matches('/').to_string();
    let anon_key = input.anon_key.trim().to_string();

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

fn validate_cloud_config(input: &SaveCloudConfigInput) -> Result<(), String> {
    if input
        .anon_key
        .to_ascii_lowercase()
        .contains("service_role")
    {
        return Err("CLOUD_SERVICE_ROLE_KEY_BLOCKED".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::tempdir;

    fn test_db() -> Database {
        let dir = tempdir().unwrap();
        Database::open(dir.path().join("banana.db")).unwrap()
    }

    #[test]
    fn missing_config_loads_as_disabled_local_only() {
        let db = test_db();
        let config = load_cloud_config(&db).unwrap();

        assert_eq!(config.supabase_url, "");
        assert_eq!(config.anon_key, "");
        assert!(!config.cloud_enabled);
        assert_eq!(config.updated_at, None);
    }

    #[test]
    fn saving_config_upserts_the_default_row() {
        let db = test_db();

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
        assert_eq!(loaded.anon_key, "anon-test-key");
        assert!(loaded.cloud_enabled);
        assert!(loaded.updated_at.is_some());
    }

    #[test]
    fn service_role_key_is_rejected_without_persisting() {
        let db = test_db();

        let error = save_cloud_config(
            &db,
            SaveCloudConfigInput {
                supabase_url: "https://example.supabase.co".into(),
                anon_key: "service_role.secret".into(),
                cloud_enabled: true,
            },
        )
        .unwrap_err();

        assert_eq!(error, "CLOUD_SERVICE_ROLE_KEY_BLOCKED");
        assert_eq!(load_cloud_config(&db).unwrap().anon_key, "");
    }
}
```

- [ ] **Step 5: Register module**

Modify `src-tauri/src/lib.rs` to add:

```rust
mod cloud_config;
```

Place it beside existing domain modules.

- [ ] **Step 6: Run Rust module tests**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml cloud_config::
```

Expected: PASS.

## Task 3: Tauri Commands And Frontend IPC

**Files:**

- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/ipc.ts`
- Test: `tests/lib/cloud-config.test.ts`

- [ ] **Step 1: Add IPC wrapper tests**

Extend `tests/lib/cloud-config.test.ts` with:

```ts
import { vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { loadCloudConfig, saveCloudConfig } from '@/lib/ipc'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('cloud config IPC', () => {
  it('loads cloud config through an empty Tauri command payload', async () => {
    vi.mocked(invoke).mockResolvedValue({
      supabaseUrl: '',
      anonKey: '',
      cloudEnabled: false,
      updatedAt: null,
    })

    await loadCloudConfig()

    expect(invoke).toHaveBeenCalledWith('load_cloud_config', {})
  })

  it('saves cloud config through a single input payload', async () => {
    vi.mocked(invoke).mockResolvedValue({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
      updatedAt: '2026-07-13T00:00:00Z',
    })

    await saveCloudConfig({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    expect(invoke).toHaveBeenCalledWith('save_cloud_config', {
      input: {
        supabaseUrl: 'https://example.supabase.co',
        anonKey: 'anon-test-key',
        cloudEnabled: true,
      },
    })
  })
})
```

- [ ] **Step 2: Run IPC tests to verify failure**

Run:

```powershell
pnpm vitest run tests/lib/cloud-config.test.ts
```

Expected: FAIL because `loadCloudConfig` and `saveCloudConfig` are not exported from `src/lib/ipc.ts`.

- [ ] **Step 3: Add frontend IPC functions**

Modify `src/lib/ipc.ts`:

```ts
import type { CloudConfig, SaveCloudConfigInput } from '@/types'
```

Add:

```ts
export async function loadCloudConfig(): Promise<CloudConfig> {
  return await invoke<CloudConfig>('load_cloud_config', {})
}

export async function saveCloudConfig(input: SaveCloudConfigInput): Promise<CloudConfig> {
  return await invoke<CloudConfig>('save_cloud_config', { input })
}
```

- [ ] **Step 4: Add Tauri command handlers**

Modify `src-tauri/src/commands.rs`:

```rust
use crate::cloud_config::{
    load_cloud_config as load_cloud_config_from_db,
    save_cloud_config as save_cloud_config_to_db,
    CloudConfigDto,
    SaveCloudConfigInput,
};
```

Add command args:

```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveCloudConfigCommandArgs {
    input: SaveCloudConfigInput,
}
```

Add commands:

```rust
#[tauri::command]
pub fn load_cloud_config(
    window: tauri::WebviewWindow,
    gate: tauri::State<crate::app_state::StartupGate>,
) -> Result<CloudConfigDto, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<crate::app_state::AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    load_cloud_config_from_db(&services.database)
}

#[tauri::command]
pub fn save_cloud_config(
    window: tauri::WebviewWindow,
    gate: tauri::State<crate::app_state::StartupGate>,
    args: crate::command_auth::MainArgs<SaveCloudConfigCommandArgs>,
) -> Result<CloudConfigDto, String> {
    gate.require_ready()?;
    let services = window
        .app_handle()
        .try_state::<crate::app_state::AppServices>()
        .ok_or_else(|| "STARTUP_NOT_READY".to_string())?;
    let _permit = services.operations.enter_user()?;
    save_cloud_config_to_db(&services.database, args.0.input)
}
```

Modify `src-tauri/src/lib.rs` command registration to include:

```rust
commands::load_cloud_config,
commands::save_cloud_config,
```

- [ ] **Step 5: Run IPC tests**

Run:

```powershell
pnpm vitest run tests/lib/cloud-config.test.ts
```

Expected: PASS.

- [ ] **Step 6: Run Rust command compile tests**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml cloud_config::
```

Expected: PASS.

## Task 4: Cloud Session Store

**Files:**

- Create: `src/stores/cloudSession.ts`
- Create: `tests/stores/cloudSession.test.ts`

- [ ] **Step 1: Write failing store tests**

Create `tests/stores/cloudSession.test.ts`:

```ts
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useCloudSessionStore } from '@/stores/cloudSession'
import { loadCloudConfig, saveCloudConfig } from '@/lib/ipc'

vi.mock('@/lib/ipc', () => ({
  loadCloudConfig: vi.fn(),
  saveCloudConfig: vi.fn(),
}))

describe('cloud session store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('loads local-only state when no cloud config exists', async () => {
    vi.mocked(loadCloudConfig).mockResolvedValue({
      supabaseUrl: '',
      anonKey: '',
      cloudEnabled: false,
      updatedAt: null,
    })
    const store = useCloudSessionStore()

    await store.load()

    expect(store.readiness).toBe('local_only')
    expect(store.config?.cloudEnabled).toBe(false)
  })

  it('saves valid cloud config and marks the app configured', async () => {
    vi.mocked(saveCloudConfig).mockResolvedValue({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
      updatedAt: '2026-07-13T00:00:00Z',
    })
    const store = useCloudSessionStore()

    await store.save({
      supabaseUrl: 'https://example.supabase.co/',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    expect(saveCloudConfig).toHaveBeenCalledWith({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })
    expect(store.readiness).toBe('configured')
    expect(store.error).toBe('')
  })

  it('keeps invalid config local and does not call the backend', async () => {
    const store = useCloudSessionStore()

    await store.save({
      supabaseUrl: 'http://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    expect(saveCloudConfig).not.toHaveBeenCalled()
    expect(store.readiness).toBe('invalid')
    expect(store.error).toBe('URL_INSECURE')
  })
})
```

- [ ] **Step 2: Run store test to verify failure**

Run:

```powershell
pnpm vitest run tests/stores/cloudSession.test.ts
```

Expected: FAIL because `src/stores/cloudSession.ts` does not exist.

- [ ] **Step 3: Implement cloud session store**

Create `src/stores/cloudSession.ts`:

```ts
import { defineStore } from 'pinia'
import type { CloudConfig, CloudReadiness, SaveCloudConfigInput } from '@/types'
import { cloudReadiness, normalizeCloudConfigInput, validateCloudConfigInput } from '@/lib/cloud-config'
import { loadCloudConfig, saveCloudConfig } from '@/lib/ipc'

export const useCloudSessionStore = defineStore('cloudSession', {
  state: () => ({
    config: null as CloudConfig | null,
    loading: false,
    saving: false,
    error: '',
  }),
  getters: {
    readiness(state): CloudReadiness {
      if (state.error && state.config?.cloudEnabled) return 'invalid'
      return cloudReadiness(state.config)
    },
  },
  actions: {
    async load() {
      this.loading = true
      this.error = ''
      try {
        this.config = await loadCloudConfig()
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
      } finally {
        this.loading = false
      }
    },
    async save(input: SaveCloudConfigInput) {
      this.saving = true
      this.error = ''
      const normalized = normalizeCloudConfigInput(input)
      const validation = validateCloudConfigInput(normalized)
      if (!validation.ok) {
        this.config = { ...normalized, updatedAt: null }
        this.error = validation.code
        this.saving = false
        return
      }
      try {
        this.config = await saveCloudConfig(normalized)
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error)
      } finally {
        this.saving = false
      }
    },
  },
})
```

- [ ] **Step 4: Run store tests**

Run:

```powershell
pnpm vitest run tests/stores/cloudSession.test.ts
```

Expected: PASS.

## Task 5: Settings UI For Cloud Configuration

**Files:**

- Modify: `src/components/SettingsModal.vue`
- Modify: `tests/components/SettingsModal.test.ts`

- [ ] **Step 1: Write failing UI tests**

Add tests to `tests/components/SettingsModal.test.ts`:

```ts
import { useCloudSessionStore } from '@/stores/cloudSession'
```

Extend mocks:

```ts
vi.mock('@/lib/ipc', () => ({
  exportLibrary: vi.fn().mockResolvedValue(undefined),
  readImportDir: vi.fn().mockResolvedValue([]),
  downloadImage: vi.fn(),
  saveLibrary: vi.fn().mockResolvedValue(undefined),
  loadCloudConfig: vi.fn().mockResolvedValue({
    supabaseUrl: '',
    anonKey: '',
    cloudEnabled: false,
    updatedAt: null,
  }),
  saveCloudConfig: vi.fn().mockResolvedValue({
    supabaseUrl: 'https://example.supabase.co',
    anonKey: 'anon-test-key',
    cloudEnabled: true,
    updatedAt: '2026-07-13T00:00:00Z',
  }),
}))
```

Add tests:

```ts
it('shows cloud configuration controls in feature settings', async () => {
  const wrapper = mount(SettingsModal)
  await new Promise((resolve) => window.setTimeout(resolve, 0))

  expect(wrapper.find('.cloud-config-section').exists()).toBe(true)
  expect(wrapper.find('.cloud-url-input').exists()).toBe(true)
  expect(wrapper.find('.cloud-anon-key-input').exists()).toBe(true)
  expect(wrapper.find('.cloud-enabled-toggle').exists()).toBe(true)
  expect(wrapper.find('.cloud-status').text()).toContain('本地离线模式')
})

it('saves valid cloud configuration from settings', async () => {
  const wrapper = mount(SettingsModal)
  await new Promise((resolve) => window.setTimeout(resolve, 0))

  await wrapper.find('.cloud-url-input').setValue('https://example.supabase.co/')
  await wrapper.find('.cloud-anon-key-input').setValue('anon-test-key')
  await wrapper.find('.cloud-enabled-toggle').setValue(true)
  await wrapper.find('.cloud-save-button').trigger('click')
  await new Promise((resolve) => window.setTimeout(resolve, 0))

  const cloud = useCloudSessionStore()
  expect(cloud.readiness).toBe('configured')
  expect((wrapper.find('.cloud-anon-key-input').element as HTMLInputElement).value).toBe('')
  expect(wrapper.find('.cloud-status').text()).toContain('云端配置已保存')
})
```

- [ ] **Step 2: Run UI tests to verify failure**

Run:

```powershell
pnpm vitest run tests/components/SettingsModal.test.ts
```

Expected: FAIL because SettingsModal does not render cloud controls.

- [ ] **Step 3: Add SettingsModal script state**

Modify `src/components/SettingsModal.vue` script:

```ts
import { useCloudSessionStore } from '@/stores/cloudSession'
```

Add:

```ts
const cloud = useCloudSessionStore()
const cloudSupabaseUrl = ref('')
const cloudAnonKey = ref('')
const cloudEnabled = ref(false)
const cloudStatus = ref('')
```

In `onMounted`, load cloud state:

```ts
onMounted(async () => {
  await Promise.all([refreshAutostart(), loadApiProviders(), loadCloudSettings()])
})
```

Add functions:

```ts
function cloudStatusText() {
  if (cloudStatus.value) return cloudStatus.value
  if (cloud.readiness === 'configured') return '云端配置已保存'
  if (cloud.readiness === 'invalid') return '云端配置无效'
  return '本地离线模式'
}

async function loadCloudSettings() {
  await cloud.load()
  cloudSupabaseUrl.value = cloud.config?.supabaseUrl ?? ''
  cloudEnabled.value = cloud.config?.cloudEnabled ?? false
  cloudAnonKey.value = ''
}

async function saveCloudSettings() {
  await cloud.save({
    supabaseUrl: cloudSupabaseUrl.value,
    anonKey: cloudAnonKey.value || cloud.config?.anonKey || '',
    cloudEnabled: cloudEnabled.value,
  })
  cloudAnonKey.value = ''
  cloudStatus.value = cloud.error ? `保存失败：${cloud.error}` : cloudStatusText()
}
```

- [ ] **Step 4: Add SettingsModal template section**

Inside the `features` tab section, add:

```vue
<section class="cloud-config-section settings-section">
  <div class="section-heading">
    <strong>云端协作</strong>
    <p>配置 Supabase 后，下一阶段可登录并启用用户云空间。</p>
  </div>
  <label>
    Supabase URL
    <input
      v-model="cloudSupabaseUrl"
      class="cloud-url-input"
      placeholder="https://example.supabase.co"
    >
  </label>
  <label>
    Supabase anon key
    <input
      v-model="cloudAnonKey"
      class="cloud-anon-key-input"
      type="password"
      placeholder="留空表示不修改"
    >
  </label>
  <label class="cloud-enabled-row">
    <input
      v-model="cloudEnabled"
      class="cloud-enabled-toggle"
      type="checkbox"
    >
    启用云端配置
  </label>
  <div class="cloud-config-actions">
    <button
      class="cloud-save-button"
      type="button"
      :disabled="cloud.saving"
      @click="saveCloudSettings"
    >
      {{ cloud.saving ? '保存中...' : '保存云端配置' }}
    </button>
    <span class="cloud-status">{{ cloudStatusText() }}</span>
  </div>
</section>
```

- [ ] **Step 5: Add minimal SettingsModal styles**

In the same file, add styles matching existing compact settings density:

```css
.cloud-config-section {
  display: grid;
  gap: 12px;
}

.section-heading p,
.cloud-status {
  margin: 3px 0 0;
  color: var(--bb-text-soft);
  font-size: 11px;
}

.cloud-enabled-row {
  display: inline-flex;
  align-items: center;
  gap: 8px;
}

.cloud-enabled-row input {
  width: auto;
  min-height: 0;
}

.cloud-config-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}
```

- [ ] **Step 6: Run UI tests**

Run:

```powershell
pnpm vitest run tests/components/SettingsModal.test.ts tests/stores/cloudSession.test.ts tests/lib/cloud-config.test.ts
```

Expected: PASS.

## Task 6: Full Phase 1 Verification

**Files:**

- All Phase 1 files.

- [ ] **Step 1: Run frontend checks**

Run:

```powershell
pnpm check
```

Expected: PASS with all frontend tests passing.

- [ ] **Step 2: Run Rust checks**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml
```

Expected: PASS with all Rust tests passing.

- [ ] **Step 3: Inspect diff boundaries**

Run:

```powershell
git diff --stat
git diff -- src/components/SettingsModal.vue src/stores/cloudSession.ts src/lib/cloud-config.ts src-tauri/src/cloud_config.rs
```

Expected: diff contains only Phase 1 cloud foundation work plus any pre-existing unrelated local changes already noted before execution.

- [ ] **Step 4: Manual desktop smoke test**

Run local debug app:

```powershell
pnpm tauri dev
```

Manual checks:

- Settings opens.
- Feature settings show cloud collaboration config.
- Saving `https://example.supabase.co` plus `anon-test-key` shows configured status.
- Clearing `启用云端配置` returns local-only status.
- Existing prompt library, reverse image settings, project board, and daily tasks still open.

## Completion Criteria

Phase 1 is complete only when:

- Frontend tests pass.
- Rust tests pass.
- Settings UI has cloud config controls.
- Backend persists config in `cloud_config`.
- Service role key is blocked.
- Existing local-only app behavior remains unchanged.
- No login or data upload is implemented yet.
