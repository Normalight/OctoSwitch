import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { tauriApi } from "../lib/api/tauri";
import { CONFIG_IMPORTED } from "../lib/constants";
import { formatError } from "../lib/formatError";
import type { ModelGroup } from "../types";

export function useModelGroups() {
  const [groups, setGroups] = useState<ModelGroup[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      setGroups(await tauriApi.listModelGroups());
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
        const list = await tauriApi.listModelGroups();
        if (!cancelled) setGroups(list);
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

  return { groups, loading, refresh, loadError, clearLoadError };
}
