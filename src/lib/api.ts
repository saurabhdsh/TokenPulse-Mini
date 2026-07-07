import { invoke } from "@tauri-apps/api/core";
import type {
  Alert,
  BudgetSettings,
  DashboardData,
  ModelCost,
  ModelPricing,
  Provider,
  ProviderCost,
  UpdateBudgetPayload,
  UsageEvent,
  WidgetStats,
  SyncReport,
  EnvDetection,
  OpenAICredentialsStatus,
  UpdateOpenAICredentialsPayload,
  AwsCredentialsStatus,
  UpdateAwsCredentialsPayload,
} from "../types";

export type {
  UpdateBudgetPayload,
  SyncReport,
  EnvDetection,
  OpenAICredentialsStatus,
  UpdateOpenAICredentialsPayload,
  AwsCredentialsStatus,
  UpdateAwsCredentialsPayload,
};

export const api = {
  getWidgetStats: (provider?: string) =>
    invoke<WidgetStats>("get_widget_stats", { provider: provider ?? null }),
  getWidgetDemoEnabled: () => invoke<boolean>("get_widget_demo_enabled"),
  setWidgetDemoEnabled: (enabled: boolean) =>
    invoke<void>("set_widget_demo_enabled", { enabled }),
  getDashboardStats: () => invoke<DashboardData>("get_dashboard_stats"),
  getProviders: () => invoke<Provider[]>("get_providers"),
  updateProviderKey: (payload: {
    provider_name: string;
    api_key: string;
    is_enabled: boolean;
  }) => invoke<void>("update_provider_key", { payload }),
  getModels: () => invoke<ModelPricing[]>("get_models"),
  updateModelPricing: (payload: {
    id: number;
    input_price_per_million: number;
    output_price_per_million: number;
    is_expensive: boolean;
  }) => invoke<void>("update_model_pricing", { payload }),
  getBudgetSettings: () => invoke<BudgetSettings>("get_budget_settings"),
  updateBudgetSettings: (payload: UpdateBudgetPayload) =>
    invoke<void>("update_budget_settings", { payload }),
  getUsageHistory: (limit?: number, offset?: number) =>
    invoke<UsageEvent[]>("get_usage_history", { limit, offset }),
  getProviderBreakdown: () => invoke<ProviderCost[]>("get_provider_breakdown"),
  getModelBreakdown: () => invoke<ModelCost[]>("get_model_breakdown"),
  getAlerts: (limit?: number) => invoke<Alert[]>("get_alerts", { limit }),
  markAlertRead: (id: number) => invoke<void>("mark_alert_read", { id }),
  syncProviderUsage: () => invoke<SyncReport[]>("sync_provider_usage"),
  refreshLiveData: () => invoke<SyncReport[]>("refresh_live_data"),
  detectEnvKeys: () => invoke<EnvDetection>("detect_env_keys"),
  getOpenAICredentials: () => invoke<OpenAICredentialsStatus>("get_openai_credentials"),
  updateOpenAICredentials: (payload: UpdateOpenAICredentialsPayload) =>
    invoke<void>("update_openai_credentials_cmd", {
      apiKey: payload.api_key ?? null,
      adminKey: payload.admin_key ?? null,
      billingToken: payload.billing_token ?? null,
      orgId: payload.org_id ?? null,
      payload,
    }),
  getAwsCredentials: () => invoke<AwsCredentialsStatus>("get_aws_credentials"),
  updateAwsCredentials: (payload: UpdateAwsCredentialsPayload) =>
    invoke<void>("update_aws_credentials_cmd", { payload }),
  openProviderWidget: (provider: string) =>
    invoke<void>("open_provider_widget", { provider }),
  openMainDashboard: () => invoke<void>("open_main_dashboard"),
  listWidgetProviders: () => invoke<string[]>("list_widget_providers"),
  setAlwaysOnTop: (pinned: boolean) =>
    invoke<void>("set_always_on_top", { pinned }),
  setWindowMode: (mode: "widget" | "expanded") =>
    invoke<void>("set_window_mode", { mode }),
};

export function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

export function formatCost(n: number): string {
  if (n >= 100) return `$${n.toFixed(0)}`;
  if (n >= 1) return `$${n.toFixed(2)}`;
  return `$${n.toFixed(4)}`;
}

export function formatTime(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export const PROVIDER_COLORS: Record<string, string> = {
  OpenAI: "#10a37f",
  Anthropic: "#d4a574",
  "AWS Bedrock": "#ff9900",
  "Azure OpenAI": "#0078d4",
  Gemini: "#4285f4",
};

export const PROVIDER_SHORT: Record<string, string> = {
  OpenAI: "OAI",
  Anthropic: "ANT",
  "AWS Bedrock": "AWS",
  "Azure OpenAI": "AZ",
  Gemini: "GEM",
};
