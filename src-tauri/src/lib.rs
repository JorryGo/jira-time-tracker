mod commands;
mod db;
mod jira;
mod state;

use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Mutex;
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .invoke_handler(tauri::generate_handler![
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::settings::settings_get_all,
            commands::settings::settings_save_jira_config,
            commands::settings::settings_load_jira_config,
            commands::jira::jira_test_connection,
            commands::jira::jira_search_issues,
            commands::jira::jira_push_worklog,
            commands::jira::jira_push_all_pending,
            commands::timer::timer_start,
            commands::timer::timer_pause,
            commands::timer::timer_resume,
            commands::timer::timer_stop,
            commands::timer::timer_get_state,
            commands::timer::timer_update_tray,
            commands::timer::timer_set_tray_icon,
            commands::worklogs::get_worklogs,
            commands::worklogs::create_worklog,
            commands::worklogs::update_worklog,
            commands::worklogs::delete_worklog,
            commands::quit_app,
        ])
        .setup(|app| {
            // Hide dock icon on macOS — pure menubar app
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Initialize SQLite database
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("Failed to create app data dir");
            let db_path = app_dir.join("jira-tracker.db");
            let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

            let pool = tauri::async_runtime::block_on(async {
                let pool = SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect(&db_url)
                    .await
                    .expect("Failed to create database pool");

                // Run migrations
                db::migrations::run_migrations(&pool)
                    .await
                    .expect("Failed to run migrations");

                pool
            });

            app.manage(state::AppState {
                db: pool,
                jira_config: Mutex::new(None),
            });

            // Build tray icon
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-idle.png")).unwrap())
                .icon_as_template(true)
                .tooltip("Jira Time Tracker")
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect: _rect,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                // On macOS tray is always at the top — use plugin positioning
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = window.move_window(Position::TrayBottomCenter);
                                }
                                // On Windows/Linux tray can be top or bottom — check available space
                                #[cfg(not(target_os = "macos"))]
                                {
                                    if let Ok(win_size) = window.outer_size() {
                                        let win_w = win_size.width as f64;
                                        let win_h = win_size.height as f64;
                                        let x = (_rect.position.x + _rect.size.width / 2.0
                                            - win_w / 2.0)
                                            as i32;
                                        let tray_bottom = _rect.position.y + _rect.size.height;
                                        let y = if let Ok(Some(monitor)) = window.current_monitor()
                                        {
                                            let screen_h = monitor.size().height as f64;
                                            if tray_bottom + win_h <= screen_h {
                                                tray_bottom as i32
                                            } else {
                                                (_rect.position.y - win_h) as i32
                                            }
                                        } else {
                                            (_rect.position.y - win_h) as i32
                                        };
                                        let _ = window.set_position(tauri::Position::Physical(
                                            tauri::PhysicalPosition::new(x, y),
                                        ));
                                    }
                                }
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                if window.label() == "main" {
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
