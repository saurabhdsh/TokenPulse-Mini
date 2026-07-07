use super::traits::{AdapterError, ProviderAdapter};
use crate::aws_config::{fetch_bedrock_cost_events, validate_aws_credentials, ResolvedAwsCredentials};
use crate::models::UsageEvent;
use chrono::{NaiveDate, TimeZone, Utc};
use uuid::Uuid;

pub struct BedrockAdapter;

impl ProviderAdapter for BedrockAdapter {
    fn provider_name(&self) -> &'static str {
        "AWS Bedrock"
    }

    fn mock_fetch_usage(&self) -> Vec<UsageEvent> {
        vec![UsageEvent {
            id: None,
            provider: "AWS Bedrock".into(),
            model: "anthropic.claude-3-5-sonnet".into(),
            prompt_tokens: 1800,
            completion_tokens: 650,
            total_tokens: 2450,
            input_cost: 0.0054,
            output_cost: 0.00975,
            total_cost: 0.01515,
            project_name: Some("analytics-pipeline".into()),
            request_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now().to_rfc3339(),
        }]
    }
}

impl BedrockAdapter {
    pub fn fetch_usage_with_creds(
        &self,
        creds: &ResolvedAwsCredentials,
    ) -> Result<Vec<UsageEvent>, AdapterError> {
        validate_aws_credentials(creds).map_err(AdapterError::ApiError)?;

        let days = fetch_bedrock_cost_events(creds, 30).map_err(AdapterError::ApiError)?;
        if days.is_empty() {
            return Err(AdapterError::ApiError(
                "No Bedrock cost data in the last 30 days. Cost Explorer can take up to 24h to update."
                    .into(),
            ));
        }

        let mut events = Vec::new();
        for day in days {
            if day.cost <= 0.0 && day.models.is_empty() {
                continue;
            }

            if day.models.is_empty() {
                events.push(cost_day_event(&day.date, "bedrock.aggregated", day.cost, day.usage_quantity));
                continue;
            }

            for model in day.models {
                if model.cost <= 0.0 {
                    continue;
                }
                let model_name = usage_type_to_model(&model.usage_type);
                events.push(cost_day_event(
                    &day.date,
                    &model_name,
                    model.cost,
                    model.usage_quantity,
                ));
            }
        }

        if events.is_empty() {
            return Err(AdapterError::ApiError(
                "Bedrock credentials valid but no billable usage found in the last 30 days.".into(),
            ));
        }

        Ok(events)
    }
}

fn cost_day_event(date: &str, model: &str, cost: f64, usage_quantity: f64) -> UsageEvent {
    let tokens = usage_quantity.max(0.0).round() as i64;
    let timestamp = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|d| Utc.from_utc_datetime(&d.and_hms_opt(12, 0, 0).unwrap()).to_rfc3339())
        .unwrap_or_else(|_| Utc::now().to_rfc3339());

    UsageEvent {
        id: None,
        provider: "AWS Bedrock".into(),
        model: model.into(),
        prompt_tokens: tokens,
        completion_tokens: 0,
        total_tokens: tokens,
        input_cost: cost,
        output_cost: 0.0,
        total_cost: cost,
        project_name: None,
        request_id: None,
        timestamp,
    }
}

fn usage_type_to_model(usage_type: &str) -> String {
    let lower = usage_type.to_lowercase();
    if lower.contains("claude") {
        return "anthropic.claude-3-5-sonnet".into();
    }
    if lower.contains("llama") {
        return "meta.llama3".into();
    }
    if lower.contains("titan") {
        return "amazon.titan-text".into();
    }
    if lower.contains("mistral") {
        return "mistral.mistral-large".into();
    }
    format!("bedrock.{usage_type}")
}
