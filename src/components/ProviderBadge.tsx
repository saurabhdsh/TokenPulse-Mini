import { PROVIDER_COLORS, PROVIDER_SHORT } from "../lib/api";

interface ProviderBadgeProps {
  provider: string;
  size?: "xs" | "sm";
  active?: boolean;
}

export function ProviderBadge({ provider, size = "xs", active = true }: ProviderBadgeProps) {
  const color = PROVIDER_COLORS[provider] ?? "#888";
  const short = PROVIDER_SHORT[provider] ?? provider.slice(0, 3).toUpperCase();
  const dim = size === "xs" ? 20 : 26;
  const fontSize = size === "xs" ? 7 : 9;

  return (
    <div
      title={provider}
      style={{
        width: dim,
        height: dim,
        borderRadius: 6,
        background: active ? `${color}22` : "rgba(255,255,255,0.04)",
        border: `1px solid ${active ? `${color}55` : "rgba(255,255,255,0.06)"}`,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        fontSize,
        fontWeight: 700,
        letterSpacing: "0.02em",
        color: active ? color : "var(--text-muted)",
        transition: "all 0.2s ease",
        flexShrink: 0,
      }}
    >
      {short}
    </div>
  );
}
