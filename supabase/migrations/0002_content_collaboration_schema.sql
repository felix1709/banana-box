create or replace function public.workspace_role(target_workspace_id uuid)
returns text
language sql
stable
security definer
set search_path = public
as $$
  select wm.role
  from public.workspace_members wm
  where wm.workspace_id = target_workspace_id
    and wm.user_id = auth.uid()
  limit 1
$$;

create or replace function public.is_workspace_member(target_workspace_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select public.workspace_role(target_workspace_id) is not null
$$;

create or replace function public.can_edit_workspace(target_workspace_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select coalesce(public.workspace_role(target_workspace_id), '') in ('owner', 'editor')
$$;

create or replace function public.can_comment_workspace(target_workspace_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select coalesce(public.workspace_role(target_workspace_id), '') in ('owner', 'editor', 'commenter')
$$;

create table if not exists public.devices (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references public.profiles(id) on delete cascade,
  label text not null,
  last_seen_at timestamptz,
  created_at timestamptz not null default now()
);

create table if not exists public.prompt_categories (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  name text not null,
  color text not null,
  position integer not null default 0,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.prompts (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  category_id uuid references public.prompt_categories(id) on delete set null,
  title text not null,
  content text not null,
  favorite boolean not null default false,
  position integer not null default 0,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.prompt_tags (
  prompt_id uuid not null references public.prompts(id) on delete cascade,
  tag text not null,
  created_at timestamptz not null default now(),
  primary key (prompt_id, tag)
);

create table if not exists public.prompt_assets (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  prompt_id uuid not null references public.prompts(id) on delete cascade,
  asset_kind text not null check (asset_kind in ('image', 'video', 'file')),
  display_name text not null,
  storage_path text,
  device_local_ref text,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.projects (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  code text not null,
  version text not null,
  name text not null,
  file_display_ref text,
  release_date date not null,
  main_stage_key text not null,
  archived boolean not null default false,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.project_stages (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  project_id uuid not null references public.projects(id) on delete cascade,
  stage_key text not null,
  position integer not null default 0,
  start_date date not null,
  end_date date not null,
  progress integer not null default 0 check (progress between 0 and 100),
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (project_id, stage_key)
);

create table if not exists public.project_tasks (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  project_id uuid not null references public.projects(id) on delete cascade,
  stage_id uuid references public.project_stages(id) on delete set null,
  title text not null,
  progress integer not null default 0 check (progress between 0 and 100),
  position integer not null default 0,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.daily_task_days (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  local_date date not null,
  settled_at timestamptz,
  report_snapshot text,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (workspace_id, local_date)
);

create table if not exists public.daily_task_groups (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  day_id uuid not null references public.daily_task_days(id) on delete cascade,
  code text not null,
  project_id uuid references public.projects(id) on delete set null,
  position integer not null default 0,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (day_id, code)
);

create table if not exists public.daily_tasks (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  group_id uuid not null references public.daily_task_groups(id) on delete cascade,
  title text not null,
  progress integer not null default 0 check (progress between 0 and 100),
  note text,
  invested_minutes integer not null default 0,
  reminder_time text not null default '',
  reminder_content text not null default '',
  position integer not null default 0,
  source_task_id uuid,
  source_snapshot_hash text,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.invites (
  id uuid primary key default gen_random_uuid(),
  scope_type text not null check (scope_type in ('workspace', 'project')),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  project_id uuid references public.projects(id) on delete cascade,
  role text not null check (role in ('editor', 'commenter', 'viewer')),
  token_hash text not null unique,
  email text,
  expires_at timestamptz not null,
  accepted_at timestamptz,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  check ((scope_type = 'workspace' and project_id is null) or (scope_type = 'project' and project_id is not null))
);

create table if not exists public.comments (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  target_type text not null check (target_type in ('workspace', 'project', 'project_task', 'daily_task')),
  target_id uuid not null,
  parent_comment_id uuid references public.comments(id) on delete cascade,
  body text not null,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.comment_mentions (
  comment_id uuid not null references public.comments(id) on delete cascade,
  mentioned_user_id uuid not null references public.profiles(id) on delete cascade,
  created_at timestamptz not null default now(),
  primary key (comment_id, mentioned_user_id)
);

create table if not exists public.notifications (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  recipient_user_id uuid not null references public.profiles(id) on delete cascade,
  actor_user_id uuid references public.profiles(id) on delete set null,
  kind text not null check (kind in ('comment', 'mention', 'project_update', 'invite')),
  target_type text not null,
  target_id uuid not null,
  read_at timestamptz,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.project_members (
  project_id uuid not null references public.projects(id) on delete cascade,
  user_id uuid not null references public.profiles(id) on delete cascade,
  role text not null check (role in ('owner', 'editor', 'commenter', 'viewer')),
  created_at timestamptz not null default now(),
  primary key (project_id, user_id)
);

alter table public.devices enable row level security;
alter table public.prompt_categories enable row level security;
alter table public.prompts enable row level security;
alter table public.prompt_tags enable row level security;
alter table public.prompt_assets enable row level security;
alter table public.projects enable row level security;
alter table public.project_stages enable row level security;
alter table public.project_tasks enable row level security;
alter table public.daily_task_days enable row level security;
alter table public.daily_task_groups enable row level security;
alter table public.daily_tasks enable row level security;
alter table public.invites enable row level security;
alter table public.comments enable row level security;
alter table public.comment_mentions enable row level security;
alter table public.notifications enable row level security;
alter table public.project_members enable row level security;

drop policy if exists "devices are owned by current user" on public.devices;
create policy "devices are owned by current user"
on public.devices for all
to authenticated
using (user_id = auth.uid())
with check (user_id = auth.uid());

drop policy if exists "prompt categories readable by workspace members" on public.prompt_categories;
create policy "prompt categories readable by workspace members"
on public.prompt_categories for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "prompt categories editable by editors" on public.prompt_categories;
create policy "prompt categories editable by editors"
on public.prompt_categories for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "prompts readable by workspace members" on public.prompts;
create policy "prompts readable by workspace members"
on public.prompts for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "prompts editable by editors" on public.prompts;
create policy "prompts editable by editors"
on public.prompts for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "prompt tags follow prompt access" on public.prompt_tags;
create policy "prompt tags follow prompt access"
on public.prompt_tags for select
to authenticated
using (exists (select 1 from public.prompts p where p.id = prompt_tags.prompt_id and public.is_workspace_member(p.workspace_id)));

drop policy if exists "prompt tags editable with prompt" on public.prompt_tags;
create policy "prompt tags editable with prompt"
on public.prompt_tags for all
to authenticated
using (exists (select 1 from public.prompts p where p.id = prompt_tags.prompt_id and public.can_edit_workspace(p.workspace_id)))
with check (exists (select 1 from public.prompts p where p.id = prompt_tags.prompt_id and public.can_edit_workspace(p.workspace_id)));

drop policy if exists "prompt assets readable by workspace members" on public.prompt_assets;
create policy "prompt assets readable by workspace members"
on public.prompt_assets for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "prompt assets editable by editors" on public.prompt_assets;
create policy "prompt assets editable by editors"
on public.prompt_assets for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "projects readable by workspace members" on public.projects;
create policy "projects readable by workspace members"
on public.projects for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "projects editable by editors" on public.projects;
create policy "projects editable by editors"
on public.projects for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "project stages readable by workspace members" on public.project_stages;
create policy "project stages readable by workspace members"
on public.project_stages for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "project stages editable by editors" on public.project_stages;
create policy "project stages editable by editors"
on public.project_stages for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "project tasks readable by workspace members" on public.project_tasks;
create policy "project tasks readable by workspace members"
on public.project_tasks for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "project tasks editable by editors" on public.project_tasks;
create policy "project tasks editable by editors"
on public.project_tasks for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "daily days readable by workspace members" on public.daily_task_days;
create policy "daily days readable by workspace members"
on public.daily_task_days for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "daily days editable by editors" on public.daily_task_days;
create policy "daily days editable by editors"
on public.daily_task_days for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "daily groups readable by workspace members" on public.daily_task_groups;
create policy "daily groups readable by workspace members"
on public.daily_task_groups for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "daily groups editable by editors" on public.daily_task_groups;
create policy "daily groups editable by editors"
on public.daily_task_groups for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "daily tasks readable by workspace members" on public.daily_tasks;
create policy "daily tasks readable by workspace members"
on public.daily_tasks for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "daily tasks editable by editors" on public.daily_tasks;
create policy "daily tasks editable by editors"
on public.daily_tasks for all
to authenticated
using (public.can_edit_workspace(workspace_id))
with check (public.can_edit_workspace(workspace_id));

drop policy if exists "invites readable by workspace owners" on public.invites;
create policy "invites readable by workspace owners"
on public.invites for select
to authenticated
using (public.workspace_role(workspace_id) = 'owner');

drop policy if exists "invites managed by workspace owners" on public.invites;
create policy "invites managed by workspace owners"
on public.invites for all
to authenticated
using (public.workspace_role(workspace_id) = 'owner')
with check (public.workspace_role(workspace_id) = 'owner');

drop policy if exists "comments readable by workspace members" on public.comments;
create policy "comments readable by workspace members"
on public.comments for select
to authenticated
using (public.is_workspace_member(workspace_id));

drop policy if exists "comments insertable by commenters" on public.comments;
create policy "comments insertable by commenters"
on public.comments for insert
to authenticated
with check (public.can_comment_workspace(workspace_id) and created_by = auth.uid() and updated_by = auth.uid());

drop policy if exists "comments editable by authors" on public.comments;
create policy "comments editable by authors"
on public.comments for update
to authenticated
using (created_by = auth.uid() and public.can_comment_workspace(workspace_id))
with check (created_by = auth.uid() and public.can_comment_workspace(workspace_id));

drop policy if exists "comment mentions readable by workspace members" on public.comment_mentions;
create policy "comment mentions readable by workspace members"
on public.comment_mentions for select
to authenticated
using (exists (select 1 from public.comments c where c.id = comment_mentions.comment_id and public.is_workspace_member(c.workspace_id)));

drop policy if exists "comment mentions insertable by comment authors" on public.comment_mentions;
create policy "comment mentions insertable by comment authors"
on public.comment_mentions for insert
to authenticated
with check (exists (select 1 from public.comments c where c.id = comment_mentions.comment_id and c.created_by = auth.uid()));

drop policy if exists "notifications readable by recipient" on public.notifications;
create policy "notifications readable by recipient"
on public.notifications for select
to authenticated
using (recipient_user_id = auth.uid());

drop policy if exists "notifications updateable by recipient" on public.notifications;
create policy "notifications updateable by recipient"
on public.notifications for update
to authenticated
using (recipient_user_id = auth.uid())
with check (recipient_user_id = auth.uid());

drop policy if exists "project members readable by workspace members" on public.project_members;
create policy "project members readable by workspace members"
on public.project_members for select
to authenticated
using (
  exists (
    select 1
    from public.projects p
    where p.id = project_members.project_id
      and public.is_workspace_member(p.workspace_id)
  )
);

drop policy if exists "project members managed by workspace owners" on public.project_members;
create policy "project members managed by workspace owners"
on public.project_members for all
to authenticated
using (
  exists (
    select 1
    from public.projects p
    where p.id = project_members.project_id
      and public.workspace_role(p.workspace_id) = 'owner'
  )
)
with check (
  exists (
    select 1
    from public.projects p
    where p.id = project_members.project_id
      and public.workspace_role(p.workspace_id) = 'owner'
  )
);

create index if not exists prompt_categories_workspace_idx on public.prompt_categories(workspace_id, deleted_at);
create index if not exists prompts_workspace_idx on public.prompts(workspace_id, deleted_at);
create index if not exists projects_workspace_idx on public.projects(workspace_id, deleted_at);
create index if not exists project_tasks_project_idx on public.project_tasks(project_id, deleted_at);
create index if not exists daily_task_days_workspace_date_idx on public.daily_task_days(workspace_id, local_date);
create index if not exists comments_target_idx on public.comments(workspace_id, target_type, target_id, deleted_at);
create index if not exists notifications_recipient_idx on public.notifications(recipient_user_id, read_at, created_at);
