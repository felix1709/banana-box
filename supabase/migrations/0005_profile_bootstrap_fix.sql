create or replace function public.bootstrap_user_workspace(
  workspace_name text,
  user_email text,
  user_display_name text
)
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  current_user_id uuid;
  profile_row public.profiles%rowtype;
  workspace_row public.workspaces%rowtype;
  safe_email text;
  safe_display_name text;
begin
  current_user_id := auth.uid();

  if current_user_id is null then
    raise exception 'AUTH_REQUIRED';
  end if;

  safe_email := coalesce(nullif(trim(user_email), ''), 'unknown@example.invalid');
  safe_display_name := coalesce(nullif(trim(user_display_name), ''), split_part(safe_email, '@', 1), 'Banana Box User');

  insert into public.profiles (id, email, display_name, avatar_url)
  values (current_user_id, safe_email, safe_display_name, null)
  on conflict (id) do update
    set email = excluded.email,
        display_name = excluded.display_name,
        updated_at = now()
  returning * into profile_row;

  select w.*
  into workspace_row
  from public.workspaces w
  join public.workspace_members wm on wm.workspace_id = w.id
  where wm.user_id = current_user_id
  order by wm.created_at
  limit 1;

  if workspace_row.id is null then
    insert into public.workspaces (name, owner_id)
    values (coalesce(nullif(trim(workspace_name), ''), 'Personal Workspace'), current_user_id)
    returning * into workspace_row;

    insert into public.workspace_members (workspace_id, user_id, role)
    values (workspace_row.id, current_user_id, 'owner')
    on conflict (workspace_id, user_id)
    do update set role = excluded.role;
  elsif nullif(trim(workspace_name), '') is not null and position('?' in workspace_row.name) > 0 then
    update public.workspaces
    set name = trim(workspace_name),
        updated_at = now()
    where id = workspace_row.id
      and owner_id = current_user_id
    returning * into workspace_row;
  end if;

  return jsonb_build_object(
    'profile', to_jsonb(profile_row),
    'workspace', to_jsonb(workspace_row)
  );
end;
$$;

grant execute on function public.bootstrap_user_workspace(text, text, text) to authenticated;
