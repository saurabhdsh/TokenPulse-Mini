import { useCallback, useEffect, useState } from "react";
import { api } from "../lib/api";

export function WidgetDemoToggle() {
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    api.getWidgetDemoEnabled()
      .then(setEnabled)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const toggle = useCallback(async () => {
    const next = !enabled;
    setSaving(true);
    try {
      await api.setWidgetDemoEnabled(next);
      setEnabled(next);
    } catch (e) {
      console.error(e);
    } finally {
      setSaving(false);
    }
  }, [enabled]);

  return (
    <div style={{
      marginTop: "auto",
      padding: "12px 10px",
      borderTop: "1px solid var(--border-subtle)",
    }}>
      <div style={{
        fontSize: 10,
        color: "var(--text-muted)",
        letterSpacing: "0.06em",
        textTransform: "uppercase",
        marginBottom: 8,
      }}>
        Widgets
      </div>
      <button
        type="button"
        onClick={toggle}
        disabled={loading || saving}
        style={{
          width: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 8,
          padding: "10px 12px",
          borderRadius: 10,
          fontSize: 12,
          fontWeight: 500,
          textAlign: "left",
          background: enabled ? "rgba(251,191,36,0.12)" : "rgba(255,255,255,0.04)",
          border: `1px solid ${enabled ? "rgba(251,191,36,0.3)" : "rgba(255,255,255,0.08)"}`,
          color: enabled ? "var(--warning)" : "var(--text-secondary)",
          opacity: loading || saving ? 0.6 : 1,
          transition: "all 0.15s",
        }}
      >
        <span>Demo overlay</span>
        <span style={{
          fontSize: 9,
          fontWeight: 700,
          letterSpacing: "0.06em",
          padding: "2px 6px",
          borderRadius: 4,
          background: enabled ? "rgba(251,191,36,0.2)" : "rgba(255,255,255,0.06)",
        }}>
          {enabled ? "ON" : "OFF"}
        </span>
      </button>
      <p style={{
        marginTop: 8,
        fontSize: 10,
        lineHeight: 1.4,
        color: "var(--text-muted)",
      }}>
        Blend sample OpenAI &amp; AWS data into live widgets.
      </p>
    </div>
  );
}
