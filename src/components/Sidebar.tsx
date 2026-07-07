import type { Page } from "../types";
import { WidgetDemoToggle } from "./WidgetDemoToggle";

interface SidebarProps {
  current: Page;
  onNavigate: (page: Page) => void;
}

const NAV: { id: Page; label: string; icon: string }[] = [
  { id: "dashboard", label: "Dashboard", icon: "◈" },
  { id: "providers", label: "Providers", icon: "◎" },
  { id: "models", label: "Models", icon: "⬡" },
  { id: "budget", label: "Budget", icon: "◐" },
  { id: "api-keys", label: "API Keys", icon: "⬢" },
  { id: "pricing", label: "Pricing", icon: "$" },
  { id: "history", label: "History", icon: "≡" },
];

export function Sidebar({ current, onNavigate }: SidebarProps) {
  return (
    <aside className="no-drag" style={{
      width: 200,
      height: "100%",
      padding: "16px 12px",
      borderRight: "1px solid var(--border-subtle)",
      display: "flex",
      flexDirection: "column",
      gap: 4,
      background: "rgba(0,0,0,0.25)",
      flexShrink: 0,
    }}>
      <div style={{ padding: "0 10px 12px" }}>
        <div style={{ fontSize: 10, color: "var(--text-muted)", letterSpacing: "0.06em", textTransform: "uppercase" }}>
          Navigation
        </div>
      </div>

      {NAV.map((item) => (
        <button
          key={item.id}
          type="button"
          onClick={() => onNavigate(item.id)}
          style={{
            display: "flex", alignItems: "center", gap: 10,
            padding: "10px 12px", borderRadius: 10, textAlign: "left",
            fontSize: 13, fontWeight: current === item.id ? 600 : 400,
            background: current === item.id ? "rgba(108,140,255,0.15)" : "transparent",
            border: current === item.id ? "1px solid rgba(108,140,255,0.25)" : "1px solid transparent",
            color: current === item.id ? "var(--accent)" : "var(--text-secondary)",
            transition: "all 0.15s",
          }}
        >
          <span style={{ fontSize: 14, opacity: 0.8 }}>{item.icon}</span>
          {item.label}
        </button>
      ))}

      <WidgetDemoToggle />
    </aside>
  );
}
