CREATE TABLE local_workspaces (
  id TEXT PRIMARY KEY NOT NULL,
  cloud_workspace_id TEXT UNIQUE,
  name TEXT NOT NULL,
  mode TEXT NOT NULL DEFAULT 'local' CHECK (mode IN ('local', 'cloud')),
  active INTEGER NOT NULL DEFAULT 0 CHECK (active IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

INSERT INTO local_workspaces (id, cloud_workspace_id, name, mode, active, created_at, updated_at)
VALUES ('local', NULL, '本地离线空间', 'local', 1, strftime('%Y-%m-%dT%H:%M:%fZ','now'), strftime('%Y-%m-%dT%H:%M:%fZ','now'));

ALTER TABLE projects ADD COLUMN local_workspace_id TEXT NOT NULL DEFAULT 'local';
ALTER TABLE projects ADD COLUMN cloud_id TEXT;
ALTER TABLE projects ADD COLUMN cloud_workspace_id TEXT;
ALTER TABLE projects ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN sync_state TEXT NOT NULL DEFAULT 'local' CHECK (sync_state IN ('local', 'synced', 'pending', 'conflict'));
ALTER TABLE projects ADD COLUMN deleted_at TEXT;

ALTER TABLE project_stages ADD COLUMN cloud_id TEXT;
ALTER TABLE project_stages ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE project_stages ADD COLUMN sync_state TEXT NOT NULL DEFAULT 'local' CHECK (sync_state IN ('local', 'synced', 'pending', 'conflict'));
ALTER TABLE project_stages ADD COLUMN deleted_at TEXT;

ALTER TABLE daily_task_days ADD COLUMN local_workspace_id TEXT NOT NULL DEFAULT 'local';
ALTER TABLE daily_task_days ADD COLUMN cloud_id TEXT;
ALTER TABLE daily_task_days ADD COLUMN cloud_workspace_id TEXT;
ALTER TABLE daily_task_days ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE daily_task_days ADD COLUMN sync_state TEXT NOT NULL DEFAULT 'local' CHECK (sync_state IN ('local', 'synced', 'pending', 'conflict'));
ALTER TABLE daily_task_days ADD COLUMN deleted_at TEXT;

ALTER TABLE daily_task_groups ADD COLUMN cloud_id TEXT;
ALTER TABLE daily_task_groups ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE daily_task_groups ADD COLUMN sync_state TEXT NOT NULL DEFAULT 'local' CHECK (sync_state IN ('local', 'synced', 'pending', 'conflict'));
ALTER TABLE daily_task_groups ADD COLUMN deleted_at TEXT;

ALTER TABLE daily_tasks ADD COLUMN cloud_id TEXT;
ALTER TABLE daily_tasks ADD COLUMN revision INTEGER NOT NULL DEFAULT 0;
ALTER TABLE daily_tasks ADD COLUMN sync_state TEXT NOT NULL DEFAULT 'local' CHECK (sync_state IN ('local', 'synced', 'pending', 'conflict'));
ALTER TABLE daily_tasks ADD COLUMN deleted_at TEXT;

CREATE TABLE sync_outbox (
  id TEXT PRIMARY KEY NOT NULL,
  local_workspace_id TEXT NOT NULL,
  cloud_workspace_id TEXT,
  target_table TEXT NOT NULL,
  target_id TEXT NOT NULL,
  operation TEXT NOT NULL CHECK (operation IN ('insert', 'update', 'delete')),
  payload_json TEXT NOT NULL,
  base_revision INTEGER,
  retry_count INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE sync_cursors (
  local_workspace_id TEXT NOT NULL,
  cloud_workspace_id TEXT NOT NULL,
  table_name TEXT NOT NULL,
  cursor_updated_at TEXT,
  cursor_revision INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL,
  PRIMARY KEY (local_workspace_id, table_name)
);

CREATE TABLE local_device_bindings (
  id TEXT PRIMARY KEY NOT NULL,
  local_workspace_id TEXT NOT NULL,
  cloud_workspace_id TEXT,
  target_table TEXT NOT NULL,
  target_id TEXT NOT NULL,
  binding_kind TEXT NOT NULL CHECK (binding_kind IN ('file_path', 'asset_path')),
  local_value TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX projects_local_workspace_idx ON projects(local_workspace_id, deleted_at, archived);
CREATE INDEX projects_cloud_id_idx ON projects(cloud_id);
CREATE INDEX project_stages_cloud_id_idx ON project_stages(cloud_id);
CREATE INDEX daily_task_days_local_workspace_idx ON daily_task_days(local_workspace_id, local_date, deleted_at);
CREATE INDEX daily_task_days_cloud_id_idx ON daily_task_days(cloud_id);
CREATE INDEX daily_task_groups_cloud_id_idx ON daily_task_groups(cloud_id);
CREATE INDEX daily_tasks_cloud_id_idx ON daily_tasks(cloud_id);
CREATE INDEX sync_outbox_workspace_created_idx ON sync_outbox(local_workspace_id, created_at);
CREATE INDEX sync_cursors_workspace_idx ON sync_cursors(local_workspace_id, cloud_workspace_id);
CREATE INDEX local_device_bindings_target_idx ON local_device_bindings(local_workspace_id, target_table, target_id);
