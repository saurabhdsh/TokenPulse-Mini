use crate::env;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

pub const DEFAULT_API_VERSION: &str = "2024-10-21";

#[derive(Debug, Clone)]
pub struct ResolvedAzureOpenAICredentials {
    pub endpoint: String,
    pub api_key: String,
    pub api_version: String,
    pub deployment_name: Option<String>,
    pub subscription_id: Option<String>,
    pub resource_group: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct AzureDeployment {
    pub id: String,
    pub model: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AzureMetricDay {
    pub date: String,
    pub model: String,
    pub tokens: i64,
}

pub fn get_endpoint() -> Option<String> {
    env::get_var("AZURE_OPENAI_ENDPOINT")
}

pub fn get_api_key() -> Option<String> {
    env::get_var("AZURE_OPENAI_API_KEY")
        .or_else(|| env::get_var("AZURE_OPENAI_KEY"))
}

pub fn get_api_version() -> Option<String> {
    env::get_var("AZURE_OPENAI_API_VERSION")
}

pub fn get_deployment_name() -> Option<String> {
    env::get_var("AZURE_OPENAI_DEPLOYMENT_NAME")
        .or_else(|| env::get_var("AZURE_OPENAI_DEPLOYMENT"))
}

pub fn get_subscription_id() -> Option<String> {
    env::get_var("AZURE_SUBSCRIPTION_ID")
}

pub fn get_resource_group() -> Option<String> {
    env::get_var("AZURE_OPENAI_RESOURCE_GROUP")
        .or_else(|| env::get_var("AZURE_RESOURCE_GROUP"))
}

pub fn normalize_endpoint(endpoint: &str) -> String {
    endpoint.trim().trim_end_matches('/').to_string()
}

pub fn resource_name_from_endpoint(endpoint: &str) -> Option<String> {
    let endpoint = normalize_endpoint(endpoint);
    let host = endpoint
        .strip_prefix("https://")
        .or_else(|| endpoint.strip_prefix("http://"))?;
    let name = host.split('.').next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub fn resolve_azure_openai_credentials(
    app_api_key: Option<String>,
    app_endpoint: Option<String>,
    app_api_version: Option<String>,
    app_deployment: Option<String>,
    app_subscription_id: Option<String>,
    app_resource_group: Option<String>,
) -> Result<ResolvedAzureOpenAICredentials, String> {
    if let (Some(endpoint), Some(api_key)) = (app_endpoint, app_api_key) {
        if !endpoint.trim().is_empty() && !api_key.trim().is_empty() {
            return Ok(ResolvedAzureOpenAICredentials {
                endpoint: normalize_endpoint(&endpoint),
                api_key: api_key.trim().to_string(),
                api_version: app_api_version
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or_else(|| DEFAULT_API_VERSION.to_string()),
                deployment_name: app_deployment.filter(|v| !v.trim().is_empty()),
                subscription_id: app_subscription_id.filter(|v| !v.trim().is_empty()),
                resource_group: app_resource_group.filter(|v| !v.trim().is_empty()),
                source: "app".into(),
            });
        }
    }

    let endpoint = get_endpoint().ok_or_else(|| {
        "Azure OpenAI endpoint not found. Save AZURE_OPENAI_ENDPOINT in API Key Settings or your environment.".to_string()
    })?;
    let api_key = get_api_key().ok_or_else(|| {
        "Azure OpenAI API key not found. Save AZURE_OPENAI_API_KEY in API Key Settings or your environment.".to_string()
    })?;

    Ok(ResolvedAzureOpenAICredentials {
        endpoint: normalize_endpoint(&endpoint),
        api_key,
        api_version: app_api_version
            .or_else(get_api_version)
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_API_VERSION.to_string()),
        deployment_name: app_deployment
            .or_else(get_deployment_name)
            .filter(|v| !v.trim().is_empty()),
        subscription_id: app_subscription_id
            .or_else(get_subscription_id)
            .filter(|v| !v.trim().is_empty()),
        resource_group: app_resource_group
            .or_else(get_resource_group)
            .filter(|v| !v.trim().is_empty()),
        source: "env".into(),
    })
}

pub fn validate_azure_openai_credentials(creds: &ResolvedAzureOpenAICredentials) -> Result<String, String> {
    let deployments = list_deployments(creds)?;
    let resource = resource_name_from_endpoint(&creds.endpoint).unwrap_or_else(|| "resource".into());
    Ok(format!(
        "Azure OpenAI · {} · {} deployment(s)",
        resource,
        deployments.len()
    ))
}

pub fn list_deployments(creds: &ResolvedAzureOpenAICredentials) -> Result<Vec<AzureDeployment>, String> {
    let url = format!(
        "{}/openai/deployments?api-version={}",
        creds.endpoint, creds.api_version
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "api-key",
        HeaderValue::from_str(creds.api_key.trim()).map_err(|e| e.to_string())?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .map_err(|e| format!("Azure OpenAI request failed: {e}"))?;

    let status = response.status();
    let body = response.text().map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(parse_azure_error(status.as_u16(), &body));
    }

    let parsed: DeploymentsResponse =
        serde_json::from_str(&body).map_err(|e| format!("Invalid Azure deployments response: {e}"))?;

    let mut deployments: Vec<AzureDeployment> = parsed
        .data
        .into_iter()
        .map(|d| AzureDeployment {
            id: d.id,
            model: d.model,
            status: d.status,
        })
        .collect();

    if let Some(name) = creds.deployment_name.as_deref() {
        deployments.retain(|d| d.id == name);
        if deployments.is_empty() {
            validate_deployment_exists(creds, name)?;
            deployments.push(AzureDeployment {
                id: name.to_string(),
                model: name.to_string(),
                status: Some("validated".into()),
            });
        }
    }

    if deployments.is_empty() {
        return Err(
            "No Azure OpenAI deployments found for this endpoint. Create a deployment in Azure AI Foundry."
                .into(),
        );
    }

    Ok(deployments)
}

fn validate_deployment_exists(creds: &ResolvedAzureOpenAICredentials, name: &str) -> Result<(), String> {
    let url = format!(
        "{}/openai/deployments/{}?api-version={}",
        creds.endpoint, name, creds.api_version
    );

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "api-key",
        HeaderValue::from_str(creds.api_key.trim()).map_err(|e| e.to_string())?,
    );

    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .map_err(|e| format!("Azure deployment check failed: {e}"))?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().unwrap_or_default();
    Err(parse_azure_error(status.as_u16(), &body))
}

pub fn fetch_azure_token_metrics(creds: &ResolvedAzureOpenAICredentials, days: i64) -> Result<Vec<AzureMetricDay>, String> {
    if !az_cli_available() {
        return Err("Azure CLI not installed. Install with `brew install azure-cli` and run `az login` for token metrics.".into());
    }

    let subscription_id = creds
        .subscription_id
        .clone()
        .or_else(get_subscription_id)
        .ok_or_else(|| {
            "AZURE_SUBSCRIPTION_ID not set. Add it in API Key Settings for Azure Monitor usage sync.".to_string()
        })?;

    let resource_group = creds
        .resource_group
        .clone()
        .or_else(get_resource_group)
        .ok_or_else(|| {
            "AZURE_OPENAI_RESOURCE_GROUP not set. Add it in API Key Settings for Azure Monitor usage sync.".to_string()
        })?;

    let resource_name = resource_name_from_endpoint(&creds.endpoint).ok_or_else(|| {
        "Could not parse Azure resource name from endpoint URL.".to_string()
    })?;

    let resource_id = format!(
        "/subscriptions/{subscription_id}/resourceGroups/{resource_group}/providers/Microsoft.CognitiveServices/accounts/{resource_name}"
    );

    let end = chrono::Utc::now();
    let start = end - chrono::Duration::days(days);

    let output = Command::new("az")
        .args([
            "monitor",
            "metrics",
            "list",
            "--resource",
            &resource_id,
            "--metric",
            "TokenTransaction",
            "--aggregation",
            "Total",
            "--interval",
            "P1D",
            "--start-time",
            &start.to_rfc3339(),
            "--end-time",
            &end.to_rfc3339(),
            "-o",
            "json",
        ])
        .output()
        .map_err(|e| format!("Failed to run Azure CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Azure Monitor metrics failed. Run `az login` and verify subscription/resource group. {stderr}"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: MetricsListResponse =
        serde_json::from_str(&stdout).map_err(|e| format!("Invalid Azure metrics JSON: {e}"))?;

    let mut days_out = Vec::new();
    for metric in parsed.value {
        for series in metric.timeseries {
            for point in series.data {
                if let (Some(total), Some(ts)) = (point.total, point.time_stamp) {
                    if total <= 0.0 {
                        continue;
                    }
                    let date = ts.chars().take(10).collect::<String>();
                    days_out.push(AzureMetricDay {
                        date,
                        model: "azure-openai.aggregated".into(),
                        tokens: total as i64,
                    });
                }
            }
        }
    }

    Ok(days_out)
}

pub fn az_cli_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| {
        Command::new("az")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

fn parse_azure_error(status: u16, body: &str) -> String {
    if let Ok(parsed) = serde_json::from_str::<AzureErrorResponse>(body) {
        if let Some(error) = parsed.error {
            return format!("Azure OpenAI API error ({status}): {}", error.message);
        }
    }
    if body.len() > 240 {
        format!("Azure OpenAI API error ({status}): {}…", &body[..240])
    } else if body.is_empty() {
        format!("Azure OpenAI API error ({status})")
    } else {
        format!("Azure OpenAI API error ({status}): {body}")
    }
}

#[derive(Debug, Deserialize)]
struct DeploymentsResponse {
    #[serde(default)]
    data: Vec<DeploymentItem>,
}

#[derive(Debug, Deserialize)]
struct DeploymentItem {
    id: String,
    model: String,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureErrorResponse {
    error: Option<AzureErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct AzureErrorDetail {
    message: String,
}

#[derive(Debug, Deserialize)]
struct MetricsListResponse {
    #[serde(default)]
    value: Vec<MetricValue>,
}

#[derive(Debug, Deserialize)]
struct MetricValue {
    #[serde(default)]
    timeseries: Vec<MetricSeries>,
}

#[derive(Debug, Deserialize)]
struct MetricSeries {
    #[serde(default)]
    data: Vec<MetricPoint>,
}

#[derive(Debug, Deserialize)]
struct MetricPoint {
    #[serde(rename = "timeStamp")]
    time_stamp: Option<String>,
    total: Option<f64>,
}
