alter table public.daily_tasks
  add column if not exists reminder_time text not null default '',
  add column if not exists reminder_content text not null default '';
