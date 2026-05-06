import { formatError } from "../formatError";
import type { FetchedModel } from "../../types/fetched_model";
import { tauriApi } from "./tauri";

export async function fetchUpstreamModels(providerId: string): Promise<FetchedModel[]> {
  return tauriApi.fetchUpstreamModels(providerId);
}

export function mapFetchModelsError(
  err: unknown,
  t: (path: string, vars?: Record<string, string | number>) => string,
  opts?: { hasProvider: boolean }
): string {
  if (opts && !opts.hasProvider) {
    return t("models.fetchModelsNeedProvider");
  }

  const msg = formatError(err);

  if (msg.includes("API key is required") || msg.includes("API Key")) {
    return t("models.fetchModelsNeedApiKey");
  }
  if (msg.includes("HTTP 401") || msg.includes("HTTP 403")) {
    return t("models.fetchModelsAuthFailed");
  }
  if (msg.includes("HTTP 404") || msg.includes("HTTP 405")) {
    return t("models.fetchModelsNotSupported");
  }
  if (msg.toLowerCase().includes("timeout") || msg.includes("timed out")) {
    return t("models.fetchModelsTimeout");
  }
  if (msg.includes("Failed to parse")) {
    return t("models.fetchModelsNotSupported");
  }

  return t("models.fetchModelsFailed");
}
