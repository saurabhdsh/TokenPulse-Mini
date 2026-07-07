use crate::models::UsageEvent;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("API key not configured")]
    MissingApiKey,
    #[error("Provider API error: {0}")]
    ApiError(String),
    #[error("Rate limited")]
    RateLimited,
}

/// Interface for real provider API connectors.
/// Mock implementations return realistic sample data; swap `fetch_usage` for live HTTP calls later.
pub trait ProviderAdapter: Send + Sync {
    fn provider_name(&self) -> &'static str;

    /// Live API fetch — not implemented yet; returns mock data when key is present.
    fn fetch_usage(&self, api_key: Option<&str>) -> Result<Vec<UsageEvent>, AdapterError> {
        match api_key {
            Some(key) if !key.is_empty() => Ok(self.mock_fetch_usage()),
            _ => Err(AdapterError::MissingApiKey),
        }
    }

    fn mock_fetch_usage(&self) -> Vec<UsageEvent>;
}
