-- Add index on request_logs.provider_id for list_request_logs_in_range JOIN performance
CREATE INDEX IF NOT EXISTS idx_request_logs_provider_id ON request_logs (provider_id);

-- Add index on model_bindings.provider_id for cascade deletes and provider lookups
CREATE INDEX IF NOT EXISTS idx_model_bindings_provider_id ON model_bindings (provider_id);
