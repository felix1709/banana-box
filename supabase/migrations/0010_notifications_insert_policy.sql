drop policy if exists "notifications insertable by workspace collaborators" on public.notifications;
create policy "notifications insertable by workspace collaborators"
on public.notifications for insert
to authenticated
with check (
  public.can_comment_workspace(workspace_id)
  and actor_user_id = auth.uid()
  and created_by = auth.uid()
  and updated_by = auth.uid()
);
