import { invoke } from "@tauri-apps/api/core";
import type {
  CopilotAccountStatus,
  CopilotStatus,
  DeviceCodeResponse,
  GatewayConfig,
  ImportReport,
  MetricKpi,
  MetricPoint,
  ModelBinding,
  ModelGroup,
  Provider
} from "../../types/index";
import type { FetchedModel } from "../../types/fetched_model";

/** 用量页统计窗口（与后端 `get_metrics_*` / `list_request_logs` 的 `window` 一致） */
export type UsageWindowKey = "5m" | "1h" | "24h" | "30d" | "custom";

export const tauriApi = {
  listProviders: () => invoke<Provider[]>("list_providers"),
  createProvider: (provider: Omit<Provider, "id" | "sort_order">) =>
    invoke<Provider>("create_provider", { provider }),
  updateProvider: (id: string, patch: Partial<Provider>) =>
    invoke<Provider>("update_provider", { id, patch }),
  deleteProvider: (id: string) => invoke<void>("delete_provider", { id }),
  listModelBindings: () => invoke<ModelBinding[]>("list_model_bindings"),
  createModelBinding: (binding: Omit<ModelBinding, "id" | "group_ids">) =>
    invoke<ModelBinding>("create_model_binding", { binding }),
  updateModelBinding: (id: string, patch: Partial<ModelBinding>) =>
    invoke<ModelBinding>("update_model_binding", { id, patch }),
  deleteModelBinding: (id: string) =>
    invoke<void>("delete_model_binding", { id }),
  listModelGroups: () => invoke<ModelGroup[]>("list_model_groups"),
  createModelGroup: (group: { alias: string }) =>
    invoke<ModelGroup>("create_model_group", { group }),
  updateModelGroup: (id: string, patch: Partial<{ alias: string; is_enabled: boolean; sort_order: number }>) =>
    invoke<ModelGroup>("update_model_group", { id, patch }),
  deleteModelGroup: (id: string) => invoke<void>("delete_model_group", { id }),
  toggleModelGroupEnabled: (id: string, enabled: boolean) =>
    invoke<ModelGroup>("update_model_group", { id, patch: { is_enabled: enabled } }),
  setModelGroupActiveBinding: (groupId: string, bindingId: string) =>
    invoke<ModelGroup>("set_model_group_active_binding", {
      groupId,
      bindingId
    }),
  addModelGroupMember: (groupId: string, bindingId: string) =>
    invoke<ModelGroup>("add_model_group_member", { groupId, bindingId }),
  removeModelGroupMember: (groupId: string, bindingId: string) =>
    invoke<ModelGroup>("remove_model_group_member", { groupId, bindingId }),
  runProviderHealthCheck: (providerId: string) =>
    invoke<{ ok: boolean; latency_ms: number; message: string }>(
      "run_provider_health_check",
      { providerId }
    ),
  /** OpenAI-compatible GET /v1/models for API-key providers (not Copilot-linked). */
  fetchUpstreamModels: (providerId: string) =>
    invoke<FetchedModel[]>("fetch_upstream_models", { providerId }),
  exportConfig: () => invoke<string>("export_config"),
  exportConfigToFile: () => invoke<void>("export_config_to_file"),
  importConfig: (json: string) => invoke<void>("import_config", { json }),
  clearAllData: () => invoke<void>("clear_all_data"),
  importCcSwitchProviders: () => invoke<ImportReport>("import_cc_switch_providers"),
  getMetricsKpi: (window: UsageWindowKey, customStart: string | null, customEnd: string | null) =>
    invoke<MetricKpi>("get_metrics_kpi", {
      window,
      customStart,
      customEnd
    }),
  getMetricsSeries: (window: UsageWindowKey, customStart: string | null, customEnd: string | null) =>
    invoke<MetricPoint[]>("get_metrics_series", {
      window,
      customStart,
      customEnd
    }),
  getRequestLogs: (
    window: UsageWindowKey,
    customStart: string | null,
    customEnd: string | null
  ) =>
    invoke<
      Array<{
        id: string;
        group_name: string;
        model_name: string;
        provider_name: string;
        latency_ms: number;
        input_tokens: number;
        output_tokens: number;
        status_code: number;
        created_at: string;
      }>
    >("list_request_logs", {
      window,
      customStart,
      customEnd
    }),
  getGatewayConfig: () => invoke<GatewayConfig>("get_gateway_config"),
  updateGatewayConfig: (config: GatewayConfig) =>
    invoke<void>("update_gateway_config", { config }),
  startCopilotAuth: () => invoke<DeviceCodeResponse>("start_copilot_auth"),
  openExternalUrl: (url: string) => invoke<void>("open_external_url", { url }),
  completeCopilotAuth: (deviceCode: string, providerId: string) =>
    invoke<CopilotStatus>("complete_copilot_auth", { deviceCode, providerId }),
  getCopilotStatus: () => invoke<CopilotStatus>("get_copilot_status"),
  refreshCopilotToken: () => invoke<CopilotStatus>("refresh_copilot_token"),
  revokeCopilotAuth: () => invoke<void>("revoke_copilot_auth"),
  listCopilotAccounts: () => invoke<CopilotAccountStatus[]>("list_copilot_accounts"),
  removeCopilotAccount: (accountId: number) => invoke<void>("remove_copilot_account", { accountId }),
  getCopilotModels: () => invoke<string[]>("get_copilot_models"),
  getCopilotUsage: () => invoke<Record<string, unknown>>("get_copilot_usage"),
  getAppVersion: () => invoke<string>("get_app_version"),
  checkForUpdate: () => invoke<{
    current_version: string;
    latest_version: string;
    has_update: boolean;
    release_notes: string;
    release_url: string;
    is_ignored: boolean;
  }>("check_for_update"),
  ignoreUpdateVersion: (version: string) =>
    invoke<void>("ignore_update_version", { version }),
  clearIgnoredUpdateVersion: () =>
    invoke<void>("clear_ignored_update_version"),
};
