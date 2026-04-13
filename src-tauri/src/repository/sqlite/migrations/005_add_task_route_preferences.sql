CREATE TABLE IF NOT EXISTS task_route_preferences (
  id TEXT PRIMARY KEY,
  task_kind TEXT NOT NULL UNIQUE,
  target_group TEXT NOT NULL,
  target_member TEXT,
  prompt_template TEXT,
  is_enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0
);
