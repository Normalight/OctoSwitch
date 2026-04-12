-- 初始 schema：供应商、模型分组/绑定、请求日志、Copilot 多账号
CREATE TABLE IF NOT EXISTS providers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  base_url TEXT NOT NULL,
  api_key_ref TEXT NOT NULL,
  timeout_ms INTEGER NOT NULL DEFAULT 60000,
  max_retries INTEGER NOT NULL DEFAULT 10,
  is_enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  api_format TEXT,
  auth_mode TEXT NOT NULL DEFAULT 'bearer',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS model_groups (
  id TEXT PRIMARY KEY,
  alias TEXT NOT NULL UNIQUE,
  active_binding_id TEXT,
  is_enabled INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS model_bindings (
  id TEXT PRIMARY KEY,
  model_name TEXT NOT NULL UNIQUE,
  provider_id TEXT NOT NULL,
  upstream_model_name TEXT NOT NULL,
  input_price_per_1m REAL NOT NULL DEFAULT 0,
  output_price_per_1m REAL NOT NULL DEFAULT 0,
  rpm_limit INTEGER NULL,
  tpm_limit INTEGER NULL,
  is_enabled INTEGER NOT NULL DEFAULT 1,
  group_id TEXT,
  FOREIGN KEY(provider_id) REFERENCES providers(id),
  FOREIGN KEY(group_id) REFERENCES model_groups(id)
);

CREATE TABLE IF NOT EXISTS model_group_members (
  group_id TEXT NOT NULL,
  binding_id TEXT NOT NULL,
  PRIMARY KEY (group_id, binding_id),
  FOREIGN KEY (group_id) REFERENCES model_groups(id) ON DELETE CASCADE,
  FOREIGN KEY (binding_id) REFERENCES model_bindings(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS request_logs (
  id TEXT PRIMARY KEY,
  model_name TEXT NOT NULL,
  group_name TEXT,
  provider_id TEXT NOT NULL,
  provider_name TEXT,
  status_code INTEGER NOT NULL,
  latency_ms INTEGER NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0,
  cache_read_input_tokens INTEGER NOT NULL DEFAULT 0,
  total_cost REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs (created_at);

CREATE TABLE IF NOT EXISTS copilot_accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_id TEXT NOT NULL UNIQUE,
  github_user_id INTEGER,
  github_login TEXT NOT NULL,
  avatar_url TEXT,
  github_token TEXT,
  copilot_token TEXT,
  token_expires_at TEXT,
  account_type TEXT NOT NULL DEFAULT 'individual',
  api_endpoint TEXT DEFAULT 'https://api.githubcopilot.com',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY);
INSERT OR IGNORE INTO schema_version (version) VALUES (1);
