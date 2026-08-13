# Local Media Dependency Setup Design

## Goal

Banana Box should let non-technical users prepare local video tooling with one click. Users should not need to understand PATH, download pages, terminal windows, Python versions, or model folders before they can use quick compression or depth-video conversion.

## Product decision

Use app-managed dependencies instead of editing the user's system PATH.

Banana Box will download and store FFmpeg under the app data directory, then backend commands will prefer that managed path before falling back to common install locations and PATH. This gives users the practical result they want while avoiding global system changes, permission prompts, and restart confusion.

## Scope

This spec covers:

- Quick compression FFmpeg setup.
- Depth-video environment setup.
- A shared in-app secondary progress dialog for setup and conversion.
- Hiding external CLI windows during depth-video conversion.
- Tests for backend path resolution, setup orchestration, frontend button behavior, and progress UI.

This spec does not cover:

- GPU-specific dependency tuning.
- Supporting macOS/Linux FFmpeg auto-install in this iteration.
- Removing the advanced manual engine picker for depth video.
- Publishing a release; release remains a separate `提交发布` step.

## User workflows

### Quick compression: FFmpeg missing

1. User selects a video and clicks compress.
2. If `ffprobe` or `ffmpeg` is unavailable, the panel shows a dependency card.
3. User clicks `一键配置 FFmpeg`.
4. A secondary setup dialog opens.
5. The dialog shows stages:
   - checking local FFmpeg
   - downloading FFmpeg essentials zip
   - extracting tools
   - verifying `ffmpeg.exe` and `ffprobe.exe`
   - setup complete
6. User clicks `完成`.
7. The compression panel can now use the managed FFmpeg without requiring PATH.

### Depth video: first-time setup

1. User opens depth-video module.
2. The panel shows a primary `一键配置深度视频环境` button.
3. A secondary setup dialog opens.
4. The dialog shows stages:
   - checking Python 3.10
   - installing Python 3.10 if needed
   - preparing scripts
   - downloading Video Depth Anything
   - installing Python dependencies
   - downloading the small model checkpoint
   - verifying local launcher
   - setup complete
5. The configured launcher path is saved to localStorage as today.
6. The user can convert without manually choosing an engine.

### Depth video: conversion progress

1. User starts conversion.
2. Banana Box asks for the output path.
3. A secondary progress dialog opens.
4. The backend starts the depth engine without showing an external console window.
5. The dialog shows a stage progress bar and log lines.
6. On success, it shows the output path.
7. On failure, it shows a short Chinese explanation and expandable details.

## UI design

Create a reusable setup/progress dialog component with:

- compact production-tool styling matching the existing dark Banana Box panels;
- title, short description, progress bar, current step, scrollable log area, and action buttons;
- success, running, and error states;
- accessible progressbar attributes;
- keyboard focus on the primary action;
- internal scrolling for logs so controls never disappear.

The quick compression panel should replace the external FFmpeg link-only guidance with:

- explanation: `视频压缩需要 FFmpeg 和 ffprobe。Banana Box 可以自动下载并配置到应用目录，不需要你手动设置 PATH。`
- primary button: `一键配置 FFmpeg`
- secondary link: `打开官方下载页` for advanced/manual users.

The depth-video panel should place `一键配置深度视频环境` as the recommended primary setup action and keep `选择本地引擎` as an advanced fallback.

## Backend design

### Managed FFmpeg

Add a managed FFmpeg directory:

`<app data>/ffmpeg/bin`

Backend path resolution order:

1. managed app data FFmpeg bin;
2. common Windows install path: `%ProgramFiles%\ffmpeg\bin`;
3. PATH command name fallback: `ffmpeg` or `ffprobe`.

Add a Tauri command:

`prepare_ffmpeg_tools() -> FfmpegSetupResult`

Return fields:

- `ffmpegPath`
- `ffprobePath`
- `binDir`
- `message`

On Windows, the command downloads `ffmpeg-release-essentials.zip` from the configured FFmpeg build URL, extracts only safe entries, locates the inner `bin` directory, copies `ffmpeg.exe` and `ffprobe.exe` into the managed bin directory, and verifies both tools can start.

### Progress events

Add event payloads for long-running operations:

- event name: `media-tool-progress`
- payload:
  - `operationId`
  - `tool`: `ffmpeg` or `depth-video`
  - `phase`
  - `progress`
  - `message`
  - `detail`
  - `level`: `info`, `success`, or `error`

Frontend generates an `operationId` and passes it into setup/conversion commands. Commands emit progress to the main window.

### Depth-video process behavior

Depth-video setup and conversion commands should run PowerShell/CMD child processes hidden on Windows using `CREATE_NO_WINDOW`. They should capture stdout/stderr and emit log lines into `media-tool-progress`.

Conversion can use stage progress even if the underlying engine does not emit a reliable percentage. If recognizable progress appears in stdout/stderr later, the parser can update the progress more accurately.

## Error handling

User-facing errors must be short and practical:

- FFmpeg download failed: ask user to check network and retry.
- FFmpeg verification failed: say the downloaded tools could not start and include details in logs.
- Python install failed: reuse existing friendly depth-video message.
- Depth engine setup failed: keep existing detailed output in expandable logs, but show a short summary first.
- Conversion failed: keep selected video and output path state so the user can retry.

## Testing requirements

Backend tests:

- managed FFmpeg path is preferred when present;
- fallback still uses common install/PATH behavior;
- FFmpeg setup script/extraction rejects unsafe zip paths;
- depth-video commands are configured to hide console windows on Windows;
- progress payload serialization is stable.

Frontend tests:

- quick compression shows one-click FFmpeg setup when tools are missing;
- clicking the setup button opens the progress dialog and calls `prepareFfmpegTools`;
- successful FFmpeg setup closes or completes the dialog and lets the user retry compression;
- depth-video one-click setup runs Python and engine setup as a single flow;
- depth-video conversion shows the shared progress dialog instead of relying only on inline progress.

## Acceptance criteria

- A normal user can prepare FFmpeg from the quick compression panel without touching PATH.
- A normal user can prepare the depth-video environment from one primary button.
- Depth-video conversion does not show a separate CLI window in release builds.
- Setup and conversion progress is visible inside Banana Box.
- Existing compression and depth-video tests continue to pass.
- `pnpm check` and `cargo test --manifest-path src-tauri\Cargo.toml` pass before completion.
