use super::traits::{AdapterError, ProviderAdapter};
use crate::azure_config::{
    self, list_deployments, validate_azure_openai_credentials, AzureMetricDay,
    ResolvedAzureOpenAICredentials,
};
use crate::models::UsageEvent;
use chrono::{NaiveDate, TimeZone, Utc};
use uuid::Uuid;

pub struct AzureOpenAIAdapter;

impl ProviderAdapter for AzureOpenAIAdapter {
    fn provider_name(&self) -> &'static str {
        "Azure OpenAI"
    }

    fn mock_fetch_usage(&self) -> Vec<UsageEvent> {
        vec![UsageEvent {
            id: None,
            provider: "Azure OpenAI".into(),
            model: "gpt-4o-mini".into(),
            prompt_tokens: 5200,
            completion_tokens: 980,
            total_tokens: 6180,
            input_cost: 0.00078,
            output_cost: 0.000588,
            total_cost: 0.001368,
            project_name: Some("tokenpulse-api".into()),
            request_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now().to_rfc3339(),
        }]
    }
}

impl AzureOpenAIAdapter {
    pub fn fetch_usage_with_creds(
        &self,
        creds: &ResolvedAzureOpenAICredentials,
    ) -> Result<Vec<UsageEvent>, AdapterError> {
        validate_azure_openai_credentials(creds).map_err(AdapterError::ApiError)?;
        let deployments = list_deployments(creds).map_err(AdapterError::ApiError)?;

        let mut events = match azure_config::fetch_azure_token_metrics(creds, 30) {
            Ok(metrics) if !metrics.is_empty() => metrics
                .into_iter()
                .map(|day| metric_day_event(&day))
                .collect(),
            Ok(_) => Vec::new(),
            Err(_) => Vec::new(),
        };

        if events.is_empty() {
            events = deployment_placeholder_events(&deployments);
        }

        if events.is_empty() {
            return Err(AdapterError::ApiError(
                "Azure OpenAI connected but no usage data available. Set subscription/resource group and run `az login` for token metrics.".into(),
            ));
        }

        Ok(events)
    }
}

fn deployment_placeholder_events(deployments: &[azure_config::AzureDeployment]) -> Vec<UsageEvent> {
    let timestamp = Utc::now().to_rfc3339();
    deployments
        .iter()
        .map(|deployment| UsageEvent {
            id: None,
            provider: "Azure OpenAI".into(),
            model: deployment.model.clone(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            input_cost: 0.0,
            output_cost: 0.0,
            total_cost: 0.0,
            project_name: Some(format!("deployment:{}", deployment.id)),
            request_id: Some(Uuid::new_v4().to_string()),
            timestamp: timestamp.clone(),
        })
        .collect()
}

fn metric_day_event(day: &AzureMetricDay) -> UsageEvent {
    let timestamp = day
        .date
        .parse::<NaiveDate>()
        .ok()
        .and_then(|d| d.and_hms_opt(12, 0, 0))
        .map(|dt| Utc.from_utc_datetime(&dt).to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    let prompt = (day.tokens as f64 * 0.7) as i64;
    let completion = day.tokens - prompt;

    UsageEvent {
        id: None,
        provider: "Azure OpenAI".into(),
        model: day.model.clone(),
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: day.tokens,
        input_cost: 0.0,
        output_cost: 0.0,
        total_cost: 0.0,
        project_name: Some("azure-monitor".into()),
        request_id: Some(Uuid::new_v4().to_string()),
        timestamp,
    }
}
