pub mod jira;
pub mod settings;
pub mod timer;
pub mod worklogs;

use tauri::Manager;

#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle) {
    // Save window position before exit
    if let Some(app_state) = app.try_state::<crate::state::AppState>() {
        let saved = *app_state.window_position.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((x, y)) = saved {
            let val = format!("{},{}", x, y);
            if let Err(e) = sqlx::query(
                "INSERT INTO settings (key, value) VALUES ('window_position', ?1) \
                 ON CONFLICT(key) DO UPDATE SET value = ?1",
            )
            .bind(val)
            .execute(&app_state.db)
            .await
            {
                eprintln!("Failed to save window position on quit: {}", e);
            }
        }
    }
    app.exit(0);
}

#[tauri::command]
pub fn set_dock_visible(_app: tauri::AppHandle, visible: bool) {
    #[cfg(target_os = "macos")]
    {
        if visible {
            let _ = _app.set_activation_policy(tauri::ActivationPolicy::Regular);
        } else {
            let _ = _app.set_activation_policy(tauri::ActivationPolicy::Accessory);
        }
    }
    let _ = visible;
}

#[tauri::command]
pub fn focus_calendar(app: tauri::AppHandle) -> bool {
    if let Some(window) = app.get_webview_window("calendar") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        true
    } else {
        false
    }
}
