use crate::db::Database;
use crate::models::*;

impl DashboardStats {
    pub fn build(db: &Database) -> rusqlite::Result<Self> {
        let today_start = db.get_today_start();
        let week_start = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();
        let month_start = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();

        let live_providers = db.connected_provider_names()?;
        let is_demo_data = live_providers.is_empty();

        let today = period_stats_for_scope(db, &today_start, &live_providers)?;
        let week = period_stats_for_scope(db, &week_start, &live_providers)?;
        let month = period_stats_for_scope(db, &month_start, &live_providers)?;
        let budget = db.get_budget_settings()?;

        let mut providers = db.get_provider_costs_since(&today_start)?;
        if providers.is_empty() {
            providers = db.get_provider_costs_since(&week_start)?;
        }
        filter_provider_costs(&mut providers, &live_providers);

        let mut models = db.get_model_costs_since(&today_start)?;
        if models.is_empty() {
            models = db.get_model_costs_since(&week_start)?;
        }
        if !live_providers.is_empty() {
            models.retain(|m| live_providers.contains(&m.provider));
        }

        let alerts = db.get_alerts(20)?;
        let provider_summaries = build_provider_summaries(
            db,
            &today_start,
            &week_start,
            &live_providers,
        )?;

        Ok(Self {
            today,
            week,
            month,
            budget,
            providers,
            models,
            alerts,
            live_providers,
            is_demo_data,
            provider_summaries,
        })
    }
}

fn period_stats_for_scope(
    db: &Database,
    since: &str,
    live_providers: &[String],
) -> rusqlite::Result<PeriodStats> {
    if live_providers.is_empty() {
        return db.get_period_stats(since);
    }

    let mut total_tokens = 0i64;
    let mut total_cost = 0.0;
    let mut input_cost = 0.0;
    let mut output_cost = 0.0;
    let mut event_count = 0i64;

    for provider in live_providers {
        let stats = db.get_period_stats_for_provider(since, provider)?;
        total_tokens += stats.total_tokens;
        total_cost += stats.total_cost;
        input_cost += stats.input_cost;
        output_cost += stats.output_cost;
        event_count += stats.event_count;
    }

    let hours = 24.0_f64.max(1.0);
    Ok(PeriodStats {
        total_tokens,
        total_cost,
        input_cost,
        output_cost,
        event_count,
        burn_rate_per_hour: total_cost / hours,
        estimated_monthly: (total_cost / hours) * 24.0 * 30.0,
    })
}

fn filter_provider_costs(providers: &mut Vec<ProviderCost>, live_providers: &[String]) {
    if live_providers.is_empty() {
        return;
    }
    providers.retain(|p| live_providers.contains(&p.provider));
    let total: f64 = providers.iter().map(|p| p.cost).sum();
    for p in providers.iter_mut() {
        p.pct = if total > 0.0 { (p.cost / total) * 100.0 } else { 0.0 };
    }
}

fn build_provider_summaries(
    db: &Database,
    today_start: &str,
    week_start: &str,
    live_providers: &[String],
) -> rusqlite::Result<Vec<DashboardProviderSummary>> {
    let all = db.get_providers()?;
    let names: Vec<String> = if live_providers.is_empty() {
        all.iter()
            .filter(|p| p.is_enabled)
            .map(|p| p.name.clone())
            .collect()
    } else {
        live_providers.to_vec()
    };

    let mut summaries = Vec::new();
    for name in names {
        let meta = all.iter().find(|p| p.name == name);
        let today = db.get_period_stats_for_provider(today_start, &name)?;
        let week = db.get_period_stats_for_provider(week_start, &name)?;
        summaries.push(DashboardProviderSummary {
            name: name.clone(),
            sync_status: meta.and_then(|p| p.sync_status.clone()),
            sync_message: meta.and_then(|p| p.sync_message.clone()),
            last_synced_at: meta.and_then(|p| p.last_synced_at.clone()),
            credit: meta.and_then(|p| p.credit.clone()),
            today_cost: today.total_cost,
            today_tokens: today.total_tokens,
            week_cost: week.total_cost,
        });
    }
    Ok(summaries)
}

pub fn provider_costs_live(db: &Database, since: &str) -> rusqlite::Result<Vec<ProviderCost>> {
    let live = db.connected_provider_names()?;
    let mut providers = db.get_provider_costs_since(since)?;
    filter_provider_costs(&mut providers, &live);
    Ok(providers)
}

pub fn model_costs_live(db: &Database, since: &str) -> rusqlite::Result<Vec<ModelCost>> {
    let live = db.connected_provider_names()?;
    let mut models = db.get_model_costs_since(since)?;
    if !live.is_empty() {
        models.retain(|m| live.contains(&m.provider));
    }
    Ok(models)
}

pub fn usage_events_live(db: &Database, limit: i64, offset: i64) -> rusqlite::Result<Vec<UsageEvent>> {
    let live = db.connected_provider_names()?;
    let events = db.get_usage_events(limit, offset)?;
    if live.is_empty() {
        return Ok(events);
    }
    Ok(events
        .into_iter()
        .filter(|e| live.contains(&e.provider))
        .collect())
}
