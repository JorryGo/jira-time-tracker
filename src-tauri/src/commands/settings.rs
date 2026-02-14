use crate::state::{AppState, JiraConfig};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn settings_get(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM settings WHERE key = ?1")
            .bind(&key)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    Ok(row.map(|r| r.0))
}

#[tauri::command]
pub async fn settings_set(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
    )
    .bind(&key)
    .bind(&value)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn settings_get_all(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM settings")
            .fetch_all(&state.db)
            .await
            .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().collect())
}

#[tauri::command]
pub async fn settings_save_jira_config(
    state: State<'_, AppState>,
    base_url: String,
    email: String,
    api_token: String,
) -> Result<(), String> {
    let pairs = [
        ("jira_base_url", &base_url),
        ("jira_email", &email),
        ("jira_api_token", &api_token),
    ];

    for (key, value) in pairs {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        )
        .bind(key)
        .bind(value)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;
    }

    *state.jira_config.lock().unwrap_or_else(|e| e.into_inner()) = Some(JiraConfig {
        base_url,
        email,
        api_token,
    });

    Ok(())
}

#[tauri::command]
pub async fn settings_load_jira_config(
    state: State<'_, AppState>,
) -> Result<Option<HashMap<String, String>>, String> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT key, value FROM settings WHERE key IN ('jira_base_url', 'jira_email', 'jira_api_token')",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let mut map: HashMap<String, String> = rows.into_iter().collect();

    if map.contains_key("jira_base_url")
        && map.contains_key("jira_email")
        && map.contains_key("jira_api_token")
    {
        let config = JiraConfig {
            base_url: map["jira_base_url"].clone(),
            email: map["jira_email"].clone(),
            api_token: map["jira_api_token"].clone(),
        };
        *state.jira_config.lock().unwrap_or_else(|e| e.into_inner()) = Some(config);
        map.insert("jira_api_token".to_string(), "***".to_string());
        Ok(Some(map))
    } else {
        Ok(None)
    }
}
