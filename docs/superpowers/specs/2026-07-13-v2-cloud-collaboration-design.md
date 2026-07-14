# Banana Box v2 Cloud Collaboration Design

## Goal

Upgrade Banana Box from a local-first desktop tool into a logged-in cloud collaboration app while preserving local offline use.

When a user is not logged in, Banana Box remains a local-only tool. After login, all syncable data belongs to a user workspace and can be synchronized across devices. Workspace and project members can collaborate with role-based permissions, comments, mentions, and realtime updates.

## Confirmed Product Rules

- Supabase is the default cloud backend.
- Unauthenticated users can keep using local offline mode.
- Authenticated users default to their cloud workspace.
- On first login, if local data exists, the app asks whether to migrate it to cloud.
- Prompt library, project management, daily tasks, comments, invitations, members, and notifications all belong to the same workspace model.
- Workspace invitation and single-project invitation are both supported.
- Project-level permissions override workspace-level permissions for that project.
- Cloud data is the collaboration source of truth.
- Local SQLite remains a cache and offline edit queue.

## Users And Workspaces

Supabase Auth owns authentication. Banana Box stores app-specific user information in a `profiles` table keyed by Supabase user ID.

Each user gets a default personal workspace. A workspace may also represent a team or shared production group. All syncable data includes `workspace_id`.

Core tables:

- `profiles`
- `workspaces`
- `workspace_members`
- `project_members`
- `devices`

Roles:

- `owner`: full control, including inviting and removing members.
- `editor`: can edit prompts, projects, project tasks, and daily tasks.
- `commenter`: can view and comment, but cannot edit plans.
- `viewer`: read-only.

## Data Ownership Model

Every cloud table that stores user content includes:

- `id`
- `workspace_id`
- `created_by`
- `updated_by`
- `revision`
- `deleted_at`
- `created_at`
- `updated_at`

Soft deletion through `deleted_at` is required so offline devices can sync deletes safely.

## Prompt Library Cloud Model

Prompt library data moves under workspace ownership.

Cloud tables:

- `prompt_categories`
- `prompts`
- `prompt_tags`
- `prompt_assets`

The local prompt library becomes a cached projection of the active workspace. A local-only library can remain available when the user is logged out.

## Project And Daily Task Cloud Model

Current local project tables become workspace-scoped cloud tables.

Cloud tables:

- `projects`
- `project_stages`
- `project_tasks`
- `daily_task_days`
- `daily_task_groups`
- `daily_tasks`

`project_tasks` is added because comments need a stable task-level target that is not only a stage row. Existing daily tasks can also receive comments.

Local file paths remain device-local metadata. A path that exists on one device may not exist on another, so cloud project records should store optional display references, while each device can keep its own local file binding.

## Invitation Model

Invitations support both workspace-wide and project-specific access.

Cloud table:

- `invites`

Invite fields:

- `scope_type`: `workspace` or `project`
- `workspace_id`
- `project_id`
- `role`
- `token_hash`
- `email`
- `expires_at`
- `accepted_at`
- `created_by`

Accepting a workspace invite creates a `workspace_members` row. Accepting a project invite creates a `project_members` row.

## Comments, Replies, Mentions, Notifications

Comments can attach to multiple target types:

- workspace
- project
- project task
- daily task

Cloud tables:

- `comments`
- `comment_mentions`
- `notifications`

Comment fields:

- `target_type`
- `target_id`
- `parent_comment_id`
- `body`
- `created_by`
- `deleted_at`

Replies use `parent_comment_id`. Mentions are parsed into `comment_mentions`, which also create `notifications` for mentioned users.

Comments are append-first data. Editing and deleting comments should preserve audit fields rather than overwriting history silently.

## Realtime Collaboration

Supabase Realtime is used for:

- project changes
- project task changes
- daily task changes
- comments and replies
- mentions and notifications
- online member presence

The app subscribes to the active workspace and, where useful, the selected project. Online users receive updates immediately. Offline users catch up on startup or reconnect using `updated_at`, `revision`, and sync cursors.

## Local Offline And Sync Engine

Local SQLite remains in use, but its role changes:

- cache cloud workspace data for fast startup;
- keep local-only data when logged out;
- store offline edits in a sync outbox;
- track sync cursors per workspace and table.

Local sync tables:

- `sync_outbox`
- `sync_cursors`
- `local_device_bindings`

Outbox entries include:

- target table
- record ID
- operation type
- payload JSON
- base revision
- created_at
- retry state

When online, the app uploads outbox entries in order. If the cloud revision no longer matches the base revision, the item becomes a conflict.

## Conflict Rules

Comments:

- append-only by default;
- no overwrite conflict for new comments.

Prompt library:

- field-level latest write can be accepted for simple fields;
- keep revision history for changed prompt content.

Projects and project tasks:

- if two users edit the same record from the same base revision, show a conflict;
- allow user to choose cloud version, local version, or manual merge.

Daily tasks:

- if different tasks are edited, merge;
- if the same task is edited, show a conflict;
- comments still append safely.

Deletes:

- cloud and local both use soft delete;
- restoring a deleted record requires an explicit action.

## Security Model

Frontend permissions are only user experience hints. The real enforcement is database policy.

Supabase Row Level Security policies must ensure:

- users can read workspace data only if they are workspace members or project members for scoped project data;
- only `owner` can manage workspace members;
- `editor` can edit workspace content;
- `commenter` can insert comments but cannot edit plans;
- `viewer` can only read;
- project-level membership can grant or restrict project access independently of workspace-level membership.

Invite tokens are stored hashed in the database. Raw invite tokens are only shown in the generated link.

## UI Changes

Global app shell:

- login/logout entry;
- current workspace switcher;
- sync status indicator;
- offline/online state;
- conflict center;
- notifications entry.

Project management:

- invite button;
- member avatars/presence;
- permission-aware controls;
- project activity/comments panel;
- task comments;
- realtime update indicators.

Prompt library:

- cloud/local state indicator;
- migration prompt after login;
- conflict and sync status for prompt edits.

Daily tasks:

- workspace-scoped daily task data;
- comments and mentions on daily task items;
- sync status for offline edits.

## Rollout Stages

1. Cloud foundation and Supabase configuration.
2. Login, profiles, workspace creation, and local logged-out mode boundary.
3. Cloud schema and RLS policies.
4. Local schema upgrade to add workspace identity and sync metadata.
5. Local-to-cloud migration wizard.
6. Sync engine with pull, push, outbox, soft delete, and conflict detection.
7. Workspace and project invitations with role management.
8. Comments, replies, mentions, and notifications.
9. Realtime subscriptions and presence.
10. Full QA, release checklist, and migration safety validation.

## Non-Goals For The First v2 Release

- Full Google Docs-style simultaneous text editing.
- Automatic merge for complex project plan conflicts.
- Public anonymous project sharing.
- Uploading local project files to cloud storage by default.
- Removing local offline mode.

## Open Decisions

- Exact Supabase project URL and environment management.
- Whether email sending uses Supabase built-in email templates or a dedicated email provider.
- Whether cloud subscription limits are needed for storage or member count.
- Final conflict UI wording and merge screen design.
