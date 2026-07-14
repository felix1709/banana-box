create extension if not exists pgcrypto;

create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  email text not null,
  display_name text not null,
  avatar_url text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.workspaces (
  id uuid primary key default gen_random_uuid(),
  name text not null,
  owner_id uuid not null references public.profiles(id) on delete cascade,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.workspace_members (
  workspace_id uuid not null references public.workspaces(id) on delete cascade,
  user_id uuid not null references public.profiles(id) on delete cascade,
  role text not null check (role in ('owner', 'editor', 'commenter', 'viewer')),
  created_at timestamptz not null default now(),
  primary key (workspace_id, user_id)
);

alter table public.profiles enable row level security;
alter table public.workspaces enable row level security;
alter table public.workspace_members enable row level security;

drop policy if exists "profiles can read own profile" on public.profiles;
create policy "profiles can read own profile"
on public.profiles for select
to authenticated
using (id = auth.uid());

drop policy if exists "profiles can upsert own profile" on public.profiles;
create policy "profiles can upsert own profile"
on public.profiles for insert
to authenticated
with check (id = auth.uid());

drop policy if exists "profiles can update own profile" on public.profiles;
create policy "profiles can update own profile"
on public.profiles for update
to authenticated
using (id = auth.uid())
with check (id = auth.uid());

drop policy if exists "members can read their workspaces" on public.workspaces;
create policy "members can read their workspaces"
on public.workspaces for select
to authenticated
using (
  exists (
    select 1 from public.workspace_members wm
    where wm.workspace_id = workspaces.id
      and wm.user_id = auth.uid()
  )
);

drop policy if exists "users can create owned workspaces" on public.workspaces;
create policy "users can create owned workspaces"
on public.workspaces for insert
to authenticated
with check (owner_id = auth.uid());

drop policy if exists "members can read workspace memberships" on public.workspace_members;
create policy "members can read workspace memberships"
on public.workspace_members for select
to authenticated
using (
  exists (
    select 1 from public.workspace_members own_membership
    where own_membership.workspace_id = workspace_members.workspace_id
      and own_membership.user_id = auth.uid()
  )
);

drop policy if exists "owners can create their own initial membership" on public.workspace_members;
create policy "owners can create their own initial membership"
on public.workspace_members for insert
to authenticated
with check (
  user_id = auth.uid()
  and role = 'owner'
  and exists (
    select 1 from public.workspaces w
    where w.id = workspace_members.workspace_id
      and w.owner_id = auth.uid()
  )
);

create index if not exists workspace_members_user_id_idx
on public.workspace_members(user_id);
