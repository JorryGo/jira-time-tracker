use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Mutex;

pub struct AppState {
    pub db: SqlitePool,
    pub jira_config: Mutex<Option<JiraConfig>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JiraConfig {
    pub base_url: String,
    pub email: String,
    pub api_token: String,
}
