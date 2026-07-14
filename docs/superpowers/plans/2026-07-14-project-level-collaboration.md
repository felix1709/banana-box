# Project Level Collaboration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add project-level public sharing, project-only invites, nickname display, and project activity history.

**Architecture:** Keep local project data as the editing source for the desktop app, then mirror collaboration fields into Supabase during cloud upload/sync. Project ownership and public status live on projects; invite creation is blocked unless the selected project is public and owned by the current user.

**Tech Stack:** Vue 3, Pinia, Vitest, Tauri Rust, SQLite migrations, Supabase SQL/RLS.

---

### Task 1: Project Model And Local Schema

**Files:**
- Modify: `src-tauri/src/db/schema.rs`
- Create: `src-tauri/migrations/0007_project_collaboration.sql`
- Modify: `src-tauri/src/projects/model.rs`
- Modify: `src-tauri/src/projects/repository.rs`
- Test: `src-tauri/src/projects/tests.rs`

- [ ] Add SQLite columns: `owner_user_id`, `is_public`, `last_activity_summary`, `last_activity_actor_name`.
- [ ] Add `project_activity_log` table for who changed what.
- [ ] Return these fields in `ProjectDto`.
- [ ] Add repository function/input for setting project public status.
- [ ] Add tests for default private project and public toggle.

### Task 2: Nickname Profile UI

**Files:**
- Modify: `src/types/auth.ts`
- Modify: `src/stores/workspaces.ts`
- Modify: `src/components/workspaces/WorkspaceSwitcher.vue`
- Test: `tests/stores/workspaces.test.ts`
- Test: `tests/components/WorkspaceSwitcher.test.ts`

- [ ] Let logged-in users edit `display_name`.
- [ ] Persist nickname to Supabase `profiles.display_name`.
- [ ] Show nickname before email/account in collaboration UI.

### Task 3: Project Board UI Rules

**Files:**
- Modify: `src/domain/production.ts`
- Modify: `src/stores/projects.ts`
- Modify: `src/components/projects/ProjectBoardPage.vue`
- Modify: `src/components/collaboration/InviteDialog.vue`
- Test: `tests/components/ProjectBoardPage.test.ts`
- Test: `tests/components/InviteDialog.test.ts`

- [ ] Show public project badge on public project notes.
- [ ] Show public toggle only when current user owns the project.
- [ ] Allow invite creation only for public selected projects.
- [ ] Keep project card layout compact and stable.

### Task 4: Project-Only Cloud Collaboration

**Files:**
- Modify: `supabase/migrations/0002_content_collaboration_schema.sql`
- Create: `supabase/migrations/0007_project_level_collaboration.sql`
- Modify: `src/stores/members.ts`
- Modify: `src/stores/cloudMigration.ts`
- Modify: `src/stores/syncStatus.ts`
- Test: `tests/lib/cloudSchema.test.ts`
- Test: `tests/stores/members.test.ts`
- Test: `tests/stores/cloudMigration.test.ts`

- [ ] Add cloud columns/policies for project ownership, public flag, and project activity.
- [ ] Change invite acceptance so project invites add the user to `project_members`, not full workspace membership.
- [ ] Sync public flags, project member access, and activity rows.

### Task 5: Verification And Release

**Files:**
- Modify: version files only when publishing.

- [ ] Run `pnpm check`.
- [ ] Run `cargo test --manifest-path src-tauri\Cargo.toml`.
- [ ] Build and publish after the user requests release or after confirming this upgrade is ready.
