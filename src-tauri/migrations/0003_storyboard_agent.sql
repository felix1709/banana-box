ALTER TABLE ai_providers
  ADD COLUMN temperature REAL NOT NULL DEFAULT 0.7
  CHECK (temperature >= 0.0 AND temperature <= 2.0);

ALTER TABLE ai_providers
  ADD COLUMN context_window_tokens INTEGER NOT NULL DEFAULT 16000
  CHECK (context_window_tokens BETWEEN 512 AND 128000);

CREATE TABLE storyboard_thread_skills (
  thread_id TEXT NOT NULL REFERENCES storyboard_threads(id) ON DELETE CASCADE,
  skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
  skill_version_id TEXT NOT NULL REFERENCES skill_versions(id) ON DELETE RESTRICT,
  enabled_at TEXT NOT NULL,
  PRIMARY KEY (thread_id, skill_id)
);

CREATE INDEX idx_storyboard_thread_skills_thread
  ON storyboard_thread_skills(thread_id, enabled_at);
