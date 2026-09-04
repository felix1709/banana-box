# PI-Web Config Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a PI-Web config repair card so each user can enter their own API key and Banana Box writes the PI-compatible config for that Windows account.

**Architecture:** Extend `src-tauri/src/pi_web.rs` with diagnostic and repair helpers plus two Tauri commands. Extend `src/lib/piWebIpc.ts` with typed IPC wrappers, then add a compact repair card to `src/components/piweb/PiWebPage.vue`.

**Tech Stack:** Rust/Tauri commands, serde JSON file edits, Vue 3 Composition API, Vitest component tests, Rust unit tests.

---

### Task 1: Backend Diagnostics And Repair

**Files:**
- Modify: `src-tauri/src/pi_web.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] Add Rust tests in `pi_web.rs` for `diagnose_pi_agent_config_at` and `repair_pi_agent_config_at`.
- [ ] Run `cargo test --manifest-path src-tauri\Cargo.toml pi_web_config` and verify the tests fail because helpers do not exist.
- [ ] Implement diagnostic structs, helper functions, and Tauri commands:
  - `get_pi_web_config_status`
  - `repair_pi_web_config`
- [ ] Register both commands in `src-tauri/src/lib.rs`.
- [ ] Re-run the focused Rust tests and verify they pass.

### Task 2: Frontend IPC And UI

**Files:**
- Modify: `src/lib/piWebIpc.ts`
- Modify: `src/components/piweb/PiWebPage.vue`
- Modify: `tests/components/PiWebPage.test.ts`

- [ ] Add failing Vue tests for the config repair card and repair submission.
- [ ] Run `pnpm test tests/components/PiWebPage.test.ts` and verify the new tests fail.
- [ ] Add IPC types and wrappers for config status and repair.
- [ ] Add the "配置修复" card with status text, API key password input, detect button, and repair button.
- [ ] Re-run the focused Vue test and verify it passes.

### Task 3: Verification

**Files:**
- No new production files.

- [ ] Run `pnpm test tests/components/PiWebPage.test.ts`.
- [ ] Run `cargo test --manifest-path src-tauri\Cargo.toml pi_web`.
- [ ] Run `pnpm check` if the focused tests pass.
- [ ] Report any remaining warnings or local untracked files.
