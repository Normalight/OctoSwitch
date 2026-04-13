import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { tauriApi } from "../lib/api/tauri";
import { CONFIG_IMPORTED } from "../lib/constants";
import type { ModelBinding } from "../types";

export function useModels(enabled: boolean = true) {
  const [models, setModels] = useState<ModelBinding[]>([]);
  const [loading, setLoading] = useState(enabled);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setModels(await tauriApi.listModelBindings());
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      return;
    }
    let cancelled = false;
    setLoading(true);
    void (async () => {
      try {
        const list = await tauriApi.listModelBindings();
        if (!cancelled) setModels(list);
      } catch {
        // ignore
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [enabled]);

  useEffect(() => {
    const onImported = () => {
      void refresh();
    };
    window.addEventListener(CONFIG_IMPORTED, onImported);
    let unlisten: (() => void) | null = null;
    void listen(CONFIG_IMPORTED, () => {
      void refresh();
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      window.removeEventListener(CONFIG_IMPORTED, onImported);
      unlisten?.();
    };
  }, [refresh]);

  return { models, loading, refresh };
}
