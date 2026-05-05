PRAGMA foreign_keys=OFF;

CREATE TABLE model_bindings_new (
  id TEXT PRIMARY KEY,
  model_name TEXT NOT NULL UNIQUE,
  provider_id TEXT NOT NULL,
  upstream_model_name TEXT NOT NULL,
  rpm_limit INTEGER NULL,
  tpm_limit INTEGER NULL,
  is_enabled INTEGER NOT NULL DEFAULT 1,
  group_id TEXT,
  FOREIGN KEY(provider_id) REFERENCES providers(id),
  FOREIGN KEY(group_id) REFERENCES model_groups(id)
);

INSERT INTO model_bindings_new (
  id,
  model_name,
  provider_id,
  upstream_model_name,
  rpm_limit,
  tpm_limit,
  is_enabled,
  group_id
)
SELECT
  id,
  model_name,
  provider_id,
  upstream_model_name,
  rpm_limit,
  tpm_limit,
  is_enabled,
  group_id
FROM model_bindings;

DROP TABLE model_bindings;
ALTER TABLE model_bindings_new RENAME TO model_bindings;

CREATE TABLE request_logs_new (
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
  created_at TEXT NOT NULL
);

INSERT INTO request_logs_new (
  id,
  model_name,
  group_name,
  provider_id,
  provider_name,
  status_code,
  latency_ms,
  input_tokens,
  output_tokens,
  cache_creation_input_tokens,
  cache_read_input_tokens,
  created_at
)
SELECT
  id,
  model_name,
  group_name,
  provider_id,
  provider_name,
  status_code,
  latency_ms,
  input_tokens,
  output_tokens,
  cache_creation_input_tokens,
  cache_read_input_tokens,
  created_at
FROM request_logs;

DROP TABLE request_logs;
ALTER TABLE request_logs_new RENAME TO request_logs;
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs (created_at);

PRAGMA foreign_keys=ON;
