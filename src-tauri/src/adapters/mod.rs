mod traits;

pub use traits::ProviderAdapter;

mod openai;
mod openai_billing;
mod anthropic;
mod bedrock;
mod azure;
mod gemini;

pub use openai::{usage_admin_key_required_message, widget_openai_sync_hint, OpenAIAdapter};
pub use openai_billing::fetch_credit_balance;
pub use anthropic::AnthropicAdapter;
pub use bedrock::BedrockAdapter;
pub use azure::AzureOpenAIAdapter;
pub use gemini::GeminiAdapter;

use crate::models::UsageEvent;

pub fn all_adapters() -> Vec<Box<dyn ProviderAdapter>> {
    vec![
        Box::new(OpenAIAdapter),
        Box::new(AnthropicAdapter),
        Box::new(BedrockAdapter),
        Box::new(AzureOpenAIAdapter),
        Box::new(GeminiAdapter),
    ]
}

pub fn fetch_all_mock() -> Vec<UsageEvent> {
    all_adapters()
        .into_iter()
        .flat_map(|a| a.mock_fetch_usage())
        .collect()
}
