use crate::db::Database;
use crate::engine::{AlertEngine, CostEngine, clear_widget_demo_events, ensure_widget_demo_events, model_costs_live, provider_costs_live, usage_events_live};
use crate::models::DashboardStats;
use crate::env;
use crate::models::*;
use crate::credentials::{
    get_aws_credentials_status, get_azure_credentials_status, get_openai_credentials_status,
    update_aws_credentials, update_azure_credentials, update_openai_credentials,
};
use crate::sync::{refresh_live_data_inner, sync_all_providers};
use chrono::{Duration, Utc};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, LogicalPosition, Manager, State, WebviewUrl, WebviewWindowBuilder};

const WIDGET_PROVIDERS: &[(&str, &str)] = &[
    ("OpenAI", "openai"),
    ("AWS Bedrock", "aws-bedrock"),
    ("Azure OpenAI", "azure-openai"),
];

pub struct WindowLayoutState {
    pub providers_before_expand: Mutex<Vec<String>>,
    pub main_expanded: Mutex<bool>,
}

impl WindowLayoutState {
    pub fn new() -> Self {
        Self {
            providers_before_expand: Mutex::new(Vec::new()),
            main_expanded: Mutex::new(false),
        }
    }
}

pub fn emit_view_state_changed(app: &AppHandle) {
    let _ = app.emit("view-state-changed", ());
}

fn set_main_expanded(app: &AppHandle, expanded: bool) {
    if let Some(state) = app.try_state::<WindowLayoutState>() {
        *state.main_expanded.lock().unwrap() = expanded;
    }
}

pub fn expand_main_dashboard(app: &AppHandle) -> Result<(), String> {
    prepare_dashboard_expand(app)?;
    set_main_expanded(app, true);

    let Some(main) = app.get_webview_window("main") else {
        return Ok(());
    };

    main
        .set_always_on_top(false)
        .map_err(|e| e.to_string())?;
    apply_window_mode(&main, "expanded")?;
    main.show().map_err(|e| e.to_string())?;
    main.set_focus().map_err(|e| e.to_string())?;
    emit_view_state_changed(app);
    Ok(())
}

pub fn provider_to_slug(provider: &str) -> Result<&'static str, String> {
    WIDGET_PROVIDERS
        .iter()
        .find(|(name, _)| *name == provider)
        .map(|(_, slug)| *slug)
        .ok_or_else(|| format!("No dedicated widget for provider: {provider}"))
}

pub fn slug_to_provider(slug: &str) -> Option<&'static str> {
    WIDGET_PROVIDERS
        .iter()
        .find(|(_, s)| *s == slug)
        .map(|(name, _)| *name)
}

pub fn widget_label_for_provider(provider: &str) -> Result<String, String> {
    Ok(format!("widget-{}", provider_to_slug(provider)?))
}

fn hide_main_widget(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        set_main_expanded(app, false);
        let _ = apply_window_mode(&main, "widget");
        emit_view_state_changed(app);
        let _ = main.hide();
    }
}

fn snapshot_visible_providers(app: &AppHandle) -> Vec<String> {
    let mut visible = Vec::new();
    for (provider, slug) in WIDGET_PROVIDERS {
        let label = format!("widget-{slug}");
        if let Some(window) = app.get_webview_window(&label) {
            if window.is_visible().unwrap_or(false) {
                visible.push(provider.to_string());
            }
        }
    }
    visible
}

fn hide_provider_widgets(app: &AppHandle) {
    for (_, slug) in WIDGET_PROVIDERS {
        let label = format!("widget-{slug}");
        if let Some(window) = app.get_webview_window(&label) {
            let _ = window.hide();
        }
    }
}

fn restore_provider_widgets(app: &AppHandle) {
    let providers = app
        .try_state::<WindowLayoutState>()
        .map(|state| {
            let mut guard = state.providers_before_expand.lock().unwrap();
            std::mem::take(&mut *guard)
        })
        .unwrap_or_default();

    for provider in providers {
        let _ = show_provider_widget(app, &provider, false);
    }
}

fn apply_provider_widget_mode(window: &tauri::WebviewWindow, slug: &str) -> Result<(), String> {
    window
        .set_resizable(false)
        .map_err(|e| e.to_string())?;
    window
        .set_min_size(None::<tauri::LogicalSize<f64>>)
        .map_err(|e| e.to_string())?;
    window
        .set_size(tauri::LogicalSize::new(320.0, 220.0))
        .map_err(|e| e.to_string())?;
    let (x, y) = widget_position(slug);
    window
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn show_provider_widget(app: &AppHandle, provider: &str, hide_main: bool) -> Result<(), String> {
    let slug = provider_to_slug(provider)?;
    let label = format!("widget-{slug}");

    if let Some(window) = app.get_webview_window(&label) {
        apply_provider_widget_mode(&window, slug)?;
        window.show().map_err(|e| e.to_string())?;
        if hide_main {
            hide_main_widget(app);
        }
        return Ok(());
    }

    let (x, y) = widget_position(slug);
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title(&format!("TokenPulse — {provider}"))
        .inner_size(320.0, 220.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .accept_first_mouse(true)
        .visible(true)
        .position(x, y)
        .build()
        .map_err(|e| e.to_string())?;

    if hide_main {
        let _ = window.set_focus();
        hide_main_widget(app);
    }

    Ok(())
}

pub fn prepare_dashboard_expand(app: &AppHandle) -> Result<(), String> {
    let visible = snapshot_visible_providers(app);
    if let Some(state) = app.try_state::<WindowLayoutState>() {
        *state.providers_before_expand.lock().unwrap() = visible;
    }
    hide_provider_widgets(app);
    Ok(())
}

pub fn ensure_provider_widget_window(
    app: &AppHandle,
    provider: &str,
    show: bool,
) -> Result<(), String> {
    if show {
        show_provider_widget(app, provider, true)
    } else {
        let slug = provider_to_slug(provider)?;
        let label = format!("widget-{slug}");
        if app.get_webview_window(&label).is_some() {
            return Ok(());
        }

        let (x, y) = widget_position(slug);
        WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
            .title(&format!("TokenPulse — {provider}"))
            .inner_size(320.0, 220.0)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .accept_first_mouse(true)
            .visible(false)
            .position(x, y)
            .build()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub fn prewarm_provider_widgets(app: &AppHandle) {
    for (provider, _) in WIDGET_PROVIDERS {
        let _ = ensure_provider_widget_window(app, provider, false);
    }
}

pub fn open_provider_widget_inner(app: &AppHandle, provider: &str) -> Result<(), String> {
    ensure_provider_widget_window(app, provider, true)
}

fn widget_position(slug: &str) -> (f64, f64) {
    match slug {
        "openai" => (72.0, 110.0),
        "aws-bedrock" => (408.0, 110.0),
        "azure-openai" => (744.0, 110.0),
        _ => (240.0, 110.0),
    }
}

pub struct AppState {
    pub db: Mutex<Database>,
}

#[tauri::command]
pub fn get_widget_stats(
    state: State<AppState>,
    provider: Option<String>,
) -> Result<WidgetStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    CostEngine::build_widget_stats_for_provider(&db, provider.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_widget_demo_enabled(state: State<AppState>) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_widget_show_demo().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_widget_demo_enabled(
    app: AppHandle,
    state: State<AppState>,
    enabled: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_widget_show_demo(enabled).map_err(|e| e.to_string())?;
    if enabled {
        ensure_widget_demo_events(&db).map_err(|e| e.to_string())?;
    } else {
        clear_widget_demo_events(&db).map_err(|e| e.to_string())?;
    }
    drop(db);
    app.emit("widget-demo-changed", enabled)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_dashboard_stats(state: State<AppState>) -> Result<DashboardStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _ = AlertEngine::evaluate(&db);
    DashboardStats::build(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_providers(state: State<AppState>) -> Result<Vec<Provider>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_providers().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_provider_key(
    state: State<AppState>,
    payload: UpdateProviderKeyPayload,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_provider_key(&payload.provider_name, &payload.api_key, payload.is_enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_models(state: State<AppState>) -> Result<Vec<ModelPricing>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_models().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_model_pricing(
    state: State<AppState>,
    payload: UpdateModelPricingPayload,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_model_pricing(
        payload.id,
        payload.input_price_per_million,
        payload.output_price_per_million,
        payload.is_expensive,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_budget_settings(state: State<AppState>) -> Result<BudgetSettings, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_budget_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_budget_settings(
    state: State<AppState>,
    payload: UpdateBudgetPayload,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_budget_settings(&payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_usage_history(
    state: State<AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<UsageEvent>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    usage_events_live(&db, limit.unwrap_or(50), offset.unwrap_or(0)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_provider_breakdown(state: State<AppState>) -> Result<Vec<ProviderCost>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let since = (Utc::now() - Duration::days(7)).to_rfc3339();
    provider_costs_live(&db, &since).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_model_breakdown(state: State<AppState>) -> Result<Vec<ModelCost>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let since = (Utc::now() - Duration::days(7)).to_rfc3339();
    model_costs_live(&db, &since).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_alerts(state: State<AppState>, limit: Option<i64>) -> Result<Vec<Alert>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_alerts(limit.unwrap_or(50)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mark_alert_read(state: State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.mark_alert_read(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn detect_env_keys(deep: Option<bool>) -> Result<EnvDetection, String> {
    let deep = deep.unwrap_or(false);
    Ok(EnvDetection {
        openai_api_key: env::get_openai_api_key().is_some(),
        openai_admin_key: env::get_openai_admin_key().is_some(),
        openai_org_id: env::get_openai_org_id().is_some(),
        openai_billing_token: env::get_openai_billing_token().is_some(),
        openai_api_probe: if deep {
            env::probe_var("OPENAI_API_KEY")
        } else {
            env::probe_var_fast("OPENAI_API_KEY")
        },
        openai_admin_probe: if deep {
            env::probe_var("OPENAI_ADMIN_KEY")
        } else {
            env::probe_var_fast("OPENAI_ADMIN_KEY")
        },
        aws_access_key_id: crate::aws_config::get_access_key_id().is_some(),
        aws_secret_access_key: crate::aws_config::get_secret_access_key().is_some(),
        aws_region: crate::aws_config::get_region().is_some(),
        aws_profile: crate::aws_config::get_profile_name().is_some(),
        aws_cli_configured: crate::aws_config::credentials_file_exists(),
        aws_cli_available: crate::aws_config::aws_cli_available(),
        azure_openai_api_key: crate::azure_config::get_api_key().is_some(),
        azure_openai_endpoint: crate::azure_config::get_endpoint().is_some(),
        azure_openai_api_version: crate::azure_config::get_api_version().is_some(),
        azure_openai_deployment: crate::azure_config::get_deployment_name().is_some(),
        azure_subscription_id: crate::azure_config::get_subscription_id().is_some(),
        azure_resource_group: crate::azure_config::get_resource_group().is_some(),
        azure_cli_available: crate::azure_config::az_cli_available(),
        applied_keys: Vec::new(),
    })
}

#[tauri::command]
pub fn get_openai_credentials(state: State<AppState>) -> Result<OpenAICredentialsStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    get_openai_credentials_status(&db)
}

#[tauri::command]
pub fn update_openai_credentials_cmd(
    state: State<AppState>,
    api_key: Option<String>,
    admin_key: Option<String>,
    billing_token: Option<String>,
    org_id: Option<String>,
    payload: Option<UpdateOpenAICredentialsPayload>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let merged = payload.unwrap_or_default();
    update_openai_credentials(
        &db,
        api_key.or(merged.api_key).as_deref(),
        admin_key.or(merged.admin_key).as_deref(),
        billing_token.or(merged.billing_token).as_deref(),
        org_id.or(merged.org_id).as_deref(),
    )
}

#[tauri::command]
pub fn get_aws_credentials(state: State<AppState>) -> Result<AwsCredentialsStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    get_aws_credentials_status(&db)
}

#[tauri::command]
pub fn update_aws_credentials_cmd(
    state: State<AppState>,
    payload: UpdateAwsCredentialsPayload,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    update_aws_credentials(
        &db,
        payload.access_key_id.as_deref(),
        payload.secret_access_key.as_deref(),
        payload.session_token.as_deref(),
        payload.region.as_deref(),
        payload.profile.as_deref(),
    )
}

#[tauri::command]
pub fn refresh_live_data(state: State<AppState>) -> Result<Vec<SyncReport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    refresh_live_data_inner(&db)
}

#[tauri::command]
pub fn start_refresh_live_data(app: AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let payload = match app.try_state::<AppState>() {
            Some(state) => match state.db.lock() {
                Ok(db) => match refresh_live_data_inner(&db) {
                    Ok(reports) => LiveSyncFinished {
                        ok: true,
                        reports,
                        error: None,
                    },
                    Err(err) => LiveSyncFinished {
                        ok: false,
                        reports: Vec::new(),
                        error: Some(err),
                    },
                },
                Err(e) => LiveSyncFinished {
                    ok: false,
                    reports: Vec::new(),
                    error: Some(e.to_string()),
                },
            },
            None => LiveSyncFinished {
                ok: false,
                reports: Vec::new(),
                error: Some("App database not ready".into()),
            },
        };
        let _ = app.emit("live-sync-finished", payload);
    });
    Ok(())
}

#[tauri::command]
pub fn get_azure_credentials(state: State<AppState>) -> Result<AzureCredentialsStatus, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    get_azure_credentials_status(&db)
}

#[tauri::command]
pub fn update_azure_credentials_cmd(
    state: State<AppState>,
    payload: UpdateAzureCredentialsPayload,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    update_azure_credentials(
        &db,
        payload.api_key.as_deref(),
        payload.endpoint.as_deref(),
        payload.api_version.as_deref(),
        payload.deployment_name.as_deref(),
        payload.subscription_id.as_deref(),
        payload.resource_group.as_deref(),
    )
}

#[tauri::command]
pub fn sync_provider_usage(state: State<AppState>) -> Result<Vec<SyncReport>, String> {
    refresh_live_data(state)
}

#[tauri::command]
pub fn open_provider_widget(app: AppHandle, provider: String) -> Result<(), String> {
    open_provider_widget_inner(&app, &provider)
}

#[tauri::command]
pub fn open_main_dashboard(app: AppHandle) -> Result<(), String> {
    expand_main_dashboard(&app)
}

#[tauri::command]
pub fn get_main_view_expanded(app: AppHandle) -> Result<bool, String> {
    Ok(app
        .try_state::<WindowLayoutState>()
        .map(|state| *state.main_expanded.lock().unwrap())
        .unwrap_or(false))
}

#[tauri::command]
pub fn prepare_dashboard_expand_cmd(app: AppHandle) -> Result<(), String> {
    prepare_dashboard_expand(&app)
}

#[tauri::command]
pub fn collapse_to_widgets(app: AppHandle, window: tauri::WebviewWindow) -> Result<(), String> {
    set_main_expanded(&app, false);
    window
        .set_always_on_top(true)
        .map_err(|e| e.to_string())?;
    apply_window_mode(&window, "widget")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    restore_provider_widgets(&app);
    emit_view_state_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_widget_providers() -> Vec<String> {
    WIDGET_PROVIDERS
        .iter()
        .map(|(name, _)| name.to_string())
        .collect()
}

#[tauri::command]
pub fn set_always_on_top(window: tauri::WebviewWindow, pinned: bool) -> Result<(), String> {
    window
        .set_always_on_top(pinned)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_window_mode(window: tauri::WebviewWindow, mode: String) -> Result<(), String> {
    apply_window_mode(&window, &mode)
}

pub fn apply_window_mode(window: &tauri::WebviewWindow, mode: &str) -> Result<(), String> {
    match mode {
        "widget" => {
            window
                .set_resizable(false)
                .map_err(|e| e.to_string())?;
            window
                .set_min_size(None::<tauri::LogicalSize<f64>>)
                .map_err(|e| e.to_string())?;
            window
                .set_size(tauri::LogicalSize::new(320.0, 220.0))
                .map_err(|e| e.to_string())?;
        }
        "expanded" => {
            let (width, height) = expanded_dimensions(window)?;
            window
                .set_resizable(true)
                .map_err(|e| e.to_string())?;
            window
                .set_min_size(Some(tauri::LogicalSize::new(720.0, 480.0)))
                .map_err(|e| e.to_string())?;
            window
                .set_size(tauri::LogicalSize::new(width, height))
                .map_err(|e| e.to_string())?;
        }
        _ => {}
    }

    center_on_screen(window)?;
    Ok(())
}

pub fn show_window_mode(window: &tauri::WebviewWindow, mode: &str) -> Result<(), String> {
    apply_window_mode(window, mode)?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

fn expanded_dimensions(window: &tauri::WebviewWindow) -> Result<(f64, f64), String> {
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let logical_width = size.width as f64 / scale;
        let logical_height = size.height as f64 / scale;
        let width = (logical_width * 0.88).clamp(720.0, 1100.0);
        let height = ((logical_height - 72.0) * 0.88).clamp(480.0, 820.0);
        Ok((width, height))
    } else {
        Ok((960.0, 640.0))
    }
}

fn center_on_screen(window: &tauri::WebviewWindow) -> Result<(), String> {
    window.center().map_err(|e| e.to_string())?;

    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        return Ok(());
    };

    let outer_pos = window.outer_position().map_err(|e| e.to_string())?;
    let outer_size = window.outer_size().map_err(|e| e.to_string())?;
    let mon_pos = monitor.position();
    let mon_size = monitor.size();

    let mut x = outer_pos.x;
    let mut y = outer_pos.y;
    let mon_right = mon_pos.x + mon_size.width as i32;
    let mon_bottom = mon_pos.y + mon_size.height as i32;
    let win_right = x + outer_size.width as i32;
    let win_bottom = y + outer_size.height as i32;

    if x < mon_pos.x {
        x = mon_pos.x;
    }
    if y < mon_pos.y {
        y = mon_pos.y;
    }
    if win_right > mon_right {
        x = mon_right - outer_size.width as i32;
    }
    if win_bottom > mon_bottom {
        y = mon_bottom - outer_size.height as i32;
    }

    if x != outer_pos.x || y != outer_pos.y {
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
