# PI-Web Repair Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move PI-Web configuration diagnosis and repair out of the main PI-Web page into a focused, scrollable Tauri secondary window.

**Architecture:** The main PI-Web page remains an operational dashboard and opens/re-focuses a uniquely labelled `pi-web-repair` WebviewWindow. The repair window uses the existing diagnostic and repair IPC commands, keeps API keys only in a password input until the invocation completes, and reports repair completion to the main window through a safe status refresh rather than sharing secret data.

**Tech Stack:** Tauri 2 WebviewWindow, Rust commands, Vue 3, Vue Test Utils/Vitest, lucide-vue.

## Global Constraints

- Window label is exactly `pi-web-repair`; repeated opens must focus the existing window rather than create duplicates.
- Secondary window size is 640×620 logical pixels, resizable, with a visible title and content-area vertical scrollbar.
- Main PI-Web page must not render a configuration path, API key field, or repair result text after migration.
- Existing `get_pi_web_config_status`, `repair_pi_web_config`, and `repair_pi_web_model_compatibility` remain the only repair operations; API key is never logged, persisted by Vue, sent through events, or redisplayed after repair.
- Auto-repair failure shows concise manual instructions only after the failed result; never expose raw API keys in error details.
- Preserve existing compact, professional product styling; include keyboard focus and disabled/loading states.

---

## File Structure

- Create: `src/components/piweb/PiWebRepairWindow.vue` — isolated repair UI and scroll boundary.
- Modify: `src/App.vue` — render repair window root according to Tauri window label.
- Modify: `src-tauri/src/pi_web.rs` and `src-tauri/src/lib.rs` — open/focus `pi-web-repair` command.
- Modify: `src/lib/piWebIpc.ts` — expose `openPiWebRepairWindow()`.
- Modify: `src/components/piweb/PiWebPage.vue` — remove embedded repair state and add window-launch action.
- Modify: `src-tauri/tauri.conf.json` — configure the repair webview’s initial options if static registration is selected; otherwise document builder options in Rust.
- Create: `tests/components/PiWebRepairWindow.test.ts`; modify `tests/components/PiWebPage.test.ts`.

### Task 1: Add the single-instance secondary-window command

**Files:**
- Modify: `src-tauri/src/pi_web.rs:376-496`
- Modify: `src-tauri/src/lib.rs:150-190`
- Test: `src-tauri/src/pi_web.rs` inline tests

**Interfaces:**

```rust
#[tauri::command]
pub fn open_pi_web_repair_window(app: tauri::AppHandle) -> Result<(), String>;
// If app.get_webview_window("pi-web-repair") exists: show + set_focus, else:
// WebviewWindowBuilder::new(&app, "pi-web-repair", WebviewUrl::App("index.html".into()))
// .title("PI-Web 配置修复").inner_size(640.0, 620.0).resizable(true).build()
```

- [ ] **Step 1: Write failing command-behaviour tests**

```rust
#[test] fn repair_window_uses_the_fixed_label() { assert_eq!(PI_WEB_REPAIR_WINDOW_LABEL, "pi-web-repair"); }
#[test] fn duplicate_open_focuses_existing_window() { assert_eq!(open_or_focus(existing_window()), OpenResult::Focused); }
```

- [ ] **Step 2: Run the Rust test filter**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pi_web::tests::repair_window -- --nocapture`

Expected: FAIL because no window label/command exists.

- [ ] **Step 3: Implement open-or-focus without changing main-window state**

Use `tauri::{Manager, WebviewUrl, WebviewWindowBuilder}`. On an existing window call `show()` and `set_focus()`; on a new window set `skip_taskbar: false`, title, size, and `min_inner_size(520.0, 440.0)`. Register the command in `invoke_handler`.

- [ ] **Step 4: Verify Rust tests and manual duplicate opening**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pi_web::tests -- --nocapture`

Expected: PASS. During `pnpm tauri dev`, invoke the command twice and confirm exactly one titled repair window is focused.

- [ ] **Step 5: Commit the native window command**

```bash
git add src-tauri/src/pi_web.rs src-tauri/src/lib.rs
git commit -m "feat: open PI-Web repair in secondary window"
```

### Task 2: Route the secondary Tauri window to its own Vue root

**Files:**
- Create: `src/components/piweb/PiWebRepairWindow.vue`
- Modify: `src/App.vue`
- Test: `tests/components/PiWebRepairWindow.test.ts`

**Interfaces:**
- `PiWebRepairWindow.vue` calls `getPiWebConfigStatus(): Promise<PiWebConfigStatus>`, `repairPiWebConfig(apiKey: string): Promise<PiWebConfigRepairResult>`, and `repairPiWebModelCompatibility(): Promise<PiWebRepairResult>`.
- `App.vue` detects `getCurrentWindow().label === 'pi-web-repair'` before mounting the normal `MainRoot`; no main navigation/store UI is shown in that label.

- [ ] **Step 1: Write failing repair-window render tests**

```ts
it('renders configuration status but no API key value', async () => {
  const wrapper = mount(PiWebRepairWindow)
  await vi.dynamicImportSettled()
  expect(wrapper.text()).toContain('settings.json')
  expect(wrapper.html()).not.toContain('sk-user-secret')
})
it('makes overflowing repair content scrollable', () => expect(source).toMatch(/\.pi-web-repair-body[\s\S]*overflow-y:\s*auto/))
```

- [ ] **Step 2: Run the new tests**

Run: `pnpm vitest run tests/components/PiWebRepairWindow.test.ts`

Expected: FAIL because the component/window root does not exist.

- [ ] **Step 3: Build the repair window UI**

Render a header, status grid for provider/model/settings/models/auth, a password input labelled `PI-Web API Key`, one repair button, model-compatibility repair when diagnostics identify it, result banner, and collapsed manual instructions only on failed repair. Put all content under `.pi-web-repair-body { overflow-y: auto; min-height: 0; }`; use `aria-live="polite"` for outcome messages and clear `apiKey.value = ''` in a `finally` block.

- [ ] **Step 4: Route the app and re-run tests**

Run: `pnpm vitest run tests/components/PiWebRepairWindow.test.ts && pnpm typecheck`

Expected: PASS. Launching a repair window shows only the repair UI, and its content remains reachable at a 440 px window height.

- [ ] **Step 5: Commit isolated repair UI**

```bash
git add src/App.vue src/components/piweb/PiWebRepairWindow.vue tests/components/PiWebRepairWindow.test.ts
git commit -m "feat: add dedicated PI-Web repair interface"
```

### Task 3: Simplify the main PI-Web page and connect it to the window

**Files:**
- Modify: `src/lib/piWebIpc.ts`
- Modify: `src/components/piweb/PiWebPage.vue`
- Modify: `tests/components/PiWebPage.test.ts`

**Interfaces:**

```ts
export async function openPiWebRepairWindow(): Promise<void> {
  await invoke('open_pi_web_repair_window', {})
}
```

- [ ] **Step 1: Write failing main-page boundaries tests**

```ts
it('opens the repair window from the main PI-Web page', async () => {
  await wrapper.get('[data-action="open-pi-web-repair"]').trigger('click')
  expect(api.openPiWebRepairWindow).toHaveBeenCalledOnce()
})
it('does not render embedded API key or repair action', () => {
  expect(wrapper.find('[data-field="pi-web-api-key"]').exists()).toBe(false)
  expect(wrapper.find('[data-action="repair-pi-web-config"]').exists()).toBe(false)
})
```

- [ ] **Step 2: Run the affected test file**

Run: `pnpm vitest run tests/components/PiWebPage.test.ts`

Expected: FAIL because the old embedded card still exists.

- [ ] **Step 3: Implement the main-page boundary**

Remove `configStatus`, `configApiKey`, `configRepairResult`, `configBusy`, `configRepairBusy`, `refreshConfigStatus`, and `repairConfig` from `PiWebPage.vue`. Replace the entire configuration card with one concise status sentence and a `data-action="open-pi-web-repair"` button; keep start/open/stop/download state in the main page. Do not call `getPiWebConfigStatus` when the page mounts.

- [ ] **Step 4: Verify all PI-Web components**

Run: `pnpm vitest run tests/components/PiWebPage.test.ts tests/components/PiWebRepairWindow.test.ts && pnpm check`

Expected: PASS; a source search finds `data-field="pi-web-api-key"` only in `PiWebRepairWindow.vue` and its tests.

- [ ] **Step 5: Commit the migration**

```bash
git add src/lib/piWebIpc.ts src/components/piweb/PiWebPage.vue tests/components/PiWebPage.test.ts
git commit -m "refactor: keep PI-Web repair out of main page"
```

### Task 4: End-to-end visual and secret-safety verification

**Files:**
- Modify: `docs/pi-web.md` — add a short user instruction for the repair button/window.

**Interfaces:**
- Consumes the complete command/UI surface from Tasks 1–3.

- [ ] **Step 1: Add a regression test that rejects secret echoing**

```ts
it('clears the input and never includes API key in results', async () => {
  await wrapper.get('[data-field="pi-web-api-key"]').setValue('sk-user-secret')
  await wrapper.get('[data-action="repair-pi-web-config"]').trigger('click')
  expect(wrapper.html()).not.toContain('sk-user-secret')
})
```

- [ ] **Step 2: Run the complete test set**

Run: `pnpm check && cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 3: Run desktop visual checks**

Run: `pnpm tauri dev`

Expected: Main PI-Web page has no crowded config form. Open repair at default and minimum sizes, use keyboard Tab to focus the input/button, perform a repair, and verify error/manual content scrolls rather than disappearing.

- [ ] **Step 4: Commit documentation**

```bash
git add docs/pi-web.md tests/components/PiWebRepairWindow.test.ts
git commit -m "docs: explain PI-Web repair window"
```

## Self-Review

- Spec coverage: Tasks 1–3 implement a named standalone window, main-page simplification, configuration diagnostics, one-click repair, model repair, focus behaviour, and scrollability; Task 4 validates visual states and API-key safety.
- Placeholder scan: all commands, component names, test selectors, dimensions, and styles are named explicitly.
- Type consistency: every frontend call uses existing `PiWebConfigStatus`, `PiWebConfigRepairResult`, and `PiWebRepairResult`, plus the one new `openPiWebRepairWindow` command.
