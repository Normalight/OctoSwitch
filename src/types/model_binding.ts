// src/types/model_binding.ts
export interface ModelBinding {
  id: string;
  model_name: string;
  provider_id: string;
  upstream_model_name: string;
  input_price_per_1m: number;
  output_price_per_1m: number;
  rpm_limit: number | null;
  tpm_limit: number | null;
  is_enabled: boolean;
  group_ids: string[];
}
