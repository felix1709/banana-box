# Local Media Dependency Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one-click local dependency setup and in-app progress dialogs for quick compression FFmpeg and depth-video workflows.

**Architecture:** Backend commands prepare managed dependencies under the app data directory and emit progress events keyed by an operation id. Frontend panels use a shared progress dialog to show setup/conversion state and keep advanced manual controls available.

**Tech Stack:** Vue 3, Pinia, Tauri v2 commands/events, Rust, PowerShell scripts, Vitest, Rust unit tests.

## Global Constraints

- Do not edit the user's system PATH; use app-managed dependency paths first.
- Keep FFmpeg under `<app data>/ffmpeg/bin`.
- Keep depth-video engine under the existing `<app data>/depth-video-engine` directory.
- Do not remove the manual depth engine picker.
- Hide Windows child process console windows for depth-video setup/conversion.
- Use event name `media-tool-progress` with `operationId`.
- Do not stage historical dirty files.

---

### Task 1: Backend FFmpeg managed path and setup command

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/ipc.ts`
- Test: `src-tauri/src/commands.rs`
- Test: `tests/lib/depth-video-ipc.test.ts` or new `tests/lib/media-tools-ipc.test.ts`

**Interfaces:**
- Produces: `prepare_ffmpeg_tools(app, gate, input) -> Result<FfmpegSetupResult, String>`
- Produces: frontend `prepareFfmpegTools(input: MediaToolOperationInput): Promise<FfmpegSetupResult>`
- Consumes: app data directory from `data_dir(&app)`

- [ ] **Step 1: Write failing Rust tests**

Add tests asserting:

```rust
#[test]
fn ffmpeg_tool_resolution_prefers_managed_bin_dir_when_present() {
    let dir = tempfile::tempdir().unwrap();
    let managed = dir.path().join("ffmpeg").join("bin");
    std::fs::create_dir_all(&managed).unwrap();
    let tool = managed.join(ffmpeg_tool_filename("ffprobe"));
    std::fs::write(&tool, b"").unwrap();

    assert_eq!(
        resolve_ffmpeg_tool("ffprobe", Some(&managed), None),
        tool
    );
}
```

Expected first run: fail because `resolve_ffmpeg_tool` does not accept a managed dir.

- [ ] **Step 2: Implement managed path resolution**

Change FFmpeg resolution to accept a managed dir:

```rust
fn resolve_ffmpeg_tool(
    tool_name: &str,
    managed_bin_dir: Option<&Path>,
    common_bin_dir: Option<&Path>,
) -> PathBuf
```

Resolution order:

1. managed dir;
2. common dir;
3. command name.

- [ ] **Step 3: Add setup result and command types**

Add:

```rust
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaToolOperationInput {
    pub operation_id: Option<String>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegSetupResult {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub bin_dir: String,
    pub message: String,
}
```

- [ ] **Step 4: Implement `prepare_ffmpeg_tools`**

Implement Windows setup:

1. create `<app data>/ffmpeg`;
2. download the configured essentials zip into that directory;
3. extract safely to a staging directory;
4. find `bin/ffmpeg.exe` and `bin/ffprobe.exe`;
5. copy them into `<app data>/ffmpeg/bin`;
6. run `ffmpeg -version` and `ffprobe -version`;
7. return `FfmpegSetupResult`.

Non-Windows should return a stable unsupported error for now:

`FFMPEG_MANAGED_SETUP_UNSUPPORTED_PLATFORM`

- [ ] **Step 5: Register the Tauri command**

Add `commands::prepare_ffmpeg_tools` to the invoke handler in `src-tauri/src/lib.rs`.

- [ ] **Step 6: Add frontend IPC wrapper**

In `src/lib/ipc.ts`, add:

```ts
export interface MediaToolOperationInput {
  operationId?: string
}

export interface FfmpegSetupResult {
  ffmpegPath: string
  ffprobePath: string
  binDir: string
  message: string
}

export async function prepareFfmpegTools(
  input: MediaToolOperationInput = {},
): Promise<FfmpegSetupResult> {
  return await invoke<FfmpegSetupResult>('prepare_ffmpeg_tools', { input })
}
```

- [ ] **Step 7: Run targeted tests**

Run:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml ffmpeg
pnpm test tests/lib/media-tools-ipc.test.ts
```

Expected: tests pass.

### Task 2: Shared media progress dialog

**Files:**
- Create: `src/components/MediaToolProgressDialog.vue`
- Modify: `tests/components/FastCompressionPanel.test.ts`
- Modify: `tests/components/DepthVideoPanel.test.ts`

**Interfaces:**
- Produces component props:
  - `open: boolean`
  - `title: string`
  - `description: string`
  - `progress: number`
  - `message: string`
  - `logs: string[]`
  - `status: 'idle' | 'running' | 'success' | 'error'`
- Emits: `close`, `retry`

- [ ] **Step 1: Write failing component usage tests**

Add expectations that progress dialog text and progressbar are visible when setup/conversion is running.

- [ ] **Step 2: Create component**

Implement compact overlay:

- translucent backdrop;
- dialog card;
- header;
- progressbar;
- current message;
- scrollable log list;
- footer actions.

- [ ] **Step 3: Add accessibility**

Use:

```vue
role="dialog"
aria-modal="true"
role="progressbar"
aria-valuemin="0"
aria-valuemax="100"
```

- [ ] **Step 4: Run frontend targeted tests**

Run:

```powershell
pnpm test tests/components/FastCompressionPanel.test.ts tests/components/DepthVideoPanel.test.ts
```

Expected: tests pass.

### Task 3: Quick compression one-click FFmpeg setup

**Files:**
- Modify: `src/components/FastCompressionPanel.vue`
- Modify: `tests/components/FastCompressionPanel.test.ts`

**Interfaces:**
- Consumes: `prepareFfmpegTools`
- Consumes: `MediaToolProgressDialog`

- [ ] **Step 1: Write failing test for setup button**

Test:

1. mock compression failure with missing `ffprobe`;
2. expect `一键配置 FFmpeg` button;
3. click button;
4. expect `prepareFfmpegTools` called;
5. expect success message.

- [ ] **Step 2: Replace link-only guidance**

Keep manual download link, but make app-managed setup primary.

- [ ] **Step 3: Add setup state**

Track:

```ts
const setupDialogOpen = ref(false)
const setupStatus = ref<'idle' | 'running' | 'success' | 'error'>('idle')
const setupProgress = ref(0)
const setupLogs = ref<string[]>([])
```

- [ ] **Step 4: Implement `onPrepareFfmpeg`**

Call:

```ts
await prepareFfmpegTools({ operationId })
```

On success, show `FFmpeg 已配置完成，可以重新开始压缩。`

- [ ] **Step 5: Run tests**

Run:

```powershell
pnpm test tests/components/FastCompressionPanel.test.ts
```

Expected: tests pass.

### Task 4: Depth-video unified setup and hidden process

**Files:**
- Modify: `src/components/DepthVideoPanel.vue`
- Modify: `src-tauri/src/commands.rs`
- Modify: `tests/components/DepthVideoPanel.test.ts`
- Test: `src-tauri/src/commands.rs`

**Interfaces:**
- Consumes existing `prepareDepthVideoPython`
- Consumes existing `prepareDepthVideoEngine`
- Consumes `MediaToolProgressDialog`

- [ ] **Step 1: Write failing frontend test**

Test that `一键配置深度视频环境` calls Python setup first, then engine setup, stores engine path, and shows the shared progress dialog.

- [ ] **Step 2: Add Windows hidden child-process helper**

Add helper:

```rust
fn configure_hidden_child_process(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}
```

Apply it to:

- `prepare_depth_video_python`
- `prepare_depth_video_engine`
- `convert_video_with_depth_engine`

- [ ] **Step 3: Add one-click depth setup function**

In frontend, create `prepareDepthEnvironment()`:

1. open dialog;
2. set progress 8, message checking Python;
3. call `prepareDepthVideoPython`;
4. set progress 42, message configuring engine;
5. call `prepareDepthVideoEngine`;
6. save engine path;
7. set progress 100, success.

- [ ] **Step 4: Keep advanced controls**

Keep `选择引擎`; keep separate Python/engine buttons only if visually secondary or collapse them under advanced controls.

- [ ] **Step 5: Run tests**

Run:

```powershell
pnpm test tests/components/DepthVideoPanel.test.ts
cargo test --manifest-path src-tauri\Cargo.toml depth_video
```

Expected: tests pass.

### Task 5: Progress events for setup/conversion

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/components/FastCompressionPanel.vue`
- Modify: `src/components/DepthVideoPanel.vue`
- Modify: tests for both panels

**Interfaces:**
- Event name: `media-tool-progress`
- Payload:

```ts
interface MediaToolProgressEvent {
  operationId: string
  tool: 'ffmpeg' | 'depth-video'
  phase: string
  progress: number
  message: string
  detail?: string
  level: 'info' | 'success' | 'error'
}
```

- [ ] **Step 1: Add payload structs**

Add Rust serializable payload struct and helper `emit_media_tool_progress`.

- [ ] **Step 2: Emit coarse setup stages**

Emit before/after downloads, extraction, verification, Python setup, engine setup, and conversion start/end.

- [ ] **Step 3: Frontend listen/unlisten**

Use `listen('media-tool-progress', handler)` while dialog is open. Ignore events with non-matching operationId.

- [ ] **Step 4: Run event tests**

Run frontend tests with mocked `listen`.

Expected: tests confirm only matching operation id updates dialog.

### Task 6: Final verification

**Files:**
- No new production files unless tests reveal issues.

- [ ] **Step 1: Run full frontend check**

```powershell
pnpm check
```

Expected: all tests pass.

- [ ] **Step 2: Run full backend tests**

```powershell
cargo test --manifest-path src-tauri\Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 3: Inspect git diff**

```powershell
git status --short
git diff --stat
```

Expected: only this feature's files plus docs are part of the intended diff; historical dirty files remain unstaged.
