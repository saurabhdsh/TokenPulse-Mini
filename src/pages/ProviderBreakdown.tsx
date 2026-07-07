import { motion } from "framer-motion";
import { useEffect, useState } from "react";
import { api, formatCost, formatTokens, PROVIDER_COLORS } from "../lib/api";
import { ProviderBadge } from "../components/ProviderBadge";
import type { ProviderCost } from "../types";

export function ProviderBreakdownPage() {
  const [providers, setProviders] = useState<ProviderCost[]>([]);
  const [liveOnly, setLiveOnly] = useState(false);

  useEffect(() => {
    Promise.all([api.getProviderBreakdown(), api.getProviders()])
      .then(([breakdown, all]) => {
        setProviders(breakdown);
        setLiveOnly(all.some((p) => p.sync_status === "connected"));
      })
      .catch(console.error);
  }, []);

  const total = providers.reduce((s, p) => s + p.cost, 0);

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <header style={{ marginBottom: 24 }}>
        <h1 style={{ fontSize: 22, fontWeight: 700 }}>Provider Breakdown</h1>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 4 }}>
          7-day spend · {formatCost(total)} total
          {liveOnly && <span style={{ color: "var(--success)" }}> · live providers only</span>}
        </p>
      </header>

      {providers.length === 0 ? (
        <div className="glass-card-sm" style={{ padding: 20, fontSize: 13, color: "var(--text-muted)" }}>
          No provider spend in the last 7 days. Sync live data from API Keys.
        </div>
      ) : (
        <div style={{ display: "grid", gap: 12 }}>
          {providers.map((p, i) => (
            <motion.div
              key={p.provider}
              className="glass-card-sm"
              initial={{ opacity: 0, x: -12 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: i * 0.05 }}
              style={{ padding: 16, display: "flex", alignItems: "center", gap: 16 }}
            >
              <ProviderBadge provider={p.provider} size="sm" active={liveOnly} />
              <div style={{ flex: 1 }}>
                <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 8 }}>
                  <span style={{ fontWeight: 600, color: PROVIDER_COLORS[p.provider] }}>{p.provider}</span>
                  <span className="mono">{formatCost(p.cost)}</span>
                </div>
                <div style={{ height: 6, borderRadius: 3, background: "rgba(255,255,255,0.06)" }}>
                  <motion.div
                    initial={{ width: 0 }}
                    animate={{ width: `${p.pct}%` }}
                    transition={{ duration: 0.8, delay: 0.1 + i * 0.05 }}
                    style={{
                      height: "100%", borderRadius: 3,
                      background: `linear-gradient(90deg, ${PROVIDER_COLORS[p.provider]}88, ${PROVIDER_COLORS[p.provider]})`,
                    }}
                  />
                </div>
                <div style={{ display: "flex", justifyContent: "space-between", marginTop: 6, fontSize: 11, color: "var(--text-muted)" }}>
                  <span>{formatTokens(p.tokens)} tokens</span>
                  <span>{p.pct.toFixed(1)}% of spend</span>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  );
}
