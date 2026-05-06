import { z } from 'zod';

export const ProviderSchema = z.object({
  id: z.string(),
  name: z.string(),
  base_url: z.string(),
  api_key_ref: z.string(),
  timeout_ms: z.number(),
  max_retries: z.number(),
  is_enabled: z.boolean(),
  sort_order: z.number(),
  api_format: z.enum(['anthropic', 'openai_chat', 'openai_responses']).nullable().optional(),
});

export const ModelBindingSchema = z.object({
  id: z.string(),
  model_name: z.string(),
  provider_id: z.string(),
  upstream_model_name: z.string(),
  rpm_limit: z.number().nullable(),
  tpm_limit: z.number().nullable(),
  is_enabled: z.boolean(),
  group_ids: z.array(z.string()),
});

export const ModelGroupSchema = z.object({
  id: z.string(),
  alias: z.string(),
  active_binding_id: z.string().nullable(),
  is_enabled: z.boolean(),
  sort_order: z.number(),
});

export const MetricKpiSchema = z.object({
  error_rate: z.number(),
  total_input_tokens: z.number(),
  total_output_tokens: z.number(),
  total_cache_read_tokens: z.number(),
  total_consumed_tokens: z.number(),
});

export const MetricPointSchema = z.object({
  bucket_time: z.string(),
  group_name: z.string(),
  provider_name: z.string(),
  model_name: z.string(),
  input_tokens: z.number(),
  output_tokens: z.number(),
  cache_read_tokens: z.number(),
  consumed_tokens: z.number(),
});

export const GatewayConfigSchema = z.object({
  host: z.string(),
  port: z.number(),
  close_to_tray: z.boolean(),
  light_tray_mode: z.boolean(),
  allow_group_member_model_path: z.boolean(),
  auto_start: z.boolean(),
  silent_autostart: z.boolean(),
  log_level: z.enum(['error', 'warn', 'info', 'debug', 'trace', 'off']),
  debug_mode: z.boolean(),
  skills_enabled: z.boolean(),
  plugin_enabled: z.boolean(),
  plugin_namespace: z.string(),
  plugin_dist_path: z.string(),
  marketplace_enabled: z.boolean(),
  skills_source_path: z.string(),
  claude_skills_path: z.string(),
  auto_update_check: z.boolean().default(true),
});
