CREATE TABLE cloud_config (
  id TEXT PRIMARY KEY NOT NULL CHECK (id = 'default'),
  supabase_url TEXT NOT NULL,
  anon_key TEXT NOT NULL,
  cloud_enabled INTEGER NOT NULL DEFAULT 0 CHECK (cloud_enabled IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
