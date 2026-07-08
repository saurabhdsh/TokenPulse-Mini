import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../lib/api";
import type { LiveSyncFinished, SyncReport } from "../types";

export function useLiveSync(onComplete?: () => void | Promise<void>) {
  const [syncing, setSyncing] = useState(false);
  const [lastReports, setLastReports] = useState<SyncReport[]>([]);
  const onCompleteRef = useRef(onComplete);
  onCompleteRef.current = onComplete;

  useEffect(() => {
    const unlisten = listen<LiveSyncFinished>("live-sync-finished", async (event) => {
      setSyncing(false);
      setLastReports(event.payload.reports);
      if (!event.payload.ok && event.payload.error) {
        console.error("Live sync failed:", event.payload.error);
      }
      try {
        await onCompleteRef.current?.();
      } catch (e) {
        console.error(e);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const startSync = useCallback(() => {
    if (syncing) return;
    setSyncing(true);
    void api.startRefreshLiveData().catch((e) => {
      setSyncing(false);
      console.error(e);
    });
  }, [syncing]);

  return { syncing, startSync, lastReports };
}

export function waitForLiveSync(timeoutMs = 120_000): Promise<LiveSyncFinished> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => {
      reject(new Error("Sync timed out"));
    }, timeoutMs);

    listen<LiveSyncFinished>("live-sync-finished", (event) => {
      window.clearTimeout(timer);
      resolve(event.payload);
    }).then((unlisten) => {
      api.startRefreshLiveData().catch((err) => {
        window.clearTimeout(timer);
        unlisten();
        reject(err);
      });
    });
  });
}

export type { SyncReport };
