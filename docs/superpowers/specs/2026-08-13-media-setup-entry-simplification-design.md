# Media Setup Entry Simplification Design

## Goal

Make local media dependency setup visible before users hit an error. Fast compression should show FFmpeg setup on the main page, and depth video should expose one clear setup action instead of several technical choices.

## User Workflow

### Fast Compression

1. User opens the Fast Compression tool.
2. User sees an FFmpeg setup card before choosing a file or compressing.
3. User clicks one button to prepare FFmpeg.
4. Banana Box downloads and verifies `ffmpeg.exe` and `ffprobe.exe` in the app-managed directory.
5. If a faster mirror fails, Banana Box automatically falls back to the official source.

### Depth Video

1. User opens the Depth Video tool.
2. User sees one setup card with one primary action: one-click setup.
3. Banana Box checks Python, the local engine, model files, and dependencies.
4. Banana Box downloads only missing parts and skips existing parts.
5. User then imports a video and converts it.

## UI Design

Fast Compression gets a compact dependency card at the top of the page. It uses the existing dark production-tool style and a primary-outline button so it reads as setup, not the main compression action. Missing-tool errors may still show the same action, but the user no longer has to fail first.

Depth Video keeps one environment card, one status line, and one primary setup button. Manual buttons for Python install, engine selection, and engine download are hidden from the normal UI to reduce confusion.

## Download Source Strategy

FFmpeg setup should try a domestic accelerated mirror first, then fall back to the existing official source. Every downloaded archive must still be extracted and verified by running both binaries. If no source works, the user sees a clear failure in the progress dialog.

## Testing

- Fast Compression component test: setup card and button are visible on first render.
- Fast Compression component test: clicking the visible setup button calls `prepareFfmpegTools`.
- Depth Video component test: only the one-click setup button is visible; old technical setup buttons are not visible.
- Existing IPC and backend tests continue to run.
