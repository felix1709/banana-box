# Banana Box v2 Phase 3 Cloud Schema RLS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the Supabase schema foundation for workspace-owned prompts, projects, daily tasks, invitations, comments, mentions, notifications, and role-aware access.

**Architecture:** Keep Supabase Auth as identity, use `profiles`, `workspaces`, and `workspace_members` from Phase 2, then add workspace-scoped content tables with shared sync columns. Enforce read/write/comment permissions through Postgres Row Level Security helper functions.

**Tech Stack:** Supabase Postgres, SQL migrations, RLS, Vitest SQL file contract tests.

---

## Implemented Files

- Create: `supabase/migrations/0002_content_collaboration_schema.sql`
- Create: `tests/lib/cloudSchema.test.ts`

## Cloud Tables

- Prompt library: `prompt_categories`, `prompts`, `prompt_tags`, `prompt_assets`
- Project management: `projects`, `project_stages`, `project_tasks`, `project_members`
- Daily tasks: `daily_task_days`, `daily_task_groups`, `daily_tasks`
- Collaboration: `invites`, `comments`, `comment_mentions`, `notifications`, `devices`

## Shared Sync Columns

Workspace content tables include:

- `workspace_id`
- `created_by`
- `updated_by`
- `revision`
- `deleted_at`
- `created_at`
- `updated_at`

## RLS Helpers

- `public.workspace_role(workspace_id)`
- `public.is_workspace_member(workspace_id)`
- `public.can_edit_workspace(workspace_id)`
- `public.can_comment_workspace(workspace_id)`

## Verification

- `pnpm vitest run tests/lib/cloudSchema.test.ts`

Expected:

- 4 tests pass.

## Notes

This phase creates database structure and basic RLS policy coverage. It does not yet upload local data, create invite links in the UI, or subscribe to realtime channels.
