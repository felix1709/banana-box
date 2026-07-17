create table if not exists public.project_schedule_change_requests (
  id uuid primary key default gen_random_uuid(),
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  project_id uuid not null references public.projects(id) on delete cascade,
  stage_key text not null,
  requested_start_date date not null,
  requested_end_date date not null,
  reason text not null,
  status text not null default 'pending' check (status in ('pending', 'approved', 'rejected')),
  requested_by uuid not null references public.profiles(id),
  decided_by uuid references public.profiles(id),
  decision_note text not null default '',
  decided_at timestamptz,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

alter table public.project_schedule_change_requests enable row level security;

create index if not exists project_schedule_change_requests_project_idx
on public.project_schedule_change_requests(project_id, status, created_at);

create index if not exists project_schedule_change_requests_recipient_idx
on public.project_schedule_change_requests(requested_by, status, created_at);

drop policy if exists "project schedule requests readable by project members" on public.project_schedule_change_requests;
create policy "project schedule requests readable by project members"
on public.project_schedule_change_requests for select
to authenticated
using (
  public.is_workspace_member(workspace_id)
  or public.is_project_member(project_id)
  or requested_by = auth.uid()
);

drop policy if exists "project schedule requests insertable by project members" on public.project_schedule_change_requests;
create policy "project schedule requests insertable by project members"
on public.project_schedule_change_requests for insert
to authenticated
with check (
  status = 'pending'
  and requested_by = auth.uid()
  and created_by = auth.uid()
  and updated_by = auth.uid()
  and public.can_comment_project(project_id)
);

drop policy if exists "project schedule requests decidable by project owners" on public.project_schedule_change_requests;
create policy "project schedule requests decidable by project owners"
on public.project_schedule_change_requests for update
to authenticated
using (public.is_project_owner(project_id))
with check (public.is_project_owner(project_id));

do $$
begin
  alter publication supabase_realtime add table public.project_schedule_change_requests;
exception when duplicate_object then null;
end $$;
