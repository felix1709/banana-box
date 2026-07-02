# Reverse Image Prompting and Compression Design

## Goal

Add three top-level product areas to Banana Box: prompt library, reverse image prompting, and fast compression. Reverse image prompting uses an OpenAI-compatible vision API. Fast compression supports images and videos, with FFmpeg allowed for video compression.

## Confirmed Decisions

- The left sidebar has exactly three top-level entries:
  - Prompt Library
  - Reverse Image
  - Fast Compression
- Prompt categories stay inside the Prompt Library view instead of being top-level sidebar entries.
- API calls use an OpenAI-compatible format.
- The default Base URL is `https://ai.leihuo.netease.com`.
- The user enters an API key.
- The settings page has a "check connection" action that validates Base URL plus API key can call the API.
- Model selection shows callable models when available.
- If callable model discovery fails, the app keeps the four default models.
- The default preferred model is `doubao-seed-1-6-vision-250815`.
- Default model options:
  - `doubao-seed-1-6-vision-250815`
  - `gpt-5.4-mini`
  - `qwen3.5-omni-plus`
  - `qwen3-vl-plus`
- Reverse image results do not save directly to the library. They open the existing prompt editor with the generated prompt prefilled into content.
- The user writes the title, category, and tags in the prompt editor.
- Dragging a file onto the floating icon always opens an action dialog first.
- For image files, the action dialog offers:
  - Reverse prompt
  - Compress image
- For video files, the action dialog offers:
  - Compress video
  - Video reverse prompt as a reserved future action
- Image and video compression both ask for target size in MB.
- Compression output uses a Save As dialog.
- Save As defaults to the source file folder.
- Save As defaults to the source filename plus `_MMddHHmm`, for example `XXX_06301205.mp4`.
- Video compression may use FFmpeg.

## User Workflow

### Prompt Library

The existing prompt library remains the main management area. It keeps search, prompt cards, prompt editing, category filtering, and import/export behavior. Categories move into the Prompt Library area because the app-level sidebar now represents major tools, not prompt categories.

### Reverse Image

The user opens Reverse Image from the left sidebar. The page supports importing an image by click, paste, or drag and drop. After an image is loaded, the user starts reverse prompting. The app sends the image to the selected OpenAI-compatible model. When a prompt is generated, the user can copy it, regenerate it, or save it to the prompt library. Saving opens the existing prompt editor with the generated text in the content field.

### Floating Icon

The floating icon accepts dropped files. The app identifies the first supported file type and opens an action dialog. Images can be reversed or compressed. Videos can be compressed, and the dialog keeps a reserved disabled or coming-soon video reverse option for later development.

### Fast Compression

The user opens Fast Compression from the left sidebar or drops a file onto the floating icon. The user enters a target size in MB. The app opens a Save As dialog whose initial folder is the source file folder and whose filename has a `_MMddHHmm` suffix. Image compression iterates quality or dimensions until it reaches a useful output near the target. Video compression uses FFmpeg to calculate a target bitrate and encode the output.

## Architecture

### Frontend State

Extend the library settings model with API configuration:

- `apiBaseUrl`
- `apiKey`
- `reverseModel`
- `availableReverseModels`

Add UI state for:

- active top-level tool
- reverse image working file/result/loading/error
- compression working file/target/output/loading/error
- floating-file action dialog
- prompt editor prefill data

### IPC Layer

The frontend continues to call system-level behavior through `src/lib/ipc.ts`. Rust commands own file reads, API calls that need local file bytes, Save As defaults, and compression.

New commands should cover:

- checking OpenAI-compatible connection
- listing models with fallback behavior in the frontend
- reverse prompting from an image path or bytes
- classifying dropped file paths
- compressing images to target MB
- compressing videos to target MB

### API Calling

Use OpenAI-compatible endpoints:

- `GET /v1/models` for model discovery when available.
- `POST /v1/chat/completions` for connection checking and image reverse prompting.

The connection check should be small and cheap. It should validate that the URL and key can make an authenticated call. If model listing is unavailable, keep default models.

For vision prompting, send a text instruction plus an image URL object using a data URL built from the selected image bytes.

### Error Handling

Show beginner-friendly errors:

- Missing API key: "Please enter an API Key first."
- Bad URL/key: "Connection failed. Check Base URL and API Key."
- Model unavailable: keep defaults and show a short warning.
- Reverse failure: keep the image loaded and allow retry.
- Compression failure: keep the input and target size so the user can retry.
- FFmpeg missing: show a clear message that video compression needs FFmpeg.

## UI Design Plan

The UI stays compact and tool-like. It should not become a landing page or marketing layout.

The left sidebar uses three stable entries with plain labels and clear active state. The Prompt Library view keeps dense prompt-card behavior. Reverse Image and Fast Compression use simple work panels: input area, current file preview/details, primary action, result area, and errors.

Settings adds one compact API section. It should include Base URL, API key, check connection, model dropdown, and a small status line. The API key input should be password-style. The default Base URL should be prefilled for new users.

Hover, disabled, loading, empty, and error states should be explicit:

- disabled reverse button when no image/API key/model exists
- loading label while calling the model
- empty upload zones for image/compression pages
- error text near the relevant action
- keyboard focus outlines on inputs and buttons

## Testing

Tests should cover settings defaults, model fallback behavior, prompt editor prefill, top-level view switching, and compression filename suffix generation. IPC-heavy behaviors can be tested at the unit level where possible and manually verified through the desktop app for drag/drop and Save As behavior.

## Scope Notes

This design includes the reserved video reverse prompt option in the floating action dialog but does not implement video reverse prompting. It also does not require provider-specific API adapters because the API format is OpenAI-compatible.
