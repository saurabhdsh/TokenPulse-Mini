use super::traits::ProviderAdapter;
use crate::models::UsageEvent;
use chrono::Utc;
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
