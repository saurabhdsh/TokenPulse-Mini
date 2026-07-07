use super::traits::ProviderAdapter;
use crate::models::UsageEvent;
use chrono::Utc;
use uuid::Uuid;

pub struct GeminiAdapter;

impl ProviderAdapter for GeminiAdapter {
    fn provider_name(&self) -> &'static str {
        "Gemini"
    }

    fn mock_fetch_usage(&self) -> Vec<UsageEvent> {
        vec![UsageEvent {
            id: None,
            provider: "Gemini".into(),
            model: "gemini-2.0-flash".into(),
            prompt_tokens: 8900,
            completion_tokens: 2100,
            total_tokens: 11000,
            input_cost: 0.00089,
            output_cost: 0.00084,
            total_cost: 0.00173,
            project_name: Some("code-assist".into()),
            request_id: Some(Uuid::new_v4().to_string()),
            timestamp: Utc::now().to_rfc3339(),
        }]
    }
}
