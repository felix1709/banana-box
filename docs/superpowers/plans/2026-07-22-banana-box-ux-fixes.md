# Banana Box UX Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hide the unfinished storyboard entry, stabilize floating daily task review positioning, improve compression dependency guidance, and localize/restyle PI-Web.

**Architecture:** Keep changes scoped to existing UI components and command helpers. Add regression tests before production edits, then update Vue/Rust behavior with minimal surface area.

**Tech Stack:** Vue 3, Pinia, Vitest, Tauri 2, Rust.

---

### Task 1: Sidebar Storyboard Visibility

**Files:**
- Modify: `src/components/AppSidebar.vue`
- Modify: `tests/components/AppSidebar.test.ts`

- [ ] Add a test asserting the storyboard tool is not rendered.
- [ ] Remove the storyboard tool from the visible sidebar tool list.
- [ ] Run `pnpm test tests/components/AppSidebar.test.ts`.

### Task 2: Floating Daily Review Position

**Files:**
- Modify: `src/components/FloatButton.vue`
- Modify: `tests/components/FloatButton.test.ts`

- [ ] Add a regression test that two consecutive daily review updates use the same original compact icon anchor.
- [ ] Store the compact anchor once per active review session.
- [ ] Reuse that anchor for every next task in the same session.
- [ ] Run `pnpm test tests/components/FloatButton.test.ts`.

### Task 3: Compression Dependency Guidance

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/components/FastCompressionPanel.vue`
- Modify: `src/lib/ipc.ts`
- Modify: `tests/components/FastCompressionPanel.test.ts`

- [ ] Add frontend test coverage for FFmpeg install guidance links.
- [ ] Add backend helpers for FFmpeg diagnostic text and install URLs.
- [ ] Render dependency guidance as Chinese text with clickable links.
- [ ] Run `pnpm test tests/components/FastCompressionPanel.test.ts` and `cargo test --manifest-path src-tauri\Cargo.toml ffmpeg`.

### Task 4: PI-Web Chinese Banana Box Styling

**Files:**
- Modify: `src/components/piweb/PiWebPage.vue`
- Modify: `tests/components/PiWebPage.test.ts`

- [ ] Add tests for Chinese state labels and no English eyebrow text.
- [ ] Replace raw state values with Chinese labels.
- [ ] Adjust layout and color treatment to match existing Banana Box panels.
- [ ] Run `pnpm test tests/components/PiWebPage.test.ts`.

### Task 5: Final Verification

**Files:**
- All changed files.

- [ ] Run focused tests for sidebar, floating button, compression, and PI-Web.
- [ ] Run `pnpm check`.
- [ ] Run relevant Rust tests.
