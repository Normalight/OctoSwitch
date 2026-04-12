-- Add cache token tracking columns for Anthropic-style prompt caching
ALTER TABLE request_logs ADD COLUMN cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE request_logs ADD COLUMN cache_read_input_tokens INTEGER NOT NULL DEFAULT 0;
