ALTER TABLE daily_tasks ADD COLUMN reminder_time TEXT NOT NULL DEFAULT '';
ALTER TABLE daily_tasks ADD COLUMN reminder_content TEXT NOT NULL DEFAULT '';
