use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Mutex;

pub struct AppState {
    pub db: SqlitePool,
    pub jira_config: Mutex<Option<JiraConfig>>,
    pub window_position: Mutex<Option<(i32, i32)>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JiraConfig {
    pub base_url: String,
    pub email: String,
    pub api_token: String,
}
