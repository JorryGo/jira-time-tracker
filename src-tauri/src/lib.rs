mod commands;
mod db;
mod jira;
mod state;

use sqlx::sqlite::SqlitePoolOptions;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
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
            commands::jira::jira_update_worklog,
            commands::jira::jira_delete_worklog,
            commands::jira::jira_import_worklogs,
            commands::timer::timer_start,
            commands::timer::timer_pause,
            commands::timer::timer_resume,
            commands::timer::timer_stop,
            commands::timer::timer_get_state,
            commands::timer::timer_update_tray,
            commands::timer::timer_update_description,
            commands::timer::timer_set_tray_icon,
            commands::worklogs::get_worklogs,
            commands::worklogs::create_worklog,
            commands::worklogs::update_worklog,
            commands::worklogs::delete_worklog,
            commands::quit_app,
            commands::set_dock_visible,
            commands::focus_calendar,
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
                .unwrap_or_else(|e| {
                    eprintln!("Failed to read window position: {}", e);
                    None
                })
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
                cached_account_id: Mutex::new(None),
                http_client: reqwest::Client::new(),
                suppress_blur_hide: AtomicBool::new(false),
            });

            // Build tray context menu (required on Linux for icon visibility)
            let show_hide = MenuItem::with_id(app, "show-hide", "Show/Hide", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_hide, &separator, &quit])?;

            // Build tray icon
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray-idle.png")).unwrap())
                .icon_as_template(cfg!(target_os = "macos"))
                .tooltip("Jira Time Tracker")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show-hide" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let is_showing = window.is_visible().unwrap_or(false)
                                    && !window.is_minimized().unwrap_or(false);
                                if is_showing {
                                    let _ = window.hide();
                                } else {
                                    app.state::<state::AppState>()
                                        .suppress_blur_hide
                                        .store(true, Ordering::SeqCst);

                                    // Restore saved position (skip on Linux — Wayland
                                    // does not allow clients to position windows)
                                    #[cfg(not(target_os = "linux"))]
                                    {
                                        let saved = *app
                                            .state::<state::AppState>()
                                            .window_position
                                            .lock()
                                            .unwrap_or_else(|e| e.into_inner());
                                        if let Some((x, y)) = saved {
                                            let _ = window.set_position(tauri::Position::Physical(
                                                tauri::PhysicalPosition::new(x, y),
                                            ));
                                        }
                                    }

                                    let _ = window.show();
                                    let _ = window.set_focus();

                                    #[cfg(target_os = "macos")]
                                    {
                                        use objc2::MainThreadMarker;
                                        use objc2_app_kit::NSApplication;
                                        if let Some(mtm) = MainThreadMarker::new() {
                                            let ns_app = NSApplication::sharedApplication(mtm);
                                            #[allow(deprecated)]
                                            ns_app.activateIgnoringOtherApps(true);
                                        }
                                    }

                                    #[cfg(target_os = "windows")]
                                    {
                                        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
                                        use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
                                        if let Ok(hwnd) = window.hwnd() {
                                            unsafe {
                                                let _ = SetForegroundWindow(hwnd);
                                                let _ = SetFocus(Some(hwnd));
                                            }
                                        }
                                    }

                                    let app_clone = app.clone();
                                    tauri::async_runtime::spawn(async move {
                                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                        app_clone
                                            .state::<state::AppState>()
                                            .suppress_blur_hide
                                            .store(false, Ordering::SeqCst);
                                    });
                                }
                            }
                        }
                        "quit" => {
                            if let Some(app_state) = app.try_state::<state::AppState>() {
                                let saved = *app_state.window_position.lock().unwrap_or_else(|e| e.into_inner());
                                if let Some((x, y)) = saved {
                                    let db = app_state.db.clone();
                                    let val = format!("{},{}", x, y);
                                    let app_clone = app.clone();
                                    tauri::async_runtime::spawn(async move {
                                        let _ = sqlx::query(
                                            "INSERT INTO settings (key, value) VALUES ('window_position', ?1) \
                                             ON CONFLICT(key) DO UPDATE SET value = ?1",
                                        )
                                        .bind(val)
                                        .execute(&db)
                                        .await;
                                        app_clone.exit(0);
                                    });
                                    return;
                                }
                            }
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
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
                            let is_showing = window.is_visible().unwrap_or(false)
                                && !window.is_minimized().unwrap_or(false);
                            if is_showing {
                                let _ = window.hide();
                            } else {
                                // Suppress blur-hide to prevent race condition:
                                // tray click can briefly steal focus, triggering blur → hide
                                app.state::<state::AppState>()
                                    .suppress_blur_hide
                                    .store(true, Ordering::SeqCst);

                                // Use saved position if the user has repositioned the window
                                let saved = *app
                                    .state::<state::AppState>()
                                    .window_position
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner());
                                if let Some((x, y)) = saved {
                                    // Skip on Linux — Wayland does not allow client-side positioning
                                    #[cfg(not(target_os = "linux"))]
                                    {
                                        let _ = window.set_position(tauri::Position::Physical(
                                            tauri::PhysicalPosition::new(x, y),
                                        ));
                                    }
                                } else {
                                    // First launch: position relative to tray icon
                                    #[cfg(target_os = "macos")]
                                    {
                                        let _ = window.move_window(Position::TrayBottomCenter);
                                    }
                                    #[cfg(target_os = "windows")]
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

                                // On macOS, explicitly activate the app so the window
                                // appears in front (Accessory apps don't auto-activate).
                                #[cfg(target_os = "macos")]
                                {
                                    use objc2::MainThreadMarker;
                                    use objc2_app_kit::NSApplication;
                                    // Tray event handler always runs on the main thread
                                    if let Some(mtm) = MainThreadMarker::new() {
                                        let ns_app = NSApplication::sharedApplication(mtm);
                                        #[allow(deprecated)]
                                        ns_app.activateIgnoringOtherApps(true);
                                    }
                                }

                                // On Windows, explicitly call SetForegroundWindow + SetFocus
                                // to ensure proper keyboard layout switching when showing from tray.
                                // Tauri's set_focus() alone doesn't trigger full input locale activation.
                                #[cfg(target_os = "windows")]
                                {
                                    use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
                                    use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
                                    if let Ok(hwnd) = window.hwnd() {
                                        unsafe {
                                            let _ = SetForegroundWindow(hwnd);
                                            let _ = SetFocus(Some(hwnd));
                                        }
                                    }
                                }

                                // Clear suppress flag after the show/focus sequence settles
                                let app_clone = app.clone();
                                tauri::async_runtime::spawn(async move {
                                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                    app_clone
                                        .state::<state::AppState>()
                                        .suppress_blur_hide
                                        .store(false, Ordering::SeqCst);
                                });
                            }
                        }
                    }
                })
                .build(app)?;

            // Show window on startup so the user sees the app immediately
            if let Some(window) = app.get_webview_window("main") {
                // On Linux/Wayland, skip set_position (clients cannot position windows)
                // and disable alwaysOnTop which can cause Wayland protocol errors
                #[cfg(target_os = "linux")]
                {
                    let _ = window.set_always_on_top(false);
                }
                #[cfg(not(target_os = "linux"))]
                {
                    let saved = *app
                        .state::<state::AppState>()
                        .window_position
                        .lock()
                        .unwrap_or_else(|e| e.into_inner());
                    if let Some((x, y)) = saved {
                        let _ = window.set_position(tauri::Position::Physical(
                            tauri::PhysicalPosition::new(x, y),
                        ));
                    }
                }
                let _ = window.show();
                let _ = window.set_focus();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                WindowEvent::Moved(pos) => {
                    if window.label() == "main" {
                        if let Some(app_state) = window.try_state::<state::AppState>() {
                            *app_state.window_position.lock().unwrap_or_else(|e| e.into_inner()) =
                                Some((pos.x, pos.y));
                        }
                    }
                }
                WindowEvent::Focused(false) => {
                    if window.label() == "main" {
                        // Persist position to SQLite (skip on Linux — Wayland doesn't
                        // support client-side positioning so saved position is unused)
                        #[cfg(not(target_os = "linux"))]
                        if let Some(app_state) = window.try_state::<state::AppState>() {
                            let saved = *app_state.window_position.lock().unwrap_or_else(|e| e.into_inner());
                            if let Some((x, y)) = saved {
                                let db = app_state.db.clone();
                                let val = format!("{},{}", x, y);
                                tauri::async_runtime::spawn(async move {
                                    if let Err(e) = sqlx::query(
                                        "INSERT INTO settings (key, value) VALUES ('window_position', ?1) \
                                         ON CONFLICT(key) DO UPDATE SET value = ?1",
                                    )
                                    .bind(val)
                                    .execute(&db)
                                    .await
                                    {
                                        eprintln!("Failed to save window position: {}", e);
                                    }
                                });
                            }
                        }
                        // On Linux, skip blur-hide entirely — native widgets (<select>,
                        // date pickers) cause focus loss that breaks the UX. Users hide
                        // the window via the tray menu or the in-app hide button instead.
                        #[cfg(not(target_os = "linux"))]
                        {
                        // Skip hide if tray click is in progress (prevents race condition
                        // where tray show + immediate blur-hide cancel each other out)
                        let suppressed = window
                            .app_handle()
                            .try_state::<state::AppState>()
                            .map(|s| s.suppress_blur_hide.load(Ordering::SeqCst))
                            .unwrap_or(false);
                        if !suppressed {
                            // Debounce hide — on Windows startDragging() briefly loses focus
                            let window_clone = window.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                                if !window_clone.is_focused().unwrap_or(true) {
                                    let _ = window_clone.hide();
                                }
                            });
                        }
                        } // #[cfg(not(target_os = "linux"))]
                    }
                }
                WindowEvent::Destroyed => {
                    #[cfg(target_os = "macos")]
                    if window.label() == "calendar" {
                        // Suppress blur-hide during policy transition: switching to
                        // Accessory deactivates the app, which fires blur on main window
                        if let Some(app_state) = window.app_handle().try_state::<state::AppState>() {
                            app_state.suppress_blur_hide.store(true, Ordering::SeqCst);
                        }
                        let _ = window
                            .app_handle()
                            .set_activation_policy(tauri::ActivationPolicy::Accessory);
                        let app_clone = window.app_handle().clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            if let Some(app_state) = app_clone.try_state::<state::AppState>() {
                                app_state.suppress_blur_hide.store(false, Ordering::SeqCst);
                            }
                        });
                    }
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // Handle macOS dock icon click: unminimize the calendar window if it exists
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                if let Some(calendar) = _app.get_webview_window("calendar") {
                    let _ = calendar.unminimize();
                    let _ = calendar.show();
                    let _ = calendar.set_focus();
                }
            }
        });
}
