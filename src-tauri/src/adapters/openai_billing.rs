use super::traits::AdapterError;
use crate::models::CreditBalance;
use chrono::Utc;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct CreditGrantsResponse {
    total_granted: Option<f64>,
    total_used: Option<f64>,
    total_available: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SubscriptionResponse {
    hard_limit_usd: Option<f64>,
    soft_limit_usd: Option<f64>,
    system_hard_limit_usd: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CostsPage {
    data: Vec<CostBucket>,
}

#[derive(Debug, Deserialize)]
struct CostBucket {
    results: Vec<CostResult>,
}

#[derive(Debug, Deserialize)]
struct CostResult {
    amount: Option<CostAmount>,
}

#[derive(Debug, Deserialize)]
struct CostAmount {
    value: Option<f64>,
    currency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorBody {
    error: Option<OpenAIErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct OpenAIErrorDetail {
    message: String,
}

/// Fetch OpenAI prepaid credits or subscription remaining balance.
pub fn fetch_credit_balance(
    api_key: &str,
    admin_key: Option<&str>,
    billing_token: Option<&str>,
    org_id: Option<&str>,
) -> Result<CreditBalance, AdapterError> {
    let now = Utc::now().to_rfc3339();

    let mut keys_to_try: Vec<String> = Vec::new();
    if let Some(token) = billing_token.filter(|k| !k.is_empty()) {
        keys_to_try.push(token.to_string());
    }
    if let Some(admin) = admin_key.filter(|k| !k.is_empty()) {
        keys_to_try.push(admin.to_string());
    }
    keys_to_try.push(api_key.to_string());

    let mut last_error = String::from("No billing credentials available");

    for key in &keys_to_try {
        if key.is_empty() {
            continue;
        }
        match fetch_prepaid_credits(key, org_id) {
            Ok(balance) => {
                return Ok(CreditBalance {
                    synced_at: now,
                    ..balance
                });
            }
            Err(AdapterError::ApiError(msg)) => last_error = msg,
            Err(e) => return Err(e),
        }

        match fetch_subscription_remaining(key, org_id) {
            Ok(balance) => {
                return Ok(CreditBalance {
                    synced_at: now,
                    ..balance
                });
            }
            Err(AdapterError::ApiError(msg)) => last_error = msg,
            Err(e) => return Err(e),
        }
    }

    Err(AdapterError::ApiError(format!(
        "{last_error}. For prepaid credits set OPENAI_BILLING_TOKEN (dashboard session token) or OPENAI_ADMIN_KEY."
    )))
}

fn fetch_prepaid_credits(key: &str, org_id: Option<&str>) -> Result<CreditBalance, AdapterError> {
    let client = http_client();
    let response = client
        .get("https://api.openai.com/dashboard/billing/credit_grants")
        .headers(auth_headers(key, org_id))
        .send()
        .map_err(|e| AdapterError::ApiError(e.to_string()))?;

    if !response.status().is_success() {
        return Err(parse_api_error(response));
    }

    let body: CreditGrantsResponse = response
        .json()
        .map_err(|e| AdapterError::ApiError(e.to_string()))?;

    let available = body
        .total_available
        .ok_or_else(|| AdapterError::ApiError("credit_grants missing total_available".into()))?;

    Ok(CreditBalance {
        available,
        granted: body.total_granted,
        used: body.total_used,
        monthly_limit: None,
        month_spend: None,
        source: "prepaid_credits".into(),
        currency: "USD".into(),
        synced_at: String::new(),
    })
}

fn fetch_subscription_remaining(key: &str, org_id: Option<&str>) -> Result<CreditBalance, AdapterError> {
    let client = http_client();
    let sub_response = client
        .get("https://api.openai.com/v1/dashboard/billing/subscription")
        .headers(auth_headers(key, org_id))
        .send()
        .map_err(|e| AdapterError::ApiError(e.to_string()))?;

    if !sub_response.status().is_success() {
        return Err(parse_api_error(sub_response));
    }

    let sub: SubscriptionResponse = sub_response
        .json()
        .map_err(|e| AdapterError::ApiError(e.to_string()))?;

    let limit = sub
        .hard_limit_usd
        .or(sub.system_hard_limit_usd)
        .or(sub.soft_limit_usd)
        .ok_or_else(|| AdapterError::ApiError("subscription missing spend limit".into()))?;

    let month_start = month_start_timestamp();
    let spend = fetch_month_spend(key, org_id, month_start)?;

    Ok(CreditBalance {
        available: (limit - spend).max(0.0),
        granted: Some(limit),
        used: Some(spend),
        monthly_limit: Some(limit),
        month_spend: Some(spend),
        source: "subscription_limit".into(),
        currency: "USD".into(),
        synced_at: String::new(),
    })
}

fn fetch_month_spend(key: &str, org_id: Option<&str>, start_time: i64) -> Result<f64, AdapterError> {
    let client = http_client();
    let response = client
        .get("https://api.openai.com/v1/organization/costs")
        .headers(auth_headers(key, org_id))
        .query(&[
            ("start_time", start_time.to_string()),
            ("bucket_width", "1d".into()),
            ("limit", "31".into()),
        ])
        .send()
        .map_err(|e| AdapterError::ApiError(e.to_string()))?;

    if !response.status().is_success() {
        return Err(parse_api_error(response));
    }

    let page: CostsPage = response
        .json()
        .map_err(|e| AdapterError::ApiError(e.to_string()))?;

    let total: f64 = page
        .data
        .iter()
        .flat_map(|b| b.results.iter())
        .filter_map(|r| r.amount.as_ref().and_then(|a| a.value))
        .sum();

    Ok(total)
}

fn month_start_timestamp() -> i64 {
    use chrono::{Datelike, TimeZone};
    let now = Utc::now();
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .unwrap()
        .timestamp()
}

fn parse_api_error(response: reqwest::blocking::Response) -> AdapterError {
    let status = response.status();
    let body = response.text().unwrap_or_default();
    let message = serde_json::from_str::<OpenAIErrorBody>(&body)
        .ok()
        .and_then(|b| b.error.map(|e| e.message))
        .unwrap_or(body);
    AdapterError::ApiError(format!("{status}: {message}"))
}

fn http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(20))
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
