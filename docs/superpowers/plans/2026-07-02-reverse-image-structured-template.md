# Reverse Image Structured Template Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make reverse image prompts return a structured Chinese long template with ten required analysis dimensions.

**Architecture:** Keep the behavior in the Rust backend because the backend builds the OpenAI-compatible vision request. Extract the prompt instruction into a small helper so tests can verify the exact output contract without calling a real model.

**Tech Stack:** Rust, Tauri command handler, existing Cargo tests.

---

### Task 1: Add Structured Reverse Image Prompt Instruction

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Test: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write the failing test**

Add a Rust unit test that calls `reverse_image_prompt_instruction()` and checks that it contains all ten required section names plus the output constraints.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri\Cargo.toml reverse_image_prompt_instruction_requires_structured_dimensions`

Expected: compile failure because `reverse_image_prompt_instruction` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add `fn reverse_image_prompt_instruction() -> &'static str` near `reverse_image_prompt`, return the structured Chinese long template, and use that helper inside the JSON request text field.

- [ ] **Step 4: Run focused test**

Run: `cargo test --manifest-path src-tauri\Cargo.toml reverse_image_prompt_instruction_requires_structured_dimensions`

Expected: pass.

- [ ] **Step 5: Run full verification**

Run:
- `pnpm check`
- `cargo test --manifest-path src-tauri\Cargo.toml`

Expected: all tests pass.
