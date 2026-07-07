export interface WidgetStats {
  today_tokens: number;
  today_cost: number;
  burn_rate_per_hour: number;
  top_provider: string;
  top_model: string;
  budget_risk: string;
  daily_budget_used_pct: number;
  daily_budget_limit: number;
  monthly_estimated: number;
  sparkline: HourlyPoint[];
  provider_breakdown: ProviderCost[];
  is_demo_data: boolean;
  show_demo_overlay: boolean;
  sync_hint: string | null;
  live_providers: string[];
  openai_credit: CreditBalance | null;
  focused_provider: string | null;
}

export interface HourlyPoint {
  hour: string;
  tokens: number;
  cost: number;
}

export interface ProviderCost {
  provider: string;
  cost: number;
  tokens: number;
  pct: number;
}

export interface ModelCost {
  model: string;
  provider: string;
  cost: number;
  tokens: number;
  request_count: number;
}

export interface PeriodStats {
  total_tokens: number;
  total_cost: number;
  input_cost: number;
  output_cost: number;
  event_count: number;
  burn_rate_per_hour: number;
  estimated_monthly: number;
}

export interface CreditBalance {
  available: number;
  granted: number | null;
  used: number | null;
  monthly_limit: number | null;
  month_spend: number | null;
  source: string;
  currency: string;
  synced_at: string;
}

export interface Provider {
  id: number;
  name: string;
  api_key: string | null;
  api_key_preview: string | null;
  is_enabled: boolean;
  key_source: string | null;
  sync_status: string | null;
  sync_message: string | null;
  last_synced_at: string | null;
  credit: CreditBalance | null;
  created_at: string;
}

export interface SyncReport {
  provider: string;
  events_synced: number;
  status: string;
  message: string;
}

export interface EnvVarProbe {
  process: boolean;
  launchctl: boolean;
  shell_profile: boolean;
  key_type: string | null;
}

export interface EnvDetection {
  openai_api_key: boolean;
  openai_admin_key: boolean;
  openai_org_id: boolean;
  openai_billing_token: boolean;
  openai_api_probe: EnvVarProbe;
  openai_admin_probe: EnvVarProbe;
  aws_access_key_id: boolean;
  aws_secret_access_key: boolean;
  aws_region: boolean;
  aws_profile: boolean;
  aws_cli_configured: boolean;
  aws_cli_available: boolean;
  applied_keys: string[];
}

export interface AwsCredentialsStatus {
  access_key_id: CredentialFieldStatus;
  secret_access_key: CredentialFieldStatus;
  session_token: CredentialFieldStatus;
  region: CredentialFieldStatus;
  profile: CredentialFieldStatus;
  aws_cli_configured: boolean;
  aws_cli_available: boolean;
}

export interface CredentialFieldStatus {
  field: string;
  label: string;
  hint: string;
  env_var: string;
  is_configured: boolean;
  source: "app" | "env" | "cli" | "none";
  preview: string | null;
}

export interface ActiveCredentialSummary {
  configured: boolean;
  source: string;
  key_type: string | null;
  preview: string | null;
}

export interface OpenAICredentialsStatus {
  api_key: CredentialFieldStatus;
  admin_key: CredentialFieldStatus;
  billing_token: CredentialFieldStatus;
  org_id: CredentialFieldStatus;
  active_admin: ActiveCredentialSummary;
}

export interface ModelPricing {
  id: number;
  provider_id: number;
  provider_name: string;
  model_name: string;
  input_price_per_million: number;
  output_price_per_million: number;
  is_expensive: boolean;
}

export interface BudgetSettings {
  id: number;
  daily_limit: number;
  monthly_limit: number;
  timezone: string;
  alert_threshold_50: number;
  alert_threshold_80: number;
  alert_threshold_100: number;
  spike_detection_enabled: boolean;
  expensive_model_warning: boolean;
}

export interface UsageEvent {
  id?: number;
  provider: string;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  input_cost: number;
  output_cost: number;
  total_cost: number;
  project_name: string | null;
  request_id: string | null;
  timestamp: string;
}

export interface Alert {
  id: number;
  alert_type: string;
  severity: string;
  message: string;
  provider: string | null;
  model: string | null;
  value: number | null;
  threshold: number | null;
  is_read: boolean;
  created_at: string;
}

export interface DashboardData {
  today: PeriodStats;
  week: PeriodStats;
  month: PeriodStats;
  budget: BudgetSettings;
  providers: ProviderCost[];
  models: ModelCost[];
  alerts: Alert[];
  live_providers: string[];
  is_demo_data: boolean;
  provider_summaries: DashboardProviderSummary[];
}

export interface DashboardProviderSummary {
  name: string;
  sync_status: string | null;
  sync_message: string | null;
  last_synced_at: string | null;
  credit: CreditBalance | null;
  today_cost: number;
  today_tokens: number;
  week_cost: number;
}

export interface UpdateBudgetPayload {
  daily_limit: number;
  monthly_limit: number;
  timezone: string;
  alert_threshold_50: number;
  alert_threshold_80: number;
  alert_threshold_100: number;
  spike_detection_enabled: boolean;
  expensive_model_warning: boolean;
}

export interface UpdateOpenAICredentialsPayload {
  api_key?: string | null;
  admin_key?: string | null;
  billing_token?: string | null;
  org_id?: string | null;
}

export interface UpdateAwsCredentialsPayload {
  access_key_id?: string | null;
  secret_access_key?: string | null;
  session_token?: string | null;
  region?: string | null;
  profile?: string | null;
}

export type ViewMode = "widget" | "expanded";

export type Page =
  | "widget"
  | "dashboard"
  | "providers"
  | "models"
  | "budget"
  | "api-keys"
  | "pricing"
  | "history";
