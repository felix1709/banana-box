create table if not exists public.shared_prompts (
  id uuid primary key default gen_random_uuid(),
  title text not null,
  title_key text not null,
  content text not null,
  tags text[] not null default '{}',
  image_ref text,
  created_by uuid not null references public.profiles(id),
  updated_by uuid not null references public.profiles(id),
  created_by_name text not null default '',
  revision integer not null default 0,
  deleted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.user_prompt_refs (
  user_id uuid not null references public.profiles(id) on delete cascade,
  shared_prompt_id uuid not null references public.shared_prompts(id) on delete cascade,
  local_prompt_id text,
  created_at timestamptz not null default now(),
  primary key (user_id, shared_prompt_id)
);

create unique index if not exists shared_prompts_title_key_unique
on public.shared_prompts(title_key)
where deleted_at is null;

create index if not exists shared_prompts_updated_idx
on public.shared_prompts(updated_at desc)
where deleted_at is null;

alter table public.shared_prompts enable row level security;
alter table public.user_prompt_refs enable row level security;

drop policy if exists "shared prompts readable by authenticated users" on public.shared_prompts;
create policy "shared prompts readable by authenticated users"
on public.shared_prompts for select
to authenticated
using (auth.uid() is not null);

drop policy if exists "shared prompts insertable by authenticated users" on public.shared_prompts;
create policy "shared prompts insertable by authenticated users"
on public.shared_prompts for insert
to authenticated
with check (
  created_by = auth.uid()
  and updated_by = auth.uid()
);

drop policy if exists "shared prompts editable by authors" on public.shared_prompts;
create policy "shared prompts editable by authors"
on public.shared_prompts for update
to authenticated
using (created_by = auth.uid())
with check (
  created_by = auth.uid()
  and updated_by = auth.uid()
);

drop policy if exists "shared prompt refs owned by users" on public.user_prompt_refs;
create policy "shared prompt refs owned by users"
on public.user_prompt_refs for all
to authenticated
using (user_id = auth.uid())
with check (user_id = auth.uid());

do $$
begin
  alter publication supabase_realtime add table public.shared_prompts;
exception when duplicate_object then null;
end $$;
