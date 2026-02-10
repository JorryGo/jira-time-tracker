use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::FromRow)]
pub struct Worklog {
    pub id: i64,
    pub issue_key: String,
    pub started_at: String,
    pub duration_seconds: i64,
    pub description: String,
    pub sync_status: String,
    pub jira_worklog_id: Option<String>,
    pub sync_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct WorklogFilter {
    pub issue_key: Option<String>,
    pub sync_status: Option<String>,
    #[allow(dead_code)]
    pub date_from: Option<String>,
    #[allow(dead_code)]
    pub date_to: Option<String>,
}

#[tauri::command]
pub async fn get_worklogs(
    state: State<'_, AppState>,
    filter: Option<WorklogFilter>,
) -> Result<Vec<Worklog>, String> {
    // Build query dynamically based on filter
    // Since sqlx doesn't support dynamic queries well with compile-time checking,
    // we use query_as with raw SQL
    let worklogs: Vec<Worklog> = if let Some(ref f) = filter {
        match (&f.sync_status, &f.issue_key) {
            (Some(status), Some(key)) if status != "all" => {
                sqlx::query_as::<_, Worklog>(
                    "SELECT * FROM worklogs WHERE sync_status = ?1 AND issue_key = ?2 ORDER BY started_at DESC",
                )
                .bind(status)
                .bind(key)
                .fetch_all(&state.db)
                .await
                .map_err(|e| e.to_string())?
            }
            (Some(status), None) if status != "all" => {
                sqlx::query_as::<_, Worklog>(
                    "SELECT * FROM worklogs WHERE sync_status = ?1 ORDER BY started_at DESC",
                )
                .bind(status)
                .fetch_all(&state.db)
                .await
                .map_err(|e| e.to_string())?
            }
            (_, Some(key)) => {
                sqlx::query_as::<_, Worklog>(
                    "SELECT * FROM worklogs WHERE issue_key = ?1 ORDER BY started_at DESC",
                )
                .bind(key)
                .fetch_all(&state.db)
                .await
                .map_err(|e| e.to_string())?
            }
            _ => {
                sqlx::query_as::<_, Worklog>(
                    "SELECT * FROM worklogs ORDER BY started_at DESC",
                )
                .fetch_all(&state.db)
                .await
                .map_err(|e| e.to_string())?
            }
        }
    } else {
        sqlx::query_as::<_, Worklog>("SELECT * FROM worklogs ORDER BY started_at DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| e.to_string())?
    };

    Ok(worklogs)
}

#[tauri::command]
pub async fn create_worklog(
    state: State<'_, AppState>,
    issue_key: String,
    started_at: String,
    duration_seconds: i64,
    description: Option<String>,
) -> Result<Worklog, String> {
    let desc = description.unwrap_or_default();

    let result = sqlx::query(
        "INSERT INTO worklogs (issue_key, started_at, duration_seconds, description, sync_status) \
         VALUES (?1, ?2, ?3, ?4, 'pending')",
    )
    .bind(&issue_key)
    .bind(&started_at)
    .bind(duration_seconds)
    .bind(&desc)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let id = result.last_insert_rowid();

    sqlx::query_as::<_, Worklog>("SELECT * FROM worklogs WHERE id = ?1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_worklog(
    state: State<'_, AppState>,
    id: i64,
    duration_seconds: Option<i64>,
    description: Option<String>,
    started_at: Option<String>,
) -> Result<Worklog, String> {
    // Check status
    let (sync_status,): (String,) =
        sqlx::query_as("SELECT sync_status FROM worklogs WHERE id = ?1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Worklog not found")?;

    if sync_status == "synced" {
        return Err("Cannot edit a synced worklog".to_string());
    }

    if let Some(dur) = duration_seconds {
        sqlx::query("UPDATE worklogs SET duration_seconds = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(dur)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(ref desc) = description {
        sqlx::query("UPDATE worklogs SET description = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(desc)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(ref sa) = started_at {
        sqlx::query("UPDATE worklogs SET started_at = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(sa)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Reset error state
    sqlx::query("UPDATE worklogs SET sync_status = 'pending', sync_error = NULL WHERE id = ?1 AND sync_status = 'error'")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, Worklog>("SELECT * FROM worklogs WHERE id = ?1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_worklog(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let (sync_status,): (String,) =
        sqlx::query_as("SELECT sync_status FROM worklogs WHERE id = ?1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Worklog not found")?;

    if sync_status == "synced" {
        return Err("Cannot delete a synced worklog".to_string());
    }

    sqlx::query("DELETE FROM worklogs WHERE id = ?1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
