CREATE TABLE projects_rebuilt (
  id TEXT PRIMARY KEY NOT NULL,
  code TEXT COLLATE NOCASE NOT NULL CHECK (length(trim(code)) > 0),
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

INSERT INTO projects_rebuilt (
  id, code, version, name, file_path, release_date, main_stage_key, archived, created_at, updated_at
)
SELECT id, code, version, name, file_path, release_date, main_stage_key, archived, created_at, updated_at
FROM projects;

DROP TABLE projects;
ALTER TABLE projects_rebuilt RENAME TO projects;

CREATE INDEX idx_projects_main_stage ON projects(main_stage_key, archived);
CREATE INDEX idx_projects_release_date ON projects(release_date);
