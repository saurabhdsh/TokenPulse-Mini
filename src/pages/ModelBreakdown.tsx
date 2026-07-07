import { motion } from "framer-motion";
import { useEffect, useState } from "react";
import { api, formatCost, formatTokens, PROVIDER_COLORS } from "../lib/api";
import type { ModelCost } from "../types";

export function ModelBreakdownPage() {
  const [models, setModels] = useState<ModelCost[]>([]);

  useEffect(() => {
    api.getModelBreakdown().then(setModels).catch(console.error);
  }, []);

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <header style={{ marginBottom: 24 }}>
        <h1 style={{ fontSize: 22, fontWeight: 700 }}>Model Breakdown</h1>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 4 }}>
          Cost and usage by model over the last 7 days
        </p>
      </header>

      {models.length === 0 ? (
        <div className="glass-card-sm" style={{ padding: 20, fontSize: 13, color: "var(--text-muted)" }}>
          No model usage yet. Connect OpenAI or AWS Bedrock and sync.
        </div>
      ) : (
        <div className="glass-card-sm" style={{ overflow: "hidden" }}>
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
            <thead>
              <tr style={{ borderBottom: "1px solid var(--border-subtle)", textAlign: "left" }}>
                {["Model", "Provider", "Requests", "Tokens", "Cost"].map((h) => (
                  <th key={h} style={{ padding: "12px 16px", fontSize: 10, fontWeight: 600, letterSpacing: "0.06em", color: "var(--text-muted)", textTransform: "uppercase" }}>
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {models.map((m, i) => (
                <motion.tr
                  key={`${m.provider}-${m.model}`}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ delay: i * 0.03 }}
                  style={{ borderBottom: "1px solid var(--border-subtle)" }}
                >
                  <td style={{ padding: "12px 16px", fontFamily: "var(--font-mono)", fontSize: 12 }}>{m.model}</td>
                  <td style={{ padding: "12px 16px", color: PROVIDER_COLORS[m.provider] }}>{m.provider}</td>
                  <td style={{ padding: "12px 16px" }} className="mono">{m.request_count}</td>
                  <td style={{ padding: "12px 16px" }} className="mono">{formatTokens(m.tokens)}</td>
                  <td style={{ padding: "12px 16px", fontWeight: 600 }}>{formatCost(m.cost)}</td>
                </motion.tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
