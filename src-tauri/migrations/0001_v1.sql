CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE ai_providers (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('reverse-image', 'storyboard')),
  display_name TEXT NOT NULL,
  base_url TEXT NOT NULL,
  models_url TEXT NOT NULL,
  chat_completions_url TEXT NOT NULL,
  default_model TEXT,
  available_models_json TEXT NOT NULL DEFAULT '[]',
  probed_model TEXT,
  structured_mode TEXT CHECK (structured_mode IS NULL OR structured_mode IN ('json_schema', 'strict_json')),
  interactive_compatible INTEGER CHECK (interactive_compatible IS NULL OR interactive_compatible IN (0, 1)),
  bound_host TEXT,
  needs_credentials INTEGER NOT NULL DEFAULT 1 CHECK (needs_credentials IN (0, 1)),
  credential_ref TEXT UNIQUE,
  config_revision INTEGER NOT NULL DEFAULT 1 CHECK (config_revision >= 1),
  capability_revision INTEGER NOT NULL DEFAULT 1 CHECK (capability_revision >= 1),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE credential_cleanup (
  credential_ref TEXT PRIMARY KEY,
  reason TEXT NOT NULL CHECK (reason IN ('candidate', 'retired')),
  created_at TEXT NOT NULL
);

CREATE TABLE projects (
  id TEXT PRIMARY KEY NOT NULL,
  code TEXT COLLATE NOCASE NOT NULL UNIQUE CHECK (length(trim(code)) > 0),
  version TEXT NOT NULL CHECK (length(trim(version)) > 0),
  name TEXT NOT NULL CHECK (length(trim(name)) > 0),
  file_path TEXT NOT NULL CHECK (length(trim(file_path)) > 0),
  release_date TEXT NOT NULL CHECK (length(release_date) = 10),
  main_stage_key TEXT NOT NULL CHECK (main_stage_key IN (
    'storyboard', 'first_cut', 'refinement', 'middle_cut',
    'effects', 'art_titles', 'music', 'final_composite'
  )),
  archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE project_stages (
  id TEXT PRIMARY KEY NOT NULL,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  stage_key TEXT NOT NULL CHECK (stage_key IN (
    'storyboard', 'first_cut', 'refinement', 'middle_cut',
    'effects', 'art_titles', 'music', 'final_composite'
  )),
  position INTEGER NOT NULL CHECK (position BETWEEN 0 AND 7),
  start_date TEXT NOT NULL CHECK (length(start_date) = 10),
  end_date TEXT NOT NULL CHECK (length(end_date) = 10),
  progress INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
  updated_at TEXT NOT NULL,
  UNIQUE (project_id, stage_key),
  UNIQUE (project_id, position),
  CHECK (start_date <= end_date)
);

CREATE INDEX idx_projects_main_stage ON projects(main_stage_key, archived);
CREATE INDEX idx_projects_release_date ON projects(release_date);
CREATE INDEX idx_project_stages_project ON project_stages(project_id, position);

CREATE TABLE daily_task_days (
  id TEXT PRIMARY KEY NOT NULL,
  local_date TEXT NOT NULL UNIQUE CHECK (length(local_date) = 10),
  settled_at TEXT,
  report_snapshot TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE daily_task_groups (
  id TEXT PRIMARY KEY NOT NULL,
  day_id TEXT NOT NULL REFERENCES daily_task_days(id) ON DELETE CASCADE,
  code TEXT COLLATE NOCASE NOT NULL CHECK (length(trim(code)) > 0),
  project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (day_id, code),
  UNIQUE (day_id, position)
);

CREATE TABLE daily_tasks (
  id TEXT PRIMARY KEY NOT NULL,
  group_id TEXT NOT NULL REFERENCES daily_task_groups(id) ON DELETE CASCADE,
  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
  progress INTEGER NOT NULL DEFAULT 0 CHECK (progress BETWEEN 0 AND 100),
  note TEXT NOT NULL DEFAULT '',
  invested_minutes INTEGER NOT NULL DEFAULT 0 CHECK (invested_minutes >= 0),
  position INTEGER NOT NULL CHECK (position >= 0),
  source_task_id TEXT REFERENCES daily_tasks(id) ON DELETE SET NULL,
  carry_target_date TEXT,
  source_snapshot_hash TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE (source_task_id, carry_target_date)
);

CREATE INDEX idx_daily_groups_day_position
  ON daily_task_groups(day_id, position);
CREATE INDEX idx_daily_tasks_group_position
  ON daily_tasks(group_id, position, id);
CREATE INDEX idx_daily_tasks_carry_source
  ON daily_tasks(source_task_id, carry_target_date);

CREATE TABLE skills (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('builtin', 'local')),
  current_version_id TEXT REFERENCES skill_versions(id) ON DELETE SET NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE skill_versions (
  id TEXT PRIMARY KEY,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  display_version TEXT NOT NULL,
  protocol_version INTEGER NOT NULL,
  content_hash TEXT NOT NULL,
  manifest_json TEXT NOT NULL,
  files_json TEXT NOT NULL,
  imported_at TEXT NOT NULL,
  UNIQUE(skill_id, content_hash)
);

CREATE TABLE storyboard_threads (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  provider_id TEXT REFERENCES ai_providers(id) ON DELETE SET NULL,
  model TEXT,
  skill_id TEXT REFERENCES skills(id) ON DELETE SET NULL,
  workflow_state TEXT NOT NULL DEFAULT 'awaiting_story' CHECK (workflow_state IN ('awaiting_story','analyzing_context','collecting_settings','confirming_settings','drafting_storyboard','confirming_storyboard','generating_output','free_chat')),
  workflow_protocol_version INTEGER NOT NULL DEFAULT 1,
  workflow_revision INTEGER NOT NULL DEFAULT 0 CHECK (workflow_revision >= 0),
  request_config_revision INTEGER NOT NULL DEFAULT 0 CHECK (request_config_revision >= 0),
  request_state_revision INTEGER NOT NULL DEFAULT 0 CHECK (request_state_revision >= 0),
  workflow_context_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE agent_requests (
  id TEXT PRIMARY KEY,
  thread_id TEXT NOT NULL REFERENCES storyboard_threads(id) ON DELETE CASCADE,
  source_request_id TEXT REFERENCES agent_requests(id) ON DELETE SET NULL,
  provider_id TEXT NOT NULL REFERENCES ai_providers(id) ON DELETE RESTRICT,
  model TEXT NOT NULL,
  skill_version_id TEXT REFERENCES skill_versions(id) ON DELETE RESTRICT,
  snapshot_json TEXT NOT NULL,
  expected_workflow_revision INTEGER NOT NULL CHECK (expected_workflow_revision >= 0),
  expected_workflow_state TEXT NOT NULL CHECK (expected_workflow_state IN ('awaiting_story','analyzing_context','collecting_settings','confirming_settings','drafting_storyboard','confirming_storyboard','generating_output','free_chat')),
  expected_latest_message_position INTEGER NOT NULL,
  expected_request_config_revision INTEGER NOT NULL CHECK (expected_request_config_revision >= 0),
  last_persisted_sequence INTEGER NOT NULL DEFAULT 0 CHECK (last_persisted_sequence >= 0),
  input_start_position INTEGER NOT NULL,
  input_end_position INTEGER NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('streaming','completed','cancelled','failed','interrupted')),
  error_code TEXT,
  created_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE UNIQUE INDEX one_active_agent_request_per_thread
ON agent_requests(thread_id) WHERE status = 'streaming';

CREATE TABLE storyboard_messages (
  id TEXT PRIMARY KEY,
  thread_id TEXT NOT NULL REFERENCES storyboard_threads(id) ON DELETE CASCADE,
  request_id TEXT REFERENCES agent_requests(id) ON DELETE SET NULL,
  responds_to_message_id TEXT REFERENCES storyboard_messages(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('user','assistant')),
  message_type TEXT NOT NULL CHECK (message_type IN ('user_text','user_choices','user_confirmation','assistant_markdown','analysis_result','choice_prompt','confirmation','final_output')),
  content_markdown TEXT NOT NULL DEFAULT '',
  structured_json TEXT,
  status TEXT NOT NULL CHECK (status IN ('complete','streaming','cancelled','failed','interrupted')),
  created_at TEXT NOT NULL,
  CHECK (
    (role = 'user' AND message_type IN ('user_text','user_choices','user_confirmation')) OR
    (role = 'assistant' AND message_type IN ('assistant_markdown','analysis_result','choice_prompt','confirmation','final_output'))
  ),
  CHECK (
    (message_type IN ('user_choices','user_confirmation') AND responds_to_message_id IS NOT NULL AND structured_json IS NOT NULL) OR
    (message_type NOT IN ('user_choices','user_confirmation') AND responds_to_message_id IS NULL)
  ),
  UNIQUE(thread_id, position),
  UNIQUE(responds_to_message_id)
);

CREATE TABLE storyboard_message_blocks (
  id TEXT PRIMARY KEY,
  message_id TEXT NOT NULL REFERENCES storyboard_messages(id) ON DELETE CASCADE,
  block_key TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('storyboard','video','scene_reference','shot')),
  title TEXT NOT NULL,
  markdown TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE(message_id, position),
  UNIQUE(message_id, block_key)
);

CREATE TABLE reminder_log (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  local_date TEXT NOT NULL,
  phase TEXT NOT NULL CHECK (phase IN ('initial','snooze')),
  state TEXT NOT NULL CHECK (state IN ('pending','shown','hidden','actioned','cancelled')),
  delivery_id TEXT NOT NULL,
  attempt_token TEXT,
  owner_id TEXT,
  lease_expires_at TEXT,
  attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count BETWEEN 0 AND 3),
  claimed_at TEXT,
  shown_at TEXT,
  acknowledged_at TEXT,
  snoozed_until TEXT,
  unread INTEGER NOT NULL DEFAULT 0 CHECK (unread IN (0, 1)),
  UNIQUE(kind, local_date, phase)
);
