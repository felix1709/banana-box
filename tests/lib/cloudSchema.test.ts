import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'

function readMigration(name: string) {
  return readFileSync(resolve(process.cwd(), 'supabase/migrations', name), 'utf8')
}

function createPolicyStatements(sql: string) {
  return [...sql.matchAll(/create policy "([^"]+)"\s+on public\.([a-z_]+)/g)]
    .map((match) => ({ name: match[1], table: match[2] }))
}

describe('cloud collaboration schema', () => {
  it('declares every phase-3 collaboration table', () => {
    const sql = readMigration('0002_content_collaboration_schema.sql')

    for (const table of [
      'devices',
      'prompt_categories',
      'prompts',
      'prompt_tags',
      'prompt_assets',
      'projects',
      'project_stages',
      'project_tasks',
      'daily_task_days',
      'daily_task_groups',
      'daily_tasks',
      'invites',
      'comments',
      'comment_mentions',
      'notifications',
    ]) {
      expect(sql).toContain(`create table if not exists public.${table}`)
    }
  })

  it('enables RLS on workspace-owned content and collaboration tables', () => {
    const sql = readMigration('0002_content_collaboration_schema.sql')

    for (const table of [
      'prompt_categories',
      'prompts',
      'projects',
      'project_tasks',
      'daily_tasks',
      'invites',
      'comments',
      'notifications',
    ]) {
      expect(sql).toContain(`alter table public.${table} enable row level security`)
    }
  })

  it('keeps shared sync columns on workspace content tables', () => {
    const sql = readMigration('0002_content_collaboration_schema.sql')

    expect(sql).toContain('workspace_id uuid not null references public.workspaces(id) on delete cascade')
    expect(sql).toContain('created_by uuid not null references public.profiles(id)')
    expect(sql).toContain('updated_by uuid not null references public.profiles(id)')
    expect(sql).toContain('revision integer not null default 0')
    expect(sql).toContain('deleted_at timestamptz')
  })

  it('defines role helper functions for RLS policies', () => {
    const sql = readMigration('0002_content_collaboration_schema.sql')

    expect(sql).toContain('create or replace function public.workspace_role')
    expect(sql).toContain('create or replace function public.can_edit_workspace')
    expect(sql).toContain('create or replace function public.can_comment_workspace')
  })

  it('adds invite acceptance and realtime publication SQL', () => {
    const sql = readMigration('0003_invite_acceptance_realtime.sql')

    expect(sql).toContain('create or replace function public.accept_invite')
    expect(sql).toContain("encode(digest(invite_token, 'sha256'), 'hex')")
    expect(sql).toContain('alter publication supabase_realtime add table public.projects')
    expect(sql).toContain('alter publication supabase_realtime add table public.comments')
  })

  it('adds a workspace bootstrap RPC for first login', () => {
    const sql = readMigration('0004_workspace_bootstrap_fix.sql')

    expect(sql).toContain('create or replace function public.ensure_personal_workspace')
    expect(sql).toContain('workspace_name text')
    expect(sql).toContain('insert into public.workspace_members')
    expect(sql).toContain('grant execute on function public.ensure_personal_workspace(text) to authenticated')
  })

  it('adds a profile-safe bootstrap RPC that runs behind RLS', () => {
    const sql = readMigration('0005_profile_bootstrap_fix.sql')

    expect(sql).toContain('create or replace function public.bootstrap_user_workspace')
    expect(sql).toContain('security definer')
    expect(sql).toContain('insert into public.profiles')
    expect(sql).toContain('on conflict (id) do update')
    expect(sql).toContain('jsonb_build_object')
    expect(sql).toContain("position('?' in workspace_row.name) > 0")
    expect(sql).toContain('update public.workspaces')
    expect(sql).toContain('grant execute on function public.bootstrap_user_workspace(text, text, text) to authenticated')
  })

  it('adds daily task reminder columns to cloud sync schema', () => {
    const baseSql = readMigration('0002_content_collaboration_schema.sql')
    const sql = readMigration('0006_daily_task_reminders.sql')

    expect(baseSql).toContain("reminder_time text not null default ''")
    expect(baseSql).toContain("reminder_content text not null default ''")
    expect(sql).toContain('alter table public.daily_tasks')
    expect(sql).toContain("add column if not exists reminder_time text not null default ''")
    expect(sql).toContain("add column if not exists reminder_content text not null default ''")
  })

  it('adds project-level collaboration without granting whole-workspace membership', () => {
    const sql = readMigration('0007_project_level_collaboration.sql')

    expect(sql).toContain('alter table public.projects')
    expect(sql).toContain('add column if not exists owner_user_id uuid')
    expect(sql).toContain('add column if not exists is_public boolean not null default false')
    expect(sql).toContain('create table if not exists public.project_activity_log')
    expect(sql).toContain('create or replace function public.project_role')
    expect(sql).toContain('matching_invite.scope_type = \'workspace\'')
    expect(sql).toContain('matching_invite.scope_type = \'project\'')
    expect(sql).toContain('insert into public.project_members')
  })

  it('adds project invite notifications and searchable profile lookup', () => {
    const sql = readMigration('0008_project_invite_notifications.sql')

    expect(sql).toContain('profiles searchable by authenticated users')
    expect(sql).toContain('create or replace function public.accept_invite_by_id')
    expect(sql).toContain('recipient_user_id = auth.uid()')
    expect(sql).toContain('matching_invite.scope_type = \'project\'')
    expect(sql).toContain('grant execute on function public.accept_invite_by_id(uuid) to authenticated')
  })

  it('adds a profile display name RPC for reliable nickname editing', () => {
    const sql = readMigration('0009_profile_display_name_rpc.sql')

    expect(sql).toContain('create or replace function public.update_own_profile_display_name')
    expect(sql).toContain('where id = auth.uid()')
    expect(sql).toContain('DISPLAY_NAME_REQUIRED')
    expect(sql).toContain('grant execute on function public.update_own_profile_display_name(text) to authenticated')
  })

  it('allows workspace collaborators to create notification rows for invites and mentions', () => {
    const sql = readMigration('0010_notifications_insert_policy.sql')

    expect(sql).toContain('notifications insertable by workspace collaborators')
    expect(sql).toContain('on public.notifications for insert')
    expect(sql).toContain('public.can_comment_workspace(workspace_id)')
    expect(sql).toContain('actor_user_id = auth.uid()')
    expect(sql).toContain('created_by = auth.uid()')
    expect(sql).toContain('updated_by = auth.uid()')
  })

  it('uses the pgcrypto extension schema when accepting invite tokens', () => {
    const sql = readMigration('0011_invite_digest_extension_path.sql')

    expect(sql).toContain('create extension if not exists pgcrypto with schema extensions')
    expect(sql).toContain('create or replace function public.accept_invite')
    expect(sql).toContain("encode(extensions.digest(invite_token, 'sha256'), 'hex')")
    expect(sql).toContain('grant execute on function public.accept_invite(text) to authenticated')
  })

  it('avoids ambiguous project_id references when accepting project invite notifications', () => {
    const sql = readMigration('0012_invite_acceptance_conflict_targets.sql')

    expect(sql).toContain('create or replace function public.accept_invite_by_id')
    expect(sql).toContain('on conflict on constraint project_members_pkey')
    expect(sql).toContain('on conflict on constraint workspace_members_pkey')
    expect(sql).not.toContain('on conflict (project_id, user_id)')
  })

  it('adds project schedule change requests for owner approval', () => {
    const sql = readMigration('0013_project_schedule_change_requests.sql')

    expect(sql).toContain('create table if not exists public.project_schedule_change_requests')
    expect(sql).toContain("status text not null default 'pending'")
    expect(sql).toContain('requested_start_date date not null')
    expect(sql).toContain('requested_end_date date not null')
    expect(sql).toContain('project schedule requests insertable by project members')
    expect(sql).toContain('project schedule requests decidable by project owners')
    expect(sql).toContain('alter publication supabase_realtime add table public.project_schedule_change_requests')
  })

  it('allows project collaborators to comment and create project-scoped notifications', () => {
    const sql = readMigration('0014_project_collaboration_rls_fixes.sql')

    expect(sql).toContain('comments readable by project collaborators')
    expect(sql).toContain('comments insertable by project collaborators')
    expect(sql).toContain("target_type = 'project'")
    expect(sql).toContain('public.can_comment_project(target_id)')
    expect(sql).toContain('notifications insertable by project collaborators')
    expect(sql).toContain("target_type = 'project_schedule_request'")
    expect(sql).toContain('public.project_schedule_change_requests r')
    expect(sql).toContain('public.can_comment_project(r.project_id)')
  })

  it('adds a global shared prompt library with reference rows instead of duplicate prompt copies', () => {
    const sql = readMigration('0015_shared_prompt_library.sql')

    expect(sql).toContain('create table if not exists public.shared_prompts')
    expect(sql).toContain('create table if not exists public.user_prompt_refs')
    expect(sql).toContain('title_key text not null')
    expect(sql).toContain('shared prompts readable by authenticated users')
    expect(sql).toContain('shared prompts insertable by authenticated users')
    expect(sql).toContain('shared prompt refs owned by users')
    expect(sql).toContain('shared_prompts_title_key_unique')
  })

  it('allows the 000001 administrator to moderate shared prompts', () => {
    const sql = readMigration('0016_shared_prompt_admin_delete.sql')

    expect(sql).toContain('shared prompts editable by authors and admin')
    expect(sql).toContain("lower(p.email) = '000001@banana-box.local'")
    expect(sql).toContain('deleted_at')
  })

  it('makes copied RLS policies safe to run more than once', () => {
    for (const migration of [
      '0001_auth_workspaces.sql',
      '0002_content_collaboration_schema.sql',
      '0004_workspace_bootstrap_fix.sql',
      '0014_project_collaboration_rls_fixes.sql',
    ]) {
      const sql = readMigration(migration)
      for (const policy of createPolicyStatements(sql)) {
        expect(sql).toContain(`drop policy if exists "${policy.name}" on public.${policy.table};`)
      }
    }
  })
})
