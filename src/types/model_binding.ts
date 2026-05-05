// src/types/model_binding.ts
export interface ModelBinding {
  id: string;
  model_name: string;
  provider_id: string;
  upstream_model_name: string;
  rpm_limit: number | null;
  tpm_limit: number | null;
  is_enabled: boolean;
  group_ids: string[];
}
