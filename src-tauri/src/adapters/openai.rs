use super::traits::{AdapterError, ProviderAdapter};
use crate::engine::CostEngine;
use crate::env::{get_openai_admin_key, get_openai_org_id};
use crate::models::UsageEvent;
use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

pub struct OpenAIAdapter;

#[derive(Debug, Deserialize)]
struct UsagePage {
    data: Vec<UsageBucket>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageBucket {
    start_time: i64,
    end_time: i64,
    results: Vec<UsageResult>,
}

#[derive(Debug, Deserialize)]
struct UsageResult {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    num_model_requests: i64,
    model: Option<String>,
    project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorBody {
    error: Option<OpenAIErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
}

impl ProviderAdapter for OpenAIAdapter {
    fn provider_name(&self) -> &'static str {
        "OpenAI"
    }

    fn fetch_usage(&self, api_key: Option<&str>) -> Result<Vec<UsageEvent>, AdapterError> {
        let api_key = api_key
            .filter(|k| !k.is_empty())
            .ok_or(AdapterError::MissingApiKey)?;
        self.fetch_usage_with_creds(api_key, None, get_openai_org_id().as_deref())
    }

    fn mock_fetch_usage(&self) -> Vec<UsageEvent> {
        let now = Utc::now().to_rfc3339();
        vec![
            UsageEvent {
                id: None,
                provider: "OpenAI".into(),
                model: "gpt-4o".into(),
                prompt_tokens: 12_500,
                completion_tokens: 3_200,
                total_tokens: 15_700,
                input_cost: 0.03125,
                output_cost: 0.032,
                total_cost: 0.06325,
                project_name: Some("chat-widget".into()),
                request_id: Some(Uuid::new_v4().to_string()),
                timestamp: now.clone(),
            },
            UsageEvent {
                id: None,
                provider: "OpenAI".into(),
                model: "gpt-4o-mini".into(),
                prompt_tokens: 48_000,
                completion_tokens: 9_100,
                total_tokens: 57_100,
                input_cost: 0.0072,
                output_cost: 0.00546,
                total_cost: 0.01266,
                project_name: Some("code-assist".into()),
                request_id: Some(Uuid::new_v4().to_string()),
                timestamp: now,
            },
        ]
    }
}

impl OpenAIAdapter {
    pub fn fetch_usage_with_creds(
        &self,
        api_key: &str,
        admin_key: Option<&str>,
        org_id: Option<&str>,
    ) -> Result<Vec<UsageEvent>, AdapterError> {
        validate_api_key(api_key, org_id)?;

        let usage_key = admin_key.filter(|k| !k.is_empty()).ok_or_else(|| {
            AdapterError::ApiError(usage_admin_key_required_message())
        })?;

        let pricing = default_openai_pricing();
        fetch_usage_events(usage_key, org_id, &pricing)
    }
}

pub fn usage_admin_key_required_message() -> String {
    "OpenAI usage sync requires an Admin Key with the api.usage.read scope. \
     Save it in API Keys → Admin Key, or set OPENAI_ADMIN_KEY in your environment. \
     Create one at platform.openai.com → Settings → Organization → Admin keys."
        .to_string()
}

fn format_usage_permission_error(api_message: &str) -> String {
    if api_message.contains("api.usage.read") {
        format!(
            "Your Admin Key is missing the api.usage.read scope. Create a new Admin key at \
             platform.openai.com (Organization → Admin keys) with Usage read enabled, \
             then save it in API Keys → Admin Key."
        )
    } else {
        format!(
            "Usage API access denied — check your Admin Key in API Keys settings. \
             ({api_message})"
        )
    }
}

pub fn widget_openai_sync_hint(message: &str) -> String {
    if message.contains("api.usage.read") || message.contains("Admin Key") {
        "Usage sync needs Admin Key (api.usage.read) · see API Keys".to_string()
    } else if message.len() > 72 {
        format!("{}…", &message[..69])
    } else {
        message.to_string()
    }
}

pub fn validate_openai_key(api_key: &str) -> Result<String, AdapterError> {
    let org_id = get_openai_org_id();
    validate_api_key(api_key, org_id.as_deref())?;
    Ok(if get_openai_admin_key().is_some() {
        "API key valid · Admin key detected for usage API".into()
    } else {
        "API key valid · Add Admin Key in API Keys for usage history (api.usage.read)".into()
    })
}

fn validate_api_key(api_key: &str, org_id: Option<&str>) -> Result<(), AdapterError> {
    let client = http_client();
    let mut headers = auth_headers(api_key, org_id);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = client
        .get("https://api.openai.com/v1/models")
        .headers(headers)
        .send()
        .map_err(|e| AdapterError::ApiError(e.to_string()))?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let body = response.text().unwrap_or_default();
    let message = serde_json::from_str::<OpenAIErrorBody>(&body)
        .ok()
        .and_then(|b| b.error.map(|e| e.message))
        .unwrap_or_else(|| body.clone());

    if status.as_u16() == 429 {
        return Err(AdapterError::RateLimited);
    }

    Err(AdapterError::ApiError(format!("{status}: {message}")))
}

fn fetch_usage_events(
    usage_key: &str,
    org_id: Option<&str>,
    pricing: &HashMap<String, (f64, f64)>,
) -> Result<Vec<UsageEvent>, AdapterError> {
    let start_time = (Utc::now() - chrono::Duration::days(30)).timestamp();
    let buckets = fetch_all_buckets(usage_key, org_id, start_time, "1d")?;

    if buckets.is_empty() {
        return Err(AdapterError::ApiError(
            "No usage data returned for the last 30 days. Confirm your Admin Key has \
             api.usage.read and the correct organization is selected."
                .into(),
        ));
    }

    let mut events = Vec::new();
    for bucket in buckets {
        let ts = bucket_timestamp(bucket.start_time);
        for result in bucket.results {
            let model = result
                .model
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| "unknown".into());
            let prompt_tokens = result.input_tokens;
            let completion_tokens = result.output_tokens;
            if prompt_tokens == 0 && completion_tokens == 0 {
                continue;
            }
            let (input_price, output_price) = pricing
                .get(&model)
                .or_else(|| pricing.get(&normalize_model_name(&model)))
                .copied()
                .unwrap_or((2.5, 10.0));
            let (input_cost, output_cost, total_cost) = CostEngine::calculate_cost(
                prompt_tokens,
                completion_tokens,
                input_price,
                output_price,
            );
            let request_id = format!(
                "openai-{}-{}-{}",
                bucket.start_time,
                model,
                result.num_model_requests
            );

            events.push(UsageEvent {
                id: None,
                provider: "OpenAI".into(),
                model,
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
                input_cost,
                output_cost,
                total_cost,
                project_name: result.project_id,
                request_id: Some(request_id),
                timestamp: ts.clone(),
            });
        }
    }

    if events.is_empty() {
        return Err(AdapterError::ApiError(
            "Usage API connected but returned zero token events for the last 30 days.".into(),
        ));
    }

    Ok(events)
}

fn fetch_all_buckets(
    usage_key: &str,
    org_id: Option<&str>,
    start_time: i64,
    bucket_width: &str,
) -> Result<Vec<UsageBucket>, AdapterError> {
    let client = http_client();
    let mut all_buckets = Vec::new();
    let mut page: Option<String> = None;

    loop {
        let mut query = vec![
            ("start_time", start_time.to_string()),
            ("bucket_width", bucket_width.to_string()),
            ("group_by", "model".to_string()),
            ("limit", "31".to_string()),
        ];
        if let Some(ref cursor) = page {
            query.push(("page", cursor.clone()));
        }

        let response = client
            .get("https://api.openai.com/v1/organization/usage/completions")
            .headers(auth_headers(usage_key, org_id))
            .query(&query)
            .send()
            .map_err(|e| AdapterError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            let message = serde_json::from_str::<OpenAIErrorBody>(&body)
                .ok()
                .and_then(|b| b.error.map(|e| e.message))
                .unwrap_or(body);

            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(AdapterError::ApiError(format_usage_permission_error(&message)));
            }
            return Err(AdapterError::ApiError(format!("{status}: {message}")));
        }

        let page_body: UsagePage = response
            .json()
            .map_err(|e| AdapterError::ApiError(e.to_string()))?;
        all_buckets.extend(page_body.data);

        if page_body.has_more {
            page = page_body.next_page;
        } else {
            break;
        }
    }

    Ok(all_buckets)
}

fn bucket_timestamp(start_time: i64) -> String {
    DateTime::<Utc>::from_timestamp(start_time, 0)
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn normalize_model_name(model: &str) -> String {
    model.split('@').next().unwrap_or(model).to_string()
}

fn default_openai_pricing() -> HashMap<String, (f64, f64)> {
    HashMap::from([
        ("gpt-4o".into(), (2.5, 10.0)),
        ("gpt-4o-mini".into(), (0.15, 0.6)),
        ("gpt-4o-2024-08-06".into(), (2.5, 10.0)),
        ("gpt-4o-mini-2024-07-18".into(), (0.15, 0.6)),
        ("o1-preview".into(), (15.0, 60.0)),
        ("o1-mini".into(), (3.0, 12.0)),
        ("o3-mini".into(), (1.1, 4.4)),
        ("gpt-4.1".into(), (2.0, 8.0)),
        ("gpt-4.1-mini".into(), (0.4, 1.6)),
        ("gpt-4.1-nano".into(), (0.1, 0.4)),
    ])
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("http client")
}

fn auth_headers(api_key: &str, org_id: Option<&str>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {api_key}")).expect("auth header"),
    );
    if let Some(org) = org_id.filter(|o| !o.is_empty()) {
        headers.insert(
            "OpenAI-Organization",
            HeaderValue::from_str(org).expect("org header"),
        );
    }
    headers
}
