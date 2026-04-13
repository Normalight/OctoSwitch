import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { tauriApi } from "../lib/api/tauri";
import { CONFIG_IMPORTED } from "../lib/constants";
import type { Provider } from "../types";

export function useProviders() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void (async () => {
      try {
        const list = await tauriApi.listProviders();
        if (!cancelled) setProviders(list);
      } catch {
        // ignore
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  const refresh = async () => {
    setLoading(true);
    try {
      setProviders(await tauriApi.listProviders());
    } finally {
      setLoading(false);
    }
  };

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
  }, []);

  return { providers, loading, refresh };
}
