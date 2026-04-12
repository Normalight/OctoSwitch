import { useEffect, useState } from "react";
import { tauriApi } from "../lib/api/tauri";
import { CONFIG_IMPORTED } from "../lib/constants";
import type { ModelGroup } from "../types";

export function useModelGroups() {
  const [groups, setGroups] = useState<ModelGroup[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void (async () => {
      try {
        const list = await tauriApi.listModelGroups();
        if (!cancelled) setGroups(list);
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
      setGroups(await tauriApi.listModelGroups());
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    const onImported = () => void refresh();
    window.addEventListener(CONFIG_IMPORTED, onImported);
    return () => window.removeEventListener(CONFIG_IMPORTED, onImported);
  }, []);

  return { groups, loading, refresh };
}
