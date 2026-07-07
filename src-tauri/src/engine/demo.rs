use crate::adapters::{BedrockAdapter, OpenAIAdapter, ProviderAdapter};
use crate::db::{Database, DEMO_PROJECT};
use crate::engine::CostEngine;
use crate::models::UsageEvent;
use chrono::{Duration, Utc};

pub fn ensure_widget_demo_events(db: &Database) -> rusqlite::Result<()> {
    for provider in ["OpenAI", "AWS Bedrock"] {
        if db.count_demo_events(provider)? > 0 {
            continue;
        }
        let events = generate_demo_events(provider);
        for event in events {
            db.insert_usage_event(&event)?;
        }
    }
    db.rebuild_daily_summaries()
}

pub fn clear_widget_demo_events(db: &Database) -> rusqlite::Result<()> {
    db.clear_widget_demo_events()
}

fn generate_demo_events(provider: &str) -> Vec<UsageEvent> {
    let templates: Vec<UsageEvent> = match provider {
        "OpenAI" => OpenAIAdapter.mock_fetch_usage(),
        "AWS Bedrock" => BedrockAdapter.mock_fetch_usage(),
        _ => vec![],
    };

    if templates.is_empty() {
        return vec![];
    }

    let now = Utc::now();
    let mut events = Vec::new();

    for days_ago in 0..14 {
        for (i, template) in templates.iter().enumerate() {
            let mut event = template.clone();
            event.id = None;
            event.provider = provider.into();
            event.project_name = Some(DEMO_PROJECT.into());
            event.request_id = Some(format!("demo-{provider}-{days_ago}-{i}"));
            event.timestamp = (now - Duration::days(days_ago) - Duration::hours(i as i64 * 2))
                .to_rfc3339();

            let pricing = default_pricing(provider, &event.model);
            let (input_cost, output_cost, total_cost) = CostEngine::calculate_cost(
                event.prompt_tokens,
                event.completion_tokens,
                pricing.0,
                pricing.1,
            );
            event.input_cost = input_cost;
            event.output_cost = output_cost;
            event.total_cost = total_cost;
            events.push(event);
        }
    }

    events
}

fn default_pricing(provider: &str, model: &str) -> (f64, f64) {
    match (provider, model) {
        ("OpenAI", m) if m.contains("mini") => (0.15, 0.60),
        ("OpenAI", _) => (2.5, 10.0),
        ("AWS Bedrock", _) => (3.0, 15.0),
        _ => (1.0, 3.0),
    }
}
