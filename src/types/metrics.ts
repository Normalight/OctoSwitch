// src/types/metrics.ts
export interface MetricKpi {
  error_rate: number;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read_tokens: number;
  total_consumed_tokens: number;
}

export interface MetricPoint {
  bucket_time: string;
  group_name: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  consumed_tokens: number;
}
