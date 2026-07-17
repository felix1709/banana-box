drop policy if exists "comments readable by workspace members" on public.comments;
create policy "comments readable by project collaborators"
on public.comments for select
to authenticated
using (
  public.is_workspace_member(workspace_id)
  or (
    target_type = 'project'
    and public.can_comment_project(target_id)
  )
);

drop policy if exists "comments insertable by commenters" on public.comments;
create policy "comments insertable by project collaborators"
on public.comments for insert
to authenticated
with check (
  created_by = auth.uid()
  and updated_by = auth.uid()
  and (
    public.can_comment_workspace(workspace_id)
    or (
      target_type = 'project'
      and public.can_comment_project(target_id)
    )
  )
);

drop policy if exists "comments editable by authors" on public.comments;
create policy "comments editable by project collaborators"
on public.comments for update
to authenticated
using (
  created_by = auth.uid()
  and (
    public.can_comment_workspace(workspace_id)
    or (
      target_type = 'project'
      and public.can_comment_project(target_id)
    )
  )
)
with check (
  created_by = auth.uid()
  and (
    public.can_comment_workspace(workspace_id)
    or (
      target_type = 'project'
      and public.can_comment_project(target_id)
    )
  )
);

drop policy if exists "comment mentions readable by workspace members" on public.comment_mentions;
create policy "comment mentions readable by project collaborators"
on public.comment_mentions for select
to authenticated
using (
  exists (
    select 1
    from public.comments c
    where c.id = comment_mentions.comment_id
      and (
        public.is_workspace_member(c.workspace_id)
        or (
          c.target_type = 'project'
          and public.can_comment_project(c.target_id)
        )
      )
  )
);

drop policy if exists "notifications insertable by workspace collaborators" on public.notifications;
create policy "notifications insertable by project collaborators"
on public.notifications for insert
to authenticated
with check (
  actor_user_id = auth.uid()
  and created_by = auth.uid()
  and updated_by = auth.uid()
  and (
    public.can_comment_workspace(workspace_id)
    or (
      target_type = 'project'
      and public.can_comment_project(target_id)
    )
    or (
      target_type = 'project_invite'
      and exists (
        select 1
        from public.invites i
        where i.id = notifications.target_id
          and i.project_id is not null
          and public.is_project_owner(i.project_id)
      )
    )
    or (
      target_type = 'project_schedule_request'
      and exists (
        select 1
        from public.project_schedule_change_requests r
        where r.id = notifications.target_id
          and r.requested_by = auth.uid()
          and public.can_comment_project(r.project_id)
      )
    )
  )
);
