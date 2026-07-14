# Banana Box v2 Phase 2 Auth Workspaces Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users sign in, sign out, load their app profile, and enter a default personal workspace while logged-out users remain local-only.

**Architecture:** Phase 2 adds a Supabase runtime client on the Vue side, backed by the Phase 1 cloud configuration. Supabase Auth owns login sessions; app-specific identity lives in `profiles`, `workspaces`, and `workspace_members`. Local prompt/project/daily-task data is not uploaded in this phase.

**Tech Stack:** Tauri 2, Rust, SQLite, Vue 3, Pinia, TypeScript, Supabase JS v2, Supabase Postgres, Supabase RLS, Vitest, Cargo tests.

---

## Scope

In scope:

- Install `@supabase/supabase-js`.
- Add a runtime-only cloud credential command so the Supabase client can be created after cloud config is enabled.
- Add Supabase SQL migration for `profiles`, `workspaces`, and `workspace_members`.
- Add email/password sign in, sign up, sign out, session restore, and auth state listener.
- Add workspace bootstrap: create or load user profile, create default personal workspace when missing, select active workspace.
- Add compact app-shell UI for login status and workspace selection.
- Keep logged-out mode local-only.

Out of scope:

- Uploading or migrating local prompts, projects, or daily tasks.
- Sync outbox, sync cursors, realtime subscriptions, invitations, comments, mentions, notifications, or conflict UI.
- Returning the anon key to settings fields or storing it in frontend app state.
- Supporting service-role keys in the desktop app.

## Security Decision

Supabase JS needs the anon key to authenticate. Phase 1 intentionally kept the anon key write-only in the settings UI. Phase 2 keeps that behavior but adds a narrower runtime command:

- `load_cloud_runtime_config` returns `{ supabaseUrl, anonKey, cloudEnabled }` only when cloud is enabled and a key exists.
- The settings UI still only receives `hasAnonKey`.
- The anon key is not shown in any input, not logged, and not saved in Pinia state.
- Supabase Auth session persistence uses the Supabase JS default storage for this phase. A later hardening phase can replace it with a native secure storage adapter if needed.

## File Structure

- Modify: `package.json`
  - Add `@supabase/supabase-js`.
- Modify: `src-tauri/src/cloud_config.rs`
  - Add runtime credential DTO and loader.
- Modify: `src-tauri/src/commands.rs`
  - Expose `load_cloud_runtime_config`.
- Modify: `src-tauri/src/lib.rs`
  - Register the new command.
- Modify: `src/lib/ipc.ts`
  - Add frontend IPC wrapper for runtime config.
- Modify: `src/types/cloud.ts`
  - Add `CloudRuntimeConfig`.
- Modify: `src/types/index.ts`
  - Re-export new cloud/auth/workspace types.
- Create: `src/types/auth.ts`
  - App profile and auth state types.
- Create: `src/types/workspace.ts`
  - Workspace, member, and role types.
- Create: `src/lib/supabaseClient.ts`
  - Create and cache Supabase client from runtime config.
- Create: `src/stores/auth.ts`
  - Own session restore, sign in, sign up, sign out, auth errors.
- Create: `src/stores/workspaces.ts`
  - Own profile loading, default workspace bootstrap, active workspace selection.
- Create: `src/components/auth/LoginPanel.vue`
  - Compact login/sign-up panel.
- Create: `src/components/workspaces/WorkspaceSwitcher.vue`
  - Shows local-only state, active workspace, and sign-out action.
- Create: `supabase/migrations/0001_auth_workspaces.sql`
  - Profiles, workspaces, workspace members, RLS.
- Modify: `src/App.vue`
  - Initialize cloud config/auth/workspaces on mount.
- Modify: `src/components/AppSidebar.vue`
  - Render login/workspace section above tools.
- Test: `tests/lib/supabaseClient.test.ts`
- Test: `tests/stores/auth.test.ts`
- Test: `tests/stores/workspaces.test.ts`
- Test: `tests/components/LoginPanel.test.ts`
- Test: `tests/components/WorkspaceSwitcher.test.ts`
- Test: Rust tests inside `src-tauri/src/cloud_config.rs`

## UI Design Plan

Visual purpose:

- Make cloud login feel like an app status control, not a marketing page.
- Keep the existing compact production-tool layout.
- Preserve the prompt/project/daily-task workspace area while adding identity controls in the sidebar.

Information hierarchy:

- Top of sidebar: cloud status, current user email or local-only label, current workspace name.
- Login panel appears inline in the sidebar when cloud config is ready but the user is logged out.
- Settings remains the place to configure Supabase URL and anon key.

Component behavior:

- Local-only: show `本地离线模式` and a settings hint.
- Configured but logged out: show email/password fields and login/sign-up toggle.
- Loading: disable buttons and show a short loading label.
- Error: show one short message under the controls.
- Logged in: show workspace selector and sign-out button.
- No workspace yet: show `正在创建个人工作区`.

Guardrails:

- Do not add a landing page.
- Do not add large cards inside the existing sidebar.
- Do not block local-only app usage.
- Do not place long cloud status text where it can overflow the sidebar.

---

## Task 1: Add Supabase Dependency

**Files:**

- Modify: `package.json`
- Modify: `pnpm-lock.yaml`

- [ ] **Step 1: Install dependency**

Run:

```powershell
pnpm add @supabase/supabase-js
```

Expected:

- `package.json` contains `@supabase/supabase-js`.
- `pnpm-lock.yaml` is updated.

- [ ] **Step 2: Verify dependency resolves**

Run:

```powershell
pnpm typecheck
```

Expected:

- Typecheck may still fail later if planned files are not created, but dependency resolution itself should not report that `@supabase/supabase-js` is missing after the package is added.

## Task 2: Add Runtime Cloud Config Command

**Files:**

- Modify: `src-tauri/src/cloud_config.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/ipc.ts`
- Modify: `src/types/cloud.ts`
- Modify: `src/types/index.ts`

- [ ] **Step 1: Write failing Rust tests**

Add tests in `src-tauri/src/cloud_config.rs`:

```rust
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
```

- [ ] **Step 2: Run failing Rust tests**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml cloud_config::tests::runtime_config -- --nocapture
```

Expected:

- FAIL because `load_cloud_runtime_config` does not exist yet.

- [ ] **Step 3: Implement runtime DTO and loader**

Add to `src-tauri/src/cloud_config.rs`:

```rust
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudRuntimeConfigDto {
    pub supabase_url: String,
    pub anon_key: String,
    pub cloud_enabled: bool,
}

pub fn load_cloud_runtime_config(db: &Database) -> Result<CloudRuntimeConfigDto, String> {
    db.with_connection(|connection| {
        let result = connection.query_row(
            "SELECT supabase_url, anon_key, cloud_enabled
             FROM cloud_config WHERE id = 'default'",
            [],
            |row| {
                let cloud_enabled = row.get::<_, i64>(2)? != 0;
                let anon_key = row.get::<_, String>(1)?;
                let supabase_url = row.get::<_, String>(0)?;
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
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(CloudRuntimeConfigDto {
                supabase_url: String::new(),
                anon_key: String::new(),
                cloud_enabled: false,
            }),
            Err(error) => Err(error.to_string()),
        }
    })
}
```

- [ ] **Step 4: Register Tauri command**

Add command wrapper in `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
pub fn load_cloud_runtime_config(
    main_args: tauri::State<'_, crate::MainArgs>,
) -> Result<crate::cloud_config::CloudRuntimeConfigDto, String> {
    let db = crate::db::Database::open_main(&main_args)?;
    crate::cloud_config::load_cloud_runtime_config(&db)
}
```

Add it to the command handler in `src-tauri/src/lib.rs` next to `load_cloud_config`:

```rust
commands::load_cloud_runtime_config,
```

- [ ] **Step 5: Add frontend types and IPC**

Add to `src/types/cloud.ts`:

```ts
export interface CloudRuntimeConfig {
  supabaseUrl: string
  anonKey: string
  cloudEnabled: boolean
}
```

Update `src/types/index.ts` export list to include `CloudRuntimeConfig`.

Update `src/lib/ipc.ts`:

```ts
import type { CloudConfig, CloudRuntimeConfig, Library, SaveCloudConfigInput } from '@/types'

export async function loadCloudRuntimeConfig(): Promise<CloudRuntimeConfig> {
  return await invoke<CloudRuntimeConfig>('load_cloud_runtime_config', {})
}
```

- [ ] **Step 6: Verify**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml cloud_config -- --nocapture
pnpm typecheck
```

Expected:

- Rust cloud config tests pass.
- TypeScript compiles after frontend imports are updated.

## Task 3: Create Auth And Workspace Types

**Files:**

- Create: `src/types/auth.ts`
- Create: `src/types/workspace.ts`
- Modify: `src/types/index.ts`

- [ ] **Step 1: Create auth types**

Create `src/types/auth.ts`:

```ts
export interface AppProfile {
  id: string
  email: string
  displayName: string
  avatarUrl: string | null
  createdAt: string
  updatedAt: string
}

export type AuthMode = 'sign-in' | 'sign-up'
```

- [ ] **Step 2: Create workspace types**

Create `src/types/workspace.ts`:

```ts
export type WorkspaceRole = 'owner' | 'editor' | 'commenter' | 'viewer'

export interface Workspace {
  id: string
  name: string
  ownerId: string
  createdAt: string
  updatedAt: string
}

export interface WorkspaceMember {
  workspaceId: string
  userId: string
  role: WorkspaceRole
  createdAt: string
}
```

- [ ] **Step 3: Re-export types**

Update `src/types/index.ts`:

```ts
export type { AppProfile, AuthMode } from './auth'
export type { Workspace, WorkspaceMember, WorkspaceRole } from './workspace'
```

- [ ] **Step 4: Verify**

Run:

```powershell
pnpm typecheck
```

Expected:

- PASS.

## Task 4: Add Supabase Client Factory

**Files:**

- Create: `src/lib/supabaseClient.ts`
- Test: `tests/lib/supabaseClient.test.ts`

- [ ] **Step 1: Write failing tests**

Create `tests/lib/supabaseClient.test.ts`:

```ts
import { describe, expect, it, vi } from 'vitest'
import { createSupabaseClientFromRuntimeConfig, getSupabaseClient } from '@/lib/supabaseClient'
import { loadCloudRuntimeConfig } from '@/lib/ipc'

vi.mock('@/lib/ipc', () => ({
  loadCloudRuntimeConfig: vi.fn(),
}))

vi.mock('@supabase/supabase-js', () => ({
  createClient: vi.fn((url: string, key: string) => ({
    __url: url,
    __key: key,
    auth: {},
    from: vi.fn(),
  })),
}))

describe('supabase client factory', () => {
  it('returns null when cloud runtime config is disabled', async () => {
    vi.mocked(loadCloudRuntimeConfig).mockResolvedValue({
      supabaseUrl: '',
      anonKey: '',
      cloudEnabled: false,
    })

    await expect(getSupabaseClient()).resolves.toBeNull()
  })

  it('creates a client when runtime config is enabled', async () => {
    vi.mocked(loadCloudRuntimeConfig).mockResolvedValue({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: 'anon-test-key',
      cloudEnabled: true,
    })

    const client = await getSupabaseClient()

    expect(client).not.toBeNull()
  })

  it('rejects incomplete runtime config', () => {
    expect(() => createSupabaseClientFromRuntimeConfig({
      supabaseUrl: 'https://example.supabase.co',
      anonKey: '',
      cloudEnabled: true,
    })).toThrow('CLOUD_RUNTIME_CONFIG_INCOMPLETE')
  })
})
```

- [ ] **Step 2: Run failing tests**

Run:

```powershell
pnpm vitest run tests/lib/supabaseClient.test.ts
```

Expected:

- FAIL because `src/lib/supabaseClient.ts` does not exist.

- [ ] **Step 3: Implement client factory**

Create `src/lib/supabaseClient.ts`:

```ts
import { createClient, type SupabaseClient } from '@supabase/supabase-js'
import { loadCloudRuntimeConfig } from '@/lib/ipc'
import type { CloudRuntimeConfig } from '@/types'

let cachedClient: SupabaseClient | null = null
let cachedSignature = ''

function runtimeSignature(config: CloudRuntimeConfig) {
  return `${config.supabaseUrl}|${config.anonKey.slice(0, 8)}|${config.cloudEnabled}`
}

export function createSupabaseClientFromRuntimeConfig(config: CloudRuntimeConfig): SupabaseClient {
  if (!config.cloudEnabled || !config.supabaseUrl || !config.anonKey) {
    throw new Error('CLOUD_RUNTIME_CONFIG_INCOMPLETE')
  }

  return createClient(config.supabaseUrl, config.anonKey, {
    auth: {
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: false,
    },
  })
}

export async function getSupabaseClient(): Promise<SupabaseClient | null> {
  const config = await loadCloudRuntimeConfig()
  if (!config.cloudEnabled || !config.supabaseUrl || !config.anonKey) return null

  const signature = runtimeSignature(config)
  if (!cachedClient || cachedSignature !== signature) {
    cachedClient = createSupabaseClientFromRuntimeConfig(config)
    cachedSignature = signature
  }

  return cachedClient
}

export function clearSupabaseClientForTests() {
  cachedClient = null
  cachedSignature = ''
}
```

- [ ] **Step 4: Verify**

Run:

```powershell
pnpm vitest run tests/lib/supabaseClient.test.ts
pnpm typecheck
```

Expected:

- PASS.

## Task 5: Add Supabase Auth Workspace SQL

**Files:**

- Create: `supabase/migrations/0001_auth_workspaces.sql`

- [ ] **Step 1: Create migration**

Create `supabase/migrations/0001_auth_workspaces.sql`:

```sql
create extension if not exists pgcrypto;

create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  email text not null,
  display_name text not null,
  avatar_url text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.workspaces (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  owner_id uuid not null references public.profiles(id) on delete cascade,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.workspace_members (
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  user_id uuid not null references public.profiles(id) on delete cascade,
  role text not null check (role in ('owner', 'editor', 'commenter', 'viewer')),
  created_at timestamptz not null default now(),
  primary key (workspace_id, user_id)
);

alter table public.profiles enable row level security;
alter table public.workspaces enable row level security;
alter table public.workspace_members enable row level security;

create policy "profiles can read own profile"
on public.profiles for select
to authenticated
using (id = auth.uid());

create policy "profiles can upsert own profile"
on public.profiles for insert
to authenticated
with check (id = auth.uid());

create policy "profiles can update own profile"
on public.profiles for update
to authenticated
using (id = auth.uid())
with check (id = auth.uid());

create policy "members can read their workspaces"
on public.workspaces for select
to authenticated
using (
  exists (
    select 1 from public.workspace_members wm
    where wm.workspace_id = workspaces.id
      and wm.user_id = auth.uid()
  )
);

create policy "users can create owned workspaces"
on public.workspaces for insert
to authenticated
with check (owner_id = auth.uid());

create policy "members can read workspace memberships"
on public.workspace_members for select
to authenticated
using (
  exists (
    select 1 from public.workspace_members own_membership
    where own_membership.workspace_id = workspace_members.workspace_id
      and own_membership.user_id = auth.uid()
  )
);

create policy "owners can create their own initial membership"
on public.workspace_members for insert
to authenticated
with check (
  user_id = auth.uid()
  and role = 'owner'
  and exists (
    select 1 from public.workspaces w
    where w.id = workspace_members.workspace_id
      and w.owner_id = auth.uid()
  )
);

create index if not exists workspace_members_user_id_idx
on public.workspace_members(user_id);
```

- [ ] **Step 2: Local SQL verification**

If Supabase CLI is available, run:

```powershell
supabase db reset
```

Expected:

- Migration applies without SQL errors.

If Supabase CLI is not installed, run:

```powershell
Test-Path supabase\migrations\0001_auth_workspaces.sql
```

Expected:

- `True`; full SQL verification remains pending until Supabase CLI is installed or a hosted project is used.

## Task 6: Add Auth Store

**Files:**

- Create: `src/stores/auth.ts`
- Test: `tests/stores/auth.test.ts`

- [ ] **Step 1: Write failing tests**

Create `tests/stores/auth.test.ts` with a mocked Supabase client:

```ts
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useAuthStore } from '@/stores/auth'
import { getSupabaseClient } from '@/lib/supabaseClient'

const authApi = {
  getSession: vi.fn(),
  signInWithPassword: vi.fn(),
  signUp: vi.fn(),
  signOut: vi.fn(),
  onAuthStateChange: vi.fn(),
}

vi.mock('@/lib/supabaseClient', () => ({
  getSupabaseClient: vi.fn(),
}))

describe('auth store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    authApi.onAuthStateChange.mockReturnValue({
      data: { subscription: { unsubscribe: vi.fn() } },
    })
  })

  it('stays local-only when no Supabase client is available', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue(null)
    const store = useAuthStore()

    await store.initialize()

    expect(store.cloudAvailable).toBe(false)
    expect(store.user).toBeNull()
  })

  it('restores an existing session', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({
      data: { session: { user: { id: 'user-1', email: 'a@example.com' } } },
      error: null,
    })
    const store = useAuthStore()

    await store.initialize()

    expect(store.cloudAvailable).toBe(true)
    expect(store.user?.id).toBe('user-1')
  })

  it('signs in with email and password', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({ data: { session: null }, error: null })
    authApi.signInWithPassword.mockResolvedValue({
      data: { session: { user: { id: 'user-2', email: 'b@example.com' } } },
      error: null,
    })
    const store = useAuthStore()
    await store.initialize()

    await store.signIn('b@example.com', 'password123')

    expect(authApi.signInWithPassword).toHaveBeenCalledWith({
      email: 'b@example.com',
      password: 'password123',
    })
    expect(store.user?.id).toBe('user-2')
  })

  it('records sign-in errors without throwing', async () => {
    vi.mocked(getSupabaseClient).mockResolvedValue({ auth: authApi } as never)
    authApi.getSession.mockResolvedValue({ data: { session: null }, error: null })
    authApi.signInWithPassword.mockResolvedValue({
      data: { session: null },
      error: { message: 'Invalid login credentials' },
    })
    const store = useAuthStore()
    await store.initialize()

    await store.signIn('bad@example.com', 'bad')

    expect(store.error).toBe('Invalid login credentials')
    expect(store.user).toBeNull()
  })
})
```

- [ ] **Step 2: Run failing tests**

Run:

```powershell
pnpm vitest run tests/stores/auth.test.ts
```

Expected:

- FAIL because `src/stores/auth.ts` does not exist.

- [ ] **Step 3: Implement auth store**

Create `src/stores/auth.ts`:

```ts
import { defineStore } from 'pinia'
import type { Session, SupabaseClient, User } from '@supabase/supabase-js'
import { getSupabaseClient } from '@/lib/supabaseClient'

export const useAuthStore = defineStore('auth', {
  state: () => ({
    client: null as SupabaseClient | null,
    session: null as Session | null,
    user: null as User | null,
    cloudAvailable: false,
    loading: false,
    error: '',
    unsubscribeAuth: null as null | (() => void),
  }),
  actions: {
    async initialize() {
      this.loading = true
      this.error = ''
      this.client = await getSupabaseClient()
      this.cloudAvailable = Boolean(this.client)
      if (!this.client) {
        this.session = null
        this.user = null
        this.loading = false
        return
      }

      const { data, error } = await this.client.auth.getSession()
      if (error) this.error = error.message
      this.session = data.session
      this.user = data.session?.user ?? null

      this.unsubscribeAuth?.()
      const { data: listener } = this.client.auth.onAuthStateChange((_event, session) => {
        this.session = session
        this.user = session?.user ?? null
      })
      this.unsubscribeAuth = () => listener.subscription.unsubscribe()
      this.loading = false
    },
    async signIn(email: string, password: string) {
      if (!this.client) await this.initialize()
      if (!this.client) {
        this.error = 'CLOUD_NOT_CONFIGURED'
        return
      }
      this.loading = true
      this.error = ''
      const { data, error } = await this.client.auth.signInWithPassword({ email, password })
      if (error) {
        this.error = error.message
        this.session = null
        this.user = null
      } else {
        this.session = data.session
        this.user = data.session?.user ?? null
      }
      this.loading = false
    },
    async signUp(email: string, password: string) {
      if (!this.client) await this.initialize()
      if (!this.client) {
        this.error = 'CLOUD_NOT_CONFIGURED'
        return
      }
      this.loading = true
      this.error = ''
      const { data, error } = await this.client.auth.signUp({ email, password })
      if (error) this.error = error.message
      this.session = data.session
      this.user = data.user ?? data.session?.user ?? null
      this.loading = false
    },
    async signOut() {
      if (this.client) await this.client.auth.signOut()
      this.session = null
      this.user = null
    },
    dispose() {
      this.unsubscribeAuth?.()
      this.unsubscribeAuth = null
    },
  },
})
```

- [ ] **Step 4: Verify**

Run:

```powershell
pnpm vitest run tests/stores/auth.test.ts
pnpm typecheck
```

Expected:

- PASS.

## Task 7: Add Workspaces Store

**Files:**

- Create: `src/stores/workspaces.ts`
- Test: `tests/stores/workspaces.test.ts`

- [ ] **Step 1: Write failing tests**

Create `tests/stores/workspaces.test.ts` using a chainable mock for `from()`:

```ts
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useWorkspacesStore } from '@/stores/workspaces'

function tableMock(result: unknown) {
  const query = {
    select: vi.fn(() => query),
    eq: vi.fn(() => query),
    single: vi.fn(async () => result),
    maybeSingle: vi.fn(async () => result),
    insert: vi.fn(() => query),
    upsert: vi.fn(() => query),
    order: vi.fn(() => query),
  }
  return query
}

describe('workspaces store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('upserts profile and loads existing workspace', async () => {
    const profileResult = { data: {
      id: 'user-1',
      email: 'a@example.com',
      display_name: 'a',
      avatar_url: null,
      created_at: 'now',
      updated_at: 'now',
    }, error: null }
    const membershipsResult = { data: [{
      role: 'owner',
      workspaces: {
        id: 'workspace-1',
        name: 'a 的个人空间',
        owner_id: 'user-1',
        created_at: 'now',
        updated_at: 'now',
      },
    }], error: null }
    const client = {
      from: vi.fn((table: string) => {
        if (table === 'profiles') return tableMock(profileResult)
        return tableMock(membershipsResult)
      }),
    }
    const store = useWorkspacesStore()

    await store.bootstrapForUser(client as never, { id: 'user-1', email: 'a@example.com' } as never)

    expect(store.profile?.id).toBe('user-1')
    expect(store.activeWorkspace?.id).toBe('workspace-1')
  })

  it('creates a default workspace when no membership exists', async () => {
    const profileResult = { data: {
      id: 'user-1',
      email: 'a@example.com',
      display_name: 'a',
      avatar_url: null,
      created_at: 'now',
      updated_at: 'now',
    }, error: null }
    const emptyMemberships = { data: [], error: null }
    const workspaceResult = { data: {
      id: 'workspace-1',
      name: 'a 的个人空间',
      owner_id: 'user-1',
      created_at: 'now',
      updated_at: 'now',
    }, error: null }
    const memberResult = { data: null, error: null }
    const client = {
      from: vi.fn((table: string) => {
        if (table === 'profiles') return tableMock(profileResult)
        if (table === 'workspaces') return tableMock(workspaceResult)
        if (table === 'workspace_members') {
          if (client.from.mock.calls.filter(([name]) => name === 'workspace_members').length === 1) {
            return tableMock(emptyMemberships)
          }
          return tableMock(memberResult)
        }
        return tableMock({ data: null, error: null })
      }),
    }
    const store = useWorkspacesStore()

    await store.bootstrapForUser(client as never, { id: 'user-1', email: 'a@example.com' } as never)

    expect(store.activeWorkspace?.id).toBe('workspace-1')
  })
})
```

- [ ] **Step 2: Run failing tests**

Run:

```powershell
pnpm vitest run tests/stores/workspaces.test.ts
```

Expected:

- FAIL because `src/stores/workspaces.ts` does not exist.

- [ ] **Step 3: Implement workspaces store**

Create `src/stores/workspaces.ts`:

```ts
import { defineStore } from 'pinia'
import type { SupabaseClient, User } from '@supabase/supabase-js'
import type { AppProfile, Workspace } from '@/types'

interface WorkspaceMembershipRow {
  role: string
  workspaces: {
    id: string
    name: string
    owner_id: string
    created_at: string
    updated_at: string
  } | null
}

function displayNameFromEmail(email: string) {
  return email.split('@')[0] || 'Banana Box User'
}

function mapProfile(row: {
  id: string
  email: string
  display_name: string
  avatar_url: string | null
  created_at: string
  updated_at: string
}): AppProfile {
  return {
    id: row.id,
    email: row.email,
    displayName: row.display_name,
    avatarUrl: row.avatar_url,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  }
}

function mapWorkspace(row: NonNullable<WorkspaceMembershipRow['workspaces']>): Workspace {
  return {
    id: row.id,
    name: row.name,
    ownerId: row.owner_id,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  }
}

export const useWorkspacesStore = defineStore('workspaces', {
  state: () => ({
    profile: null as AppProfile | null,
    workspaces: [] as Workspace[],
    activeWorkspaceId: '',
    loading: false,
    error: '',
  }),
  getters: {
    activeWorkspace(state): Workspace | null {
      return state.workspaces.find((workspace) => workspace.id === state.activeWorkspaceId) ?? null
    },
  },
  actions: {
    async bootstrapForUser(client: SupabaseClient, user: User) {
      this.loading = true
      this.error = ''
      const email = user.email ?? ''
      const displayName = displayNameFromEmail(email)

      const profileResponse = await client
        .from('profiles')
        .upsert({
          id: user.id,
          email,
          display_name: displayName,
          avatar_url: null,
        })
        .select()
        .single()

      if (profileResponse.error) {
        this.error = profileResponse.error.message
        this.loading = false
        return
      }
      this.profile = mapProfile(profileResponse.data)

      await this.loadMembershipsOrCreateDefault(client, user.id, displayName)
      this.loading = false
    },
    async loadMembershipsOrCreateDefault(client: SupabaseClient, userId: string, displayName: string) {
      const membershipsResponse = await client
        .from('workspace_members')
        .select('role, workspaces(id, name, owner_id, created_at, updated_at)')
        .eq('user_id', userId)
        .order('created_at')

      if (membershipsResponse.error) {
        this.error = membershipsResponse.error.message
        return
      }

      const existing = ((membershipsResponse.data ?? []) as WorkspaceMembershipRow[])
        .map((row) => row.workspaces ? mapWorkspace(row.workspaces) : null)
        .filter((workspace): workspace is Workspace => Boolean(workspace))

      if (existing.length > 0) {
        this.workspaces = existing
        this.activeWorkspaceId = this.activeWorkspaceId || existing[0].id
        return
      }

      const workspaceResponse = await client
        .from('workspaces')
        .insert({
          name: `${displayName} 的个人空间`,
          owner_id: userId,
        })
        .select()
        .single()

      if (workspaceResponse.error) {
        this.error = workspaceResponse.error.message
        return
      }

      await client.from('workspace_members').insert({
        workspace_id: workspaceResponse.data.id,
        user_id: userId,
        role: 'owner',
      })

      const workspace = mapWorkspace(workspaceResponse.data)
      this.workspaces = [workspace]
      this.activeWorkspaceId = workspace.id
    },
    setActiveWorkspace(workspaceId: string) {
      if (this.workspaces.some((workspace) => workspace.id === workspaceId)) {
        this.activeWorkspaceId = workspaceId
      }
    },
    clear() {
      this.profile = null
      this.workspaces = []
      this.activeWorkspaceId = ''
      this.error = ''
    },
  },
})
```

- [ ] **Step 4: Verify**

Run:

```powershell
pnpm vitest run tests/stores/workspaces.test.ts
pnpm typecheck
```

Expected:

- PASS.

## Task 8: Add Login Panel UI

**Files:**

- Create: `src/components/auth/LoginPanel.vue`
- Test: `tests/components/LoginPanel.test.ts`

- [ ] **Step 1: Write failing component tests**

Create `tests/components/LoginPanel.test.ts`:

```ts
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import LoginPanel from '@/components/auth/LoginPanel.vue'
import { useAuthStore } from '@/stores/auth'

describe('LoginPanel', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('signs in with email and password', async () => {
    const auth = useAuthStore()
    auth.signIn = vi.fn()
    const wrapper = mount(LoginPanel)

    await wrapper.find('[data-field="auth-email"]').setValue('a@example.com')
    await wrapper.find('[data-field="auth-password"]').setValue('password123')
    await wrapper.find('[data-action="auth-submit"]').trigger('click')

    expect(auth.signIn).toHaveBeenCalledWith('a@example.com', 'password123')
  })

  it('can switch to sign-up mode', async () => {
    const auth = useAuthStore()
    auth.signUp = vi.fn()
    const wrapper = mount(LoginPanel)

    await wrapper.find('[data-action="auth-mode"]').trigger('click')
    await wrapper.find('[data-field="auth-email"]').setValue('b@example.com')
    await wrapper.find('[data-field="auth-password"]').setValue('password123')
    await wrapper.find('[data-action="auth-submit"]').trigger('click')

    expect(auth.signUp).toHaveBeenCalledWith('b@example.com', 'password123')
  })
})
```

- [ ] **Step 2: Run failing test**

Run:

```powershell
pnpm vitest run tests/components/LoginPanel.test.ts
```

Expected:

- FAIL because component does not exist.

- [ ] **Step 3: Implement compact login panel**

Create `src/components/auth/LoginPanel.vue` with:

```vue
<script setup lang="ts">
import { ref } from 'vue'
import { LogIn, UserPlus } from '@lucide/vue'
import { useAuthStore } from '@/stores/auth'
import type { AuthMode } from '@/types'

const auth = useAuthStore()
const mode = ref<AuthMode>('sign-in')
const email = ref('')
const password = ref('')

async function submit() {
  if (mode.value === 'sign-in') {
    await auth.signIn(email.value.trim(), password.value)
  } else {
    await auth.signUp(email.value.trim(), password.value)
  }
  password.value = ''
}

function toggleMode() {
  mode.value = mode.value === 'sign-in' ? 'sign-up' : 'sign-in'
}
</script>

<template>
  <section class="login-panel" aria-label="云端登录">
    <p class="login-title">{{ mode === 'sign-in' ? '登录云端' : '注册账号' }}</p>
    <input
      v-model="email"
      class="login-input"
      data-field="auth-email"
      type="email"
      autocomplete="email"
      placeholder="邮箱"
    >
    <input
      v-model="password"
      class="login-input"
      data-field="auth-password"
      type="password"
      autocomplete="current-password"
      placeholder="密码"
      @keydown.enter="submit"
    >
    <button
      class="login-submit"
      data-action="auth-submit"
      type="button"
      :disabled="auth.loading || !email || password.length < 6"
      @click="submit"
    >
      <LogIn v-if="mode === 'sign-in'" :size="14" aria-hidden="true" />
      <UserPlus v-else :size="14" aria-hidden="true" />
      <span>{{ mode === 'sign-in' ? '登录' : '注册' }}</span>
    </button>
    <button
      class="login-mode"
      data-action="auth-mode"
      type="button"
      @click="toggleMode"
    >
      {{ mode === 'sign-in' ? '创建新账号' : '返回登录' }}
    </button>
    <p v-if="auth.error" class="login-error">{{ auth.error }}</p>
  </section>
</template>
```

Add scoped styles matching compact sidebar controls. Keep button height at least `32px`, input width `100%`, and text wrapping disabled with ellipsis where needed.

- [ ] **Step 4: Verify**

Run:

```powershell
pnpm vitest run tests/components/LoginPanel.test.ts
pnpm typecheck
```

Expected:

- PASS.

## Task 9: Add Workspace Switcher UI

**Files:**

- Create: `src/components/workspaces/WorkspaceSwitcher.vue`
- Test: `tests/components/WorkspaceSwitcher.test.ts`

- [ ] **Step 1: Write failing component tests**

Create `tests/components/WorkspaceSwitcher.test.ts`:

```ts
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import WorkspaceSwitcher from '@/components/workspaces/WorkspaceSwitcher.vue'
import { useAuthStore } from '@/stores/auth'
import { useWorkspacesStore } from '@/stores/workspaces'

describe('WorkspaceSwitcher', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('shows local-only state when cloud is unavailable', () => {
    const auth = useAuthStore()
    auth.cloudAvailable = false

    const wrapper = mount(WorkspaceSwitcher)

    expect(wrapper.text()).toContain('本地离线模式')
  })

  it('shows active workspace for a logged-in user', () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    auth.user = { id: 'user-1', email: 'a@example.com' } as never
    const workspaces = useWorkspacesStore()
    workspaces.workspaces = [{
      id: 'workspace-1',
      name: '个人空间',
      ownerId: 'user-1',
      createdAt: 'now',
      updatedAt: 'now',
    }]
    workspaces.activeWorkspaceId = 'workspace-1'

    const wrapper = mount(WorkspaceSwitcher)

    expect(wrapper.text()).toContain('个人空间')
    expect(wrapper.text()).toContain('a@example.com')
  })

  it('signs out and clears workspace state', async () => {
    const auth = useAuthStore()
    auth.cloudAvailable = true
    auth.user = { id: 'user-1', email: 'a@example.com' } as never
    auth.signOut = vi.fn()
    const workspaces = useWorkspacesStore()
    workspaces.clear = vi.fn()

    const wrapper = mount(WorkspaceSwitcher)
    await wrapper.find('[data-action="auth-sign-out"]').trigger('click')

    expect(auth.signOut).toHaveBeenCalled()
    expect(workspaces.clear).toHaveBeenCalled()
  })
})
```

- [ ] **Step 2: Run failing test**

Run:

```powershell
pnpm vitest run tests/components/WorkspaceSwitcher.test.ts
```

Expected:

- FAIL because component does not exist.

- [ ] **Step 3: Implement switcher**

Create `src/components/workspaces/WorkspaceSwitcher.vue` with:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import { LogOut, Cloud, HardDrive } from '@lucide/vue'
import { useAuthStore } from '@/stores/auth'
import { useWorkspacesStore } from '@/stores/workspaces'

const auth = useAuthStore()
const workspaces = useWorkspacesStore()
const activeWorkspace = computed(() => workspaces.activeWorkspace)

async function signOut() {
  await auth.signOut()
  workspaces.clear()
}
</script>

<template>
  <section class="workspace-switcher" aria-label="当前工作区">
    <div v-if="!auth.cloudAvailable" class="workspace-state">
      <HardDrive :size="14" aria-hidden="true" />
      <span>本地离线模式</span>
    </div>
    <template v-else-if="auth.user">
      <div class="workspace-state">
        <Cloud :size="14" aria-hidden="true" />
        <span>{{ activeWorkspace?.name || '正在创建个人工作区' }}</span>
      </div>
      <p class="workspace-user">{{ auth.user.email }}</p>
      <button
        class="workspace-sign-out"
        data-action="auth-sign-out"
        type="button"
        @click="signOut"
      >
        <LogOut :size="14" aria-hidden="true" />
        <span>退出</span>
      </button>
    </template>
    <div v-else class="workspace-state">
      <Cloud :size="14" aria-hidden="true" />
      <span>云端未登录</span>
    </div>
  </section>
</template>
```

Add compact scoped styles. Use `overflow: hidden`, `text-overflow: ellipsis`, and `white-space: nowrap` for workspace and email text.

- [ ] **Step 4: Verify**

Run:

```powershell
pnpm vitest run tests/components/WorkspaceSwitcher.test.ts
pnpm typecheck
```

Expected:

- PASS.

## Task 10: Wire Auth Into App Shell

**Files:**

- Modify: `src/App.vue`
- Modify: `src/components/AppSidebar.vue`
- Test: update existing app/sidebar component tests if present, otherwise rely on component store tests and typecheck.

- [ ] **Step 1: Modify App initialization**

In `src/App.vue`, import stores:

```ts
import { watch } from 'vue'
import { useCloudSessionStore } from '@/stores/cloudSession'
import { useAuthStore } from '@/stores/auth'
import { useWorkspacesStore } from '@/stores/workspaces'
```

Create store instances:

```ts
const cloud = useCloudSessionStore()
const auth = useAuthStore()
const workspaces = useWorkspacesStore()
```

Update `onMounted`:

```ts
onMounted(async () => {
  await Promise.all([lib.load(), projects.load(), cloud.load()])
  await auth.initialize()
  if (auth.client && auth.user) {
    await workspaces.bootstrapForUser(auth.client, auth.user)
  }
  fullscreen.value = await getCurrentWindow().isFullscreen().catch(() => false)
  ui.showPanel()
  window.addEventListener('mouseup', clearResizeActive)
  unlistenFloatingDrop = await listen('floating-file-dropped', (event) => {
    if (!isFloatingFileDropPayload(event.payload)) return
    ui.showPanel()
    ui.openFloatingActionDialog(event.payload)
  })
})
```

Add watcher:

```ts
watch(
  () => auth.user?.id ?? '',
  async (userId) => {
    if (!userId || !auth.client || !auth.user) {
      workspaces.clear()
      return
    }
    await workspaces.bootstrapForUser(auth.client, auth.user)
  },
)
```

Update `onUnmounted`:

```ts
auth.dispose()
```

- [ ] **Step 2: Modify sidebar**

In `src/components/AppSidebar.vue`, import:

```ts
import LoginPanel from '@/components/auth/LoginPanel.vue'
import WorkspaceSwitcher from '@/components/workspaces/WorkspaceSwitcher.vue'
import { useAuthStore } from '@/stores/auth'
```

Create auth store:

```ts
const auth = useAuthStore()
```

Render at top of nav before tools:

```vue
<WorkspaceSwitcher />
<LoginPanel v-if="auth.cloudAvailable && !auth.user" />
<div class="sidebar-divider" aria-hidden="true" />
```

Add style:

```css
.sidebar-divider {
  height: 1px;
  margin: 2px 0 4px;
  background: rgba(123, 255, 226, 0.12);
}
```

- [ ] **Step 3: Verify**

Run:

```powershell
pnpm check
```

Expected:

- Typecheck, lint, and Vitest pass.

## Task 11: Desktop Debug Verification

**Files:**

- No code changes.

- [ ] **Step 1: Start debug mode**

Run:

```powershell
pnpm tauri dev
```

Expected:

- App opens without getting stuck on the recovery page.
- Logged-out users can still use prompt library, project management, and daily tasks.

- [ ] **Step 2: Verify local-only UI**

Manual checks:

- Sidebar shows `本地离线模式` when cloud config is disabled.
- Existing tools still open.
- Settings still opens.

- [ ] **Step 3: Verify cloud-ready logged-out UI**

Manual checks:

- Save a valid Supabase URL and anon key in settings.
- Sidebar changes to `云端未登录`.
- Login form appears.
- Password field clears after login/sign-up attempt.

- [ ] **Step 4: Verify logged-in workspace UI**

Manual checks with a real Supabase test project:

- Sign up or sign in.
- Sidebar shows the email.
- App creates or loads one personal workspace.
- Sign out clears the workspace display and returns to logged-out state.
- Prompt/project/daily-task local data is not uploaded or deleted.

## Task 12: Full Verification

**Files:**

- No code changes unless verification exposes a bug.

- [ ] **Step 1: Frontend verification**

Run:

```powershell
pnpm check
```

Expected:

- PASS.

- [ ] **Step 2: Rust verification**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml
```

Expected:

- PASS.

- [ ] **Step 3: Worktree review**

Run:

```powershell
git status --short
```

Expected:

- Only intentional Phase 2 files are changed, plus dependency lockfile.
- No API keys or Supabase service-role secrets appear in tracked files.

---

## Acceptance Criteria

- User can configure Supabase in settings and keep using local-only mode while logged out.
- User can sign up, sign in, restore session after restart, and sign out.
- App creates or loads the user profile.
- App creates or loads a default personal workspace.
- Sidebar shows local-only, cloud logged-out, and cloud logged-in states clearly.
- No prompt/project/daily-task data is uploaded in Phase 2.
- Settings UI never echoes the anon key.
- Service-role keys remain blocked.
- `pnpm check` passes.
- `cargo test --manifest-path src-tauri\Cargo.toml` passes.

## Self-Review

Spec coverage:

- Login/logout: covered by Tasks 4, 6, 8, 10, and 11.
- Profiles: covered by Tasks 5 and 7.
- Default workspace: covered by Tasks 5, 7, 9, 10, and 11.
- Logged-out local-only mode: covered by Tasks 6, 9, 10, and 11.
- No migration/upload: explicitly out of scope and manually verified in Task 11.
- Security boundary for anon key: covered by Tasks 2 and 4.

Placeholder scan:

- No implementation step uses TBD, TODO, or unspecified error handling.
- The only conditional path is Supabase CLI availability, with an explicit fallback check.

Type consistency:

- `CloudRuntimeConfig`, `AppProfile`, `Workspace`, and `WorkspaceRole` names are introduced before use.
- Store and component imports use the same file names created in earlier tasks.

## Execution Choice

Plan complete. Recommended execution after user approval:

1. **Inline Execution**: best for this repo right now because Phase 1 has uncommitted changes and Phase 2 touches connected app-shell files.
2. **Subagent-Driven**: possible, but less ideal until Phase 1 changes are committed or clearly separated.

Do not implement this plan until the user confirms.
