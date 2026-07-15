create or replace function public.update_own_profile_display_name(new_display_name text)
returns public.profiles
language plpgsql
security definer
set search_path = public
as $$
declare
  normalized_name text;
  updated_profile public.profiles%rowtype;
begin
  if auth.uid() is null then
    raise exception 'AUTH_REQUIRED';
  end if;

  normalized_name := nullif(trim(new_display_name), '');
  if normalized_name is null then
    raise exception 'DISPLAY_NAME_REQUIRED';
  end if;

  update public.profiles
  set display_name = normalized_name,
      updated_at = now()
  where id = auth.uid()
  returning *
  into updated_profile;

  if updated_profile.id is null then
    raise exception 'PROFILE_NOT_FOUND';
  end if;

  return updated_profile;
end;
$$;

grant execute on function public.update_own_profile_display_name(text) to authenticated;
