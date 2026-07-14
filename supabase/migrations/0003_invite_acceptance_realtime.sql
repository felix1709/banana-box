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

  insert into public.workspace_members (workspace_id, user_id, role)
  values (matching_invite.workspace_id, auth.uid(), matching_invite.role)
  on conflict (workspace_id, user_id)
  do update set role = excluded.role;

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

grant execute on function public.accept_invite(text) to authenticated;

do $$
begin
  alter publication supabase_realtime add table public.projects;
exception when duplicate_object then null;
end $$;

do $$
begin
  alter publication supabase_realtime add table public.daily_tasks;
exception when duplicate_object then null;
end $$;

do $$
begin
  alter publication supabase_realtime add table public.comments;
exception when duplicate_object then null;
end $$;

do $$
begin
  alter publication supabase_realtime add table public.notifications;
exception when duplicate_object then null;
end $$;
