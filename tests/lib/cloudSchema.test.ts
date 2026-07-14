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

  it('makes copied RLS policies safe to run more than once', () => {
    for (const migration of [
      '0001_auth_workspaces.sql',
      '0002_content_collaboration_schema.sql',
      '0004_workspace_bootstrap_fix.sql',
    ]) {
      const sql = readMigration(migration)
      for (const policy of createPolicyStatements(sql)) {
        expect(sql).toContain(`drop policy if exists "${policy.name}" on public.${policy.table};`)
      }
    }
  })
})
