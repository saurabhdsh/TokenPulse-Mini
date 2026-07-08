use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub api_key: Option<String>,
    pub api_key_preview: Option<String>,
    pub is_enabled: bool,
    pub key_source: Option<String>,
    pub sync_status: Option<String>,
    pub sync_message: Option<String>,
    pub last_synced_at: Option<String>,
    pub credit: Option<CreditBalance>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBalance {
    pub available: f64,
    pub granted: Option<f64>,
    pub used: Option<f64>,
    pub monthly_limit: Option<f64>,
    pub month_spend: Option<f64>,
    pub source: String,
    pub currency: String,
    pub synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub id: i64,
    pub provider_id: i64,
    pub provider_name: String,
    pub model_name: String,
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
    pub is_expensive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: Option<i64>,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub project_name: Option<String>,
    pub request_id: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    pub id: i64,
    pub date: String,
    pub provider: String,
    pub model: Option<String>,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub event_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetSettings {
    pub id: i64,
    pub daily_limit: f64,
    pub monthly_limit: f64,
    pub timezone: String,
    pub alert_threshold_50: f64,
    pub alert_threshold_80: f64,
    pub alert_threshold_100: f64,
    pub spike_detection_enabled: bool,
    pub expensive_model_warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: i64,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
    pub is_read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetStats {
    pub today_tokens: i64,
    pub today_cost: f64,
    pub burn_rate_per_hour: f64,
    pub top_provider: String,
    pub top_model: String,
    pub budget_risk: String,
    pub daily_budget_used_pct: f64,
    pub daily_budget_limit: f64,
    pub monthly_estimated: f64,
    pub sparkline: Vec<HourlyPoint>,
    pub provider_breakdown: Vec<ProviderCost>,
    pub is_demo_data: bool,
    pub show_demo_overlay: bool,
    pub sync_hint: Option<String>,
    pub live_providers: Vec<String>,
    pub openai_credit: Option<CreditBalance>,
    pub focused_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyPoint {
    pub hour: String,
    pub tokens: i64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCost {
    pub provider: String,
    pub cost: f64,
    pub tokens: i64,
    pub pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCost {
    pub model: String,
    pub provider: String,
    pub cost: f64,
    pub tokens: i64,
    pub request_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodStats {
    pub total_tokens: i64,
    pub total_cost: f64,
    pub input_cost: f64,
    pub output_cost: f64,
    pub event_count: i64,
    pub burn_rate_per_hour: f64,
    pub estimated_monthly: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardProviderSummary {
    pub name: String,
    pub sync_status: Option<String>,
    pub sync_message: Option<String>,
    pub last_synced_at: Option<String>,
    pub credit: Option<CreditBalance>,
    pub today_cost: f64,
    pub today_tokens: i64,
    pub week_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub today: PeriodStats,
    pub week: PeriodStats,
    pub month: PeriodStats,
    pub budget: BudgetSettings,
    pub providers: Vec<ProviderCost>,
    pub models: Vec<ModelCost>,
    pub alerts: Vec<Alert>,
    pub live_providers: Vec<String>,
    pub is_demo_data: bool,
    pub provider_summaries: Vec<DashboardProviderSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBudgetPayload {
    pub daily_limit: f64,
    pub monthly_limit: f64,
    pub timezone: String,
    pub alert_threshold_50: f64,
    pub alert_threshold_80: f64,
    pub alert_threshold_100: f64,
    pub spike_detection_enabled: bool,
    pub expensive_model_warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProviderKeyPayload {
    pub provider_name: String,
    pub api_key: String,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateModelPricingPayload {
    pub id: i64,
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
    pub is_expensive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncReport {
    pub provider: String,
    pub events_synced: i64,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSyncFinished {
    pub ok: bool,
    pub reports: Vec<SyncReport>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvDetection {
    pub openai_api_key: bool,
    pub openai_admin_key: bool,
    pub openai_org_id: bool,
    pub openai_billing_token: bool,
    pub openai_api_probe: crate::env::EnvVarProbe,
    pub openai_admin_probe: crate::env::EnvVarProbe,
    pub aws_access_key_id: bool,
    pub aws_secret_access_key: bool,
    pub aws_region: bool,
    pub aws_profile: bool,
    pub aws_cli_configured: bool,
    pub aws_cli_available: bool,
    pub azure_openai_api_key: bool,
    pub azure_openai_endpoint: bool,
    pub azure_openai_api_version: bool,
    pub azure_openai_deployment: bool,
    pub azure_subscription_id: bool,
    pub azure_resource_group: bool,
    pub azure_cli_available: bool,
    pub applied_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialFieldStatus {
    pub field: String,
    pub label: String,
    pub hint: String,
    pub env_var: String,
    pub is_configured: bool,
    pub source: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveCredentialSummary {
    pub configured: bool,
    pub source: String,
    pub key_type: Option<String>,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICredentialsStatus {
    pub api_key: CredentialFieldStatus,
    pub admin_key: CredentialFieldStatus,
    pub billing_token: CredentialFieldStatus,
    pub org_id: CredentialFieldStatus,
    pub active_admin: ActiveCredentialSummary,
}

#[derive(Debug, Clone)]
pub struct ResolvedOpenAICredentials {
    pub api_key: Option<String>,
    pub admin_key: Option<String>,
    pub billing_token: Option<String>,
    pub org_id: Option<String>,
}

pub type ResolvedAwsCredentials = crate::aws_config::ResolvedAwsCredentials;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsCredentialsStatus {
    pub access_key_id: CredentialFieldStatus,
    pub secret_access_key: CredentialFieldStatus,
    pub session_token: CredentialFieldStatus,
    pub region: CredentialFieldStatus,
    pub profile: CredentialFieldStatus,
    pub aws_cli_configured: bool,
    pub aws_cli_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAwsCredentialsPayload {
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub region: Option<String>,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureCredentialsStatus {
    pub api_key: CredentialFieldStatus,
    pub endpoint: CredentialFieldStatus,
    pub api_version: CredentialFieldStatus,
    pub deployment_name: CredentialFieldStatus,
    pub subscription_id: CredentialFieldStatus,
    pub resource_group: CredentialFieldStatus,
    pub azure_cli_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateAzureCredentialsPayload {
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub api_version: Option<String>,
    pub deployment_name: Option<String>,
    pub subscription_id: Option<String>,
    pub resource_group: Option<String>,
}

impl Default for UpdateAzureCredentialsPayload {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: None,
            api_version: None,
            deployment_name: None,
            subscription_id: None,
            resource_group: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateOpenAICredentialsPayload {
    pub api_key: Option<String>,
    pub admin_key: Option<String>,
    pub billing_token: Option<String>,
    pub org_id: Option<String>,
}

impl Default for UpdateOpenAICredentialsPayload {
    fn default() -> Self {
        Self {
            api_key: None,
            admin_key: None,
            billing_token: None,
            org_id: None,
        }
    }
}
