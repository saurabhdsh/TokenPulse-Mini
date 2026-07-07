use super::traits::ProviderAdapter;
use crate::models::UsageEvent;
use chrono::Utc;
use uuid::Uuid;

pub struct AnthropicAdapter;

impl ProviderAdapter for AnthropicAdapter {
    fn provider_name(&self) -> &'static str {
        "Anthropic"
    }

    fn mock_fetch_usage(&self) -> Vec<UsageEvent> {
        vec![UsageEvent {
            id: None,
            provider: "Anthropic".into(),
            model: "claude-sonnet-4".into(),
            prompt_tokens: 4100,
            completion_tokens: 1200,
            total_tokens: 5300,
            input_cost: 0.0123,
            output_cost: 0.018,
            total_cost: 0.0303,
            project_name: Some("doc-summarizer".into()),
            request_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now().to_rfc3339(),
        }]
    }
}
