mod schema;

use crate::models::*;
use chrono::{Duration, Local, Utc};
use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension, Result};
use std::path::PathBuf;
use uuid::Uuid;

pub const DEMO_PROJECT: &str = "__demo__";
pub const WIDGET_DEMO_PROVIDERS: &[&str] = &["OpenAI", "AWS Bedrock"];

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(app_data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir).ok();
        let db_path = app_data_dir.join("tokenpulse.db");
        let conn = Connection::open(db_path)?;
        conn.execute_batch(schema::SCHEMA)?;
        let db = Self { conn };
        db.ensure_defaults()?;
        Ok(db)
    }

    fn ensure_defaults(&self) -> Result<()> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM providers", [], |r| r.get(0))?;
        if count == 0 {
            self.seed_providers_and_models()?;
        }

        let budget_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM budget_settings", [], |r| r.get(0))?;
        if budget_count == 0 {
            self.conn.execute(
                "INSERT INTO budget_settings (id, daily_limit, monthly_limit, timezone) VALUES (1, 50.0, 1500.0, 'America/New_York')",
                [],
            )?;
        }

        let seeded: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'mock_seeded'",
                [],
                |r| r.get(0),
            )
            .ok();

        if seeded.is_none() {
            self.seed_mock_usage()?;
            self.conn.execute(
                "INSERT INTO app_meta (key, value) VALUES ('mock_seeded', 'true')",
                [],
            )?;
        }

        self.migrate_schema()?;
        Ok(())
    }

    fn migrate_schema(&self) -> Result<()> {
        let columns = [
            "ALTER TABLE providers ADD COLUMN key_source TEXT",
            "ALTER TABLE providers ADD COLUMN sync_status TEXT",
            "ALTER TABLE providers ADD COLUMN sync_message TEXT",
            "ALTER TABLE providers ADD COLUMN credit_available REAL",
            "ALTER TABLE providers ADD COLUMN credit_granted REAL",
            "ALTER TABLE providers ADD COLUMN credit_used REAL",
            "ALTER TABLE providers ADD COLUMN credit_monthly_limit REAL",
            "ALTER TABLE providers ADD COLUMN credit_month_spend REAL",
            "ALTER TABLE providers ADD COLUMN credit_source TEXT",
            "ALTER TABLE providers ADD COLUMN credit_currency TEXT",
            "ALTER TABLE providers ADD COLUMN credit_synced_at TEXT",
        ];
        for sql in columns {
            self.conn.execute(sql, []).ok();
        }
        Ok(())
    }

    fn seed_providers_and_models(&self) -> Result<()> {
        let providers = [
            ("OpenAI", vec![
                ("gpt-4o", 2.50, 10.00, false),
                ("gpt-4o-mini", 0.15, 0.60, false),
                ("o1-preview", 15.00, 60.00, true),
                ("o1-mini", 3.00, 12.00, false),
            ]),
            ("Anthropic", vec![
                ("claude-sonnet-4", 3.00, 15.00, false),
                ("claude-opus-4", 15.00, 75.00, true),
                ("claude-haiku-3.5", 0.80, 4.00, false),
            ]),
            ("AWS Bedrock", vec![
                ("anthropic.claude-3-5-sonnet", 3.00, 15.00, false),
                ("amazon.titan-text-premier", 0.50, 1.50, false),
                ("meta.llama3-70b", 2.65, 3.50, false),
            ]),
            ("Azure OpenAI", vec![
                ("gpt-4o", 2.50, 10.00, false),
                ("gpt-4o-mini", 0.15, 0.60, false),
            ]),
            ("Gemini", vec![
                ("gemini-2.0-flash", 0.10, 0.40, false),
                ("gemini-1.5-pro", 1.25, 5.00, false),
                ("gemini-1.5-flash", 0.075, 0.30, false),
            ]),
        ];

        let now = Utc::now().to_rfc3339();
        for (name, models) in providers {
            self.conn.execute(
                "INSERT INTO providers (name, is_enabled, created_at) VALUES (?1, 1, ?2)",
                params![name, now],
            )?;
            let provider_id: i64 = self.conn.last_insert_rowid();
            for (model_name, input_p, output_p, expensive) in models {
                self.conn.execute(
                    "INSERT INTO models (provider_id, model_name, input_price_per_million, output_price_per_million, is_expensive) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![provider_id, model_name, input_p, output_p, if expensive { 1 } else { 0 }],
                )?;
            }
        }
        Ok(())
    }

    fn seed_mock_usage(&self) -> Result<()> {
        let mut rng = rand::thread_rng();
        let providers_models: Vec<(String, String, f64, f64, bool)> = {
            let mut stmt = self.conn.prepare(
                "SELECT p.name, m.model_name, m.input_price_per_million, m.output_price_per_million, m.is_expensive
                 FROM models m JOIN providers p ON p.id = m.provider_id",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, i64>(4)? != 0,
                ))
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let projects = [
            "tokenpulse-api",
            "chat-widget",
            "doc-summarizer",
            "code-assist",
            "analytics-pipeline",
        ];

        let now = Utc::now();
        for hours_ago in (0..24).rev() {
            let events_in_hour = rng.gen_range(3..12);
            for _ in 0..events_in_hour {
                let idx = rng.gen_range(0..providers_models.len());
                let (provider, model, input_p, output_p, _) = &providers_models[idx];
                let prompt_tokens = rng.gen_range(500..8000);
                let completion_tokens = rng.gen_range(100..4000);
                let total_tokens = prompt_tokens + completion_tokens;
                let input_cost = (prompt_tokens as f64 / 1_000_000.0) * input_p;
                let output_cost = (completion_tokens as f64 / 1_000_000.0) * output_p;
                let total_cost = input_cost + output_cost;
                let minute_offset = rng.gen_range(0..59);
                let ts = now - Duration::hours(hours_ago) + Duration::minutes(minute_offset);
                let project = projects[rng.gen_range(0..projects.len())];

                self.conn.execute(
                    "INSERT INTO usage_events (provider, model, prompt_tokens, completion_tokens, total_tokens, input_cost, output_cost, total_cost, project_name, request_id, timestamp)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        provider,
                        model,
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                        input_cost,
                        output_cost,
                        total_cost,
                        project,
                        Uuid::new_v4().to_string(),
                        ts.to_rfc3339(),
                    ],
                )?;
            }
        }

        // Add some events from previous days for monthly stats
        for days_ago in 1..7 {
            for _ in 0..rng.gen_range(15..40) {
                let idx = rng.gen_range(0..providers_models.len());
                let (provider, model, input_p, output_p, _) = &providers_models[idx];
                let prompt_tokens = rng.gen_range(300..6000);
                let completion_tokens = rng.gen_range(50..3000);
                let total_tokens = prompt_tokens + completion_tokens;
                let input_cost = (prompt_tokens as f64 / 1_000_000.0) * input_p;
                let output_cost = (completion_tokens as f64 / 1_000_000.0) * output_p;
                let ts = now - Duration::days(days_ago) - Duration::hours(rng.gen_range(0..23));
                self.conn.execute(
                    "INSERT INTO usage_events (provider, model, prompt_tokens, completion_tokens, total_tokens, input_cost, output_cost, total_cost, project_name, request_id, timestamp)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        provider,
                        model,
                        prompt_tokens,
                        completion_tokens,
                        total_tokens,
                        input_cost,
                        output_cost,
                        input_cost + output_cost,
                        projects[rng.gen_range(0..projects.len())],
                        Uuid::new_v4().to_string(),
                        ts.to_rfc3339(),
                    ],
                )?;
            }
        }

        self.rebuild_daily_summaries()?;
        Ok(())
    }

    pub fn rebuild_daily_summaries(&self) -> Result<()> {
        self.conn.execute("DELETE FROM daily_summary", [])?;
        self.conn.execute(
            "INSERT INTO daily_summary (date, provider, model, total_tokens, total_cost, event_count)
             SELECT date(timestamp) as d, provider, model,
                    SUM(total_tokens), SUM(total_cost), COUNT(*)
             FROM usage_events
             GROUP BY d, provider, model",
            [],
        )?;
        Ok(())
    }

    pub fn get_providers(&self) -> Result<Vec<Provider>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, api_key, is_enabled, key_source, sync_status, sync_message, last_synced_at,
                    credit_available, credit_granted, credit_used, credit_monthly_limit, credit_month_spend,
                    credit_source, credit_currency, credit_synced_at, created_at
             FROM providers ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| {
            let api_key: Option<String> = row.get(2)?;
            let key_source: Option<String> = row.get(4)?;
            let preview = api_key.as_deref().map(crate::env::mask_key);
            let exposed_key = match key_source.as_deref() {
                Some("env") => None,
                _ => api_key,
            };
            let credit_available: Option<f64> = row.get(8)?;
            let credit_granted: Option<f64> = row.get(9)?;
            let credit_used: Option<f64> = row.get(10)?;
            let credit_monthly_limit: Option<f64> = row.get(11)?;
            let credit_month_spend: Option<f64> = row.get(12)?;
            let credit_source: Option<String> = row.get(13)?;
            let credit_currency: Option<String> = row.get(14)?;
            let credit_synced_at: Option<String> = row.get(15)?;
            let credit = credit_available.map(|available| CreditBalance {
                available,
                granted: credit_granted,
                used: credit_used,
                monthly_limit: credit_monthly_limit,
                month_spend: credit_month_spend,
                source: credit_source.unwrap_or_else(|| "unknown".into()),
                currency: credit_currency.unwrap_or_else(|| "USD".into()),
                synced_at: credit_synced_at.unwrap_or_default(),
            });
            Ok(Provider {
                id: row.get(0)?,
                name: row.get(1)?,
                api_key: exposed_key,
                api_key_preview: preview,
                is_enabled: row.get::<_, i64>(3)? != 0,
                key_source,
                sync_status: row.get(5)?,
                sync_message: row.get(6)?,
                last_synced_at: row.get(7)?,
                credit,
                created_at: row.get(16)?,
            })
        })?;
        rows.collect()
    }

    pub fn set_provider_credit(&self, name: &str, credit: &CreditBalance) -> Result<()> {
        self.conn.execute(
            "UPDATE providers SET credit_available = ?1, credit_granted = ?2, credit_used = ?3,
             credit_monthly_limit = ?4, credit_month_spend = ?5, credit_source = ?6,
             credit_currency = ?7, credit_synced_at = ?8 WHERE name = ?9",
            params![
                credit.available,
                credit.granted,
                credit.used,
                credit.monthly_limit,
                credit.month_spend,
                credit.source,
                credit.currency,
                credit.synced_at,
                name,
            ],
        )?;
        Ok(())
    }

    pub fn get_openai_credit(&self) -> Result<Option<CreditBalance>> {
        Ok(self
            .get_provider_by_name("OpenAI")?
            .credit)
    }

    pub fn get_provider_by_name(&self, name: &str) -> Result<Provider> {
        let providers = self.get_providers()?;
        providers
            .into_iter()
            .find(|p| p.name == name)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn set_provider_key(
        &self,
        name: &str,
        api_key: &str,
        enabled: bool,
        key_source: &str,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE providers SET api_key = ?1, is_enabled = ?2, key_source = ?3 WHERE name = ?4",
            params![api_key, if enabled { 1 } else { 0 }, key_source, name],
        )?;
        Ok(())
    }

    pub fn update_provider_key(&self, name: &str, api_key: &str, enabled: bool) -> Result<()> {
        if api_key.is_empty() {
            self.conn.execute(
                "UPDATE providers SET is_enabled = ?1 WHERE name = ?2",
                params![if enabled { 1 } else { 0 }, name],
            )?;
        } else {
            self.set_provider_key(name, api_key, enabled, "app")?;
        }
        Ok(())
    }

    pub fn clear_provider_api_key(&self, name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE providers SET api_key = NULL, key_source = NULL WHERE name = ?1",
            params![name],
        )?;
        Ok(())
    }

    pub fn set_secret(&self, key: &str, value: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO app_secrets (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now],
        )?;
        Ok(())
    }

    pub fn get_secret(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM app_secrets WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn delete_secret(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM app_secrets WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn set_provider_sync_status(
        &self,
        name: &str,
        status: &str,
        message: &str,
        synced_at: Option<String>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE providers SET sync_status = ?1, sync_message = ?2, last_synced_at = ?3 WHERE name = ?4",
            params![status, message, synced_at, name],
        )?;
        Ok(())
    }

    pub fn replace_provider_events(&self, provider: &str, events: &[UsageEvent]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM usage_events WHERE provider = ?1 AND (project_name IS NULL OR project_name != ?2)",
            params![provider, DEMO_PROJECT],
        )?;
        for event in events {
            self.insert_usage_event(event)?;
        }
        self.rebuild_daily_summaries()
    }

    pub fn get_openai_pricing_map(&self) -> Result<std::collections::HashMap<String, (f64, f64)>> {
        self.get_provider_pricing_map("OpenAI")
    }

    pub fn get_bedrock_pricing_map(&self) -> Result<std::collections::HashMap<String, (f64, f64)>> {
        self.get_provider_pricing_map("AWS Bedrock")
    }

    pub fn get_azure_pricing_map(&self) -> Result<std::collections::HashMap<String, (f64, f64)>> {
        self.get_provider_pricing_map("Azure OpenAI")
    }

    fn get_provider_pricing_map(
        &self,
        provider_name: &str,
    ) -> Result<std::collections::HashMap<String, (f64, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.model_name, m.input_price_per_million, m.output_price_per_million
             FROM models m JOIN providers p ON p.id = m.provider_id
             WHERE p.name = ?1",
        )?;
        let rows = stmt.query_map(params![provider_name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (row.get::<_, f64>(1)?, row.get::<_, f64>(2)?),
            ))
        })?;
        rows.collect()
    }

    pub fn get_provider_api_key(&self, name: &str) -> Result<Option<String>> {
        self.conn.query_row(
            "SELECT api_key FROM providers WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )
    }

    pub fn get_models(&self) -> Result<Vec<ModelPricing>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.provider_id, p.name, m.model_name, m.input_price_per_million, m.output_price_per_million, m.is_expensive
             FROM models m JOIN providers p ON p.id = m.provider_id
             ORDER BY p.name, m.model_name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ModelPricing {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                provider_name: row.get(2)?,
                model_name: row.get(3)?,
                input_price_per_million: row.get(4)?,
                output_price_per_million: row.get(5)?,
                is_expensive: row.get::<_, i64>(6)? != 0,
            })
        })?;
        rows.collect()
    }

    pub fn update_model_pricing(
        &self,
        id: i64,
        input_price: f64,
        output_price: f64,
        is_expensive: bool,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE models SET input_price_per_million = ?1, output_price_per_million = ?2, is_expensive = ?3 WHERE id = ?4",
            params![input_price, output_price, if is_expensive { 1 } else { 0 }, id],
        )?;
        Ok(())
    }

    pub fn get_budget_settings(&self) -> Result<BudgetSettings> {
        self.conn.query_row(
            "SELECT id, daily_limit, monthly_limit, timezone, alert_threshold_50, alert_threshold_80, alert_threshold_100, spike_detection_enabled, expensive_model_warning FROM budget_settings WHERE id = 1",
            [],
            |row| {
                Ok(BudgetSettings {
                    id: row.get(0)?,
                    daily_limit: row.get(1)?,
                    monthly_limit: row.get(2)?,
                    timezone: row.get(3)?,
                    alert_threshold_50: row.get(4)?,
                    alert_threshold_80: row.get(5)?,
                    alert_threshold_100: row.get(6)?,
                    spike_detection_enabled: row.get::<_, i64>(7)? != 0,
                    expensive_model_warning: row.get::<_, i64>(8)? != 0,
                })
            },
        )
    }

    pub fn update_budget_settings(&self, settings: &UpdateBudgetPayload) -> Result<()> {
        self.conn.execute(
            "UPDATE budget_settings SET daily_limit = ?1, monthly_limit = ?2, timezone = ?3,
             alert_threshold_50 = ?4, alert_threshold_80 = ?5, alert_threshold_100 = ?6,
             spike_detection_enabled = ?7, expensive_model_warning = ?8 WHERE id = 1",
            params![
                settings.daily_limit,
                settings.monthly_limit,
                settings.timezone,
                settings.alert_threshold_50,
                settings.alert_threshold_80,
                settings.alert_threshold_100,
                if settings.spike_detection_enabled { 1 } else { 0 },
                if settings.expensive_model_warning { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    pub fn insert_usage_event(&self, event: &UsageEvent) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO usage_events (provider, model, prompt_tokens, completion_tokens, total_tokens, input_cost, output_cost, total_cost, project_name, request_id, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                event.provider,
                event.model,
                event.prompt_tokens,
                event.completion_tokens,
                event.total_tokens,
                event.input_cost,
                event.output_cost,
                event.total_cost,
                event.project_name,
                event.request_id,
                event.timestamp,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_widget_show_demo(&self) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT value FROM app_meta WHERE key = 'widget_show_demo'",
                [],
                |r| {
                    let v: String = r.get(0)?;
                    Ok(v == "true")
                },
            )
            .optional()?
            .unwrap_or(false))
    }

    pub fn set_widget_show_demo(&self, enabled: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_meta (key, value) VALUES ('widget_show_demo', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![if enabled { "true" } else { "false" }],
        )?;
        Ok(())
    }

    pub fn count_demo_events(&self, provider: &str) -> Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM usage_events WHERE provider = ?1 AND project_name = ?2",
            params![provider, DEMO_PROJECT],
            |r| r.get(0),
        )
    }

    pub fn clear_widget_demo_events(&self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM usage_events WHERE project_name = ?1",
            params![DEMO_PROJECT],
        )?;
        self.rebuild_daily_summaries()
    }

    pub fn purge_non_live_events(&self) -> Result<()> {
        if self.get_widget_show_demo()? {
            self.conn.execute(
                "DELETE FROM usage_events WHERE
                    (provider NOT IN (SELECT name FROM providers WHERE sync_status = 'connected')
                     AND (project_name IS NULL OR project_name != ?1))
                 OR (project_name = ?1 AND provider NOT IN ('OpenAI', 'AWS Bedrock'))",
                params![DEMO_PROJECT],
            )?;
        } else {
            self.conn.execute(
                "DELETE FROM usage_events WHERE provider NOT IN (
                    SELECT name FROM providers WHERE sync_status = 'connected'
                ) OR project_name = ?1",
                params![DEMO_PROJECT],
            )?;
        }
        self.rebuild_daily_summaries()
    }

    pub fn connected_provider_names(&self) -> Result<Vec<String>> {
        Ok(self
            .get_providers()?
            .into_iter()
            .filter(|p| p.sync_status.as_deref() == Some("connected"))
            .map(|p| p.name)
            .collect())
    }

    pub fn get_usage_events(&self, limit: i64, offset: i64) -> Result<Vec<UsageEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, provider, model, prompt_tokens, completion_tokens, total_tokens, input_cost, output_cost, total_cost, project_name, request_id, timestamp
             FROM usage_events ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit, offset], |row| {
            Ok(UsageEvent {
                id: Some(row.get(0)?),
                provider: row.get(1)?,
                model: row.get(2)?,
                prompt_tokens: row.get(3)?,
                completion_tokens: row.get(4)?,
                total_tokens: row.get(5)?,
                input_cost: row.get(6)?,
                output_cost: row.get(7)?,
                total_cost: row.get(8)?,
                project_name: row.get(9)?,
                request_id: row.get(10)?,
                timestamp: row.get(11)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_period_stats(&self, since: &str) -> Result<PeriodStats> {
        self.get_period_stats_filtered(since, None)
    }

    pub fn get_period_stats_for_provider(&self, since: &str, provider: &str) -> Result<PeriodStats> {
        self.get_period_stats_filtered(since, Some(provider))
    }

    pub fn get_period_stats_widget(
        &self,
        since: &str,
        scope: &WidgetQueryScope,
    ) -> Result<PeriodStats> {
        if scope.demo_only {
            if let Some(p) = scope.provider.as_deref() {
                return self.conn.query_row(
                    "SELECT COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0), COALESCE(SUM(input_cost), 0), COALESCE(SUM(output_cost), 0), COALESCE(COUNT(*), 0)
                     FROM usage_events WHERE timestamp >= ?1 AND provider = ?2 AND project_name = ?3",
                    params![since, p, DEMO_PROJECT],
                    period_stats_row,
                );
            }
        }

        if !scope.live_only {
            return match scope.provider.as_deref() {
                Some(p) => self.get_period_stats_for_provider(since, p),
                None => self.get_period_stats(since),
            };
        }

        if let Some(p) = scope.provider.as_deref() {
            if scope.include_demo_overlay {
                self.conn.query_row(
                    "SELECT COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0), COALESCE(SUM(input_cost), 0), COALESCE(SUM(output_cost), 0), COALESCE(COUNT(*), 0)
                     FROM usage_events WHERE timestamp >= ?1 AND provider = ?2",
                    params![since, p],
                    period_stats_row,
                )
            } else {
                self.conn.query_row(
                    "SELECT COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0), COALESCE(SUM(input_cost), 0), COALESCE(SUM(output_cost), 0), COALESCE(COUNT(*), 0)
                     FROM usage_events WHERE timestamp >= ?1 AND provider = ?2
                     AND (project_name IS NULL OR project_name != ?3)",
                    params![since, p, DEMO_PROJECT],
                    period_stats_row,
                )
            }
        } else if scope.include_demo_overlay {
            let live = scope.live_providers.join("','");
            let sql = format!(
                "SELECT COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0), COALESCE(SUM(input_cost), 0), COALESCE(SUM(output_cost), 0), COALESCE(COUNT(*), 0)
                 FROM usage_events WHERE timestamp >= ?1 AND (
                    ((project_name IS NULL OR project_name != ?2) AND provider IN ('{live}'))
                    OR (project_name = ?2 AND provider IN ('OpenAI', 'AWS Bedrock'))
                 )"
            );
            self.conn.query_row(&sql, params![since, DEMO_PROJECT], period_stats_row)
        } else {
            let live = scope.live_providers.join("','");
            let sql = format!(
                "SELECT COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0), COALESCE(SUM(input_cost), 0), COALESCE(SUM(output_cost), 0), COALESCE(COUNT(*), 0)
                 FROM usage_events WHERE timestamp >= ?1
                 AND (project_name IS NULL OR project_name != ?2)
                 AND provider IN ('{live}')"
            );
            self.conn.query_row(&sql, params![since, DEMO_PROJECT], period_stats_row)
        }
    }

    pub fn get_provider_costs_since_widget(
        &self,
        since: &str,
        scope: &WidgetQueryScope,
    ) -> Result<Vec<ProviderCost>> {
        if !scope.live_only {
            return match scope.provider.as_deref() {
                Some(p) => self.get_provider_costs_for_provider_since(since, p),
                None => self.get_provider_costs_since(since),
            };
        }

        if scope.demo_only {
            if let Some(p) = scope.provider.as_deref() {
                let rows: Vec<(String, f64, i64)> = self
                    .conn
                    .prepare(
                        "SELECT provider, SUM(total_cost), SUM(total_tokens) FROM usage_events
                         WHERE timestamp >= ?1 AND provider = ?2 AND project_name = ?3
                         GROUP BY provider",
                    )?
                    .query_map(params![since, p, DEMO_PROJECT], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })?
                    .filter_map(|r| r.ok())
                    .collect();
                let total: f64 = rows.iter().map(|(_, c, _)| c).sum();
                return Ok(rows
                    .into_iter()
                    .map(|(provider, cost, tokens)| ProviderCost {
                        provider,
                        cost,
                        tokens,
                        pct: if total > 0.0 { (cost / total) * 100.0 } else { 0.0 },
                    })
                    .collect());
            }
        }

        let rows: Vec<(String, f64, i64)> = if let Some(p) = scope.provider.as_deref() {
            if scope.include_demo_overlay {
                self.conn
                    .prepare(
                        "SELECT provider, SUM(total_cost), SUM(total_tokens) FROM usage_events
                         WHERE timestamp >= ?1 AND provider = ?2 GROUP BY provider",
                    )?
                    .query_map(params![since, p], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })?
                    .filter_map(|r| r.ok())
                    .collect()
            } else {
                self.conn
                    .prepare(
                        "SELECT provider, SUM(total_cost), SUM(total_tokens) FROM usage_events
                         WHERE timestamp >= ?1 AND provider = ?2
                         AND (project_name IS NULL OR project_name != ?3)
                         GROUP BY provider",
                    )?
                    .query_map(params![since, p, DEMO_PROJECT], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })?
                    .filter_map(|r| r.ok())
                    .collect()
            }
        } else if scope.include_demo_overlay {
            let live = scope.live_providers.join("','");
            let sql = format!(
                "SELECT provider, SUM(total_cost), SUM(total_tokens) FROM usage_events
                 WHERE timestamp >= ?1 AND (
                    ((project_name IS NULL OR project_name != ?2) AND provider IN ('{live}'))
                    OR (project_name = ?2 AND provider IN ('OpenAI', 'AWS Bedrock'))
                 ) GROUP BY provider ORDER BY SUM(total_cost) DESC"
            );
            self.conn
                .prepare(&sql)?
                .query_map(params![since, DEMO_PROJECT], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            let live = scope.live_providers.join("','");
            let sql = format!(
                "SELECT provider, SUM(total_cost), SUM(total_tokens) FROM usage_events
                 WHERE timestamp >= ?1 AND (project_name IS NULL OR project_name != ?2)
                 AND provider IN ('{live}')
                 GROUP BY provider ORDER BY SUM(total_cost) DESC"
            );
            self.conn
                .prepare(&sql)?
                .query_map(params![since, DEMO_PROJECT], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?
                .filter_map(|r| r.ok())
                .collect()
        };

        let total: f64 = rows.iter().map(|(_, c, _)| c).sum();
        Ok(rows
            .into_iter()
            .map(|(provider, cost, tokens)| ProviderCost {
                provider,
                cost,
                tokens,
                pct: if total > 0.0 { (cost / total) * 100.0 } else { 0.0 },
            })
            .collect())
    }

    pub fn get_hourly_sparkline_widget(
        &self,
        hours: i64,
        scope: &WidgetQueryScope,
    ) -> Result<Vec<HourlyPoint>> {
        if !scope.live_only && !scope.demo_only {
            return match scope.provider.as_deref() {
                Some(p) => self.get_hourly_sparkline_for_provider(hours, p),
                None => self.get_hourly_sparkline(hours),
            };
        }

        let since = (Utc::now() - Duration::hours(hours)).to_rfc3339();
        sparkline_query(self, &since, scope, true)
    }

    pub fn get_daily_sparkline_widget(
        &self,
        days: i64,
        scope: &WidgetQueryScope,
    ) -> Result<Vec<HourlyPoint>> {
        if !scope.live_only && !scope.demo_only {
            return match scope.provider.as_deref() {
                Some(p) => self.get_daily_sparkline_for_provider(days, p),
                None => self.get_daily_sparkline(days),
            };
        }

        let since = (Utc::now() - Duration::days(days)).to_rfc3339();
        sparkline_query(self, &since, scope, false)
    }

    pub fn get_top_provider_model_widget(
        &self,
        since: &str,
        scope: &WidgetQueryScope,
    ) -> Result<(String, String)> {
        if !scope.live_only && !scope.demo_only {
            if let Some(p) = scope.provider.as_deref() {
                let model = self.get_top_model_for_provider(since, p)?;
                return Ok((p.to_string(), model));
            }
            return self.get_top_provider_model(since);
        }

        if scope.demo_only {
            if let Some(p) = scope.provider.as_deref() {
                let model = self.get_top_model_for_provider_widget(since, p, scope)?;
                return Ok((p.to_string(), model));
            }
        }

        if let Some(p) = scope.provider.as_deref() {
            let model = self.get_top_model_for_provider_widget(since, p, scope)?;
            return Ok((p.to_string(), model));
        }

        let top_provider = top_provider_widget(self, since, scope)?;
        let top_model = if top_provider == "—" {
            "—".to_string()
        } else {
            self.get_top_model_for_provider_widget(since, &top_provider, scope)?
        };
        Ok((top_provider, top_model))
    }

    pub fn get_top_model_for_provider_widget(
        &self,
        since: &str,
        provider: &str,
        scope: &WidgetQueryScope,
    ) -> Result<String> {
        if !scope.live_only && !scope.demo_only {
            return self.get_top_model_for_provider(since, provider);
        }

        if scope.demo_only {
            return Ok(self
                .conn
                .query_row(
                    "SELECT model FROM usage_events WHERE timestamp >= ?1 AND provider = ?2 AND project_name = ?3
                     GROUP BY model ORDER BY SUM(total_cost) DESC LIMIT 1",
                    params![since, provider, DEMO_PROJECT],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| "—".to_string()));
        }

        let sql = if scope.include_demo_overlay {
            "SELECT model FROM usage_events WHERE timestamp >= ?1 AND provider = ?2
             GROUP BY model ORDER BY SUM(total_cost) DESC LIMIT 1"
        } else {
            "SELECT model FROM usage_events WHERE timestamp >= ?1 AND provider = ?2
             AND (project_name IS NULL OR project_name != ?3)
             GROUP BY model ORDER BY SUM(total_cost) DESC LIMIT 1"
        };

        if scope.include_demo_overlay {
            Ok(self
                .conn
                .query_row(sql, params![since, provider], |r| r.get(0))
                .unwrap_or_else(|_| "—".to_string()))
        } else {
            Ok(self
                .conn
                .query_row(sql, params![since, provider, DEMO_PROJECT], |r| r.get(0))
                .unwrap_or_else(|_| "—".to_string()))
        }
    }

    pub fn build_widget_query_scope(
        &self,
        provider: Option<&str>,
    ) -> Result<WidgetQueryScope> {
        let mut live_providers = self.connected_provider_names()?;
        if let Some(p) = provider {
            live_providers.retain(|name| name == p);
        }
        let show_demo = self.get_widget_show_demo()?;
        let live_only = !live_providers.is_empty();
        let is_widget_demo_provider = provider
            .as_deref()
            .map(|p| WIDGET_DEMO_PROVIDERS.contains(&p))
            .unwrap_or(false);
        let demo_only = show_demo && !live_only && is_widget_demo_provider;
        Ok(WidgetQueryScope {
            live_only,
            include_demo_overlay: show_demo && live_only,
            demo_only,
            live_providers,
            provider: provider.map(str::to_string),
        })
    }

    fn get_period_stats_filtered(&self, since: &str, provider: Option<&str>) -> Result<PeriodStats> {
        let map_row = |row: &rusqlite::Row<'_>| {
            let total_tokens: i64 = row.get(0)?;
            let total_cost: f64 = row.get(1)?;
            let event_count: i64 = row.get(4)?;
            let hours = 24.0_f64.max(1.0);
            Ok(PeriodStats {
                total_tokens,
                total_cost,
                input_cost: row.get(2)?,
                output_cost: row.get(3)?,
                event_count,
                burn_rate_per_hour: total_cost / hours,
                estimated_monthly: (total_cost / hours) * 24.0 * 30.0,
            })
        };

        if let Some(p) = provider {
            self.conn.query_row(
                "SELECT COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0), COALESCE(SUM(input_cost), 0), COALESCE(SUM(output_cost), 0), COALESCE(COUNT(*), 0)
                 FROM usage_events WHERE timestamp >= ?1 AND provider = ?2",
                params![since, p],
                map_row,
            )
        } else {
            self.conn.query_row(
                "SELECT COALESCE(SUM(total_tokens), 0), COALESCE(SUM(total_cost), 0), COALESCE(SUM(input_cost), 0), COALESCE(SUM(output_cost), 0), COALESCE(COUNT(*), 0)
                 FROM usage_events WHERE timestamp >= ?1",
                params![since],
                map_row,
            )
        }
    }

    /// Local calendar midnight as UTC RFC3339 (`…Z`) so SQLite `timestamp >= ?` compares correctly
    /// against usage events stored in UTC (mixed `+05:30` / `+00:00` strings sort lexicographically wrong).
    pub fn get_today_start(&self) -> String {
        Local::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
            .with_timezone(&Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    pub fn get_provider_costs_since(&self, since: &str) -> Result<Vec<ProviderCost>> {
        self.get_provider_costs_filtered(since, None)
    }

    pub fn get_provider_costs_for_provider_since(
        &self,
        since: &str,
        provider: &str,
    ) -> Result<Vec<ProviderCost>> {
        self.get_provider_costs_filtered(since, Some(provider))
    }

    fn get_provider_costs_filtered(
        &self,
        since: &str,
        provider: Option<&str>,
    ) -> Result<Vec<ProviderCost>> {
        let rows: Vec<(String, f64, i64)> = if let Some(p) = provider {
            self.conn
                .prepare(
                    "SELECT provider, SUM(total_cost), SUM(total_tokens) FROM usage_events
                     WHERE timestamp >= ?1 AND provider = ?2 GROUP BY provider",
                )?
                .query_map(params![since, p], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?
                .filter_map(|r| r.ok())
                .collect()
        } else {
            self.conn
                .prepare(
                    "SELECT provider, SUM(total_cost), SUM(total_tokens) FROM usage_events
                     WHERE timestamp >= ?1 GROUP BY provider ORDER BY SUM(total_cost) DESC",
                )?
                .query_map(params![since], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?
                .filter_map(|r| r.ok())
                .collect()
        };

        let total: f64 = rows.iter().map(|(_, c, _)| c).sum();
        Ok(rows
            .into_iter()
            .map(|(provider, cost, tokens)| ProviderCost {
                provider,
                cost,
                tokens,
                pct: if total > 0.0 { (cost / total) * 100.0 } else { 0.0 },
            })
            .collect())
    }

    pub fn get_model_costs_since(&self, since: &str) -> Result<Vec<ModelCost>> {
        let mut stmt = self.conn.prepare(
            "SELECT model, provider, SUM(total_cost), SUM(total_tokens), COUNT(*) FROM usage_events WHERE timestamp >= ?1 GROUP BY model, provider ORDER BY SUM(total_cost) DESC",
        )?;
        let rows = stmt.query_map(params![since], |row| {
            Ok(ModelCost {
                model: row.get(0)?,
                provider: row.get(1)?,
                cost: row.get(2)?,
                tokens: row.get(3)?,
                request_count: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_hourly_sparkline(&self, hours: i64) -> Result<Vec<HourlyPoint>> {
        self.get_hourly_sparkline_filtered(hours, None)
    }

    pub fn get_hourly_sparkline_for_provider(
        &self,
        hours: i64,
        provider: &str,
    ) -> Result<Vec<HourlyPoint>> {
        self.get_hourly_sparkline_filtered(hours, Some(provider))
    }

    fn get_hourly_sparkline_filtered(
        &self,
        hours: i64,
        provider: Option<&str>,
    ) -> Result<Vec<HourlyPoint>> {
        let since = (Utc::now() - Duration::hours(hours)).to_rfc3339();
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(HourlyPoint {
                hour: row.get(0)?,
                tokens: row.get(1)?,
                cost: row.get(2)?,
            })
        };

        if let Some(p) = provider {
            let mut stmt = self.conn.prepare(
                "SELECT strftime('%H:00', timestamp) as hour, SUM(total_tokens), SUM(total_cost)
                 FROM usage_events WHERE timestamp >= ?1 AND provider = ?2
                 GROUP BY strftime('%Y-%m-%d %H', timestamp) ORDER BY timestamp",
            )?;
            let rows = stmt.query_map(params![since, p], map_row)?;
            rows.collect()
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT strftime('%H:00', timestamp) as hour, SUM(total_tokens), SUM(total_cost)
                 FROM usage_events WHERE timestamp >= ?1
                 GROUP BY strftime('%Y-%m-%d %H', timestamp) ORDER BY timestamp",
            )?;
            let rows = stmt.query_map(params![since], map_row)?;
            rows.collect()
        }
    }

    pub fn get_daily_sparkline(&self, days: i64) -> Result<Vec<HourlyPoint>> {
        self.get_daily_sparkline_filtered(days, None)
    }

    pub fn get_daily_sparkline_for_provider(
        &self,
        days: i64,
        provider: &str,
    ) -> Result<Vec<HourlyPoint>> {
        self.get_daily_sparkline_filtered(days, Some(provider))
    }

    fn get_daily_sparkline_filtered(
        &self,
        days: i64,
        provider: Option<&str>,
    ) -> Result<Vec<HourlyPoint>> {
        let since = (Utc::now() - Duration::days(days)).to_rfc3339();
        let map_row = |row: &rusqlite::Row<'_>| {
            let d: String = row.get(0)?;
            Ok(HourlyPoint {
                hour: d.chars().skip(5).collect(),
                tokens: row.get(1)?,
                cost: row.get(2)?,
            })
        };

        if let Some(p) = provider {
            let mut stmt = self.conn.prepare(
                "SELECT date(timestamp) as d, SUM(total_tokens), SUM(total_cost)
                 FROM usage_events WHERE timestamp >= ?1 AND provider = ?2
                 GROUP BY date(timestamp) ORDER BY d",
            )?;
            let rows = stmt.query_map(params![since, p], map_row)?;
            rows.collect()
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT date(timestamp) as d, SUM(total_tokens), SUM(total_cost)
                 FROM usage_events WHERE timestamp >= ?1
                 GROUP BY date(timestamp) ORDER BY d",
            )?;
            let rows = stmt.query_map(params![since], map_row)?;
            rows.collect()
        }
    }

    pub fn get_top_model_for_provider(&self, since: &str, provider: &str) -> Result<String> {
        Ok(self
            .conn
            .query_row(
                "SELECT model FROM usage_events WHERE timestamp >= ?1 AND provider = ?2 GROUP BY model ORDER BY SUM(total_cost) DESC LIMIT 1",
                params![since, provider],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "—".to_string()))
    }

    pub fn get_top_provider_model(&self, since: &str) -> Result<(String, String)> {
        let top_provider: String = self
            .conn
            .query_row(
                "SELECT provider FROM usage_events WHERE timestamp >= ?1 GROUP BY provider ORDER BY SUM(total_cost) DESC LIMIT 1",
                params![since],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "—".to_string());

        let top_model: String = self
            .conn
            .query_row(
                "SELECT model FROM usage_events WHERE timestamp >= ?1 GROUP BY model ORDER BY SUM(total_cost) DESC LIMIT 1",
                params![since],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "—".to_string());

        Ok((top_provider, top_model))
    }

    pub fn insert_alert(&self, alert: &Alert) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO alerts (alert_type, severity, message, provider, model, value, threshold, is_read, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                alert.alert_type,
                alert.severity,
                alert.message,
                alert.provider,
                alert.model,
                alert.value,
                alert.threshold,
                if alert.is_read { 1 } else { 0 },
                alert.created_at,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_alerts(&self, limit: i64) -> Result<Vec<Alert>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, alert_type, severity, message, provider, model, value, threshold, is_read, created_at
             FROM alerts ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(Alert {
                id: row.get(0)?,
                alert_type: row.get(1)?,
                severity: row.get(2)?,
                message: row.get(3)?,
                provider: row.get(4)?,
                model: row.get(5)?,
                value: row.get(6)?,
                threshold: row.get(7)?,
                is_read: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn mark_alert_read(&self, id: i64) -> Result<()> {
        self.conn
            .execute("UPDATE alerts SET is_read = 1 WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_model_pricing(&self, provider: &str, model: &str) -> Result<Option<(f64, f64, bool)>> {
        match self.conn.query_row(
            "SELECT m.input_price_per_million, m.output_price_per_million, m.is_expensive
             FROM models m JOIN providers p ON p.id = m.provider_id
             WHERE p.name = ?1 AND m.model_name = ?2",
            params![provider, model],
            |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? != 0)),
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn get_hourly_cost_last_n(&self, n: i64) -> Result<Vec<f64>> {
        let mut costs = Vec::new();
        let now = Utc::now();
        for h in (0..n).rev() {
            let start = now - Duration::hours(h + 1);
            let end = now - Duration::hours(h);
            let cost: f64 = self.conn.query_row(
                "SELECT COALESCE(SUM(total_cost), 0) FROM usage_events WHERE timestamp >= ?1 AND timestamp < ?2",
                params![start.to_rfc3339(), end.to_rfc3339()],
                |r| r.get(0),
            )?;
            costs.push(cost);
        }
        Ok(costs)
    }

    pub fn get_provider_hourly_cost(&self, provider: &str) -> Result<f64> {
        let since = (Utc::now() - Duration::hours(1)).to_rfc3339();
        self.conn.query_row(
            "SELECT COALESCE(SUM(total_cost), 0) FROM usage_events WHERE provider = ?1 AND timestamp >= ?2",
            params![provider, since],
            |r| r.get(0),
        )
    }

    pub fn count_alerts_today(&self, alert_type: &str) -> Result<i64> {
        let today = Local::now().date_naive().to_string();
        self.conn.query_row(
            "SELECT COUNT(*) FROM alerts WHERE alert_type = ?1 AND date(created_at) = ?2",
            params![alert_type, today],
            |r| r.get(0),
        )
    }
}

#[derive(Debug, Clone)]
pub struct WidgetQueryScope {
    pub live_only: bool,
    pub include_demo_overlay: bool,
    pub demo_only: bool,
    pub live_providers: Vec<String>,
    pub provider: Option<String>,
}

fn period_stats_row(row: &rusqlite::Row<'_>) -> Result<PeriodStats> {
    let total_tokens: i64 = row.get(0)?;
    let total_cost: f64 = row.get(1)?;
    let event_count: i64 = row.get(4)?;
    let hours = 24.0_f64.max(1.0);
    Ok(PeriodStats {
        total_tokens,
        total_cost,
        input_cost: row.get(2)?,
        output_cost: row.get(3)?,
        event_count,
        burn_rate_per_hour: total_cost / hours,
        estimated_monthly: (total_cost / hours) * 24.0 * 30.0,
    })
}

fn widget_where_sql(scope: &WidgetQueryScope, since_param: &str) -> (String, bool) {
    if scope.demo_only {
        return (
            format!("timestamp >= {since_param} AND provider = ? AND project_name = ?"),
            false,
        );
    }

    if scope.provider.is_some() {
        if scope.include_demo_overlay {
            (
                format!("timestamp >= {since_param} AND provider = ?"),
                false,
            )
        } else {
            (
                format!(
                    "timestamp >= {since_param} AND provider = ? AND (project_name IS NULL OR project_name != ?)"
                ),
                true,
            )
        }
    } else if scope.include_demo_overlay {
        let live = scope.live_providers.join("','");
        (
            format!(
                "timestamp >= {since_param} AND (
                    ((project_name IS NULL OR project_name != ?) AND provider IN ('{live}'))
                    OR (project_name = ? AND provider IN ('OpenAI', 'AWS Bedrock'))
                )"
            ),
            false,
        )
    } else {
        let live = scope.live_providers.join("','");
        (
            format!(
                "timestamp >= {since_param} AND (project_name IS NULL OR project_name != ?) AND provider IN ('{live}')"
            ),
            false,
        )
    }
}

fn sparkline_query(
    db: &Database,
    since: &str,
    scope: &WidgetQueryScope,
    hourly: bool,
) -> Result<Vec<HourlyPoint>> {
    let (where_sql, extra_demo_param) = widget_where_sql(scope, "?1");
    let group_expr = if hourly {
        "strftime('%H:00', timestamp)"
    } else {
        "date(timestamp)"
    };
    let sql = format!(
        "SELECT {group_expr} as bucket, SUM(total_tokens), SUM(total_cost)
         FROM usage_events WHERE {where_sql}
         GROUP BY bucket ORDER BY bucket"
    );

    let map_row = |row: &rusqlite::Row<'_>| {
        let bucket: String = row.get(0)?;
        let hour = if hourly {
            bucket
        } else {
            bucket.chars().skip(5).collect()
        };
        Ok(HourlyPoint {
            hour,
            tokens: row.get(1)?,
            cost: row.get(2)?,
        })
    };

    if let Some(p) = scope.provider.as_deref() {
        if scope.demo_only {
            db.conn
                .prepare(&sql)?
                .query_map(params![since, p, DEMO_PROJECT], map_row)?
                .collect()
        } else if extra_demo_param {
            db.conn
                .prepare(&sql)?
                .query_map(params![since, p, DEMO_PROJECT], map_row)?
                .collect()
        } else {
            db.conn
                .prepare(&sql)?
                .query_map(params![since, p], map_row)?
                .collect()
        }
    } else if scope.include_demo_overlay {
        db.conn
            .prepare(&sql)?
            .query_map(params![since, DEMO_PROJECT, DEMO_PROJECT], map_row)?
            .collect()
    } else {
        db.conn
            .prepare(&sql)?
            .query_map(params![since, DEMO_PROJECT], map_row)?
            .collect()
    }
}

fn top_provider_widget(db: &Database, since: &str, scope: &WidgetQueryScope) -> Result<String> {
    let (where_sql, extra_demo_param) = widget_where_sql(scope, "?1");
    let sql = format!(
        "SELECT provider FROM usage_events WHERE {where_sql}
         GROUP BY provider ORDER BY SUM(total_cost) DESC LIMIT 1"
    );

    if let Some(p) = scope.provider.as_deref() {
        if scope.demo_only {
            Ok(db
                .conn
                .query_row(&sql, params![since, p, DEMO_PROJECT], |r| r.get(0))
                .unwrap_or_else(|_| "—".to_string()))
        } else if extra_demo_param {
            Ok(db
                .conn
                .query_row(&sql, params![since, p, DEMO_PROJECT], |r| r.get(0))
                .unwrap_or_else(|_| "—".to_string()))
        } else {
            Ok(db
                .conn
                .query_row(&sql, params![since, p], |r| r.get(0))
                .unwrap_or_else(|_| "—".to_string()))
        }
    } else if scope.include_demo_overlay {
        Ok(db
            .conn
            .query_row(&sql, params![since, DEMO_PROJECT, DEMO_PROJECT], |r| r.get(0))
            .unwrap_or_else(|_| "—".to_string()))
    } else {
        Ok(db
            .conn
            .query_row(&sql, params![since, DEMO_PROJECT], |r| r.get(0))
            .unwrap_or_else(|_| "—".to_string()))
    }
}
