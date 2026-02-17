use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub latest_version: String,
    pub current_version: String,
    pub release_url: String,
    pub release_notes: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    };
    let l = parse(latest);
    let c = parse(current);
    for i in 0..l.len().max(c.len()) {
        let lv = l.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if lv > cv {
            return true;
        }
        if lv < cv {
            return false;
        }
    }
    false
}

const CACHE_DURATION_SECS: i64 = 86400; // 24 hours

#[tauri::command]
pub async fn check_for_update(
    app: tauri::AppHandle,
    force: bool,
) -> Result<UpdateCheckResult, String> {
    let state = app.state::<crate::state::AppState>();
    let current_version = app.package_info().version.to_string();

    // Check cache unless forced
    if !force {
        let last_checked: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'update_last_checked'",
        )
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        if let Some(ts) = last_checked {
            if let Ok(ts) = ts.parse::<i64>() {
                let now = chrono::Utc::now().timestamp();
                if now - ts < CACHE_DURATION_SECS {
                    // Return cached result
                    let cached: Option<String> = sqlx::query_scalar(
                        "SELECT value FROM settings WHERE key = 'update_cached_result'",
                    )
                    .fetch_optional(&state.db)
                    .await
                    .unwrap_or(None);

                    if let Some(json) = cached {
                        if let Ok(mut result) =
                            serde_json::from_str::<UpdateCheckResult>(&json)
                        {
                            result.current_version = current_version;
                            return Ok(result);
                        }
                    }
                }
            }
        }
    }

    // Fetch latest release from GitHub
    let response = state
        .http_client
        .get("https://api.github.com/repos/JorryGo/jira-time-tracker/releases/latest")
        .header("User-Agent", "jira-time-tracker")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    let latest_version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);

    let result = UpdateCheckResult {
        update_available: is_newer(latest_version, &current_version),
        latest_version: latest_version.to_string(),
        current_version,
        release_url: release.html_url,
        release_notes: release.body.unwrap_or_default(),
    };

    // Cache result
    let now = chrono::Utc::now().timestamp().to_string();
    let json = serde_json::to_string(&result).unwrap_or_default();

    let _ = sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('update_last_checked', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = ?1",
    )
    .bind(&now)
    .execute(&state.db)
    .await;

    let _ = sqlx::query(
        "INSERT INTO settings (key, value) VALUES ('update_cached_result', ?1) \
         ON CONFLICT(key) DO UPDATE SET value = ?1",
    )
    .bind(&json)
    .execute(&state.db)
    .await;

    Ok(result)
}
