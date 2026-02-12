use crate::jira::client::JiraClient;
use crate::jira::types::{JiraIssue, JiraUser};
use crate::state::AppState;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Convert any date string to Jira format: "2021-01-17T12:34:00.000+0000"
fn format_for_jira(date_str: &str) -> Result<String, String> {
    let dt = DateTime::parse_from_rfc3339(date_str)
        .map_err(|e| format!("Invalid date '{}': {}", date_str, e))?;
    Ok(dt.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string())
}

fn get_client(state: &AppState) -> Result<JiraClient, String> {
    let config = state
        .jira_config
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "Jira not configured. Go to Settings to set up connection.".to_string())?;
    Ok(JiraClient::new(
        &config.base_url,
        &config.email,
        &config.api_token,
    ))
}

#[tauri::command]
pub async fn jira_test_connection(
    base_url: String,
    email: String,
    api_token: String,
) -> Result<JiraUser, String> {
    let client = JiraClient::new(&base_url, &email, &api_token);
    client.get_myself().await
}

#[tauri::command]
pub async fn jira_search_issues(
    state: State<'_, AppState>,
    jql: String,
    max_results: Option<u32>,
) -> Result<Vec<JiraIssue>, String> {
    let client = get_client(&state)?;
    let issues = client
        .search_issues(&jql, max_results.unwrap_or(50))
        .await?;

    // Cache issues locally
    for issue in &issues {
        let _ = sqlx::query(
            "INSERT INTO issues (issue_key, summary, project_key, status, issue_type, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now')) \
             ON CONFLICT(issue_key) DO UPDATE SET \
             summary = ?2, project_key = ?3, status = ?4, issue_type = ?5, updated_at = datetime('now')",
        )
        .bind(&issue.issue_key)
        .bind(&issue.summary)
        .bind(&issue.project_key)
        .bind(&issue.status)
        .bind(&issue.issue_type)
        .execute(&state.db)
        .await;
    }

    Ok(issues)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PushSummary {
    pub total: u32,
    pub success: u32,
    pub failed: u32,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn jira_push_worklog(
    state: State<'_, AppState>,
    worklog_id: i64,
) -> Result<(), String> {
    let client = get_client(&state)?;

    let row: Option<(String, String, i64, String, String)> = sqlx::query_as(
        "SELECT issue_key, started_at, duration_seconds, description, sync_status FROM worklogs WHERE id = ?1",
    )
    .bind(worklog_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let (issue_key, started_at, duration, description, sync_status) =
        row.ok_or("Worklog not found")?;

    if sync_status == "synced" {
        return Err("Worklog already synced".to_string());
    }

    // Jira expects: "2021-01-17T12:34:00.000+0000"
    let started_jira = format_for_jira(&started_at)?;

    match client
        .add_worklog(&issue_key, duration, &started_jira, &description)
        .await
    {
        Ok(resp) => {
            sqlx::query(
                "UPDATE worklogs SET sync_status = 'synced', jira_worklog_id = ?1, sync_error = NULL, updated_at = datetime('now') WHERE id = ?2",
            )
            .bind(&resp.id)
            .bind(worklog_id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => {
            sqlx::query(
                "UPDATE worklogs SET sync_status = 'error', sync_error = ?1, updated_at = datetime('now') WHERE id = ?2",
            )
            .bind(&e)
            .bind(worklog_id)
            .execute(&state.db)
            .await
            .map_err(|e| e.to_string())?;
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn jira_push_all_pending(
    state: State<'_, AppState>,
    date: String,
) -> Result<PushSummary, String> {
    let date_from = format!("{}T00:00:00", date);
    let date_to = format!("{}T23:59:59", date);
    let ids: Vec<(i64,)> =
        sqlx::query_as("SELECT id FROM worklogs WHERE sync_status = 'pending' AND started_at >= ?1 AND started_at <= ?2")
            .bind(&date_from)
            .bind(&date_to)
            .fetch_all(&state.db)
            .await
            .map_err(|e| e.to_string())?;

    let total = ids.len() as u32;
    let mut success = 0u32;
    let mut errors = Vec::new();

    let client = get_client(&state)?;

    for (id,) in ids {
        let row: Option<(String, String, i64, String)> = sqlx::query_as(
            "SELECT issue_key, started_at, duration_seconds, description FROM worklogs WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| e.to_string())?;

        if let Some((issue_key, started_at, duration, description)) = row {
            let started_jira = format_for_jira(&started_at)?;

            match client
                .add_worklog(&issue_key, duration, &started_jira, &description)
                .await
            {
                Ok(resp) => {
                    let _ = sqlx::query(
                        "UPDATE worklogs SET sync_status = 'synced', jira_worklog_id = ?1, sync_error = NULL, updated_at = datetime('now') WHERE id = ?2",
                    )
                    .bind(&resp.id)
                    .bind(id)
                    .execute(&state.db)
                    .await;
                    success += 1;
                }
                Err(e) => {
                    let _ = sqlx::query(
                        "UPDATE worklogs SET sync_status = 'error', sync_error = ?1, updated_at = datetime('now') WHERE id = ?2",
                    )
                    .bind(&e)
                    .bind(id)
                    .execute(&state.db)
                    .await;
                    errors.push(format!("{}: {}", issue_key, e));
                }
            }
        }
    }

    Ok(PushSummary {
        total,
        success,
        failed: total - success,
        errors,
    })
}
