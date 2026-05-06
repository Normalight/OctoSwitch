import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { tauriApi } from "../lib/api/tauri";
import { CONFIG_IMPORTED } from "../lib/constants";
import { formatError } from "../lib/formatError";
import type { Provider, ProviderSummary } from "../types";

export function useProviders() {
  const [providers, setProviders] = useState<ProviderSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      setProviders(await tauriApi.listProviders());
    } catch (e) {
      setLoadError(formatError(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const clearLoadError = useCallback(() => setLoadError(null), []);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setLoadError(null);
    void (async () => {
      try {
        const list = await tauriApi.listProviders();
        if (!cancelled) setProviders(list);
      } catch (e) {
        if (!cancelled) setLoadError(formatError(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  /** Fetch the full Provider (with unredacted api_key_ref) for editing. */
  const fetchProvider = async (id: string): Promise<Provider> => {
    return tauriApi.getProvider(id);
  };

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    void listen(CONFIG_IMPORTED, () => {
      void refresh();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [refresh]);

  return {
    providers,
    loading,
    refresh,
    fetchProvider,
    loadError,
    clearLoadError,
  };
}
