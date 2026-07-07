import { motion } from "framer-motion";
import { useWindowDrag } from "../hooks/useWindowDrag";
import { useWidgetStats } from "../hooks/useWidgetStats";
import { formatCost, formatTokens, PROVIDER_COLORS } from "../lib/api";
import { ProgressRing } from "./ProgressRing";
import { ProviderBadge } from "./ProviderBadge";
import { Sparkline } from "./Sparkline";

const PROVIDERS = ["OpenAI", "Anthropic", "AWS Bedrock", "Azure OpenAI", "Gemini"];

interface MiniWidgetProps {
  provider?: string | null;
  pinned: boolean;
  onTogglePin: () => void;
  onExpand: () => void;
  onHide: () => void;
}

export function MiniWidget({ provider, pinned, onTogglePin, onExpand, onHide }: MiniWidgetProps) {
  const { stats, loading } = useWidgetStats(15000, provider);
  const onDragMouseDown = useWindowDrag();

  const riskClass =
    stats?.budget_risk === "Critical" ? "risk-critical" :
    stats?.budget_risk === "High" ? "risk-high" :
    stats?.budget_risk === "Moderate" ? "risk-moderate" : "risk-low";

  const activeProviders = new Set(
    stats?.live_providers.length
      ? [
          ...stats.live_providers,
          ...(stats.show_demo_overlay ? ["OpenAI", "AWS Bedrock"] : []),
        ]
      : stats?.provider_breakdown.map((p) => p.provider) ?? [],
  );

  const liveBadge = provider
    ? stats?.show_demo_overlay && stats.live_providers.includes(provider)
      ? `${provider === "OpenAI" ? "OPENAI" : "AWS"} LIVE+DEMO`
      : stats?.live_providers.includes(provider)
        ? `${provider === "OpenAI" ? "OPENAI" : "AWS"} LIVE`
        : stats?.is_demo_data
          ? "DEMO"
          : "OFFLINE"
    : stats?.show_demo_overlay
      ? "LIVE+DEMO"
      : stats?.live_providers.includes("OpenAI")
        ? "OPENAI LIVE"
        : stats?.live_providers.length
          ? "LIVE"
          : stats?.is_demo_data
            ? "DEMO"
            : "LIVE";

  const badgeLabel = liveBadge;

  const isOffline = provider && stats && !stats.live_providers.includes(provider) && !stats.is_demo_data;
  const badgeStyle = stats?.show_demo_overlay && !stats?.is_demo_data
    ? { bg: "rgba(108,140,255,0.15)", color: "var(--accent)", border: "rgba(108,140,255,0.35)" }
    : stats?.is_demo_data
      ? { bg: "rgba(251,191,36,0.15)", color: "var(--warning)", border: "rgba(251,191,36,0.35)" }
      : isOffline
        ? { bg: "rgba(248,113,113,0.12)", color: "var(--danger)", border: "rgba(248,113,113,0.3)" }
        : { bg: "rgba(52,211,153,0.15)", color: "var(--success)", border: "rgba(52,211,153,0.35)" };

  const headerTitle = provider ?? "TOKENPULSE";

  return (
    <motion.div
      className="glass-card"
      initial={{ opacity: 0, scale: 0.96 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
      style={{
        width: 320,
        height: 220,
        padding: "12px 14px",
        display: "flex",
        flexDirection: "column",
        gap: 8,
        position: "relative",
        overflow: "hidden",
      }}
      onMouseDown={onDragMouseDown}
    >
      {/* Ambient glow */}
      <div style={{
        position: "absolute", top: -40, right: -40, width: 120, height: 120,
        background: "radial-gradient(circle, var(--accent-glow) 0%, transparent 70%)",
        pointerEvents: "none",
      }} />

      {/* Header */}
      <div
        className="window-titlebar-leading"
        data-tauri-drag-region
        style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}
        onMouseDown={onDragMouseDown}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <div style={{
            width: 7, height: 7, borderRadius: "50%",
            background: "var(--accent)",
            boxShadow: "0 0 8px var(--accent-glow)",
            animation: "pulse-glow 2s ease infinite",
          }} />
          <span style={{ fontSize: 11, fontWeight: 600, letterSpacing: "0.04em" }}>
            {provider ? headerTitle : "TOKENPULSE"}
          </span>
          {!provider && <span style={{ fontSize: 9, color: "var(--text-muted)", fontWeight: 500 }}>MINI</span>}
          {stats && (
            <span style={{
              fontSize: 8, fontWeight: 700, letterSpacing: "0.08em",
              padding: "2px 6px", borderRadius: 4,
              background: badgeStyle.bg,
              color: badgeStyle.color,
              border: `1px solid ${badgeStyle.border}`,
            }}>
              {badgeLabel}
            </span>
          )}
        </div>
        <div data-no-drag style={{ display: "flex", gap: 2 }}>
          <IconBtn title={pinned ? "Unpin" : "Pin on top"} onClick={onTogglePin} active={pinned}>
            📌
          </IconBtn>
          <IconBtn title={provider ? "Open full dashboard" : "Expand"} onClick={onExpand}>⤢</IconBtn>
          <IconBtn title="Hide" onClick={onHide}>−</IconBtn>
        </div>
      </div>

      {loading || !stats ? (
        <WidgetSkeleton />
      ) : (
        <>
          {/* Main stats row */}
          <div style={{ display: "flex", gap: 10, alignItems: "flex-start" }}>
            <div style={{ flex: 1, display: "grid", gridTemplateColumns: "1fr 1fr", gap: "6px 12px" }}>
              <Stat label="Today Tokens" value={formatTokens(stats.today_tokens)} />
              <Stat label="Today Cost" value={formatCost(stats.today_cost)} accent />
              <Stat label="Burn Rate/hr" value={formatCost(stats.burn_rate_per_hour)} />
              <Stat
                label={provider === "OpenAI" && stats.openai_credit ? "Credit Left" : provider ? "Est. Monthly" : stats.openai_credit ? "Credit Left" : "Est. Monthly"}
                value={
                  provider === "OpenAI" && stats.openai_credit
                    ? formatCost(stats.openai_credit.available)
                    : formatCost(stats.monthly_estimated)
                }
                accent={provider === "OpenAI" && !!stats.openai_credit}
              />
            </div>
            <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 2 }}>
              <div style={{ position: "relative" }}>
                <ProgressRing pct={stats.daily_budget_used_pct} />
                <span style={{
                  position: "absolute", inset: 0, display: "flex", alignItems: "center",
                  justifyContent: "center", fontSize: 9, fontWeight: 700,
                }}>
                  {Math.round(stats.daily_budget_used_pct)}%
                </span>
              </div>
              <span className="stat-label" style={{ fontSize: 8 }}>Budget</span>
            </div>
          </div>

          {/* Sparkline + top provider/model */}
          <div style={{
            display: "flex", alignItems: "center", justifyContent: "space-between",
            padding: "6px 8px", borderRadius: 8,
            background: "rgba(0,0,0,0.2)", border: "1px solid var(--border-subtle)",
          }}>
            <div>
              <div className="stat-label" style={{ marginBottom: 2 }}>{stats.live_providers.length ? "14d Usage" : "24h Usage"}</div>
              <Sparkline data={stats.sparkline} width={100} height={22} />
            </div>
            <div style={{ textAlign: "right" }}>
              {provider ? (
                <>
                  <div className="stat-label">Top Model</div>
                  <div className="mono" style={{ fontSize: 11, fontWeight: 600, color: "var(--text-secondary)" }}>
                    {stats.top_model === "—"
                      ? "No usage yet"
                      : stats.top_model.length > 18
                        ? stats.top_model.slice(0, 16) + "…"
                        : stats.top_model}
                  </div>
                </>
              ) : (
                <>
                  <div className="stat-label">Top Provider</div>
                  <div style={{ fontSize: 11, fontWeight: 600, color: PROVIDER_COLORS[stats.top_provider] ?? "inherit" }}>
                    {stats.top_provider}
                  </div>
                  <div className="stat-label" style={{ marginTop: 2 }}>Top Model</div>
                  <div className="mono" style={{ color: "var(--text-secondary)" }}>
                    {stats.top_model.length > 16 ? stats.top_model.slice(0, 14) + "…" : stats.top_model}
                  </div>
                </>
              )}
            </div>
          </div>

          {/* Footer: risk + provider badges */}
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: "auto" }}>
            <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span className="stat-label">Risk</span>
                <span className={riskClass} style={{ fontSize: 11, fontWeight: 600 }}>
                  {stats.budget_risk}
                </span>
              </div>
              {stats.sync_hint && (
                <span style={{
                  fontSize: 8,
                  color: stats.show_demo_overlay ? "var(--accent)" : stats.is_demo_data ? "var(--warning)" : "var(--text-muted)",
                  maxWidth: 140,
                  lineHeight: 1.2,
                }}>
                  {stats.sync_hint}
                </span>
              )}
            </div>
            <div style={{ display: "flex", gap: 3 }}>
              {(provider ? [provider] : PROVIDERS).map((p) => (
                <ProviderBadge key={p} provider={p} active={activeProviders.has(p)} />
              ))}
            </div>
          </div>
        </>
      )}
    </motion.div>
  );
}

function Stat({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div>
      <div className="stat-label">{label}</div>
      <div className="stat-value" style={{ color: accent ? "var(--accent)" : undefined }}>
        {value}
      </div>
    </div>
  );
}

function IconBtn({ children, onClick, title, active }: {
  children: React.ReactNode; onClick: () => void; title: string; active?: boolean;
}) {
  return (
    <button
      title={title}
      onClick={onClick}
      style={{
        width: 22, height: 22, borderRadius: 6, fontSize: 11,
        display: "flex", alignItems: "center", justifyContent: "center",
        background: active ? "rgba(108,140,255,0.2)" : "rgba(255,255,255,0.04)",
        border: `1px solid ${active ? "rgba(108,140,255,0.3)" : "rgba(255,255,255,0.06)"}`,
        transition: "all 0.15s",
      }}
    >
      {children}
    </button>
  );
}

function WidgetSkeleton() {
  return (
    <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 8 }}>
      <div className="skeleton" style={{ height: 60 }} />
      <div className="skeleton" style={{ height: 36 }} />
      <div className="skeleton" style={{ height: 20 }} />
    </div>
  );
}
