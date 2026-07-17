drop policy if exists "shared prompts editable by authors" on public.shared_prompts;
drop policy if exists "shared prompts editable by authors and admin" on public.shared_prompts;

-- Admin moderation uses deleted_at soft deletes from the application.
create policy "shared prompts editable by authors and admin"
on public.shared_prompts for update
to authenticated
using (
  created_by = auth.uid()
  or exists (
    select 1
    from public.profiles p
    where p.id = auth.uid()
      and lower(p.email) = '000001@banana-box.local'
  )
)
with check (
  updated_by = auth.uid()
  and (
    created_by = auth.uid()
    or exists (
      select 1
      from public.profiles p
      where p.id = auth.uid()
        and lower(p.email) = '000001@banana-box.local'
    )
  )
);
