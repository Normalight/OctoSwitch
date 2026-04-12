// src/types/provider.ts
export type ProviderApiFormat = "anthropic" | "openai_chat" | "openai_responses";
export type ProviderAuthMode = "bearer" | "anthropic_api_key";

export interface Provider {
  id: string;
  name: string;
  base_url: string;
  api_key_ref: string;
  timeout_ms: number;
  max_retries: number;
  is_enabled: boolean;
  sort_order: number;
  api_format?: ProviderApiFormat | null;
  auth_mode?: ProviderAuthMode;
}

export interface ImportDetail {
  cc_name: string;
  status: "imported" | "skipped_duplicate" | "skipped_existing";
  provider_id: string | null;
  models_imported: string[];
  models_skipped: string[];
  reason: string | null;
}

export interface ImportReport {
  providers_imported: number;
  providers_skipped: number;
  models_bound: number;
  models_skipped: number;
  details: ImportDetail[];
}
