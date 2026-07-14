# Simplified API Provider Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users configure each AI provider with only an API root URL and API Key while preserving existing backend validation and key handling.

**Architecture:** Implement this as a front-end simplification in the settings modal. The UI stores one editable API URL, defaults bare hosts to `/v1`, derives the existing `modelsUrl` and `chatCompletionsUrl` before save, and keeps backend provider commands unchanged.

**Tech Stack:** Vue 3, Pinia, Vitest, Tauri IPC, existing Rust provider service.

---

### Task 1: Update Settings Modal Tests

**Files:**
- Modify: `tests/components/SettingsModal.test.ts`

- [ ] **Step 1: Change the public settings test**

Update `loads public reverse-provider settings with a write-only password input` so it asserts that `.api-base-url-input` is shown with the saved root URL, and `.api-models-url-input` plus `.api-chat-completions-url-input` are not rendered.

- [ ] **Step 2: Change the save test**

In `saves the password once and clears the local input after saving`, set `.api-base-url-input` to `https://custom.example.com/v1`, then expect `saveAiProvider` to receive:

```ts
baseUrl: 'https://custom.example.com/v1',
modelsUrl: 'https://custom.example.com/v1/models',
chatCompletionsUrl: 'https://custom.example.com/v1/chat/completions',
```

- [ ] **Step 3: Add unsaved detection tests**

Add one test that changes `.api-base-url-input`, clicks `.api-check-button`, and expects `checkAiProviderConnection` not to be called. Add another small check inside the same test for entering `.api-key-input`, clicking detect, and expecting the same behavior.

- [ ] **Step 4: Run targeted test before implementation**

Run: `pnpm vitest run tests/components/SettingsModal.test.ts`

Expected: tests fail because the UI still shows the advanced endpoint fields and detection does not guard unsaved edits.

### Task 2: Simplify Settings Modal Implementation

**Files:**
- Modify: `src/components/SettingsModal.vue`

- [ ] **Step 1: Add endpoint derivation helpers**

Add helpers near the existing API functions:

```ts
function trimTrailingSlashes(value: string) {
  return value.trim().replace(/\/+$/, '')
}

function providerEndpointsFromBaseUrl(baseUrl: string) {
  const normalized = trimTrailingSlashes(baseUrl)
  return {
    baseUrl: normalized,
    modelsUrl: `${normalized}/models`,
    chatCompletionsUrl: `${normalized}/chat/completions`,
  }
}
```

- [ ] **Step 2: Remove local endpoint fields**

Remove `apiModelsUrl` and `apiChatCompletionsUrl` refs. Update `applyProvider` so it only loads `provider.baseUrl` into the editable URL field.

- [ ] **Step 3: Derive endpoints on save**

In `saveApiSettings`, call `providerEndpointsFromBaseUrl(apiBaseUrl.value)` and pass the derived values into `providers.save`.

- [ ] **Step 4: Guard connection detection**

Add a helper that compares the editable URL with the saved provider base URL and checks whether `apiKey` is non-empty. In `onCheckApiConnection`, if there are unsaved changes, set a status message asking the user to save first and return before calling `checkAiProviderConnection`.

- [ ] **Step 5: Simplify the template**

Rename the visible label from `Base URL` to `API URL`, update the placeholder to `https://api.example.com/v1`, and remove the `Models URL` and `Chat Completions URL` label/input blocks.

- [ ] **Step 6: Run targeted test**

Run: `pnpm vitest run tests/components/SettingsModal.test.ts`

Expected: pass.

### Task 3: Verify Broader Frontend Health

**Files:**
- No further code changes expected.

- [ ] **Step 1: Run typecheck and tests**

Run: `pnpm typecheck && pnpm vitest run tests/stores/providers.test.ts tests/components/SettingsModal.test.ts`

Expected: pass.

- [ ] **Step 2: Check git diff**

Run: `git diff -- src/components/SettingsModal.vue tests/components/SettingsModal.test.ts`

Expected: diff only contains simplified API settings changes.
