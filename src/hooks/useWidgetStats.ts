import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import type { WidgetStats } from "../types";

export function useWidgetStats(intervalMs = 30000, provider?: string | null) {
  const [stats, setStats] = useState<WidgetStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const data = await api.getWidgetStats(provider ?? undefined);
      setStats(data);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [provider]);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, intervalMs);
    const unlisten = listen<boolean>("widget-demo-changed", () => {
      refresh();
    });
    return () => {
      clearInterval(id);
      unlisten.then((fn) => fn());
    };
  }, [refresh, intervalMs]);

  return { stats, loading, error, refresh };
}
