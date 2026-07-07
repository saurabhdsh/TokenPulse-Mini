import { motion } from "framer-motion";
import { useCallback, useEffect, useState } from "react";
import { api, formatCost, formatTokens, PROVIDER_COLORS } from "../lib/api";
import { ProviderBadge } from "../components/ProviderBadge";
import type { DashboardData } from "../types";
import { ProgressRing } from "../components/ProgressRing";

export function DashboardPage() {
  const [data, setData] = useState<DashboardData | null>(null);
  const [syncing, setSyncing] = useState(false);

  const load = useCallback(async () => {
    const stats = await api.getDashboardStats();
    setData(stats);
  }, []);

  useEffect(() => {
    load().catch(console.error);
    const id = setInterval(() => load().catch(console.error), 30000);
    return () => clearInterval(id);
  }, [load]);

  const sync = async () => {
    setSyncing(true);
    try {
      await api.refreshLiveData();
      await load();
    } finally {
      setSyncing(false);
    }
  };

  if (!data) return <PageLoader />;

  const dailyPct = data.budget.daily_limit > 0
    ? (data.today.total_cost / data.budget.daily_limit) * 100
    : 0;

  const subtitle = data.is_demo_data
    ? "Sample data — connect a provider in API Keys to see live spend"
    : data.live_providers.length === 1
      ? `Live data from ${data.live_providers[0]}`
      : `Live data from ${data.live_providers.join(" + ")}`;

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <header style={{ marginBottom: 20, display: "flex", justifyContent: "space-between", gap: 12, alignItems: "flex-start" }}>
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 4 }}>
            <h1 style={{ fontSize: 22, fontWeight: 700, letterSpacing: "-0.02em" }}>Overview</h1>
            <span style={{
              fontSize: 9, fontWeight: 700, letterSpacing: "0.08em",
              padding: "3px 8px", borderRadius: 4,
              background: data.is_demo_data ? "rgba(251,191,36,0.15)" : "rgba(52,211,153,0.15)",
              color: data.is_demo_data ? "var(--warning)" : "var(--success)",
              border: `1px solid ${data.is_demo_data ? "rgba(251,191,36,0.35)" : "rgba(52,211,153,0.35)"}`,
            }}>
              {data.is_demo_data ? "DEMO" : "LIVE"}
            </span>
          </div>
          <p style={{ fontSize: 13, color: "var(--text-secondary)" }}>{subtitle}</p>
        </div>
        <button onClick={sync} disabled={syncing} style={{
          padding: "10px 16px", borderRadius: 10, fontSize: 12, fontWeight: 600,
          background: "rgba(108,140,255,0.15)", border: "1px solid rgba(108,140,255,0.25)",
          color: "var(--accent)", opacity: syncing ? 0.6 : 1, flexShrink: 0,
        }}>
          {syncing ? "Syncing…" : "↻ Sync"}
        </button>
      </header>

      {data.provider_summaries.length > 0 && (
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))", gap: 12, marginBottom: 20 }}>
          {data.provider_summaries.map((p, i) => (
            <motion.div
              key={p.name}
              className="glass-card-sm"
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.05 }}
              style={{ padding: 14 }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
                <ProviderBadge provider={p.name} size="sm" active={p.sync_status === "connected"} />
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ fontWeight: 600, fontSize: 13, color: PROVIDER_COLORS[p.name] }}>{p.name}</div>
                  <div style={{ fontSize: 10, color: p.sync_status === "connected" ? "var(--success)" : "var(--text-muted)" }}>
                    {p.sync_status === "connected"
                      ? "Synced"
                      : p.sync_status === "error"
                        ? "Sync error"
                        : data.is_demo_data
                          ? "Mock data"
                          : "Not connected"}
                  </div>
                </div>
              </div>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, fontSize: 12 }}>
                <div>
                  <div className="stat-label">Today</div>
                  <div style={{ fontWeight: 600 }}>{formatCost(p.today_cost)}</div>
                </div>
                <div>
                  <div className="stat-label">7 days</div>
                  <div style={{ fontWeight: 600 }}>{formatCost(p.week_cost)}</div>
                </div>
              </div>
              {p.credit && (
                <div style={{ marginTop: 10, padding: "6px 8px", borderRadius: 6, background: "rgba(52,211,153,0.08)", fontSize: 10, color: "var(--success)" }}>
                  ${p.credit.available.toFixed(2)}{" "}
                  {p.credit.source === "subscription_limit" ? "limit left" : "credits"}
                </div>
              )}
              {p.sync_message && p.sync_status === "error" && (
                <div style={{ marginTop: 8, fontSize: 10, color: "var(--warning)", lineHeight: 1.3 }}>
                  {p.sync_message}
                </div>
              )}
            </motion.div>
          ))}
        </div>
      )}

      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 12, marginBottom: 20 }}>
        <MetricCard label="Today" tokens={data.today.total_tokens} cost={data.today.total_cost} delay={0} />
        <MetricCard label="This Week" tokens={data.week.total_tokens} cost={data.week.total_cost} delay={0.05} />
        <MetricCard label="This Month" tokens={data.month.total_tokens} cost={data.month.total_cost} delay={0.1} />
        <motion.div
          className="glass-card-sm"
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.15 }}
          style={{ padding: 16, display: "flex", alignItems: "center", gap: 14 }}
        >
          <ProgressRing pct={dailyPct} size={52} stroke={4} />
          <div>
            <div className="stat-label">Daily Budget</div>
            <div className="stat-value">{formatCost(data.today.total_cost)}</div>
            <div style={{ fontSize: 11, color: "var(--text-muted)" }}>
              of {formatCost(data.budget.daily_limit)}
            </div>
          </div>
        </motion.div>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "1.4fr 1fr", gap: 16 }}>
        <motion.div className="glass-card-sm" initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ padding: 20 }}>
          <div className="stat-label" style={{ marginBottom: 12 }}>
            Provider Spend Today
            {!data.is_demo_data && data.live_providers.length > 0 && (
              <span style={{ color: "var(--text-muted)", fontWeight: 400 }}> · live only</span>
            )}
          </div>
          {data.providers.length === 0 ? (
            <EmptyHint text="No spend recorded today. Try syncing or check the 7-day view on Providers." />
          ) : (
            data.providers.map((p, i) => (
              <div key={p.provider} style={{ marginBottom: 12 }}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                  <span style={{ fontSize: 13, fontWeight: 500, color: PROVIDER_COLORS[p.provider] }}>
                    {p.provider}
                  </span>
                  <span className="mono">{formatCost(p.cost)} · {p.pct.toFixed(0)}%</span>
                </div>
                <div style={{ height: 4, borderRadius: 2, background: "rgba(255,255,255,0.06)" }}>
                  <motion.div
                    initial={{ width: 0 }}
                    animate={{ width: `${Math.max(p.pct, 2)}%` }}
                    transition={{ delay: 0.2 + i * 0.05, duration: 0.8 }}
                    style={{
                      height: "100%", borderRadius: 2,
                      background: PROVIDER_COLORS[p.provider] ?? "var(--accent)",
                    }}
                  />
                </div>
              </div>
            ))
          )}
        </motion.div>

        <motion.div className="glass-card-sm" initial={{ opacity: 0 }} animate={{ opacity: 1 }} style={{ padding: 20 }}>
          <div className="stat-label" style={{ marginBottom: 12 }}>Burn & Forecast</div>
          <div style={{ display: "grid", gap: 14 }}>
            <div>
              <div className="stat-label">Burn Rate / Hour</div>
              <div className="stat-value-lg" style={{ color: "var(--accent)" }}>
                {formatCost(data.today.burn_rate_per_hour)}
              </div>
            </div>
            <div>
              <div className="stat-label">Est. Monthly Bill</div>
              <div className="stat-value-lg">{formatCost(data.today.estimated_monthly)}</div>
            </div>
            <div>
              <div className="stat-label">Monthly Budget</div>
              <div style={{ fontSize: 14, fontWeight: 600 }}>
                {formatCost(data.month.total_cost)} / {formatCost(data.budget.monthly_limit)}
              </div>
            </div>
          </div>
        </motion.div>
      </div>

      {data.models.length > 0 && (
        <motion.div className="glass-card-sm" style={{ marginTop: 16, padding: 16 }}>
          <div className="stat-label" style={{ marginBottom: 10 }}>Top Models Today</div>
          <div style={{ display: "grid", gap: 8 }}>
            {data.models.slice(0, 5).map((m) => (
              <div key={`${m.provider}-${m.model}`} style={{ display: "flex", justifyContent: "space-between", fontSize: 12 }}>
                <span>
                  <span style={{ color: PROVIDER_COLORS[m.provider], fontWeight: 500 }}>{m.provider}</span>
                  <span className="mono" style={{ marginLeft: 8, color: "var(--text-secondary)" }}>{m.model}</span>
                </span>
                <span className="mono">{formatCost(m.cost)}</span>
              </div>
            ))}
          </div>
        </motion.div>
      )}

      {data.alerts.length > 0 && (
        <motion.div className="glass-card-sm" style={{ marginTop: 16, padding: 16 }}>
          <div className="stat-label" style={{ marginBottom: 10 }}>Recent Alerts</div>
          {data.alerts.slice(0, 5).map((a) => (
            <div key={a.id} style={{
              padding: "8px 10px", borderRadius: 8, marginBottom: 6,
              background: a.severity === "critical" ? "rgba(239,68,68,0.1)" :
                a.severity === "high" ? "rgba(251,146,60,0.1)" : "rgba(251,191,36,0.08)",
              border: "1px solid var(--border-subtle)", fontSize: 12,
            }}>
              {a.message}
            </div>
          ))}
        </motion.div>
      )}
    </div>
  );
}

function MetricCard({ label, tokens, cost, delay }: { label: string; tokens: number; cost: number; delay: number }) {
  return (
    <motion.div
      className="glass-card-sm"
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay }}
      style={{ padding: 16 }}
    >
      <div className="stat-label">{label}</div>
      <div className="stat-value-lg" style={{ marginTop: 4 }}>{formatCost(cost)}</div>
      <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 2 }}>
        {formatTokens(tokens)} tokens
      </div>
    </motion.div>
  );
}

function EmptyHint({ text }: { text: string }) {
  return (
    <p style={{ fontSize: 12, color: "var(--text-muted)", lineHeight: 1.5, margin: 0 }}>{text}</p>
  );
}

function PageLoader() {
  return (
    <div style={{ padding: 24, display: "flex", flexDirection: "column", gap: 12 }}>
      <div className="skeleton" style={{ height: 32, width: 200 }} />
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 12 }}>
        {[1, 2, 3, 4].map((i) => <div key={i} className="skeleton" style={{ height: 90 }} />)}
      </div>
    </div>
  );
}
