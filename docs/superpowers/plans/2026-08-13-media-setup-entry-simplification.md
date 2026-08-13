# Media Setup Entry Simplification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make FFmpeg setup visible before compression and simplify depth-video setup to one automatic action.

**Architecture:** Keep the existing Vue panels and Tauri commands. Move the FFmpeg setup call into an always-visible card, remove depth-video manual setup buttons from the template, and make backend FFmpeg download try multiple URLs.

**Tech Stack:** Vue 3, Vitest, Tauri Rust commands, PowerShell setup scripts.

## Global Constraints

- Do not touch unrelated historical dirty files.
- Do not commit or print updater private key material.
- Preserve the existing Banana Box compact production-tool UI language.
- New behavior must be covered by tests before implementation.

---

### Task 1: Fast Compression Setup Entry

**Files:**
- Modify: `tests/components/FastCompressionPanel.test.ts`
- Modify: `src/components/FastCompressionPanel.vue`

**Interfaces:**
- Consumes: `prepareFfmpegTools({ operationId })`
- Produces: always-visible `.ffmpeg-setup-card` and `.prepare-ffmpeg-button`

- [ ] Add a test that mounts `FastCompressionPanel` and expects the one-click FFmpeg button before any compression attempt.
- [ ] Run the component test and verify it fails because the button is only rendered after an error.
- [ ] Move the FFmpeg setup UI into an always-visible top card.
- [ ] Keep missing-tool error guidance, but do not duplicate the primary setup button there.
- [ ] Run the component test and verify it passes.

### Task 2: Depth Video Setup Simplification

**Files:**
- Modify: `tests/components/DepthVideoPanel.test.ts`
- Modify: `src/components/DepthVideoPanel.vue`

**Interfaces:**
- Consumes: `prepareDepthVideoPython()` and `prepareDepthVideoEngine()`
- Produces: one visible `.prepare-depth-environment-button`

- [ ] Add a test that verifies the depth-video panel does not render `.install-python-button`, `.pick-depth-engine-button`, or `.prepare-depth-engine-button`.
- [ ] Run the component test and verify it fails while old buttons still exist.
- [ ] Remove manual setup buttons from the visible UI.
- [ ] Update copy so the one-click setup says it checks existing pieces and downloads only missing pieces.
- [ ] Run the component test and verify it passes.

### Task 3: FFmpeg Download Source Fallback

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Interfaces:**
- Consumes: `prepare_ffmpeg_tools`
- Produces: FFmpeg download tries mirror URL first, official URL second.

- [ ] Add or update Rust tests for the download source list if a small pure helper exists.
- [ ] Implement a helper that returns ordered FFmpeg archive URLs.
- [ ] Make setup try each URL until one downloads, extracts, and verifies.
- [ ] Emit progress detail showing which source is being tried.
- [ ] Run Rust tests touching FFmpeg setup helpers.

### Task 4: Verification

**Files:**
- No production file changes.

- [ ] Run `pnpm vitest run tests/components/FastCompressionPanel.test.ts tests/components/DepthVideoPanel.test.ts`.
- [ ] Run `cargo test --manifest-path src-tauri\Cargo.toml commands::tests::ffmpeg`.
- [ ] Run `pnpm check` if targeted frontend tests pass.
- [ ] Report exact evidence and any remaining risk.
