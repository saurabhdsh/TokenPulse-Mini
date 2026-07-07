mod aws_config;
mod credentials;
mod adapters;
mod commands;
mod db;
mod engine;
mod env;
mod models;
mod sync;

use commands::{apply_window_mode, AppState};
use db::Database;
use sync::{apply_env_keys, sync_all_providers};
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
            {
                let _ = apply_env_keys(&database);
                let _ = sync_all_providers(&database);
            }
            app.manage(AppState {
                db: Mutex::new(database),
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
            let quit_item = MenuItem::with_id(app, "quit", "Quit TokenPulse", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &hide_item,
                    &openai_widget,
                    &bedrock_widget,
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
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "expand" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = apply_window_mode(&window, "expanded");
                            let _ = window.emit("navigate", "dashboard");
                        }
                    }
                    "widget-openai" => {
                        let _ = commands::open_provider_widget_inner(app, "OpenAI");
                    }
                    "widget-bedrock" => {
                        let _ = commands::open_provider_widget_inner(app, "AWS Bedrock");
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
            commands::detect_env_keys,
            commands::get_openai_credentials,
            commands::update_openai_credentials_cmd,
            commands::get_aws_credentials,
            commands::update_aws_credentials_cmd,
            commands::set_always_on_top,
            commands::set_window_mode,
            commands::open_provider_widget,
            commands::open_main_dashboard,
            commands::list_widget_providers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
