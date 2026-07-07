import { useEffect, useState, type ReactNode } from "react";
import { api, PROVIDER_COLORS } from "../lib/api";
import { ProviderBadge } from "../components/ProviderBadge";
import type {
  AwsCredentialsStatus,
  CredentialFieldStatus,
  EnvDetection,
  OpenAICredentialsStatus,
  Provider,
} from "../types";

type OpenAIField = "api_key" | "admin_key" | "billing_token" | "org_id";
type AwsField = "access_key_id" | "secret_access_key" | "session_token" | "region" | "profile";

const LIVE_PROVIDERS = new Set(["OpenAI", "AWS Bedrock"]);

function providerStatusLabel(provider: Provider): string {
  if (provider.sync_status === "connected") {
    if (provider.name === "AWS Bedrock") return "Live · synced from AWS Cost Explorer";
    return "Live · synced from OpenAI API";
  }
  if (provider.sync_status === "error") return provider.sync_message ?? "Sync error";

  if (provider.name === "OpenAI") {
    if (provider.key_source === "app" || provider.key_source === "manual") return "API key saved in app";
    if (provider.key_source === "env") return "Using OPENAI_API_KEY from macOS env";
  }
  if (provider.name === "AWS Bedrock" && provider.is_enabled) {
    return "Configure AWS credentials below or run `aws configure`";
  }
  if (provider.is_enabled) return "No credentials configured";
  return "Disabled";
}

function sourceBadge(source: CredentialFieldStatus["source"]) {
  if (source === "app") return { text: "Saved in app", color: "var(--accent)" };
  if (source === "env") return { text: "From macOS env", color: "var(--success)" };
  if (source === "cli") return { text: "From AWS CLI", color: "#ff9900" };
  return { text: "Not set", color: "var(--text-muted)" };
}

export function ApiKeySettingsPage() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [env, setEnv] = useState<EnvDetection | null>(null);
  const [openaiCreds, setOpenaiCreds] = useState<OpenAICredentialsStatus | null>(null);
  const [awsCreds, setAwsCreds] = useState<AwsCredentialsStatus | null>(null);
  const [openaiDrafts, setOpenaiDrafts] = useState<Record<OpenAIField, string>>({
    api_key: "",
    admin_key: "",
    billing_token: "",
    org_id: "",
  });
  const [awsDrafts, setAwsDrafts] = useState<Record<AwsField, string>>({
    access_key_id: "",
    secret_access_key: "",
    session_token: "",
    region: "",
    profile: "",
  });
  const [savedOpenAI, setSavedOpenAI] = useState<OpenAIField | null>(null);
  const [savedAws, setSavedAws] = useState<AwsField | null>(null);
  const [syncing, setSyncing] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  const load = async () => {
    const [detected, providerList, openai, aws] = await Promise.all([
      api.detectEnvKeys(),
      api.getProviders(),
      api.getOpenAICredentials(),
      api.getAwsCredentials(),
    ]);
    setEnv(detected);
    setProviders(providerList);
    setOpenaiCreds(openai);
    setAwsCreds(aws);
    setOpenaiDrafts({ api_key: "", admin_key: "", billing_token: "", org_id: "" });
    setAwsDrafts({
      access_key_id: "",
      secret_access_key: "",
      session_token: "",
      region: "",
      profile: "",
    });
  };

  useEffect(() => {
    load().catch(console.error);
  }, []);

  const saveOpenAI = async (field: OpenAIField) => {
    const value = openaiDrafts[field];
    if (!value.trim()) return;
    setSaveError(null);
    try {
      await api.updateOpenAICredentials({ [field]: value.trim() });
      setSavedOpenAI(field);
      setTimeout(() => setSavedOpenAI(null), 2000);
      await load();
      const creds = await api.getOpenAICredentials();
      if (field === "admin_key" && !creds.active_admin.configured) {
        throw new Error("Admin key did not persist. Please try saving again.");
      }
      await api.refreshLiveData().catch(console.error);
      await load();
    } catch (e) {
      setSaveError(String(e));
    }
  };

  const clearOpenAI = async (field: OpenAIField) => {
    await api.updateOpenAICredentials({ [field]: "" });
    setOpenaiDrafts((d) => ({ ...d, [field]: "" }));
    await load();
  };

  const saveAws = async (field: AwsField) => {
    const value = awsDrafts[field];
    if (!value.trim()) return;
    await api.updateAwsCredentials({ [field]: value.trim() });
    setSavedAws(field);
    setTimeout(() => setSavedAws(null), 2000);
    await load();
    await api.refreshLiveData().catch(console.error);
    await load();
  };

  const clearAws = async (field: AwsField) => {
    await api.updateAwsCredentials({ [field]: "" });
    setAwsDrafts((d) => ({ ...d, [field]: "" }));
    await load();
  };

  const toggle = async (name: string) => {
    const provider = providers.find((p) => p.name === name);
    if (!provider) return;
    await api.updateProviderKey({
      provider_name: name,
      api_key: "",
      is_enabled: !provider.is_enabled,
    });
    await load();
  };

  const syncNow = async () => {
    setSyncing(true);
    try {
      await api.refreshLiveData();
      await load();
    } finally {
      setSyncing(false);
    }
  };

  const openai = providers.find((p) => p.name === "OpenAI");
  const bedrock = providers.find((p) => p.name === "AWS Bedrock");
  const mockProviders = providers.filter((p) => !LIVE_PROVIDERS.has(p.name));

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <header style={{ marginBottom: 24, display: "flex", justifyContent: "space-between", gap: 12 }}>
        <div>
          <h1 style={{ fontSize: 22, fontWeight: 700 }}>API Key Settings</h1>
          <p style={{ fontSize: 13, color: "var(--text-secondary)", marginTop: 4 }}>
            Save credentials in the app, or use macOS env vars / AWS CLI config
          </p>
        </div>
        <button onClick={syncNow} disabled={syncing} style={{
          padding: "10px 16px", borderRadius: 10, fontSize: 12, fontWeight: 600, alignSelf: "flex-start",
          background: "rgba(108,140,255,0.15)", border: "1px solid rgba(108,140,255,0.25)",
          color: "var(--accent)", opacity: syncing ? 0.6 : 1,
        }}>
          {syncing ? "Syncing…" : "↻ Sync Live Data"}
        </button>
      </header>

      {saveError && (
        <div style={{
          marginBottom: 16, padding: "10px 12px", borderRadius: 8, maxWidth: 720,
          background: "rgba(239,68,68,0.1)", border: "1px solid rgba(239,68,68,0.25)",
          fontSize: 12, color: "#fca5a5",
        }}>
          {saveError}
        </div>
      )}

      {openaiCreds && openai && (
        <ProviderCredentialCard
          provider={openai}
          title="OpenAI"
          onToggle={() => toggle("OpenAI")}
          onOpenWidget={() => api.openProviderWidget("OpenAI")}
          extra={
            <>
              {openaiCreds.active_admin.configured ? (
                <div style={{
                  marginBottom: 12, padding: "10px 12px", borderRadius: 8,
                  background: openaiCreds.active_admin.key_type === "admin"
                    ? "rgba(52,211,153,0.08)" : "rgba(251,191,36,0.08)",
                  border: `1px solid ${openaiCreds.active_admin.key_type === "admin"
                    ? "rgba(52,211,153,0.2)" : "rgba(251,191,36,0.25)"}`,
                }}>
                  <div style={{
                    fontSize: 11, fontWeight: 600,
                    color: openaiCreds.active_admin.key_type === "admin" ? "var(--success)" : "var(--warning)",
                  }}>
                    Admin Key active ({openaiCreds.active_admin.source === "app" ? "saved in app" : "from macOS env"})
                    {openaiCreds.active_admin.preview ? ` · ${openaiCreds.active_admin.preview}` : ""}
                    {openaiCreds.active_admin.key_type ? ` · type: ${openaiCreds.active_admin.key_type}` : ""}
                  </div>
                  {openaiCreds.active_admin.key_type === "project" && (
                    <div style={{ fontSize: 10, color: "var(--warning)", marginTop: 4 }}>
                      This looks like a project API key — usage sync needs an sk-admin-… Admin Key.
                    </div>
                  )}
                </div>
              ) : (
                <div style={{
                  marginBottom: 12, padding: "10px 12px", borderRadius: 8,
                  background: "rgba(251,191,36,0.08)", border: "1px solid rgba(251,191,36,0.2)",
                  fontSize: 11, color: "var(--warning)",
                }}>
                  No Admin Key configured yet — save one below for live usage sync.
                </div>
              )}
              {openai.credit ? (
                <div style={{ marginBottom: 16, padding: "10px 12px", borderRadius: 8, background: "rgba(52,211,153,0.08)", border: "1px solid rgba(52,211,153,0.2)" }}>
                  <div style={{ fontSize: 11, fontWeight: 600, color: "var(--success)" }}>
                    ${openai.credit.available.toFixed(2)}{" "}
                    {openai.credit.source === "subscription_limit" ? "monthly limit remaining" : "credits available"}
                  </div>
                </div>
              ) : null}
            </>
          }
        >
          <CredentialRow field={openaiCreds.api_key} value={openaiDrafts.api_key} onChange={(v) => setOpenaiDrafts({ ...openaiDrafts, api_key: v })} onSave={() => saveOpenAI("api_key")} onClear={() => clearOpenAI("api_key")} saved={savedOpenAI === "api_key"} required />
          <CredentialRow field={openaiCreds.admin_key} value={openaiDrafts.admin_key} onChange={(v) => setOpenaiDrafts({ ...openaiDrafts, admin_key: v })} onSave={() => saveOpenAI("admin_key")} onClear={() => clearOpenAI("admin_key")} saved={savedOpenAI === "admin_key"} />
          <CredentialRow field={openaiCreds.billing_token} value={openaiDrafts.billing_token} onChange={(v) => setOpenaiDrafts({ ...openaiDrafts, billing_token: v })} onSave={() => saveOpenAI("billing_token")} onClear={() => clearOpenAI("billing_token")} saved={savedOpenAI === "billing_token"} />
          <CredentialRow field={openaiCreds.org_id} value={openaiDrafts.org_id} onChange={(v) => setOpenaiDrafts({ ...openaiDrafts, org_id: v })} onSave={() => saveOpenAI("org_id")} onClear={() => clearOpenAI("org_id")} saved={savedOpenAI === "org_id"} mono={false} password={false} />
        </ProviderCredentialCard>
      )}

      {awsCreds && bedrock && (
        <ProviderCredentialCard
          provider={bedrock}
          title="AWS Bedrock"
          onToggle={() => toggle("AWS Bedrock")}
          onOpenWidget={() => api.openProviderWidget("AWS Bedrock")}
          extra={(
            <div style={{ marginBottom: 12, fontSize: 11, color: "var(--text-muted)", lineHeight: 1.5 }}>
              {awsCreds.aws_cli_configured ? "✓ ~/.aws/credentials found" : "No ~/.aws/credentials file"}
              {" · "}
              {awsCreds.aws_cli_available ? "AWS CLI installed" : "Install AWS CLI (`brew install awscli`) for live sync"}
            </div>
          )}
        >
          <CredentialRow field={awsCreds.access_key_id} value={awsDrafts.access_key_id} onChange={(v) => setAwsDrafts({ ...awsDrafts, access_key_id: v })} onSave={() => saveAws("access_key_id")} onClear={() => clearAws("access_key_id")} saved={savedAws === "access_key_id"} required />
          <CredentialRow field={awsCreds.secret_access_key} value={awsDrafts.secret_access_key} onChange={(v) => setAwsDrafts({ ...awsDrafts, secret_access_key: v })} onSave={() => saveAws("secret_access_key")} onClear={() => clearAws("secret_access_key")} saved={savedAws === "secret_access_key"} required />
          <CredentialRow field={awsCreds.session_token} value={awsDrafts.session_token} onChange={(v) => setAwsDrafts({ ...awsDrafts, session_token: v })} onSave={() => saveAws("session_token")} onClear={() => clearAws("session_token")} saved={savedAws === "session_token"} />
          <CredentialRow field={awsCreds.region} value={awsDrafts.region} onChange={(v) => setAwsDrafts({ ...awsDrafts, region: v })} onSave={() => saveAws("region")} onClear={() => clearAws("region")} saved={savedAws === "region"} mono={false} password={false} placeholder="us-east-1" />
          <CredentialRow field={awsCreds.profile} value={awsDrafts.profile} onChange={(v) => setAwsDrafts({ ...awsDrafts, profile: v })} onSave={() => saveAws("profile")} onClear={() => clearAws("profile")} saved={savedAws === "profile"} mono={false} password={false} placeholder="default" />
        </ProviderCredentialCard>
      )}

      {env && (
        <div className="glass-card-sm" style={{ padding: 14, marginBottom: 16, maxWidth: 720, fontSize: 12 }}>
          <div className="stat-label" style={{ marginBottom: 8 }}>macOS environment variables only</div>
          <p style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 8 }}>
            Keys saved in the app above work independently. &quot;Not found&quot; here is normal if you only saved in-app.
          </p>
          <div style={{ display: "grid", gap: 6 }}>
            <EnvRow label="OPENAI_API_KEY" ok={env.openai_api_key} probe={env.openai_api_probe} />
            <EnvRow
              label="OPENAI_ADMIN_KEY"
              ok={env.openai_admin_key}
              probe={env.openai_admin_probe}
              warnIfType="project"
              warnMessage="Wrong key type — need sk-admin-… Admin Key, not project API key"
            />
            <EnvRow label="AWS_ACCESS_KEY_ID" ok={env.aws_access_key_id} />
            <EnvRow label="AWS_SECRET_ACCESS_KEY" ok={env.aws_secret_access_key} />
            <EnvRow label="AWS_REGION" ok={env.aws_region} />
            <EnvRow label="AWS_PROFILE" ok={env.aws_profile} />
            <EnvRow label="~/.aws/credentials" ok={env.aws_cli_configured} />
            <EnvRow label="aws CLI" ok={env.aws_cli_available} />
          </div>
        </div>
      )}

      {mockProviders.length > 0 && (
        <div style={{ maxWidth: 720 }}>
          <div className="stat-label" style={{ marginBottom: 10 }}>Other Providers (mock)</div>
          <div style={{ display: "grid", gap: 12 }}>
            {mockProviders.map((p) => (
              <div key={p.name} className="glass-card-sm" style={{ padding: 16, opacity: 0.85 }}>
                <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                  <ProviderBadge provider={p.name} size="sm" active={p.is_enabled} />
                  <div style={{ flex: 1, fontWeight: 600, color: PROVIDER_COLORS[p.name] }}>{p.name}</div>
                  <button onClick={() => toggle(p.name)} style={{
                    padding: "6px 12px", borderRadius: 8, fontSize: 11,
                    background: p.is_enabled ? "rgba(52,211,153,0.15)" : "rgba(255,255,255,0.05)",
                    border: `1px solid ${p.is_enabled ? "rgba(52,211,153,0.3)" : "var(--border-subtle)"}`,
                    color: p.is_enabled ? "var(--success)" : "var(--text-muted)",
                  }}>
                    {p.is_enabled ? "ON" : "OFF"}
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function ProviderCredentialCard({
  provider,
  title,
  onToggle,
  onOpenWidget,
  extra,
  children,
}: {
  provider: Provider;
  title: string;
  onToggle: () => void;
  onOpenWidget?: () => void;
  extra?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="glass-card-sm" style={{ padding: 18, marginBottom: 16, maxWidth: 720 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16 }}>
        <ProviderBadge provider={title} size="sm" active={provider.is_enabled} />
        <div style={{ flex: 1 }}>
          <div style={{ fontWeight: 600, color: PROVIDER_COLORS[title] }}>{title}</div>
          <div style={{ fontSize: 11, color: provider.sync_status === "connected" ? "var(--success)" : "var(--text-muted)" }}>
            {providerStatusLabel(provider)}
          </div>
        </div>
        <button onClick={onToggle} style={{
          padding: "6px 12px", borderRadius: 8, fontSize: 11,
          background: provider.is_enabled ? "rgba(52,211,153,0.15)" : "rgba(255,255,255,0.05)",
          border: `1px solid ${provider.is_enabled ? "rgba(52,211,153,0.3)" : "var(--border-subtle)"}`,
          color: provider.is_enabled ? "var(--success)" : "var(--text-muted)",
        }}>
          {provider.is_enabled ? "ON" : "OFF"}
        </button>
        {onOpenWidget && (
          <button onClick={onOpenWidget} style={{
            padding: "6px 10px", borderRadius: 8, fontSize: 11, fontWeight: 600,
            background: "rgba(108,140,255,0.15)", border: "1px solid rgba(108,140,255,0.25)",
            color: "var(--accent)",
          }}>
            Open Widget
          </button>
        )}
      </div>
      {extra}
      <div style={{ display: "grid", gap: 12 }}>{children}</div>
    </div>
  );
}

function CredentialRow({
  field,
  value,
  onChange,
  onSave,
  onClear,
  saved,
  required,
  mono = true,
  password = true,
  placeholder,
}: {
  field: CredentialFieldStatus;
  value: string;
  onChange: (v: string) => void;
  onSave: () => void;
  onClear: () => void;
  saved: boolean;
  required?: boolean;
  mono?: boolean;
  password?: boolean;
  placeholder?: string;
}) {
  const badge = sourceBadge(field.source);
  const canClear = field.source === "app";

  return (
    <div style={{ padding: 12, borderRadius: 10, background: "rgba(0,0,0,0.2)", border: "1px solid var(--border-subtle)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, marginBottom: 4 }}>
        <div style={{ fontSize: 12, fontWeight: 600 }}>
          {field.label}{required ? " *" : ""}
        </div>
        <span style={{ fontSize: 10, color: badge.color, fontWeight: 600 }}>{badge.text}</span>
      </div>
      <div style={{ fontSize: 10, color: "var(--text-muted)", marginBottom: 8 }}>
        {field.hint} · env: <span className="mono">{field.env_var}</span>
      </div>
      {field.preview && (
        <div className="mono" style={{ fontSize: 10, color: "var(--text-secondary)", marginBottom: 8 }}>
          Active: {field.preview}
        </div>
      )}
      <div style={{ display: "flex", gap: 8 }}>
        <input
          type={password ? "password" : "text"}
          placeholder={placeholder ?? `Paste ${field.label.toLowerCase()}…`}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          style={{
            flex: 1, padding: "10px 12px", borderRadius: 8,
            background: "rgba(0,0,0,0.3)", border: "1px solid var(--border-subtle)",
            fontSize: 12, fontFamily: mono ? "var(--font-mono)" : "inherit", outline: "none",
          }}
        />
        <button
          onClick={onSave}
          disabled={!value.trim()}
          style={{
            padding: "10px 14px", borderRadius: 8, fontSize: 12, fontWeight: 600,
            background: "rgba(108,140,255,0.2)", border: "1px solid rgba(108,140,255,0.3)",
            color: "var(--accent)", opacity: value.trim() ? 1 : 0.5,
          }}
        >
          {saved ? "✓" : "Save"}
        </button>
        {canClear && (
          <button
            onClick={onClear}
            style={{
              padding: "10px 12px", borderRadius: 8, fontSize: 11,
              background: "rgba(255,255,255,0.05)", border: "1px solid var(--border-subtle)",
              color: "var(--text-muted)",
            }}
          >
            Clear
          </button>
        )}
      </div>
    </div>
  );
}

function EnvRow({
  label,
  ok,
  probe,
  warnIfType,
  warnMessage,
}: {
  label: string;
  ok: boolean;
  probe?: { process: boolean; launchctl: boolean; shell_profile: boolean; key_type: string | null };
  warnIfType?: string;
  warnMessage?: string;
}) {
  const warn = warnIfType && probe?.key_type === warnIfType;
  return (
    <div style={{ display: "grid", gap: 2 }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12 }}>
        <span className="mono">{label}</span>
        <span style={{ color: ok ? "var(--success)" : warn ? "var(--warning)" : "var(--text-muted)" }}>
          {ok ? (probe?.key_type ? `detected (${probe.key_type})` : "detected") : "not found"}
        </span>
      </div>
      {probe && (
        <div style={{ fontSize: 10, color: "var(--text-muted)", paddingLeft: 2 }}>
          shell: {probe.shell_profile ? "yes" : "no"} · launchctl: {probe.launchctl ? "yes" : "no"} · process: {probe.process ? "yes" : "no"}
        </div>
      )}
      {warn && warnMessage && (
        <div style={{ fontSize: 10, color: "var(--warning)" }}>{warnMessage}</div>
      )}
    </div>
  );
}
