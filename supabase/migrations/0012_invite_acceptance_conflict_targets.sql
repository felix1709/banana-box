create schema if not exists extensions;
create extension if not exists pgcrypto with schema extensions;

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

  select i.*
  into matching_invite
  from public.invites i
  where i.token_hash = encode(extensions.digest(invite_token, 'sha256'), 'hex')
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
    on conflict on constraint workspace_members_pkey
    do update set role = excluded.role;
  end if;

  if matching_invite.scope_type = 'project' and matching_invite.project_id is not null then
    insert into public.project_members (project_id, user_id, role)
    values (matching_invite.project_id, auth.uid(), matching_invite.role)
    on conflict on constraint project_members_pkey
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

create or replace function public.accept_invite_by_id(invite_id uuid)
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

  select i.*
  into matching_invite
  from public.invites i
  where i.id = invite_id
    and i.deleted_at is null
    and i.accepted_at is null
    and i.expires_at > now()
    and exists (
      select 1
      from public.notifications n
      where n.target_type = 'project_invite'
        and n.target_id = i.id
        and n.recipient_user_id = auth.uid()
        and n.deleted_at is null
        and n.read_at is null
    )
  limit 1;

  if matching_invite.id is null then
    raise exception 'INVITE_NOT_FOUND_OR_EXPIRED';
  end if;

  if matching_invite.scope_type = 'project' and matching_invite.project_id is not null then
    insert into public.project_members (project_id, user_id, role)
    values (matching_invite.project_id, auth.uid(), matching_invite.role)
    on conflict on constraint project_members_pkey
    do update set role = excluded.role;
  end if;

  if matching_invite.scope_type = 'workspace' then
    insert into public.workspace_members (workspace_id, user_id, role)
    values (matching_invite.workspace_id, auth.uid(), matching_invite.role)
    on conflict on constraint workspace_members_pkey
    do update set role = excluded.role;
  end if;

  update public.invites
  set accepted_at = now(),
      updated_at = now(),
      updated_by = auth.uid()
  where id = matching_invite.id;

  update public.notifications
  set read_at = now(),
      updated_at = now(),
      updated_by = auth.uid()
  where target_type = 'project_invite'
    and target_id = matching_invite.id
    and recipient_user_id = auth.uid();

  workspace_id := matching_invite.workspace_id;
  project_id := matching_invite.project_id;
  role := matching_invite.role;
  return next;
end;
$$;

grant execute on function public.accept_invite_by_id(uuid) to authenticated;
