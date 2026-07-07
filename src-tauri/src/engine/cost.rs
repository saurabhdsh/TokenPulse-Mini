use crate::db::Database;
use crate::aws_config::widget_sync_hint;
use crate::engine::demo::ensure_widget_demo_events;
use crate::models::*;
use chrono::{Duration, Utc};

pub struct CostEngine;

impl CostEngine {
    pub fn calculate_cost(
        prompt_tokens: i64,
        completion_tokens: i64,
        input_price_per_million: f64,
        output_price_per_million: f64,
    ) -> (f64, f64, f64) {
        let input_cost = (prompt_tokens as f64 / 1_000_000.0) * input_price_per_million;
        let output_cost = (completion_tokens as f64 / 1_000_000.0) * output_price_per_million;
        (input_cost, output_cost, input_cost + output_cost)
    }

    pub fn build_widget_stats(db: &Database) -> rusqlite::Result<WidgetStats> {
        Self::build_widget_stats_for_provider(db, None)
    }

    pub fn build_widget_stats_for_provider(
        db: &Database,
        provider: Option<&str>,
    ) -> rusqlite::Result<WidgetStats> {
        let providers = db.get_providers()?;
        let show_demo = db.get_widget_show_demo()?;

        if show_demo {
            ensure_widget_demo_events(db).ok();
        }

        let scope = db.build_widget_query_scope(provider)?;

        let mut live_providers: Vec<String> = providers
            .iter()
            .filter(|p| p.sync_status.as_deref() == Some("connected"))
            .map(|p| p.name.clone())
            .collect();

        if let Some(name) = provider {
            live_providers.retain(|p| p == name);
        }

        let is_connected = provider
            .map(|name| {
                providers
                    .iter()
                    .any(|p| p.name == name && p.sync_status.as_deref() == Some("connected"))
            })
            .unwrap_or(!live_providers.is_empty());

        let is_demo_data = scope.demo_only;
        let show_demo_overlay = scope.include_demo_overlay || scope.demo_only;

        if provider.is_none() && scope.live_only && !show_demo {
            db.purge_non_live_events().ok();
        }

        let today_start = db.get_today_start();
        let week_start = (Utc::now() - Duration::days(7)).to_rfc3339();
        let today = db.get_period_stats_widget(&today_start, &scope)?;
        let budget = db.get_budget_settings()?;

        let (top_provider, top_model) = {
            let (p, m) = db.get_top_provider_model_widget(&today_start, &scope)?;
            if p == "—" {
                db.get_top_provider_model_widget(&week_start, &scope)?
            } else {
                let model = if m == "—" {
                    db.get_top_model_for_provider_widget(&week_start, &p, &scope)?
                } else {
                    m
                };
                (p, model)
            }
        };

        let (top_provider, top_model) = if let Some(p) = provider {
            let model = if top_model == "—" || top_provider != p {
                db.get_top_model_for_provider_widget(&week_start, p, &scope)?
            } else {
                top_model
            };
            (p.to_string(), model)
        } else {
            (top_provider, top_model)
        };

        let mut provider_breakdown = db.get_provider_costs_since_widget(&today_start, &scope)?;
        if provider_breakdown.is_empty() {
            provider_breakdown = db.get_provider_costs_since_widget(&week_start, &scope)?;
        }

        let mut sparkline = db.get_hourly_sparkline_widget(24, &scope)?;
        if sparkline.iter().all(|pt| pt.tokens == 0 && pt.cost == 0.0) {
            sparkline = db.get_daily_sparkline_widget(14, &scope)?;
        }

        if sparkline.is_empty() {
            sparkline = (0..7)
                .map(|i| HourlyPoint {
                    hour: format!("-{}d", 6 - i),
                    tokens: 0,
                    cost: 0.0,
                })
                .collect();
        }

        let daily_pct = if budget.daily_limit > 0.0 {
            (today.total_cost / budget.daily_limit) * 100.0
        } else {
            0.0
        };

        let budget_risk = if daily_pct >= 100.0 {
            "Critical"
        } else if daily_pct >= 80.0 {
            "High"
        } else if daily_pct >= 50.0 {
            "Moderate"
        } else {
            "Low"
        }
        .to_string();

        let sync_hint = if scope.demo_only {
            Some(format!(
                "Demo sample data for {}",
                provider.unwrap_or("OpenAI/AWS")
            ))
        } else if show_demo_overlay {
            Some("Live data + OpenAI/AWS demo overlay".into())
        } else if let Some(p) = provider {
            if is_connected && today.total_cost == 0.0 {
                Some(format!("No {p} usage today · showing 7d top"))
            } else {
                providers.iter().find(|pr| pr.name == p).and_then(|pr| {
                    if pr.sync_status.as_deref() == Some("connected") {
                        None
                    } else if pr.sync_status.as_deref() == Some("error") {
                        pr.sync_message
                            .as_deref()
                            .map(|msg| widget_sync_hint(p, msg))
                    } else if today.total_cost == 0.0 && today.total_tokens == 0 {
                        Some(format!("No {p} data · turn on Demo overlay or fix credentials"))
                    } else {
                        pr.sync_message.clone()
                    }
                })
            }
        } else if !is_demo_data && today.total_cost == 0.0 {
            Some("No usage today · showing 7d top".into())
        } else {
            let errors: Vec<String> = providers
                .iter()
                .filter(|p| live_providers.contains(&p.name))
                .filter_map(|p| {
                    if p.sync_status.as_deref() == Some("error") {
                        p.sync_message
                            .as_deref()
                            .map(|msg| widget_sync_hint(&p.name, msg))
                    } else {
                        None
                    }
                })
                .collect();
            if errors.is_empty() {
                None
            } else {
                Some(errors.join(" · "))
            }
        };

        let openai_credit = if provider.is_none() || provider == Some("OpenAI") {
            db.get_openai_credit()?
        } else {
            None
        };

        Ok(WidgetStats {
            today_tokens: today.total_tokens,
            today_cost: today.total_cost,
            burn_rate_per_hour: today.burn_rate_per_hour,
            top_provider,
            top_model,
            budget_risk,
            daily_budget_used_pct: daily_pct.min(150.0),
            daily_budget_limit: budget.daily_limit,
            monthly_estimated: today.estimated_monthly,
            sparkline,
            provider_breakdown,
            is_demo_data,
            show_demo_overlay,
            sync_hint,
            live_providers,
            openai_credit,
            focused_provider: provider.map(str::to_string),
        })
    }
}
