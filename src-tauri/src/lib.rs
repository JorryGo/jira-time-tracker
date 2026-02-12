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
#[cfg(target_os = "macos")]
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

            let (pool, saved_window_pos) = tauri::async_runtime::block_on(async {
                let pool = SqlitePoolOptions::new()
                    .max_connections(5)
                    .connect(&db_url)
                    .await
                    .expect("Failed to create database pool");

                db::migrations::run_migrations(&pool)
                    .await
                    .expect("Failed to run migrations");

                let saved_pos = sqlx::query_scalar::<_, String>(
                    "SELECT value FROM settings WHERE key = 'window_position'",
                )
                .fetch_optional(&pool)
                .await
                .ok()
                .flatten()
                .and_then(|s| {
                    let mut parts = s.split(',');
                    let x = parts.next()?.parse::<i32>().ok()?;
                    let y = parts.next()?.parse::<i32>().ok()?;
                    Some((x, y))
                });

                (pool, saved_pos)
            });

            app.manage(state::AppState {
                db: pool,
                jira_config: Mutex::new(None),
                window_position: Mutex::new(saved_window_pos),
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
                        position: _click_pos,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                // Use saved position if the user has repositioned the window
                                let saved = *app
                                    .state::<state::AppState>()
                                    .window_position
                                    .lock()
                                    .unwrap();
                                if let Some((x, y)) = saved {
                                    let _ = window.set_position(tauri::Position::Physical(
                                        tauri::PhysicalPosition::new(x, y),
                                    ));
                                } else {
                                    // First launch: position relative to tray icon
                                    #[cfg(target_os = "macos")]
                                    {
                                        let _ = window.move_window(Position::TrayBottomCenter);
                                    }
                                    #[cfg(not(target_os = "macos"))]
                                    {
                                        if let Ok(win_size) = window.outer_size() {
                                            let win_w = win_size.width as f64;
                                            let win_h = win_size.height as f64;
                                            let x = (_click_pos.x - win_w / 2.0) as i32;
                                            // Show above click if tray is at bottom, below if at top
                                            let y = if _click_pos.y > win_h {
                                                (_click_pos.y - win_h) as i32
                                            } else {
                                                _click_pos.y as i32
                                            };
                                            let _ = window.set_position(
                                                tauri::Position::Physical(
                                                    tauri::PhysicalPosition::new(x, y),
                                                ),
                                            );
                                        }
                                    }
                                }
                                let _ = window.show();
                                let _ = window.set_focus();

                                // On Windows, explicitly call SetForegroundWindow + SetFocus
                                // to ensure proper keyboard layout switching when showing from tray.
                                // Tauri's set_focus() alone doesn't trigger full input locale activation.
                                #[cfg(target_os = "windows")]
                                {
                                    use windows::Win32::UI::WindowsAndMessaging::{
                                        SetForegroundWindow, SetFocus,
                                    };
                                    if let Ok(hwnd) = window.hwnd() {
                                        unsafe {
                                            let _ = SetForegroundWindow(hwnd);
                                            let _ = SetFocus(Some(hwnd));
                                        }
                                    }
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                WindowEvent::Moved(pos) => {
                    if window.label() == "main" {
                        if let Some(app_state) = window.try_state::<state::AppState>() {
                            *app_state.window_position.lock().unwrap() =
                                Some((pos.x, pos.y));
                        }
                    }
                }
                WindowEvent::Focused(false) => {
                    if window.label() == "main" {
                        // Persist position to SQLite
                        if let Some(app_state) = window.try_state::<state::AppState>() {
                            let saved = *app_state.window_position.lock().unwrap();
                            if let Some((x, y)) = saved {
                                let db = app_state.db.clone();
                                let val = format!("{},{}", x, y);
                                tauri::async_runtime::spawn(async move {
                                    let _ = sqlx::query(
                                        "INSERT INTO settings (key, value) VALUES ('window_position', ?1) \
                                         ON CONFLICT(key) DO UPDATE SET value = ?1",
                                    )
                                    .bind(val)
                                    .execute(&db)
                                    .await;
                                });
                            }
                        }
                        // Debounce hide — on Windows startDragging() briefly loses focus
                        let window_clone = window.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            if !window_clone.is_focused().unwrap_or(true) {
                                let _ = window_clone.hide();
                            }
                        });
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
