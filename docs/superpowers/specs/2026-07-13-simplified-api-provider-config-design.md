# Simplified API Provider Config Design

## Goal

Simplify API setup so a beginner only needs to enter an API root URL and API Key, then use connection detection to populate compatible models.

## User Workflow

The API settings page keeps the existing provider selector. For each provider, the user enters one `API URL`, such as `https://api.example.com/v1`, and an API Key. If the user enters only a bare host, such as `https://api.example.com`, the app treats it as `https://api.example.com/v1`. The app derives the OpenAI-compatible endpoints internally:

- models: `<API URL>/models`
- chat completions: `<API URL>/chat/completions`

After saving, the user clicks connection detection. The app calls the saved models endpoint, fills the model dropdown with detected models, and lets the user save the selected model. If an older local save has endpoints missing `/v1`, the UI asks the user to save first instead of testing the stale endpoint.

## UI Direction

Keep the compact production-tool style already used by the settings modal. Remove visible `Models URL` and `Chat Completions URL` fields to reduce cognitive load. Rename `Base URL` to `API URL` and use an OpenAI-compatible `/v1` example in the placeholder.

The panel must remain scrollable inside the modal. Existing provider-specific controls, such as storyboard temperature and context length, stay unchanged.

## Architecture

Use a front-end-only simplification for this iteration. `SettingsModal.vue` will derive endpoint URLs before calling the existing `save_ai_provider` command. The Rust backend continues to enforce URL validation, HTTPS rules, cross-origin checks, host binding, and write-only key storage.

This keeps the security model intact while improving the visible setup experience.

## Error Handling

If the user tries to detect before saving a changed URL or newly entered key, the UI asks them to save first. This avoids detecting an old stored provider configuration.

If the URL is invalid or credentials fail, the existing backend error codes and UI status messages remain the source of truth.

## Testing

Update the settings modal tests to verify:

- only the simplified API URL field is visible,
- saving derives `/models` and `/chat/completions`,
- detection prompts the user to save first when URL or key has unsaved changes,
- existing key-clearing behavior remains intact.
