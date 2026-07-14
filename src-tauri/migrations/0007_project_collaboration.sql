ALTER TABLE projects ADD COLUMN owner_user_id TEXT NOT NULL DEFAULT '';
ALTER TABLE projects ADD COLUMN is_public INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN last_activity_summary TEXT NOT NULL DEFAULT '';
ALTER TABLE projects ADD COLUMN last_activity_actor_name TEXT NOT NULL DEFAULT '';

CREATE TABLE project_activity_log (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  actor_user_id TEXT NOT NULL DEFAULT '',
  actor_name TEXT NOT NULL DEFAULT '',
  summary TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX project_activity_log_project_idx
ON project_activity_log(project_id, created_at);
