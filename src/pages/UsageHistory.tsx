import { useEffect, useState } from "react";
import { api, formatCost, formatTime, PROVIDER_COLORS } from "../lib/api";
import type { SyncReport, UsageEvent } from "../types";

export function UsageHistoryPage() {
  const [events, setEvents] = useState<UsageEvent[]>([]);
  const [syncing, setSyncing] = useState(false);
  const [lastSync, setLastSync] = useState<SyncReport[]>([]);

  const load = () => api.getUsageHistory(100, 0).then(setEvents).catch(console.error);

  useEffect(() => { load(); }, []);

  const sync = async () => {
    setSyncing(true);
    try {
      const reports = await api.refreshLiveData();
      setLastSync(reports);
      await load();
    } finally {
      setSyncing(false);
    }
  };

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <header style={{ marginBottom: 24, display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
        <div>
          <h1 style={{ fontSize: 22, fontWeight: 700 }}>Usage History</h1>
          <p style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 4 }}>
            {events.length} recent events
          </p>
        </div>
        <button onClick={sync} disabled={syncing} style={{
          padding: "10px 16px", borderRadius: 10, fontSize: 12, fontWeight: 600,
          background: "rgba(108,140,255,0.15)", border: "1px solid rgba(108,140,255,0.25)",
          color: "var(--accent)", opacity: syncing ? 0.6 : 1,
        }}>
          {syncing ? "Syncing…" : "↻ Sync Live Data"}
        </button>
      </header>

      {lastSync.length > 0 && (
        <div className="glass-card-sm" style={{ padding: 12, marginBottom: 12, fontSize: 12 }}>
          {lastSync.map((r) => (
            <div key={r.provider} style={{ color: r.status === "connected" ? "var(--success)" : "var(--warning)" }}>
              {r.provider}: {r.message}
            </div>
          ))}
        </div>
      )}

      <div className="glass-card-sm" style={{ overflow: "hidden" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
          <thead>
            <tr style={{ borderBottom: "1px solid var(--border-subtle)" }}>
              {["Time", "Provider", "Model", "Tokens", "Cost", "Project"].map((h) => (
                <th key={h} style={{
                  padding: "10px 14px", textAlign: "left", fontSize: 10,
                  fontWeight: 600, letterSpacing: "0.06em", color: "var(--text-muted)",
                  textTransform: "uppercase",
                }}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {events.map((e) => (
              <tr key={e.id ?? e.request_id} style={{ borderBottom: "1px solid var(--border-subtle)" }}>
                <td style={{ padding: "10px 14px", whiteSpace: "nowrap" }} className="mono">
                  {formatTime(e.timestamp)}
                </td>
                <td style={{ padding: "10px 14px", color: PROVIDER_COLORS[e.provider] }}>{e.provider}</td>
                <td style={{ padding: "10px 14px", fontFamily: "var(--font-mono)", fontSize: 11 }}>{e.model}</td>
                <td style={{ padding: "10px 14px" }} className="mono">{e.total_tokens.toLocaleString()}</td>
                <td style={{ padding: "10px 14px", fontWeight: 600 }}>{formatCost(e.total_cost)}</td>
                <td style={{ padding: "10px 14px", color: "var(--text-muted)" }}>{e.project_name ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
