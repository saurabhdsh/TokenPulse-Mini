use crate::adapters::fetch_credit_balance;
use crate::adapters::usage_admin_key_required_message;
use crate::adapters::{AzureOpenAIAdapter, BedrockAdapter, OpenAIAdapter};
use crate::credentials::{resolve_azure_credentials, resolve_bedrock_credentials, resolve_openai_credentials};
use crate::db::Database;
use crate::engine::AlertEngine;
use crate::env;
use crate::models::UsageEvent;
use chrono::Utc;

use crate::models::SyncReport;

pub fn apply_env_keys(db: &Database) -> Result<Vec<String>, String> {
    let mut applied = Vec::new();

    if let Some(key) = env::get_openai_api_key() {
        let existing = db.get_provider_by_name("OpenAI").map_err(|e| e.to_string())?;
        let app_saved = existing.key_source.as_deref().is_some_and(|s| s == "app" || s == "manual")
            && existing
                .api_key
                .as_ref()
                .map(|k| !k.is_empty())
                .unwrap_or(false);

        if !app_saved {
            db.set_provider_key("OpenAI", &key, true, "env")
                .map_err(|e| e.to_string())?;
            applied.push("OPENAI_API_KEY".into());
        }
    }

    if env::get_openai_admin_key().is_some() {
        applied.push("OPENAI_ADMIN_KEY".into());
    }
    if env::get_openai_billing_token().is_some() {
        applied.push("OPENAI_BILLING_TOKEN".into());
    }

    if crate::aws_config::get_access_key_id().is_some() {
        applied.push("AWS_ACCESS_KEY_ID".into());
    }
    if crate::aws_config::credentials_file_exists() {
        applied.push("AWS_CLI_PROFILE".into());
    }

    if let Some(key) = crate::azure_config::get_api_key() {
        let existing = db
            .get_provider_by_name("Azure OpenAI")
            .map_err(|e| e.to_string())?;
        let app_saved = existing
            .key_source
            .as_deref()
            .is_some_and(|s| s == "app" || s == "manual")
            && existing
                .api_key
                .as_ref()
                .map(|k| !k.is_empty())
                .unwrap_or(false);

        if !app_saved {
            db.set_provider_key("Azure OpenAI", &key, true, "env")
                .map_err(|e| e.to_string())?;
            applied.push("AZURE_OPENAI_API_KEY".into());
        }
    }

    if crate::azure_config::get_endpoint().is_some() {
        applied.push("AZURE_OPENAI_ENDPOINT".into());
    }

    Ok(applied)
}

pub fn sync_all_providers(db: &Database) -> Result<Vec<SyncReport>, String> {
    let mut reports = Vec::new();
    let providers = db.get_providers().map_err(|e| e.to_string())?;

    for provider in providers {
        if !provider.is_enabled {
            continue;
        }

        if provider.name == "OpenAI" {
            match sync_openai(db, &provider) {
                Ok(report) => reports.push(report),
                Err(err) => {
                    db.set_provider_sync_status(
                        "OpenAI",
                        "error",
                        &err,
                        Some(Utc::now().to_rfc3339()),
                    )
                    .map_err(|e| e.to_string())?;
                    reports.push(SyncReport {
                        provider: "OpenAI".into(),
                        events_synced: 0,
                        status: "error".into(),
                        message: err,
                    });
                }
            }
        } else if provider.name == "AWS Bedrock" {
            match sync_bedrock(db) {
                Ok(report) => reports.push(report),
                Err(err) => {
                    db.set_provider_sync_status(
                        "AWS Bedrock",
                        "error",
                        &err,
                        Some(Utc::now().to_rfc3339()),
                    )
                    .map_err(|e| e.to_string())?;
                    reports.push(SyncReport {
                        provider: "AWS Bedrock".into(),
                        events_synced: 0,
                        status: "error".into(),
                        message: err,
                    });
                }
            }
        } else if provider.name == "Azure OpenAI" {
            match sync_azure(db, &provider) {
                Ok(report) => reports.push(report),
                Err(err) => {
                    db.set_provider_sync_status(
                        "Azure OpenAI",
                        "error",
                        &err,
                        Some(Utc::now().to_rfc3339()),
                    )
                    .map_err(|e| e.to_string())?;
                    reports.push(SyncReport {
                        provider: "Azure OpenAI".into(),
                        events_synced: 0,
                        status: "error".into(),
                        message: err,
                    });
                }
            }
        }
    }

    Ok(reports)
}

fn sync_openai(
    db: &Database,
    provider: &crate::models::Provider,
) -> Result<SyncReport, String> {
    let creds = resolve_openai_credentials(db)?;
    let api_key = creds
        .api_key
        .filter(|k| !k.is_empty())
        .ok_or_else(|| {
            "OpenAI API key not found. Save it in API Key Settings or set OPENAI_API_KEY in your environment.".to_string()
        })?;

    if provider.api_key.as_deref().unwrap_or("").is_empty() {
        let source = if env::get_openai_api_key().is_some() {
            "env"
        } else {
            "app"
        };
        db.set_provider_key("OpenAI", &api_key, true, source)
            .map_err(|e| e.to_string())?;
    }

    if creds.admin_key.as_deref().filter(|k| !k.is_empty()).is_none() {
        return Err(usage_admin_key_required_message());
    }

    let adapter = OpenAIAdapter;
    let events = adapter
        .fetch_usage_with_creds(
            &api_key,
            creds.admin_key.as_deref(),
            creds.org_id.as_deref(),
        )
        .map_err(|e| e.to_string())?;

    let pricing_map = db
        .get_openai_pricing_map()
        .map_err(|e| e.to_string())?;
    let events = apply_db_pricing(events, &pricing_map);

    db.replace_provider_events("OpenAI", &events)
        .map_err(|e| e.to_string())?;

    db.purge_non_live_events().map_err(|e| e.to_string())?;

    let mut credit_note = String::new();
    match fetch_credit_balance(
        &api_key,
        creds.admin_key.as_deref(),
        creds.billing_token.as_deref(),
        creds.org_id.as_deref(),
    ) {
        Ok(credit) => {
            db.set_provider_credit("OpenAI", &credit)
                .map_err(|e| e.to_string())?;
            credit_note = format!(
                " · ${:.2} {} remaining",
                credit.available,
                if credit.source == "subscription_limit" {
                    "limit"
                } else {
                    "credits"
                }
            );
        }
        Err(err) => {
            credit_note = format!(" · credits unavailable ({err})");
        }
    }

    let message = format!(
        "Synced {} OpenAI usage buckets from live API{}",
        events.len(),
        credit_note
    );
    let synced_at = Utc::now().to_rfc3339();
    db.set_provider_sync_status("OpenAI", "connected", &message, Some(synced_at.clone()))
        .map_err(|e| e.to_string())?;

    Ok(SyncReport {
        provider: "OpenAI".into(),
        events_synced: events.len() as i64,
        status: "connected".into(),
        message,
    })
}

fn sync_bedrock(db: &Database) -> Result<SyncReport, String> {
    let creds = resolve_bedrock_credentials(db)?;
    let adapter = BedrockAdapter;
    let events = adapter
        .fetch_usage_with_creds(&creds)
        .map_err(|e| e.to_string())?;

    let pricing_map = db
        .get_bedrock_pricing_map()
        .map_err(|e| e.to_string())?;
    let events = apply_db_pricing(events, &pricing_map);

    db.replace_provider_events("AWS Bedrock", &events)
        .map_err(|e| e.to_string())?;

    db.purge_non_live_events().map_err(|e| e.to_string())?;

    let identity = crate::aws_config::validate_aws_credentials(&creds)?;
    let message = format!(
        "Synced {} Bedrock cost buckets via AWS Cost Explorer · {}",
        events.len(),
        identity
    );
    let synced_at = Utc::now().to_rfc3339();
    db.set_provider_sync_status("AWS Bedrock", "connected", &message, Some(synced_at))
        .map_err(|e| e.to_string())?;

    Ok(SyncReport {
        provider: "AWS Bedrock".into(),
        events_synced: events.len() as i64,
        status: "connected".into(),
        message,
    })
}

fn sync_azure(
    db: &Database,
    provider: &crate::models::Provider,
) -> Result<SyncReport, String> {
    let creds = resolve_azure_credentials(db)?;
    let adapter = AzureOpenAIAdapter;
    let events = adapter
        .fetch_usage_with_creds(&creds)
        .map_err(|e| e.to_string())?;

    if provider.api_key.as_deref().unwrap_or("").is_empty() {
        let source = if crate::azure_config::get_api_key().is_some() {
            "env"
        } else {
            "app"
        };
        db.set_provider_key("Azure OpenAI", &creds.api_key, true, source)
            .map_err(|e| e.to_string())?;
    }

    let pricing_map = db
        .get_azure_pricing_map()
        .map_err(|e| e.to_string())?;
    let events = apply_db_pricing(events, &pricing_map);

    db.replace_provider_events("Azure OpenAI", &events)
        .map_err(|e| e.to_string())?;

    db.purge_non_live_events().map_err(|e| e.to_string())?;

    let identity = crate::azure_config::validate_azure_openai_credentials(&creds)?;
    let metrics_note = if crate::azure_config::az_cli_available() {
        " · Azure CLI detected"
    } else {
        " · install Azure CLI + `az login` for token metrics"
    };
    let message = format!(
        "Synced {} Azure OpenAI usage bucket(s) · {}{}",
        events.len(),
        identity,
        metrics_note
    );
    let synced_at = Utc::now().to_rfc3339();
    db.set_provider_sync_status("Azure OpenAI", "connected", &message, Some(synced_at))
        .map_err(|e| e.to_string())?;

    Ok(SyncReport {
        provider: "Azure OpenAI".into(),
        events_synced: events.len() as i64,
        status: "connected".into(),
        message,
    })
}

fn apply_db_pricing(
    events: Vec<UsageEvent>,
    pricing: &std::collections::HashMap<String, (f64, f64)>,
) -> Vec<UsageEvent> {
    use crate::engine::CostEngine;

    events
        .into_iter()
        .map(|mut event| {
            if let Some((input_p, output_p)) = pricing.get(&event.model) {
                let (input_cost, output_cost, total_cost) = CostEngine::calculate_cost(
                    event.prompt_tokens,
                    event.completion_tokens,
                    *input_p,
                    *output_p,
                );
                event.input_cost = input_cost;
                event.output_cost = output_cost;
                event.total_cost = total_cost;
            }
            event
        })
        .collect()
}

/// Full live refresh: env import, provider sync, alert evaluation.
pub fn refresh_live_data_inner(db: &Database) -> Result<Vec<SyncReport>, String> {
    apply_env_keys(db)?;
    let reports = sync_all_providers(db)?;
    let _ = AlertEngine::evaluate(db);
    Ok(reports)
}
