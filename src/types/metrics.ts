// src/types/metrics.ts
export interface MetricKpi {
  avg_qps: number;
  avg_tps: number;
  error_rate: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_creation_tokens: number;
  total_cache_read_tokens: number;
  total_cost: number;
}

export interface MetricPoint {
  bucket_time: string;
  qps: number;
  tps: number;
  cost: number;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
}
