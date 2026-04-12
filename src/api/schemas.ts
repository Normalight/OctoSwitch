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
  input_price_per_1m: z.number(),
  output_price_per_1m: z.number(),
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
  avg_qps: z.number(),
  avg_tps: z.number(),
  error_rate: z.number(),
  total_input_tokens: z.number(),
  total_output_tokens: z.number(),
  total_cost: z.number(),
});

export const MetricPointSchema = z.object({
  bucket_time: z.string(),
  qps: z.number(),
  tps: z.number(),
  cost: z.number(),
  input_tokens: z.number(),
  output_tokens: z.number(),
});

export const RequestLogSchema = z.object({
  id: z.string(),
  group_name: z.string(),
  model_name: z.string(),
  provider_name: z.string(),
  latency_ms: z.number(),
  input_tokens: z.number(),
  output_tokens: z.number(),
  status_code: z.number(),
  created_at: z.string(),
});
