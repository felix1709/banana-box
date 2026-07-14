alter table public.projects
  add column if not exists owner_user_id uuid references public.profiles(id),
  add column if not exists is_public boolean not null default false,
  add column if not exists last_activity_summary text not null default '',
  add column if not exists last_activity_actor_name text not null default '';

create table if not exists public.project_activity_log (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  project_id uuid not null references public.projects(id) on delete cascade,
  actor_user_id uuid references public.profiles(id) on delete set null,
  actor_name text not null default '',
  summary text not null,
  created_at timestamptz not null default now()
);

alter table public.project_activity_log enable row level security;

create or replace function public.project_role(target_project_id uuid)
returns text
language sql
stable
security definer
set search_path = public
as $$
  select pm.role
  from public.project_members pm
  where pm.project_id = target_project_id
    and pm.user_id = auth.uid()
  limit 1
$$;

create or replace function public.is_project_member(target_project_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select public.project_role(target_project_id) is not null
$$;

create or replace function public.can_edit_project(target_project_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select coalesce(public.project_role(target_project_id), '') in ('owner', 'editor')
$$;

create or replace function public.can_comment_project(target_project_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select coalesce(public.project_role(target_project_id), '') in ('owner', 'editor', 'commenter')
$$;

create or replace function public.is_project_owner(target_project_id uuid)
returns boolean
language sql
stable
security definer
set search_path = public
as $$
  select exists (
    select 1
    from public.projects p
    where p.id = target_project_id
      and coalesce(p.owner_user_id, p.created_by) = auth.uid()
  )
$$;

drop policy if exists "projects readable by workspace members" on public.projects;
create policy "projects readable by workspace members"
on public.projects for select
to authenticated
using (public.is_workspace_member(workspace_id) or public.is_project_member(id));

drop policy if exists "projects editable by editors" on public.projects;
create policy "projects editable by editors"
on public.projects for all
to authenticated
using (public.can_edit_workspace(workspace_id) or public.can_edit_project(id))
with check (public.can_edit_workspace(workspace_id) or public.can_edit_project(id));

drop policy if exists "project stages readable by workspace members" on public.project_stages;
create policy "project stages readable by workspace members"
on public.project_stages for select
to authenticated
using (public.is_workspace_member(workspace_id) or public.is_project_member(project_id));

drop policy if exists "project stages editable by editors" on public.project_stages;
create policy "project stages editable by editors"
on public.project_stages for all
to authenticated
using (public.can_edit_workspace(workspace_id) or public.can_edit_project(project_id))
with check (public.can_edit_workspace(workspace_id) or public.can_edit_project(project_id));

drop policy if exists "project tasks readable by workspace members" on public.project_tasks;
create policy "project tasks readable by workspace members"
on public.project_tasks for select
to authenticated
using (public.is_workspace_member(workspace_id) or public.is_project_member(project_id));

drop policy if exists "project tasks editable by editors" on public.project_tasks;
create policy "project tasks editable by editors"
on public.project_tasks for all
to authenticated
using (public.can_edit_workspace(workspace_id) or public.can_edit_project(project_id))
with check (public.can_edit_workspace(workspace_id) or public.can_edit_project(project_id));

drop policy if exists "project members readable by workspace members" on public.project_members;
create policy "project members readable by workspace members"
on public.project_members for select
to authenticated
using (
  user_id = auth.uid()
  or exists (
    select 1
    from public.projects p
    where p.id = project_members.project_id
      and (public.is_workspace_member(p.workspace_id) or public.is_project_member(p.id))
  )
);

drop policy if exists "project members managed by workspace owners" on public.project_members;
create policy "project members managed by workspace owners"
on public.project_members for all
to authenticated
using (public.is_project_owner(project_id))
with check (public.is_project_owner(project_id));

drop policy if exists "project activity readable by project members" on public.project_activity_log;
create policy "project activity readable by project members"
on public.project_activity_log for select
to authenticated
using (public.is_workspace_member(workspace_id) or public.is_project_member(project_id));

drop policy if exists "project activity insertable by editors" on public.project_activity_log;
create policy "project activity insertable by editors"
on public.project_activity_log for insert
to authenticated
with check (public.can_edit_workspace(workspace_id) or public.can_edit_project(project_id));

create or replace function public.accept_invite(invite_token text)
returns table (
  workspace_id uuid,
  project_id uuid,
  role text
)
language plpgsql
security definer
set search_path = public
as $$
declare
  matching_invite public.invites%rowtype;
begin
  if auth.uid() is null then
    raise exception 'AUTH_REQUIRED';
  end if;

  select *
  into matching_invite
  from public.invites i
  where i.token_hash = encode(digest(invite_token, 'sha256'), 'hex')
    and i.deleted_at is null
    and i.accepted_at is null
    and i.expires_at > now()
  limit 1;

  if matching_invite.id is null then
    raise exception 'INVITE_NOT_FOUND_OR_EXPIRED';
  end if;

  if matching_invite.scope_type = 'workspace' then
    insert into public.workspace_members (workspace_id, user_id, role)
    values (matching_invite.workspace_id, auth.uid(), matching_invite.role)
    on conflict (workspace_id, user_id)
    do update set role = excluded.role;
  end if;

  if matching_invite.scope_type = 'project' and matching_invite.project_id is not null then
    insert into public.project_members (project_id, user_id, role)
    values (matching_invite.project_id, auth.uid(), matching_invite.role)
    on conflict (project_id, user_id)
    do update set role = excluded.role;
  end if;

  update public.invites
  set accepted_at = now(),
      updated_at = now(),
      updated_by = auth.uid()
  where id = matching_invite.id;

  workspace_id := matching_invite.workspace_id;
  project_id := matching_invite.project_id;
  role := matching_invite.role;
  return next;
end;
$$;

grant execute on function public.project_role(uuid) to authenticated;
grant execute on function public.is_project_member(uuid) to authenticated;
grant execute on function public.can_edit_project(uuid) to authenticated;
grant execute on function public.can_comment_project(uuid) to authenticated;
grant execute on function public.is_project_owner(uuid) to authenticated;
grant execute on function public.accept_invite(text) to authenticated;

do $$
begin
  alter publication supabase_realtime add table public.project_activity_log;
exception when duplicate_object then null;
end $$;
