import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { BudgetSettings } from "../types";

const TIMEZONES = [
  "America/New_York",
  "America/Chicago",
  "America/Denver",
  "America/Los_Angeles",
  "UTC",
  "Europe/London",
  "Europe/Paris",
  "Asia/Tokyo",
  "Asia/Kolkata",
  "Australia/Sydney",
];

export function BudgetSettingsPage() {
  const [settings, setSettings] = useState<BudgetSettings | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    api.getBudgetSettings().then(setSettings).catch(console.error);
  }, []);

  const save = async () => {
    if (!settings) return;
    await api.updateBudgetSettings({
      daily_limit: settings.daily_limit,
      monthly_limit: settings.monthly_limit,
      timezone: settings.timezone,
      alert_threshold_50: settings.alert_threshold_50,
      alert_threshold_80: settings.alert_threshold_80,
      alert_threshold_100: settings.alert_threshold_100,
      spike_detection_enabled: settings.spike_detection_enabled,
      expensive_model_warning: settings.expensive_model_warning,
    });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  if (!settings) return <div style={{ padding: 24 }}><div className="skeleton" style={{ height: 300 }} /></div>;

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%", maxWidth: 560 }}>
      <header style={{ marginBottom: 24 }}>
        <h1 style={{ fontSize: 22, fontWeight: 700 }}>Budget Settings</h1>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 4 }}>
          Configure limits, timezone, and alert thresholds
        </p>
      </header>

      <div className="glass-card-sm" style={{ padding: 20, display: "flex", flexDirection: "column", gap: 16 }}>
        <Field label="Daily Budget Limit ($)">
          <input type="number" value={settings.daily_limit}
            onChange={(e) => setSettings({ ...settings, daily_limit: +e.target.value })}
            style={inputStyle} />
        </Field>
        <Field label="Monthly Budget Limit ($)">
          <input type="number" value={settings.monthly_limit}
            onChange={(e) => setSettings({ ...settings, monthly_limit: +e.target.value })}
            style={inputStyle} />
        </Field>
        <Field label="Timezone">
          <select value={settings.timezone}
            onChange={(e) => setSettings({ ...settings, timezone: e.target.value })}
            style={inputStyle}>
            {TIMEZONES.map((tz) => <option key={tz} value={tz}>{tz}</option>)}
          </select>
        </Field>

        <div className="stat-label" style={{ marginTop: 8 }}>Alert Thresholds (% of daily budget)</div>
        <Field label="Warning at 50%">
          <input type="number" step="0.05" min="0" max="1" value={settings.alert_threshold_50}
            onChange={(e) => setSettings({ ...settings, alert_threshold_50: +e.target.value })}
            style={inputStyle} />
        </Field>
        <Field label="High alert at 80%">
          <input type="number" step="0.05" min="0" max="1" value={settings.alert_threshold_80}
            onChange={(e) => setSettings({ ...settings, alert_threshold_80: +e.target.value })}
            style={inputStyle} />
        </Field>
        <Field label="Critical at 100%">
          <input type="number" step="0.05" min="0" max="2" value={settings.alert_threshold_100}
            onChange={(e) => setSettings({ ...settings, alert_threshold_100: +e.target.value })}
            style={inputStyle} />
        </Field>

        <Toggle label="Sudden spike detection" checked={settings.spike_detection_enabled}
          onChange={(v) => setSettings({ ...settings, spike_detection_enabled: v })} />
        <Toggle label="Expensive model warnings" checked={settings.expensive_model_warning}
          onChange={(v) => setSettings({ ...settings, expensive_model_warning: v })} />

        <button onClick={save} style={{
          marginTop: 8, padding: "12px 20px", borderRadius: 10, fontWeight: 600, fontSize: 13,
          background: "linear-gradient(135deg, var(--accent), #8b5cf6)",
          border: "none", color: "#fff",
        }}>
          {saved ? "✓ Saved" : "Save Settings"}
        </button>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label style={{ fontSize: 12, color: "var(--text-secondary)", display: "block", marginBottom: 6 }}>{label}</label>
      {children}
    </div>
  );
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
      <span style={{ fontSize: 13 }}>{label}</span>
      <button onClick={() => onChange(!checked)} style={{
        width: 44, height: 24, borderRadius: 12, padding: 2,
        background: checked ? "var(--accent)" : "rgba(255,255,255,0.1)",
        border: "1px solid var(--border-subtle)", transition: "background 0.2s",
      }}>
        <div style={{
          width: 18, height: 18, borderRadius: "50%", background: "#fff",
          transform: checked ? "translateX(20px)" : "translateX(0)",
          transition: "transform 0.2s",
        }} />
      </button>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  width: "100%", padding: "10px 12px", borderRadius: 8,
  background: "rgba(0,0,0,0.3)", border: "1px solid var(--border-subtle)",
  fontSize: 13, outline: "none",
};
