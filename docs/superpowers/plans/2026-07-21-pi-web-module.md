# PI-Web Module Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Banana Box `PI-Web` module that checks the local runtime, starts PI-Web as a managed background service, and opens PI-Web in a separate browser/window.

**Architecture:** The Vue frontend adds a compact PI-Web control page and routes it through the existing `ui.activeTool` pattern. The Tauri backend adds a focused `pi_web` module that owns status, environment checks, process startup, endpoint polling, and process cleanup. The first implementation supports a bundled PI-Web resource path when present and provides a Node/npm fallback with clear diagnostics so the UI is usable before final release packaging is tuned.

**Tech Stack:** Vue 3, TypeScript, Pinia, Vitest, Tauri 2, Rust, reqwest, std::process.

---

## File Structure

- Create: `src/components/piweb/PiWebPage.vue`
  - Renders the Banana Box PI-Web control console.
- Create: `src/lib/piWebIpc.ts`
  - Type-safe frontend wrappers for PI-Web Tauri commands.
- Modify: `src/stores/ui.ts`
  - Adds `pi-web` to `ActiveTool`.
- Modify: `src/components/AppSidebar.vue`
  - Adds the `PI-Web` sidebar item.
- Modify: `src/App.vue`
  - Imports and renders `PiWebPage` for `ui.activeTool === 'pi-web'`.
- Create: `tests/components/PiWebPage.test.ts`
  - Covers main UI states and button calls.
- Modify: `tests/components/AppSidebar.test.ts`
  - Verifies the sidebar can switch to PI-Web.
- Create: `src-tauri/src/pi_web.rs`
  - Owns PI-Web backend types, status, checks, process lifecycle, endpoint polling, and Tauri commands.
- Modify: `src-tauri/src/lib.rs`
  - Registers the PI-Web module state and Tauri commands.
- Modify: `src-tauri/tauri.conf.json`
  - Adds a future resource entry for bundled PI-Web files when present.

## Task 1: Frontend IPC Types

**Files:**
- Create: `src/lib/piWebIpc.ts`

- [ ] **Step 1: Create the IPC wrapper**

```ts
import { invoke } from '@tauri-apps/api/core'

export type PiWebServiceState = 'missingRuntime' | 'stopped' | 'checking' | 'starting' | 'running' | 'error'

export interface PiWebDiagnosticLink {
  label: string
  url: string
}

export interface PiWebStatus {
  state: PiWebServiceState
  url: string
  port: number
  message: string
  detail: string
  missingDependency: string
  installLinks: PiWebDiagnosticLink[]
  canStart: boolean
  canOpen: boolean
  canStop: boolean
}

export async function getPiWebStatus(): Promise<PiWebStatus> {
  return await invoke<PiWebStatus>('get_pi_web_status', {})
}

export async function startPiWeb(): Promise<PiWebStatus> {
  return await invoke<PiWebStatus>('start_pi_web', {})
}

export async function stopPiWeb(): Promise<PiWebStatus> {
  return await invoke<PiWebStatus>('stop_pi_web', {})
}

export async function openPiWeb(): Promise<PiWebStatus> {
  return await invoke<PiWebStatus>('open_pi_web', {})
}
```

- [ ] **Step 2: Run frontend typecheck**

Run: `pnpm typecheck`

Expected: FAIL if backend command strings are irrelevant to TypeScript; PASS once the file compiles.

## Task 2: UI Routing And Sidebar

**Files:**
- Modify: `src/stores/ui.ts`
- Modify: `src/components/AppSidebar.vue`
- Modify: `tests/components/AppSidebar.test.ts`

- [ ] **Step 1: Add a failing sidebar test**

Add this test to `tests/components/AppSidebar.test.ts`:

```ts
  it('switches to the PI-Web tool', async () => {
    const wrapper = mount(AppSidebar)
    const ui = useUiStore()

    await wrapper.get('[data-tool="pi-web"]').trigger('click')

    expect(ui.activeTool).toBe('pi-web')
    expect(wrapper.text()).toContain('PI-Web')
  })
```

- [ ] **Step 2: Run the sidebar test and confirm failure**

Run: `pnpm test tests/components/AppSidebar.test.ts`

Expected: FAIL because `[data-tool="pi-web"]` does not exist yet.

- [ ] **Step 3: Add the active tool type**

In `src/stores/ui.ts`, extend `ActiveTool`:

```ts
export type ActiveTool =
  | 'shared-library'
  | 'prompts'
  | 'reverse-image'
  | 'compression'
  | 'projects'
  | 'daily-tasks'
  | 'storyboard'
  | 'pi-web'
```

- [ ] **Step 4: Add the sidebar item**

In `src/components/AppSidebar.vue`, add the item to `tools`:

```ts
  { id: 'pi-web', label: 'PI-Web' },
```

- [ ] **Step 5: Run the sidebar test**

Run: `pnpm test tests/components/AppSidebar.test.ts`

Expected: PASS.

## Task 3: PI-Web Control Page

**Files:**
- Create: `src/components/piweb/PiWebPage.vue`
- Create: `tests/components/PiWebPage.test.ts`
- Modify: `src/App.vue`

- [ ] **Step 1: Add component tests**

Create `tests/components/PiWebPage.test.ts`:

```ts
import { mount } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import PiWebPage from '@/components/piweb/PiWebPage.vue'

const status = {
  state: 'stopped',
  url: 'http://127.0.0.1:30141',
  port: 30141,
  message: 'PI-Web 未启动',
  detail: '',
  missingDependency: '',
  installLinks: [],
  canStart: true,
  canOpen: false,
  canStop: false,
}

const api = vi.hoisted(() => ({
  getPiWebStatus: vi.fn(),
  startPiWeb: vi.fn(),
  stopPiWeb: vi.fn(),
  openPiWeb: vi.fn(),
}))

vi.mock('@/lib/piWebIpc', () => api)

describe('PiWebPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    api.getPiWebStatus.mockResolvedValue({ ...status })
    api.startPiWeb.mockResolvedValue({ ...status, state: 'running', canOpen: true, canStop: true })
    api.stopPiWeb.mockResolvedValue({ ...status })
    api.openPiWeb.mockResolvedValue({ ...status, state: 'running', canOpen: true, canStop: true })
  })

  it('shows status and starts PI-Web from the main action', async () => {
    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('PI-Web')
    expect(wrapper.text()).toContain('http://127.0.0.1:30141')

    await wrapper.get('[data-action="start-pi-web"]').trigger('click')
    expect(api.startPiWeb).toHaveBeenCalledOnce()
    expect(wrapper.text()).toContain('running')
  })

  it('shows missing dependency links', async () => {
    api.getPiWebStatus.mockResolvedValue({
      ...status,
      state: 'missingRuntime',
      message: '缺少 Node.js',
      missingDependency: 'Node.js',
      canStart: false,
      installLinks: [{ label: '下载 Node.js', url: 'https://nodejs.org/' }],
    })

    const wrapper = mount(PiWebPage)
    await vi.dynamicImportSettled()

    expect(wrapper.text()).toContain('缺少 Node.js')
    expect(wrapper.get('[data-install-link="Node.js"]').attributes('href')).toBe('https://nodejs.org/')
  })
})
```

- [ ] **Step 2: Run the component test and confirm failure**

Run: `pnpm test tests/components/PiWebPage.test.ts`

Expected: FAIL because `PiWebPage.vue` does not exist yet.

- [ ] **Step 3: Create `PiWebPage.vue`**

Implement the page with:

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { ExternalLink, Play, RefreshCw, Square } from '@lucide/vue'
import { getPiWebStatus, openPiWeb, startPiWeb, stopPiWeb, type PiWebStatus } from '@/lib/piWebIpc'

const status = ref<PiWebStatus | null>(null)
const busy = ref(false)

const stateLabel = computed(() => status.value?.state ?? 'checking')
const primaryAction = computed(() => {
  if (!status.value) return 'refresh'
  if (status.value.canOpen) return 'open'
  if (status.value.canStart) return 'start'
  return 'refresh'
})

async function refresh() {
  busy.value = true
  try {
    status.value = await getPiWebStatus()
  } finally {
    busy.value = false
  }
}

async function runPrimaryAction() {
  if (busy.value) return
  busy.value = true
  try {
    if (primaryAction.value === 'open') status.value = await openPiWeb()
    else if (primaryAction.value === 'start') status.value = await startPiWeb()
    else status.value = await getPiWebStatus()
  } finally {
    busy.value = false
  }
}

async function stop() {
  if (busy.value || !status.value?.canStop) return
  busy.value = true
  try {
    status.value = await stopPiWeb()
  } finally {
    busy.value = false
  }
}

onMounted(refresh)
</script>
```

The template must include `data-action="start-pi-web"` on the primary button, `data-action="stop-pi-web"` on the stop button, and `data-install-link="Node.js"` for Node.js install links.

- [ ] **Step 4: Route the page in `src/App.vue`**

Add:

```ts
import PiWebPage from '@/components/piweb/PiWebPage.vue'
```

and render:

```vue
<PiWebPage v-else-if="ui.activeTool === 'pi-web'" />
```

- [ ] **Step 5: Run frontend tests**

Run: `pnpm test tests/components/PiWebPage.test.ts tests/components/AppSidebar.test.ts`

Expected: PASS.

## Task 4: Backend PI-Web Service Types

**Files:**
- Create: `src-tauri/src/pi_web.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add backend unit tests for status helpers**

Create tests inside `src-tauri/src/pi_web.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_flags_match_state() {
        let stopped = PiWebStatus::stopped(30141);
        assert!(stopped.can_start);
        assert!(!stopped.can_open);
        assert!(!stopped.can_stop);

        let running = PiWebStatus::running(30141);
        assert!(!running.can_start);
        assert!(running.can_open);
        assert!(running.can_stop);
    }

    #[test]
    fn missing_node_status_has_install_link() {
        let status = PiWebStatus::missing_dependency("Node.js", "缺少 Node.js");
        assert_eq!(status.state, PiWebServiceState::MissingRuntime);
        assert_eq!(status.missing_dependency, "Node.js");
        assert!(status.install_links.iter().any(|link| link.url.contains("nodejs.org")));
    }
}
```

- [ ] **Step 2: Run Rust tests and confirm failure**

Run: `cargo test --manifest-path src-tauri\Cargo.toml pi_web`

Expected: FAIL because the module does not exist yet.

- [ ] **Step 3: Implement core types**

Create:

```rust
use crate::command_auth::MainArgs;
use serde::Serialize;
use std::{
    process::Child,
    sync::Mutex,
};

const PI_WEB_PORT: u16 = 30141;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PiWebServiceState {
    MissingRuntime,
    Stopped,
    Checking,
    Starting,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiWebDiagnosticLink {
    label: String,
    url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PiWebStatus {
    state: PiWebServiceState,
    url: String,
    port: u16,
    message: String,
    detail: String,
    missing_dependency: String,
    install_links: Vec<PiWebDiagnosticLink>,
    can_start: bool,
    can_open: bool,
    can_stop: bool,
}

pub struct PiWebService {
    child: Mutex<Option<Child>>,
}

impl Default for PiWebService {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }
}
```

Add helper constructors for `stopped`, `running`, `error`, and `missing_dependency`.

- [ ] **Step 4: Register module and state**

In `src-tauri/src/lib.rs`, add:

```rust
mod pi_web;
```

and in `run()`:

```rust
.manage(pi_web::PiWebService::default())
```

- [ ] **Step 5: Run Rust helper tests**

Run: `cargo test --manifest-path src-tauri\Cargo.toml pi_web`

Expected: PASS.

## Task 5: Backend Commands And Lifecycle

**Files:**
- Modify: `src-tauri/src/pi_web.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add command argument types**

Add:

```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PiWebEmptyCommandArgs {}
```

- [ ] **Step 2: Implement status command**

Add:

```rust
#[tauri::command]
pub fn get_pi_web_status(
    service: tauri::State<PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebStatus, String> {
    service.status()
}
```

`status()` should return running when the stored child has not exited, otherwise stopped or missing dependency.

- [ ] **Step 3: Implement start command**

Add:

```rust
#[tauri::command]
pub async fn start_pi_web(
    app: tauri::AppHandle,
    service: tauri::State<'_, PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebStatus, String> {
    service.start(app).await
}
```

Startup behavior:

- Prefer a bundled launch script if resource files are present.
- Fall back to `npx @agegr/pi-web@latest` during development.
- Use `Command::new("cmd").args(["/C", "npx", "@agegr/pi-web@latest"])` on Windows fallback.
- Store the `Child` in `PiWebService.child`.
- Poll `http://127.0.0.1:30141` for up to 30 seconds.

- [ ] **Step 4: Implement open and stop commands**

Add:

```rust
#[tauri::command]
pub fn open_pi_web(
    app: tauri::AppHandle,
    service: tauri::State<PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebStatus, String> {
    let status = service.status()?;
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_url(status.url.clone(), None::<String>)
        .map_err(|error| error.to_string())?;
    Ok(status)
}

#[tauri::command]
pub fn stop_pi_web(
    service: tauri::State<PiWebService>,
    _args: MainArgs<PiWebEmptyCommandArgs>,
) -> Result<PiWebStatus, String> {
    service.stop()
}
```

- [ ] **Step 5: Register commands**

In `src-tauri/src/lib.rs`, add to `generate_handler!`:

```rust
pi_web::get_pi_web_status,
pi_web::start_pi_web,
pi_web::open_pi_web,
pi_web::stop_pi_web,
```

- [ ] **Step 6: Run Rust tests**

Run: `cargo test --manifest-path src-tauri\Cargo.toml pi_web`

Expected: PASS.

## Task 6: Bundled Resource Hook

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `src-tauri/resources/pi-web/.gitkeep`

- [ ] **Step 1: Add a resource placeholder**

Create `src-tauri/resources/pi-web/.gitkeep`.

- [ ] **Step 2: Add resource config**

In `src-tauri/tauri.conf.json`, under `bundle`, add:

```json
"resources": [
  "resources/pi-web"
]
```

- [ ] **Step 3: Make backend detect the resource directory**

In `PiWebService::start`, first check:

```rust
let resource_dir = app
    .path()
    .resource_dir()
    .map_err(|_| "PI_WEB_RESOURCE_DIR_UNAVAILABLE".to_string())?
    .join("pi-web");
```

If the directory contains a launch script, use it. If it only contains `.gitkeep`, treat it as not bundled and use development fallback.

- [ ] **Step 4: Run Tauri config-sensitive build check**

Run: `pnpm typecheck`

Expected: PASS. Full `pnpm tauri build` remains release verification, not required for this task.

## Task 7: Full Verification

**Files:**
- All changed files.

- [ ] **Step 1: Run frontend checks**

Run: `pnpm test tests/components/PiWebPage.test.ts tests/components/AppSidebar.test.ts`

Expected: PASS.

- [ ] **Step 2: Run Rust PI-Web tests**

Run: `cargo test --manifest-path src-tauri\Cargo.toml pi_web`

Expected: PASS.

- [ ] **Step 3: Run full project check**

Run: `pnpm check`

Expected: PASS.

- [ ] **Step 4: Run browser/manual check**

Run: `pnpm dev`, open the Vite app, select `PI-Web`, and verify:

- The page fits the Banana Box theme.
- The status and address are visible.
- Long diagnostics scroll instead of overflowing.
- Buttons do not overlap at narrow widths.

- [ ] **Step 5: Document remaining release packaging work**

If the first implementation still uses the development fallback, add a note to the final response that release packaging still needs the real bundled PI-Web runtime files before publishing.
