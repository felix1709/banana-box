# Reverse Image Prompting and Compression Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add OpenAI-compatible reverse image prompting, a three-entry sidebar, floating-icon action selection, and target-MB image/video compression.

**Architecture:** Extend existing Vue/Pinia state and Tauri IPC instead of introducing a new app layer. Keep prompt saving routed through the existing prompt editor, and put file/API/compression work behind Rust commands exposed through `src/lib/ipc.ts`.

**Tech Stack:** Vue 3, TypeScript, Pinia, Tauri 2, Rust, ureq, FFmpeg command-line integration for video compression.

---

## File Structure

- Modify `src/types/index.ts`: add API settings fields.
- Modify `src/stores/library.ts`: add default API settings and migration-safe settings normalization.
- Modify `src/stores/ui.ts`: add active tool, prompt editor prefill, and floating action dialog state.
- Modify `src/App.vue`: render the three top-level tools and route the main content.
- Modify `src/components/SettingsModal.vue`: add API settings UI.
- Modify `src/components/PromptEditor.vue`: accept prefilled reverse prompt content.
- Modify `src/components/FloatButton.vue`: accept dropped files and request an action instead of direct execution.
- Create `src/components/AppSidebar.vue`: top-level sidebar with Prompt Library, Reverse Image, Fast Compression.
- Create `src/components/ReverseImagePanel.vue`: image import, reverse prompt, and save-to-editor flow.
- Create `src/components/FastCompressionPanel.vue`: file import, target MB, and compression flow.
- Create `src/components/FloatingActionDialog.vue`: choose action after dropping onto the floating icon.
- Modify `src/lib/ipc.ts`: add wrappers for API and compression commands.
- Modify `src-tauri/src/library.rs`: add settings fields and defaults.
- Modify `src-tauri/src/commands.rs`: add OpenAI-compatible API calls and compression commands.
- Modify `src-tauri/src/lib.rs`: register new commands.
- Modify `src-tauri/Cargo.toml`: add MIME/base64 or image-processing dependencies if required.
- Add tests under `tests/components` and `tests/stores`.

## Task 1: Settings Model and Defaults

- [ ] **Step 1: Update TypeScript settings type**

Modify `src/types/index.ts` so `Settings` includes:

```ts
export interface Settings {
  hotkey: string
  theme: 'auto' | 'light' | 'dark'
  apiBaseUrl: string
  apiKey: string
  reverseModel: string
  availableReverseModels: string[]
}
```

- [ ] **Step 2: Add default model constants**

Add constants to `src/stores/library.ts`:

```ts
export const DEFAULT_API_BASE_URL = 'https://ai.leihuo.netease.com'
export const DEFAULT_REVERSE_MODEL = 'doubao-seed-1-6-vision-250815'
export const DEFAULT_REVERSE_MODELS = [
  'doubao-seed-1-6-vision-250815',
  'gpt-5.4-mini',
  'qwen3.5-omni-plus',
  'qwen3-vl-plus',
]
```

- [ ] **Step 3: Normalize settings on load**

Add a helper in `src/stores/library.ts`:

```ts
function normalizeSettings(settings: Partial<Library['settings']>): Library['settings'] {
  return {
    hotkey: settings.hotkey ?? 'Ctrl+Shift+B',
    theme: settings.theme ?? 'auto',
    apiBaseUrl: settings.apiBaseUrl ?? DEFAULT_API_BASE_URL,
    apiKey: settings.apiKey ?? '',
    reverseModel: settings.reverseModel ?? DEFAULT_REVERSE_MODEL,
    availableReverseModels:
      settings.availableReverseModels?.length ? settings.availableReverseModels : DEFAULT_REVERSE_MODELS,
  }
}
```

- [ ] **Step 4: Run focused store tests**

Run:

```bash
pnpm test tests/stores/library.test.ts
```

Expected: existing tests pass or expose needed fixture updates.

## Task 2: Three-Entry Sidebar

- [ ] **Step 1: Extend UI store**

Modify `src/stores/ui.ts`:

```ts
type ActiveTool = 'prompts' | 'reverse-image' | 'compression'

state: () => ({
  activeTool: 'prompts' as ActiveTool,
  ...
}),
actions: {
  setActiveTool(tool: ActiveTool) {
    this.activeTool = tool
  },
  ...
}
```

- [ ] **Step 2: Create AppSidebar component**

Create `src/components/AppSidebar.vue` with three buttons bound to `ui.activeTool`.

- [ ] **Step 3: Update App layout**

Modify `src/App.vue` to render `AppSidebar` in the sidebar and show `CategoryTree` only inside the Prompt Library view.

- [ ] **Step 4: Verify UI switching**

Run:

```bash
pnpm typecheck
```

Expected: no TypeScript errors.

## Task 3: API Settings UI

- [ ] **Step 1: Add IPC wrappers**

Modify `src/lib/ipc.ts`:

```ts
export interface CheckApiConnectionInput {
  baseUrl: string
  apiKey: string
}

export interface CheckApiConnectionResult {
  ok: boolean
  message: string
  models: string[]
}

export async function checkApiConnection(input: CheckApiConnectionInput): Promise<CheckApiConnectionResult> {
  return await invoke<CheckApiConnectionResult>('check_api_connection', { input })
}
```

- [ ] **Step 2: Add settings form controls**

Modify `src/components/SettingsModal.vue` to include Base URL, API key, check connection button, model dropdown, and status text.

- [ ] **Step 3: Save settings**

Ensure settings are persisted through `lib.persist()` after the API section is saved or after successful connection check.

- [ ] **Step 4: Test settings modal**

Run:

```bash
pnpm test tests/components/SettingsModal.test.ts
```

Expected: tests pass after updating expectations for the new API fields.

## Task 4: Rust API Commands

- [ ] **Step 1: Add Rust request/response structs**

Modify `src-tauri/src/commands.rs` with serde structs for check connection and reverse prompt.

- [ ] **Step 2: Implement OpenAI-compatible URL joining**

Add a helper that trims trailing slash and appends `/v1/models` or `/v1/chat/completions` only when needed.

- [ ] **Step 3: Implement `check_api_connection`**

Try `GET /v1/models` first. If it succeeds, return model ids. If it fails but authenticated chat completion succeeds, return `ok: true` with empty models so the frontend can fall back to defaults.

- [ ] **Step 4: Register the command**

Modify `src-tauri/src/lib.rs` and add `commands::check_api_connection` to `tauri::generate_handler!`.

- [ ] **Step 5: Check Rust build**

Run:

```bash
pnpm tauri build --debug
```

Expected: Rust commands compile.

## Task 5: Reverse Image Panel and Prompt Editor Prefill

- [ ] **Step 1: Extend UI store with editor prefill**

Add:

```ts
editorPrefill: null as null | {
  title?: string
  content?: string
  categoryId?: string | null
  tags?: string[]
  image?: string | null
}
```

Update `openEditor(id, prefill?)`.

- [ ] **Step 2: Update PromptEditor**

When creating a new prompt, initialize fields from `ui.editorPrefill`.

- [ ] **Step 3: Create ReverseImagePanel**

Support click import, paste, drag/drop, preview, reverse button, result textarea, copy, regenerate, and save-to-editor.

- [ ] **Step 4: Add reverse prompt IPC wrapper and Rust command**

Implement `reverse_image_prompt` that sends the image as a data URL to `/v1/chat/completions`.

- [ ] **Step 5: Verify save flow**

Manual check: reverse result opens the prompt editor with content prefilled and title/category/tags empty.

## Task 6: Floating Action Dialog

- [ ] **Step 1: Add dialog component**

Create `src/components/FloatingActionDialog.vue` with image actions and video actions.

- [ ] **Step 2: Wire dropped files**

Make dropped files on the floating icon emit to the main window or open the main panel with dialog state.

- [ ] **Step 3: Route actions**

Reverse action opens Reverse Image flow. Compression action opens Fast Compression flow.

- [ ] **Step 4: Preserve video reverse placeholder**

Show video reverse as disabled or coming soon.

## Task 7: Fast Compression

- [ ] **Step 1: Create FastCompressionPanel**

Support file import, target MB input, current file details, compression button, and status.

- [ ] **Step 2: Add Save As helper command**

Use the source file path to build a default filename with `_MMddHHmm`.

- [ ] **Step 3: Implement image compression**

Use a Rust image-processing crate to encode output near the target MB.

- [ ] **Step 4: Implement video compression**

Use FFmpeg. Compute target bitrate from target bytes and duration. If FFmpeg is missing, return a clear error.

- [ ] **Step 5: Manual verify**

Test with one image and one video. Confirm output location defaults to source folder and suffix format is correct.

## Task 8: Full Verification

- [ ] **Step 1: Run frontend checks**

```bash
pnpm typecheck
pnpm lint
pnpm test
```

- [ ] **Step 2: Run desktop build check**

```bash
pnpm tauri build --debug
```

- [ ] **Step 3: Browser/manual UI verification**

Start the app and verify:

- sidebar has three top-level entries
- settings API section is compact and usable
- reverse image can open editor with prefilled content
- floating drop shows action dialog
- compression asks for target MB and Save As path

## Self-Review

- Spec coverage: all confirmed requirements map to tasks.
- Placeholder scan: no unresolved feature placeholders remain, except video reverse prompt, which is explicitly out of current implementation scope but reserved in UI.
- Type consistency: settings field names match TypeScript, store, and Rust camelCase serialization.
