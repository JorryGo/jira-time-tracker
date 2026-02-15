use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct TimerState {
    pub issue_key: String,
    pub started_at: String,
    pub accumulated_secs: i64,
    pub is_paused: bool,
    pub paused_at: Option<String>,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoppedWorklog {
    pub id: i64,
    pub issue_key: String,
    pub duration_seconds: i64,
}

#[tauri::command]
pub async fn timer_get_state(
    state: State<'_, AppState>,
) -> Result<Option<TimerState>, String> {
    let row: Option<(String, String, i64, bool, Option<String>, String)> = sqlx::query_as(
        "SELECT issue_key, started_at, accumulated_secs, is_paused, paused_at, description FROM active_timer WHERE id = 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(row.map(|(issue_key, started_at, accumulated_secs, is_paused, paused_at, description)| TimerState {
        issue_key,
        started_at,
        accumulated_secs,
        is_paused,
        paused_at,
        description,
    }))
}

#[tauri::command]
pub async fn timer_start(
    state: State<'_, AppState>,
    issue_key: String,
) -> Result<TimerState, String> {
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;

    // If timer is running, stop it first (inlined to use transaction)
    let existing: Option<(String, String, i64, bool, Option<String>, String)> = sqlx::query_as(
        "SELECT issue_key, started_at, accumulated_secs, is_paused, paused_at, description FROM active_timer WHERE id = 1",
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    if let Some((old_key, started_at, accumulated, is_paused, _, old_description)) = existing {
        let total_secs = if is_paused {
            accumulated
        } else {
            let now = Utc::now();
            let started = chrono::DateTime::parse_from_rfc3339(&started_at)
                .map_err(|e| format!("Invalid started_at: {}", e))?;
            let elapsed = (now - started.with_timezone(&Utc)).num_seconds();
            accumulated + elapsed
        };
        let total_secs = total_secs.max(0);
        // Round up to nearest minute (minimum 60s)
        let total_secs = if total_secs > 0 {
            ((total_secs + 59) / 60) * 60
        } else {
            total_secs
        };

        let stop_time = Utc::now();
        let worklog_started = stop_time - chrono::Duration::seconds(total_secs);
        let started_str = worklog_started.to_rfc3339();

        sqlx::query(
            "INSERT INTO worklogs (issue_key, started_at, duration_seconds, description, sync_status) \
             VALUES (?1, ?2, ?3, ?4, 'pending')",
        )
        .bind(&old_key)
        .bind(&started_str)
        .bind(total_secs)
        .bind(&old_description)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query("DELETE FROM active_timer WHERE id = 1")
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT OR REPLACE INTO active_timer (id, issue_key, started_at, accumulated_secs, is_paused, paused_at, description) \
         VALUES (1, ?1, ?2, 0, 0, NULL, '')",
    )
    .bind(&issue_key)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(TimerState {
        issue_key,
        started_at: now,
        accumulated_secs: 0,
        is_paused: false,
        paused_at: None,
        description: String::new(),
    })
}

#[tauri::command]
pub async fn timer_pause(
    state: State<'_, AppState>,
) -> Result<TimerState, String> {
    let row: (String, String, i64, bool, Option<String>, String) = sqlx::query_as(
        "SELECT issue_key, started_at, accumulated_secs, is_paused, paused_at, description FROM active_timer WHERE id = 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("No active timer")?;

    let (issue_key, started_at, accumulated, is_paused, _, description) = row;
    if is_paused {
        return Err("Timer is already paused".to_string());
    }

    let now = Utc::now();
    let started = chrono::DateTime::parse_from_rfc3339(&started_at)
        .map_err(|e| format!("Invalid started_at: {}", e))?;
    let elapsed = (now - started.with_timezone(&Utc)).num_seconds();
    let new_accumulated = accumulated + elapsed;
    let now_str = now.to_rfc3339();

    sqlx::query(
        "UPDATE active_timer SET accumulated_secs = ?1, is_paused = 1, paused_at = ?2 WHERE id = 1",
    )
    .bind(new_accumulated)
    .bind(&now_str)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(TimerState {
        issue_key,
        started_at,
        accumulated_secs: new_accumulated,
        is_paused: true,
        paused_at: Some(now_str),
        description,
    })
}

#[tauri::command]
pub async fn timer_resume(
    state: State<'_, AppState>,
) -> Result<TimerState, String> {
    let row: (String, String, i64, bool, Option<String>, String) = sqlx::query_as(
        "SELECT issue_key, started_at, accumulated_secs, is_paused, paused_at, description FROM active_timer WHERE id = 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("No active timer")?;

    let (issue_key, _, accumulated, is_paused, _, description) = row;
    if !is_paused {
        return Err("Timer is not paused".to_string());
    }

    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "UPDATE active_timer SET started_at = ?1, is_paused = 0, paused_at = NULL WHERE id = 1",
    )
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(TimerState {
        issue_key,
        started_at: now,
        accumulated_secs: accumulated,
        is_paused: false,
        paused_at: None,
        description,
    })
}

async fn stop_timer_internal(db: &SqlitePool) -> Result<StoppedWorklog, String> {
    let row: (String, String, i64, bool, Option<String>, String) = sqlx::query_as(
        "SELECT issue_key, started_at, accumulated_secs, is_paused, paused_at, description FROM active_timer WHERE id = 1",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| e.to_string())?
    .ok_or("No active timer")?;

    let (issue_key, started_at, accumulated, is_paused, _, description) = row;

    let total_secs = if is_paused {
        accumulated
    } else {
        let now = Utc::now();
        let started = chrono::DateTime::parse_from_rfc3339(&started_at)
            .map_err(|e| format!("Invalid started_at: {}", e))?;
        let elapsed = (now - started.with_timezone(&Utc)).num_seconds();
        accumulated + elapsed
    };
    let total_secs = total_secs.max(0);
    // Round up to nearest minute (minimum 60s)
    let total_secs = if total_secs > 0 {
        ((total_secs + 59) / 60) * 60
    } else {
        total_secs
    };

    let stop_time = Utc::now();
    let worklog_started = stop_time - chrono::Duration::seconds(total_secs);
    let started_str = worklog_started.to_rfc3339();

    // Create worklog
    let result = sqlx::query(
        "INSERT INTO worklogs (issue_key, started_at, duration_seconds, description, sync_status) \
         VALUES (?1, ?2, ?3, ?4, 'pending')",
    )
    .bind(&issue_key)
    .bind(&started_str)
    .bind(total_secs)
    .bind(&description)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();

    // Delete active timer
    sqlx::query("DELETE FROM active_timer WHERE id = 1")
        .execute(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(StoppedWorklog {
        id,
        issue_key,
        duration_seconds: total_secs,
    })
}

#[tauri::command]
pub async fn timer_stop(
    state: State<'_, AppState>,
) -> Result<StoppedWorklog, String> {
    stop_timer_internal(&state.db).await
}

#[tauri::command]
pub async fn timer_update_tray(
    app_handle: tauri::AppHandle,
    display_text: String,
) -> Result<(), String> {
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        tray.set_title(Some(&display_text)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn timer_update_description(
    state: State<'_, AppState>,
    description: String,
) -> Result<(), String> {
    sqlx::query("UPDATE active_timer SET description = ?1 WHERE id = 1")
        .bind(&description)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn timer_set_tray_icon(
    app_handle: tauri::AppHandle,
    icon_name: String,
) -> Result<(), String> {
    let icon_bytes: &[u8] = match icon_name.as_str() {
        "working" => include_bytes!("../../icons/tray-working.png"),
        "paused" => include_bytes!("../../icons/tray-paused.png"),
        _ => include_bytes!("../../icons/tray-idle.png"),
    };
    let icon = tauri::image::Image::from_bytes(icon_bytes).map_err(|e| e.to_string())?;
    if let Some(tray) = app_handle.tray_by_id("main-tray") {
        tray.set_icon(Some(icon)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
