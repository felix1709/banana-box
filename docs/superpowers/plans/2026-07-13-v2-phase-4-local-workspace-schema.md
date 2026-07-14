# Banana Box v2 Phase 4 Local Workspace Schema Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the local SQLite database so local-only data and cloud workspace cache data can coexist safely.

**Architecture:** Keep existing local behavior as the default `local` workspace, add sync metadata columns to existing production tables, and create local queue/cursor/binding tables for later offline sync.

**Tech Stack:** Tauri 2, Rust, SQLite migrations, rusqlite tests.

---

## Implemented Files

- Create: `src-tauri/migrations/0005_workspace_sync_foundation.sql`
- Modify: `src-tauri/src/db/schema.rs`

## Local Tables Added

- `local_workspaces`
- `sync_outbox`
- `sync_cursors`
- `local_device_bindings`

## Local Columns Added

`projects` and `daily_task_days` now include:

- `local_workspace_id`
- `cloud_id`
- `cloud_workspace_id`
- `revision`
- `sync_state`
- `deleted_at`

Child tables such as `project_stages`, `daily_task_groups`, and `daily_tasks` now include:

- `cloud_id`
- `revision`
- `sync_state`
- `deleted_at`

## Safety Rules

- Existing data remains in the default `local` workspace.
- No local data is uploaded by this phase.
- No existing project or daily-task query is filtered differently yet.
- Deletes still behave as before until sync engine and soft-delete behavior are implemented deliberately.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml schema::tests -- --nocapture`

Expected:

- 12 schema tests pass.
