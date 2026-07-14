# Banana Box v2 Cloud Collaboration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade Banana Box into a logged-in cloud collaboration app where prompts, projects, daily tasks, comments, invitations, permissions, sync, and realtime updates all belong to user workspaces.

**Architecture:** Use Supabase Auth, Postgres, Row Level Security, and Realtime as the cloud collaboration backend. Keep local SQLite as offline cache and local-only mode, then add workspace identity, sync cursors, outbox, migration, role-aware UI, comments, and realtime subscriptions in staged slices.

**Tech Stack:** Tauri 2, Rust, SQLite, Vue 3, Pinia, TypeScript, Supabase Auth, Supabase Postgres, Supabase Realtime, Supabase RLS, Vitest, Cargo tests.

---

## Scope Note

This is the master plan for the v2 cloud collaboration upgrade. The full implementation is too large and risky for one monolithic code plan. Each phase below must get its own detailed implementation plan before code changes begin.

The master sequence is still strict: do not implement realtime collaboration before identity, workspace ownership, schema, local migration, and sync foundations exist.

## Current Baseline

Current Banana Box stores production data locally:

- Prompt library is local app data.
- Project management uses local SQLite tables `projects` and `project_stages`.
- Daily tasks use local SQLite tables `daily_task_days`, `daily_task_groups`, and `daily_tasks`.
- There is no login, cloud workspace, member role, invitation, comment model, notification model, sync outbox, sync cursor, or realtime subscription.

Existing project files that v2 will eventually touch:

- `src/domain/production.ts`
- `src/stores/library.ts`
- `src/stores/projects.ts`
- `src/stores/dailyTasks.ts`
- `src/lib/ipc.ts`
- `src/lib/productionIpc.ts`
- `src/components/SettingsModal.vue`
- `src/components/projects/ProjectBoardPage.vue`
- `src/components/projects/ProjectEditor.vue`
- `src/components/projects/ProjectTimeline.vue`
- `src/components/daily/DailyTasksPage.vue`
- `src-tauri/migrations/*.sql`
- `src-tauri/src/db/schema.rs`
- `src-tauri/src/projects/*`
- `src-tauri/src/daily_tasks/*`
- `src-tauri/src/library.rs`
- `tests/components/*`
- `tests/stores/*`
- `src-tauri/src/*/tests.rs`

## Phase 1: Cloud Foundation And Configuration

**Outcome:** The app can store Supabase configuration, validate cloud settings, and clearly distinguish local-only mode from cloud-capable mode.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-1-cloud-foundation.md`

**Files likely touched:**

- Create: `src/types/cloud.ts`
- Create: `src/lib/cloud-config.ts`
- Create: `src/stores/cloudSession.ts`
- Create: `tests/stores/cloudSession.test.ts`
- Create: `src-tauri/migrations/0004_cloud_foundation.sql`
- Modify: `src/components/SettingsModal.vue`
- Modify: `src-tauri/src/db/schema.rs`

**Acceptance criteria:**

- User can enter Supabase URL and anon key in settings.
- Secrets are not logged.
- Cloud settings can be validated without forcing login.
- App has a visible local-only/cloud-ready state.
- Existing local-only workflows continue to pass.

**Verification:**

- `pnpm check`
- `cargo test --manifest-path src-tauri/Cargo.toml`

## Phase 2: Login, Profiles, Workspace Creation

**Outcome:** Users can log in, load their profile, and enter a default workspace. Logged-out users remain in local-only mode.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-2-auth-workspaces.md`

**Files likely touched:**

- Create: `src/lib/supabaseClient.ts`
- Create: `src/stores/auth.ts`
- Create: `src/stores/workspaces.ts`
- Create: `src/components/auth/LoginPanel.vue`
- Create: `src/components/workspaces/WorkspaceSwitcher.vue`
- Create: `supabase/migrations/0001_auth_workspaces.sql`
- Create: `tests/stores/auth.test.ts`
- Create: `tests/stores/workspaces.test.ts`
- Modify: `src/App.vue`
- Modify: `src/components/AppSidebar.vue`

**Acceptance criteria:**

- User can log in and log out.
- New user gets or creates a personal workspace.
- App shell shows active workspace.
- Logged-out mode does not upload local data.
- Auth state survives app restart using secure session storage.

**Verification:**

- `pnpm check`
- Supabase local migration dry run or SQL lint command chosen in the phase plan.

## Phase 3: Cloud Schema And RLS

**Outcome:** Supabase schema exists for profiles, workspaces, members, prompts, projects, daily tasks, invites, comments, mentions, notifications, and sync events, with enforceable RLS policies.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-3-cloud-schema-rls.md`

**Files likely touched:**

- Create: `supabase/migrations/0002_content_schema.sql`
- Create: `supabase/migrations/0003_collaboration_schema.sql`
- Create: `supabase/tests/rls.sql`
- Create: `docs/cloud-schema.md`

**Acceptance criteria:**

- Every user-content table has `workspace_id`, `created_by`, `updated_by`, `revision`, `deleted_at`, `created_at`, and `updated_at`.
- Workspace role policies enforce `owner`, `editor`, `commenter`, and `viewer`.
- Project membership can grant project-scoped access.
- Project-level membership overrides workspace access for that project.
- Invite tokens are stored hashed.
- Viewers cannot write.
- Commenters can comment but cannot edit plans.
- Editors can edit content.
- Owners can manage members.

**Verification:**

- Supabase local test suite or SQL policy test runner selected in the phase plan.
- Manual SQL role simulation tests documented in `docs/cloud-schema.md`.

## Phase 4: Local Schema Workspace Identity

**Outcome:** Local SQLite can represent local-only data and cloud workspace cached data without mixing them.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-4-local-workspace-schema.md`

**Files likely touched:**

- Create: `src-tauri/migrations/0004_workspace_identity.sql`
- Modify: `src-tauri/src/db/schema.rs`
- Modify: `src-tauri/src/library.rs`
- Modify: `src-tauri/src/projects/model.rs`
- Modify: `src-tauri/src/projects/repository.rs`
- Modify: `src-tauri/src/daily_tasks/model.rs`
- Modify: `src-tauri/src/daily_tasks/repository.rs`
- Modify: `src/domain/production.ts`
- Modify: `tests/stores/projects.test.ts`
- Modify: `tests/stores/dailyTasks.test.ts`
- Modify: `src-tauri/src/projects/tests.rs`
- Modify: `src-tauri/src/daily_tasks/tests.rs`

**Acceptance criteria:**

- Local records can be scoped to either local-only mode or a cloud workspace.
- Existing local data migrates into a local-only workspace marker.
- Project and daily task reads filter by active local workspace context.
- No existing local data is deleted during migration.

**Verification:**

- `pnpm check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- A migration fixture test with pre-v2 local data.

## Phase 5: Local-To-Cloud Migration Wizard

**Outcome:** After first login, the user can migrate local prompts, projects, and daily tasks into the active cloud workspace.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-5-cloud-migration-wizard.md`

**Files likely touched:**

- Create: `src/components/cloud/CloudMigrationDialog.vue`
- Create: `src/stores/cloudMigration.ts`
- Create: `src/lib/cloudMigration.ts`
- Create: `tests/components/CloudMigrationDialog.test.ts`
- Create: `tests/stores/cloudMigration.test.ts`
- Modify: `src/App.vue`
- Modify: `src/stores/library.ts`
- Modify: `src/stores/projects.ts`
- Modify: `src/stores/dailyTasks.ts`

**Acceptance criteria:**

- Dialog appears after login when local-only data exists.
- User can choose migrate, keep local, or decide later.
- Migration is idempotent.
- Partial failure does not delete local data.
- Migrated records receive cloud IDs and workspace IDs.
- Local file paths are kept as device-local bindings, not treated as cloud file uploads.

**Verification:**

- `pnpm check`
- Migration dry-run test with prompts, projects, and daily tasks.

## Phase 6: Sync Engine

**Outcome:** The app can pull cloud data, cache it locally, push offline edits, track cursors, soft delete records, and detect conflicts.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-6-sync-engine.md`

**Files likely touched:**

- Create: `src/lib/sync/syncClient.ts`
- Create: `src/lib/sync/outbox.ts`
- Create: `src/stores/syncStatus.ts`
- Create: `src/components/cloud/SyncStatusIndicator.vue`
- Create: `src/components/cloud/ConflictCenter.vue`
- Create: `src-tauri/migrations/0005_sync_outbox.sql`
- Create: `tests/lib/sync/outbox.test.ts`
- Create: `tests/stores/syncStatus.test.ts`
- Modify: `src/stores/library.ts`
- Modify: `src/stores/projects.ts`
- Modify: `src/stores/dailyTasks.ts`

**Acceptance criteria:**

- Startup pulls cloud workspace changes.
- Local edits while offline enter `sync_outbox`.
- Online reconnect pushes pending outbox entries in order.
- Cloud updates refresh local cache.
- Soft deletes sync safely.
- Revision mismatch creates a conflict instead of overwriting silently.

**Verification:**

- `pnpm check`
- Unit tests for outbox ordering, retry, soft delete, and conflict detection.

## Phase 7: Invitations And Permissions

**Outcome:** Users can invite collaborators to a workspace or a single project, and role-aware UI plus RLS-backed permissions are enforced.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-7-invites-permissions.md`

**Files likely touched:**

- Create: `src/components/collaboration/InviteDialog.vue`
- Create: `src/components/collaboration/MemberList.vue`
- Create: `src/stores/members.ts`
- Create: `src/lib/invites.ts`
- Create: `tests/components/InviteDialog.test.ts`
- Create: `tests/stores/members.test.ts`
- Modify: `src/components/projects/ProjectBoardPage.vue`
- Modify: `src/components/projects/ProjectEditor.vue`
- Modify: `supabase/migrations/*`

**Acceptance criteria:**

- Owner can generate workspace invite link.
- Owner can generate project invite link.
- Invite can default to viewer, commenter, or editor.
- Accepted workspace invite creates workspace membership.
- Accepted project invite creates project membership.
- Permission controls hide or disable unavailable actions.
- Backend/RLS denies forbidden writes even if UI is bypassed.

**Verification:**

- `pnpm check`
- Supabase RLS invite tests.

## Phase 8: Comments, Replies, Mentions, Notifications

**Outcome:** Members can comment on projects and tasks, reply in threads, mention users, and receive notifications.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-8-comments-notifications.md`

**Files likely touched:**

- Create: `src/components/collaboration/CommentPanel.vue`
- Create: `src/components/collaboration/CommentThread.vue`
- Create: `src/components/collaboration/MentionInput.vue`
- Create: `src/components/collaboration/NotificationsMenu.vue`
- Create: `src/stores/comments.ts`
- Create: `src/stores/notifications.ts`
- Create: `tests/components/CommentPanel.test.ts`
- Create: `tests/stores/comments.test.ts`
- Modify: `src/components/projects/ProjectBoardPage.vue`
- Modify: `src/components/daily/DailyTasksPage.vue`

**Acceptance criteria:**

- Comment can attach to project, project task, or daily task.
- Replies use `parent_comment_id`.
- `@member` creates mention records and notifications.
- Commenters can comment but cannot edit plans.
- Viewers cannot comment.
- Deleted comments remain as tombstones for sync.

**Verification:**

- `pnpm check`
- RLS tests for commenter and viewer behavior.

## Phase 9: Realtime Subscriptions And Presence

**Outcome:** Online collaborators see project changes, task changes, comments, mentions, notifications, and presence updates in realtime.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-9-realtime-presence.md`

**Files likely touched:**

- Create: `src/lib/realtime/workspaceRealtime.ts`
- Create: `src/stores/presence.ts`
- Create: `src/components/collaboration/PresenceAvatars.vue`
- Create: `tests/stores/presence.test.ts`
- Modify: `src/stores/projects.ts`
- Modify: `src/stores/dailyTasks.ts`
- Modify: `src/stores/comments.ts`
- Modify: `src/stores/notifications.ts`
- Modify: `src/components/projects/ProjectBoardPage.vue`

**Acceptance criteria:**

- Active workspace subscribes to relevant cloud changes.
- Online project edits appear without manual refresh.
- New comments and mentions appear without manual refresh.
- Presence shows online members for active workspace or project.
- Reconnect pulls missed changes before resuming live subscriptions.

**Verification:**

- `pnpm check`
- Realtime client tests using mocked Supabase channels.
- Manual two-window collaboration QA in the phase plan.

## Phase 10: Conflict UI, QA, Release Readiness

**Outcome:** v2 can be tested and released with clear migration safety, conflict handling, and collaboration acceptance paths.

**Detailed plan to create:** `docs/superpowers/plans/2026-07-13-v2-phase-10-qa-release.md`

**Files likely touched:**

- Create: `docs/cloud-collaboration-qa.md`
- Create: `tests/components/ConflictCenter.test.ts`
- Modify: `fabu.MD` if release checklist needs cloud-specific checks.
- Modify: `README.md` if user-facing setup docs are required.

**Acceptance criteria:**

- Cloud setup failure has clear UI.
- Login/logout flows pass.
- Local-only mode still works.
- Local-to-cloud migration is reversible by retaining local data.
- Workspace invite and project invite flows pass.
- Owner/editor/commenter/viewer permissions pass.
- Realtime comments and project updates pass.
- Offline edit, reconnect, conflict, and manual resolution pass.
- Release checklist covers Supabase environment requirements.

**Verification:**

- `pnpm check`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- Browser or desktop QA checklist from `docs/cloud-collaboration-qa.md`

## Execution Rules

- Do not begin a phase until its detailed phase plan exists and has been reviewed.
- Do not skip tests for schema, permissions, sync, or conflict handling.
- Do not store Supabase service role keys in the desktop app.
- Do not upload local project files by default.
- Do not delete local data during migration.
- Keep each phase independently verifiable.

## Recommended Start

Start with Phase 1. It creates safe configuration and visible cloud readiness without touching existing local project behavior.
