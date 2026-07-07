import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { ModelPricing } from "../types";

export function ModelPricingPage() {
  const [models, setModels] = useState<ModelPricing[]>([]);
  const [editing, setEditing] = useState<number | null>(null);
  const [editValues, setEditValues] = useState({ input: 0, output: 0, expensive: false });

  useEffect(() => {
    api.getModels().then(setModels).catch(console.error);
  }, []);

  const startEdit = (m: ModelPricing) => {
    setEditing(m.id);
    setEditValues({
      input: m.input_price_per_million,
      output: m.output_price_per_million,
      expensive: m.is_expensive,
    });
  };

  const save = async (id: number) => {
    await api.updateModelPricing({
      id,
      input_price_per_million: editValues.input,
      output_price_per_million: editValues.output,
      is_expensive: editValues.expensive,
    });
    setModels((prev) => prev.map((m) =>
      m.id === id
        ? { ...m, input_price_per_million: editValues.input, output_price_per_million: editValues.output, is_expensive: editValues.expensive }
        : m
    ));
    setEditing(null);
  };

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <header style={{ marginBottom: 24 }}>
        <h1 style={{ fontSize: 22, fontWeight: 700 }}>Model Pricing</h1>
        <p style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 4 }}>
          Per-million token pricing for cost calculation
        </p>
      </header>

      <div className="glass-card-sm" style={{ overflow: "hidden" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
          <thead>
            <tr style={{ borderBottom: "1px solid var(--border-subtle)" }}>
              {["Provider", "Model", "Input $/M", "Output $/M", "Expensive", ""].map((h) => (
                <th key={h} style={{
                  padding: "10px 14px", textAlign: "left", fontSize: 10,
                  fontWeight: 600, letterSpacing: "0.06em", color: "var(--text-muted)",
                  textTransform: "uppercase",
                }}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {models.map((m) => (
              <tr key={m.id} style={{ borderBottom: "1px solid var(--border-subtle)" }}>
                <td style={{ padding: "10px 14px" }}>{m.provider_name}</td>
                <td style={{ padding: "10px 14px", fontFamily: "var(--font-mono)", fontSize: 11 }}>{m.model_name}</td>
                <td style={{ padding: "10px 14px" }}>
                  {editing === m.id ? (
                    <input type="number" step="0.01" value={editValues.input}
                      onChange={(e) => setEditValues({ ...editValues, input: +e.target.value })}
                      style={{ width: 80, padding: 4, borderRadius: 4, background: "rgba(0,0,0,0.3)", border: "1px solid var(--border-subtle)" }} />
                  ) : `$${m.input_price_per_million.toFixed(2)}`}
                </td>
                <td style={{ padding: "10px 14px" }}>
                  {editing === m.id ? (
                    <input type="number" step="0.01" value={editValues.output}
                      onChange={(e) => setEditValues({ ...editValues, output: +e.target.value })}
                      style={{ width: 80, padding: 4, borderRadius: 4, background: "rgba(0,0,0,0.3)", border: "1px solid var(--border-subtle)" }} />
                  ) : `$${m.output_price_per_million.toFixed(2)}`}
                </td>
                <td style={{ padding: "10px 14px" }}>
                  {editing === m.id ? (
                    <input type="checkbox" checked={editValues.expensive}
                      onChange={(e) => setEditValues({ ...editValues, expensive: e.target.checked })} />
                  ) : m.is_expensive ? "⚠️" : "—"}
                </td>
                <td style={{ padding: "10px 14px" }}>
                  {editing === m.id ? (
                    <button onClick={() => save(m.id)} style={{ fontSize: 11, color: "var(--accent)" }}>Save</button>
                  ) : (
                    <button onClick={() => startEdit(m)} style={{ fontSize: 11, color: "var(--text-muted)" }}>Edit</button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
