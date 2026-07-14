create or replace function public.ensure_personal_workspace(workspace_name text)
returns public.workspaces
language plpgsql
security definer
set search_path = public
as $$
declare
  existing_workspace public.workspaces%rowtype;
begin
  if auth.uid() is null then
    raise exception 'AUTH_REQUIRED';
  end if;

  select w.*
  into existing_workspace
  from public.workspaces w
  join public.workspace_members wm on wm.workspace_id = w.id
  where wm.user_id = auth.uid()
  order by wm.created_at
  limit 1;

  if existing_workspace.id is not null then
    return existing_workspace;
  end if;

  insert into public.workspaces (name, owner_id)
  values (coalesce(nullif(trim(workspace_name), ''), '个人空间'), auth.uid())
  returning * into existing_workspace;

  insert into public.workspace_members (workspace_id, user_id, role)
  values (existing_workspace.id, auth.uid(), 'owner')
  on conflict (workspace_id, user_id)
  do update set role = excluded.role;

  return existing_workspace;
end;
$$;

grant execute on function public.ensure_personal_workspace(text) to authenticated;

drop policy if exists "owners can read owned workspaces" on public.workspaces;
create policy "owners can read owned workspaces"
on public.workspaces for select
to authenticated
using (owner_id = auth.uid());
