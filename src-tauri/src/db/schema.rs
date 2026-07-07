pub const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    api_key TEXT,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    key_source TEXT,
    sync_status TEXT,
    sync_message TEXT,
    last_synced_at TEXT,
    credit_available REAL,
    credit_granted REAL,
    credit_used REAL,
    credit_monthly_limit REAL,
    credit_month_spend REAL,
    credit_source TEXT,
    credit_currency TEXT,
    credit_synced_at TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS models (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id INTEGER NOT NULL,
    model_name TEXT NOT NULL,
    input_price_per_million REAL NOT NULL,
    output_price_per_million REAL NOT NULL,
    is_expensive INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (provider_id) REFERENCES providers(id),
    UNIQUE(provider_id, model_name)
);

CREATE TABLE IF NOT EXISTS usage_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_tokens INTEGER NOT NULL,
    completion_tokens INTEGER NOT NULL,
    total_tokens INTEGER NOT NULL,
    input_cost REAL NOT NULL,
    output_cost REAL NOT NULL,
    total_cost REAL NOT NULL,
    project_name TEXT,
    request_id TEXT,
    timestamp TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_events_timestamp ON usage_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_usage_events_provider ON usage_events(provider);

CREATE TABLE IF NOT EXISTS daily_summary (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT,
    total_tokens INTEGER NOT NULL,
    total_cost REAL NOT NULL,
    event_count INTEGER NOT NULL,
    UNIQUE(date, provider, model)
);

CREATE TABLE IF NOT EXISTS budget_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    daily_limit REAL NOT NULL DEFAULT 50.0,
    monthly_limit REAL NOT NULL DEFAULT 1500.0,
    timezone TEXT NOT NULL DEFAULT 'America/New_York',
    alert_threshold_50 REAL NOT NULL DEFAULT 0.5,
    alert_threshold_80 REAL NOT NULL DEFAULT 0.8,
    alert_threshold_100 REAL NOT NULL DEFAULT 1.0,
    spike_detection_enabled INTEGER NOT NULL DEFAULT 1,
    expensive_model_warning INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    provider TEXT,
    model TEXT,
    value REAL,
    threshold REAL,
    is_read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS app_secrets (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;
