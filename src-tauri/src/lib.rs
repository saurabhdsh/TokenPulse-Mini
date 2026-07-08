mod aws_config;
mod azure_config;
mod credentials;
mod adapters;
mod commands;
mod db;
mod engine;
mod env;
mod models;
mod sync;

use commands::{AppState, WindowLayoutState};
use db::Database;
use crate::sync::refresh_live_data_inner;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");

            let database = Database::new(app_data_dir).expect("failed to initialize database");
            app.manage(AppState {
                db: Mutex::new(database),
            });
            app.manage(WindowLayoutState::new());

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let payload = match app_handle.try_state::<AppState>() {
                    Some(state) => match state.db.lock() {
                        Ok(db) => match refresh_live_data_inner(&db) {
                            Ok(reports) => crate::models::LiveSyncFinished {
                                ok: true,
                                reports,
                                error: None,
                            },
                            Err(err) => crate::models::LiveSyncFinished {
                                ok: false,
                                reports: Vec::new(),
                                error: Some(err),
                            },
                        },
                        Err(e) => crate::models::LiveSyncFinished {
                            ok: false,
                            reports: Vec::new(),
                            error: Some(e.to_string()),
                        },
                    },
                    None => crate::models::LiveSyncFinished {
                        ok: false,
                        reports: Vec::new(),
                        error: Some("App database not ready".into()),
                    },
                };
                let _ = app_handle.emit("live-sync-finished", payload);
            });

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            let prewarm = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                commands::prewarm_provider_widgets(&prewarm);
            });

            let show_item = MenuItem::with_id(app, "show", "Show Widget", true, None::<&str>)?;
            let hide_item = MenuItem::with_id(app, "hide", "Hide Widget", true, None::<&str>)?;
            let expand_item =
                MenuItem::with_id(app, "expand", "Open Dashboard", true, None::<&str>)?;
            let openai_widget =
                MenuItem::with_id(app, "widget-openai", "OpenAI Widget", true, None::<&str>)?;
            let bedrock_widget = MenuItem::with_id(
                app,
                "widget-bedrock",
                "AWS Bedrock Widget",
                true,
                None::<&str>,
            )?;
            let azure_widget = MenuItem::with_id(
                app,
                "widget-azure",
                "Azure OpenAI Widget",
                true,
                None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit TokenPulse", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &hide_item,
                    &openai_widget,
                    &bedrock_widget,
                    &azure_widget,
                    &expand_item,
                    &quit_item,
                ],
            )?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("TokenPulse Mini")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let expanded = app
                                .try_state::<commands::WindowLayoutState>()
                                .map(|state| *state.main_expanded.lock().unwrap())
                                .unwrap_or(false);
                            let mode = if expanded { "expanded" } else { "widget" };
                            let _ = commands::apply_window_mode(&window, mode);
                            let _ = window.show();
                            let _ = window.set_focus();
                            commands::emit_view_state_changed(app);
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "expand" => {
                        let _ = commands::expand_main_dashboard(app);
                    }
                    "widget-openai" => {
                        let _ = commands::open_provider_widget_inner(app, "OpenAI");
                    }
                    "widget-bedrock" => {
                        let _ = commands::open_provider_widget_inner(app, "AWS Bedrock");
                    }
                    "widget-azure" => {
                        let _ = commands::open_provider_widget_inner(app, "Azure OpenAI");
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_widget_stats,
            commands::get_widget_demo_enabled,
            commands::set_widget_demo_enabled,
            commands::get_dashboard_stats,
            commands::get_providers,
            commands::update_provider_key,
            commands::get_models,
            commands::update_model_pricing,
            commands::get_budget_settings,
            commands::update_budget_settings,
            commands::get_usage_history,
            commands::get_provider_breakdown,
            commands::get_model_breakdown,
            commands::get_alerts,
            commands::mark_alert_read,
            commands::sync_provider_usage,
            commands::refresh_live_data,
            commands::start_refresh_live_data,
            commands::detect_env_keys,
            commands::get_openai_credentials,
            commands::update_openai_credentials_cmd,
            commands::get_aws_credentials,
            commands::update_aws_credentials_cmd,
            commands::get_azure_credentials,
            commands::update_azure_credentials_cmd,
            commands::set_always_on_top,
            commands::set_window_mode,
            commands::open_provider_widget,
            commands::get_main_view_expanded,
            commands::open_main_dashboard,
            commands::prepare_dashboard_expand_cmd,
            commands::collapse_to_widgets,
            commands::list_widget_providers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
