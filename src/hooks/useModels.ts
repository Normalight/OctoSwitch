import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { tauriApi } from "../lib/api/tauri";
import { CONFIG_IMPORTED } from "../lib/constants";
import { formatError } from "../lib/formatError";
import type { ModelBinding } from "../types";

export function useModels(enabled: boolean = true) {
  const [models, setModels] = useState<ModelBinding[]>([]);
  const [loading, setLoading] = useState(enabled);
  const [loadError, setLoadError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      setModels(await tauriApi.listModelBindings());
    } catch (e) {
      setLoadError(formatError(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const clearLoadError = useCallback(() => setLoadError(null), []);

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    setLoadError(null);
    void (async () => {
      try {
        const list = await tauriApi.listModelBindings();
        if (!cancelled) setModels(list);
      } catch (e) {
        if (!cancelled) setLoadError(formatError(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [enabled]);

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

  return { models, loading, refresh, loadError, clearLoadError };
}
