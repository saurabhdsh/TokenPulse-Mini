use crate::db::Database;
use crate::models::Alert;
use chrono::Utc;

pub struct AlertEngine;

impl AlertEngine {
    pub fn evaluate(db: &Database) -> rusqlite::Result<Vec<Alert>> {
        let mut new_alerts = Vec::new();
        let today_start = db.get_today_start();
        let today = db.get_period_stats(&today_start)?;
        let budget = db.get_budget_settings()?;
        let now = Utc::now().to_rfc3339();

        let pct = if budget.daily_limit > 0.0 {
            today.total_cost / budget.daily_limit
        } else {
            0.0
        };

        let thresholds = [
            ("daily_budget_50", budget.alert_threshold_50, "warning", "Daily budget at 50%"),
            ("daily_budget_80", budget.alert_threshold_80, "high", "Daily budget at 80%"),
            ("daily_budget_100", budget.alert_threshold_100, "critical", "Daily budget exceeded"),
        ];

        for (alert_type, threshold, severity, msg) in thresholds {
            if pct >= threshold && db.count_alerts_today(alert_type)? == 0 {
                let alert = Alert {
                    id: 0,
                    alert_type: alert_type.into(),
                    severity: severity.into(),
                    message: format!("{} — ${:.2} of ${:.2}", msg, today.total_cost, budget.daily_limit),
                    provider: None,
                    model: None,
                    value: Some(today.total_cost),
                    threshold: Some(budget.daily_limit * threshold),
                    is_read: false,
                    created_at: now.clone(),
                };
                let id = db.insert_alert(&alert)?;
                new_alerts.push(Alert { id, ..alert });
            }
        }

        if budget.spike_detection_enabled {
            let hourly = db.get_hourly_cost_last_n(6)?;
            if hourly.len() >= 2 {
                let current = *hourly.last().unwrap_or(&0.0);
                let avg: f64 = hourly[..hourly.len() - 1].iter().sum::<f64>()
                    / (hourly.len() - 1).max(1) as f64;
                if avg > 0.0 && current > avg * 2.5 && db.count_alerts_today("usage_spike")? == 0 {
                    let alert = Alert {
                        id: 0,
                        alert_type: "usage_spike".into(),
                        severity: "high".into(),
                        message: format!(
                            "Sudden usage spike detected — ${:.2}/hr vs avg ${:.2}/hr",
                            current, avg
                        ),
                        provider: None,
                        model: None,
                        value: Some(current),
                        threshold: Some(avg * 2.5),
                        is_read: false,
                        created_at: now.clone(),
                    };
                    let id = db.insert_alert(&alert)?;
                    new_alerts.push(Alert { id, ..alert });
                }
            }
        }

        if budget.expensive_model_warning {
            let models = db.get_model_costs_since(&today_start)?;
            for m in models {
                if let Some((_, _, expensive)) = db.get_model_pricing(&m.provider, &m.model)? {
                    if expensive && m.cost > 5.0 && db.count_alerts_today("expensive_model")? == 0 {
                        let alert = Alert {
                            id: 0,
                            alert_type: "expensive_model".into(),
                            severity: "warning".into(),
                            message: format!(
                                "Expensive model {} cost ${:.2} today",
                                m.model, m.cost
                            ),
                            provider: Some(m.provider.clone()),
                            model: Some(m.model.clone()),
                            value: Some(m.cost),
                            threshold: Some(5.0),
                            is_read: false,
                            created_at: now.clone(),
                        };
                        let id = db.insert_alert(&alert)?;
                        new_alerts.push(Alert { id, ..alert });
                        break;
                    }
                }
            }
        }

        let providers = db.get_provider_costs_since(&today_start)?;
        if providers.len() >= 2 {
            let top = &providers[0];
            let second = &providers[1];
            if second.cost > 0.0 && top.cost > second.cost * 3.0
                && db.count_alerts_today("provider_spike")? == 0
            {
                let alert = Alert {
                    id: 0,
                    alert_type: "provider_spike".into(),
                    severity: "warning".into(),
                    message: format!(
                        "{} cost spike — ${:.2} ({:.0}% of spend)",
                        top.provider, top.cost, top.pct
                    ),
                    provider: Some(top.provider.clone()),
                    model: None,
                    value: Some(top.cost),
                    threshold: None,
                    is_read: false,
                    created_at: now,
                };
                let id = db.insert_alert(&alert)?;
                new_alerts.push(Alert { id, ..alert });
            }
        }

        Ok(new_alerts)
    }
}
