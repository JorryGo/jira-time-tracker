use crate::jira::client::{extract_adf_text, JiraClient};
use crate::jira::types::{JiraIssue, JiraUser};
use crate::state::AppState;
use chrono::{DateTime, Local, NaiveDate};
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
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or_else(|| "Jira not configured. Go to Settings to set up connection.".to_string())?;
    Ok(JiraClient::new(
        state.http_client.clone(),
        &config.base_url,
        &config.email,
        &config.api_token,
    ))
}

#[tauri::command]
pub async fn jira_test_connection(
    state: State<'_, AppState>,
    base_url: String,
    email: String,
    api_token: String,
) -> Result<JiraUser, String> {
    let client = JiraClient::new(state.http_client.clone(), &base_url, &email, &api_token);
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
        if let Err(e) = sqlx::query(
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
        .await
        {
            eprintln!("Failed to cache issue {}: {}", issue.issue_key, e);
        }
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
    let push_date_from = format!("{}T00:00:00", date);
    let push_date_to = format!("{}T23:59:59", date);
    let rows: Vec<(i64, String, String, i64, String)> = sqlx::query_as(
        "SELECT id, issue_key, started_at, duration_seconds, description \
         FROM worklogs WHERE sync_status = 'pending' AND started_at >= ?1 AND started_at <= ?2",
    )
    .bind(&push_date_from)
    .bind(&push_date_to)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let total = rows.len() as u32;
    let mut success = 0u32;
    let mut errors = Vec::new();

    let client = get_client(&state)?;

    for (i, (id, issue_key, started_at, duration, description)) in rows.into_iter().enumerate() {
        // Rate limit: pause between requests to avoid Jira API throttling
        if i > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        let started_jira = format_for_jira(&started_at)?;

        match client
            .add_worklog(&issue_key, duration, &started_jira, &description)
            .await
        {
            Ok(resp) => {
                if let Err(db_err) = sqlx::query(
                    "UPDATE worklogs SET sync_status = 'synced', jira_worklog_id = ?1, sync_error = NULL, updated_at = datetime('now') WHERE id = ?2",
                )
                .bind(&resp.id)
                .bind(id)
                .execute(&state.db)
                .await
                {
                    eprintln!("Failed to update worklog {} after push: {}", id, db_err);
                }
                success += 1;
            }
            Err(e) => {
                if let Err(db_err) = sqlx::query(
                    "UPDATE worklogs SET sync_status = 'error', sync_error = ?1, updated_at = datetime('now') WHERE id = ?2",
                )
                .bind(&e)
                .bind(id)
                .execute(&state.db)
                .await
                {
                    eprintln!("Failed to record sync error for worklog {}: {}", id, db_err);
                }
                errors.push(format!("{}: {}", issue_key, e));
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

#[tauri::command]
pub async fn jira_update_worklog(
    state: State<'_, AppState>,
    worklog_id: i64,
    duration_seconds: Option<i64>,
    description: Option<String>,
    started_at: Option<String>,
) -> Result<(), String> {
    let client = get_client(&state)?;

    let row: Option<(String, Option<String>, String, i64, String, String)> = sqlx::query_as(
        "SELECT issue_key, jira_worklog_id, sync_status, duration_seconds, description, started_at FROM worklogs WHERE id = ?1",
    )
    .bind(worklog_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let (issue_key, jira_wl_id, sync_status, cur_duration, cur_description, cur_started_at) =
        row.ok_or("Worklog not found")?;

    if sync_status != "synced" {
        return Err("Worklog is not synced to Jira".to_string());
    }

    let jira_id = jira_wl_id.ok_or("Worklog has no Jira worklog ID")?;

    let final_duration = duration_seconds.unwrap_or(cur_duration);
    let final_description = description.unwrap_or(cur_description);
    let final_started_at = started_at.unwrap_or(cur_started_at);

    let started_jira = format_for_jira(&final_started_at)?;

    // Push to Jira first — if it fails, nothing changes locally
    let resp = client
        .update_worklog(&issue_key, &jira_id, final_duration, &started_jira, &final_description)
        .await?;

    // Update locally on success
    sqlx::query(
        "UPDATE worklogs SET duration_seconds = ?1, description = ?2, started_at = ?3, jira_worklog_id = ?4, jira_updated_at = datetime('now'), updated_at = datetime('now') WHERE id = ?5",
    )
    .bind(final_duration)
    .bind(&final_description)
    .bind(&final_started_at)
    .bind(&resp.id)
    .bind(worklog_id)
    .execute(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn jira_delete_worklog(
    state: State<'_, AppState>,
    worklog_id: i64,
) -> Result<(), String> {
    let row: Option<(String, Option<String>, String)> = sqlx::query_as(
        "SELECT issue_key, jira_worklog_id, sync_status FROM worklogs WHERE id = ?1",
    )
    .bind(worklog_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let (issue_key, jira_worklog_id, sync_status) = row.ok_or("Worklog not found")?;

    if sync_status != "synced" {
        return Err("Worklog is not synced to Jira".to_string());
    }

    let jira_id = jira_worklog_id.ok_or("Worklog has no Jira worklog ID")?;

    let client = get_client(&state)?;
    client.delete_worklog(&issue_key, &jira_id).await?;

    sqlx::query("DELETE FROM worklogs WHERE id = ?1")
        .bind(worklog_id)
        .execute(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportSummary {
    pub imported: u32,
    pub updated: u32,
    pub deleted: u32,
    pub skipped: u32,
    pub issues_checked: u32,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn jira_import_worklogs(
    state: State<'_, AppState>,
    date: String,
) -> Result<ImportSummary, String> {
    let client = get_client(&state)?;

    let my_account_id = {
        let cached = state
            .cached_account_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(id) = cached {
            id
        } else {
            let user = client.get_myself().await?;
            let id = user.account_id;
            *state
                .cached_account_id
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(id.clone());
            id
        }
    };

    let target_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| format!("Invalid date '{}': {}", date, e))?;

    // Search ±1 day to cover timezone differences between Jira account TZ and local TZ.
    // JQL worklogDate uses the Jira account timezone, which may differ from the user's
    // system timezone. Precise local-date filtering happens later in Rust.
    let day_before = target_date - chrono::Duration::days(1);
    let day_after = target_date + chrono::Duration::days(1);
    let jql = format!(
        "worklogDate >= \"{}\" AND worklogDate <= \"{}\" AND worklogAuthor = currentUser()",
        day_before, day_after
    );
    let mut warnings: Vec<String> = Vec::new();

    let jql_result = client.search_issues(&jql, 50).await;
    let jql_succeeded = jql_result.is_ok();
    if let Err(ref e) = jql_result {
        warnings.push(format!("JQL search failed: {}", e));
    }
    let issues = jql_result.unwrap_or_default();
    let issue_keys: Vec<String> = issues.into_iter().map(|i| i.issue_key).collect();

    // Only fetch worklogs started after (target_date - 2 days) to reduce payload
    let started_after_epoch = day_before
        .and_hms_opt(0, 0, 0)
        .map(|dt| dt.and_utc().timestamp_millis());

    // Load existing synced worklogs for the target date range (not ALL worklogs)
    let date_from = format!("{}T00:00:00", day_before);
    let date_to = format!("{}T23:59:59", day_after);
    let existing_rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT jira_worklog_id, jira_updated_at FROM worklogs \
         WHERE jira_worklog_id IS NOT NULL AND started_at >= ?1 AND started_at <= ?2",
    )
    .bind(&date_from)
    .bind(&date_to)
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let existing_map: std::collections::HashMap<String, Option<String>> =
        existing_rows.into_iter().collect();

    let mut imported = 0u32;
    let mut updated = 0u32;
    let mut skipped = 0u32;
    let issues_checked = issue_keys.len() as u32;
    let mut seen_jira_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut failed_issues: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Fetch worklogs for all issues in parallel (max 5 concurrent)
    use futures::stream::{self, StreamExt};

    let worklog_results: Vec<_> = stream::iter(issue_keys)
        .map(|key| {
            let client = client.clone();
            async move {
                let result = client.get_worklogs(&key, started_after_epoch).await;
                (key, result)
            }
        })
        .buffer_unordered(5)
        .collect()
        .await;

    for (issue_key, result) in &worklog_results {
        let response = match result {
            Ok(r) => r,
            Err(e) => {
                warnings.push(format!("Failed to fetch worklogs for {}: {}", issue_key, e));
                failed_issues.insert(issue_key.clone());
                continue;
            }
        };

        for entry in &response.worklogs {
            if entry.author.account_id != my_account_id {
                continue;
            }

            // Parse Jira timestamp and compare date in local timezone
            let parsed = match DateTime::parse_from_str(
                &entry.started,
                "%Y-%m-%dT%H:%M:%S%.3f%z",
            ) {
                Ok(dt) => dt,
                Err(_) => continue,
            };
            let local_date = parsed.with_timezone(&Local).date_naive();
            if local_date != target_date {
                continue;
            }

            seen_jira_ids.insert(entry.id.clone());

            let started_rfc3339 = parsed.with_timezone(&Local).to_rfc3339();
            let description = entry
                .comment
                .as_ref()
                .map(extract_adf_text)
                .unwrap_or_default();

            match existing_map.get(&entry.id) {
                None => {
                    // New worklog — insert (OR IGNORE to handle concurrent imports)
                    sqlx::query(
                        "INSERT OR IGNORE INTO worklogs (issue_key, started_at, duration_seconds, description, sync_status, jira_worklog_id, jira_updated_at) \
                         VALUES (?1, ?2, ?3, ?4, 'synced', ?5, ?6)",
                    )
                    .bind(issue_key)
                    .bind(&started_rfc3339)
                    .bind(entry.time_spent_seconds)
                    .bind(&description)
                    .bind(&entry.id)
                    .bind(&entry.updated)
                    .execute(&state.db)
                    .await
                    .map_err(|e| e.to_string())?;
                    imported += 1;
                }
                Some(local_updated) => {
                    if local_updated.as_deref() != Some(&entry.updated) {
                        // Changed in Jira — update locally
                        sqlx::query(
                            "UPDATE worklogs SET started_at = ?1, duration_seconds = ?2, description = ?3, jira_updated_at = ?4, updated_at = datetime('now') \
                             WHERE jira_worklog_id = ?5",
                        )
                        .bind(&started_rfc3339)
                        .bind(entry.time_spent_seconds)
                        .bind(&description)
                        .bind(&entry.updated)
                        .bind(&entry.id)
                        .execute(&state.db)
                        .await
                        .map_err(|e| e.to_string())?;
                        updated += 1;
                    } else {
                        skipped += 1;
                    }
                }
            }
        }
    }

    // Detect worklogs deleted from Jira.
    // Skip deletion entirely if JQL search failed (can't trust results).
    // Skip worklogs from issues where get_worklogs API call failed.
    // All other synced worklogs for target_date not seen in Jira → deleted in Jira.
    let mut deleted = 0u32;
    if jql_succeeded {
        let local_synced: Vec<(i64, String, String, String)> = sqlx::query_as(
            "SELECT id, issue_key, jira_worklog_id, started_at FROM worklogs \
             WHERE sync_status = 'synced' AND jira_worklog_id IS NOT NULL \
             AND started_at >= ?1 AND started_at <= ?2",
        )
        .bind(&date_from)
        .bind(&date_to)
        .fetch_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

        for (id, issue_key, jira_id, started_at) in local_synced {
            // Don't delete from issues where get_worklogs failed
            if failed_issues.contains(&issue_key) {
                continue;
            }
            if let Ok(dt) = DateTime::parse_from_rfc3339(&started_at) {
                let local_date = dt.with_timezone(&Local).date_naive();
                if local_date == target_date && !seen_jira_ids.contains(&jira_id) {
                    sqlx::query("DELETE FROM worklogs WHERE id = ?1")
                        .bind(id)
                        .execute(&state.db)
                        .await
                        .map_err(|e| e.to_string())?;
                    deleted += 1;
                }
            }
        }
    }

    Ok(ImportSummary {
        imported,
        updated,
        deleted,
        skipped,
        issues_checked,
        warnings,
    })
}
