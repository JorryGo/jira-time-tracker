use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;

use super::types::*;

#[derive(Clone)]
pub struct JiraClient {
    client: Client,
    base_url: String,
    auth_header: String,
}

impl JiraClient {
    pub fn new(base_url: &str, email: &str, api_token: &str) -> Self {
        let credentials = format!("{}:{}", email, api_token);
        let auth_header = format!("Basic {}", general_purpose::STANDARD.encode(credentials));
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
        }
    }

    pub async fn get_myself(&self) -> Result<JiraUser, String> {
        self.client
            .get(format!("{}/rest/api/3/myself", self.base_url))
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Jira API error: {}", e))?
            .json::<JiraUser>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    pub async fn search_issues(
        &self,
        jql: &str,
        max_results: u32,
    ) -> Result<Vec<JiraIssue>, String> {
        let body = serde_json::json!({
            "jql": jql,
            "fields": ["summary", "status", "issuetype", "project"],
            "maxResults": max_results
        });

        let raw = self
            .client
            .post(format!("{}/rest/api/3/search/jql", self.base_url))
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Jira API error: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let resp: JiraSearchResponse = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse response: {}. Body preview: {}", e, &raw[..raw.len().min(500)]))?;

        Ok(resp.issues.into_iter().map(JiraIssue::from).collect())
    }

    pub async fn add_worklog(
        &self,
        issue_key: &str,
        time_spent_seconds: i64,
        started: &str,
        comment: &str,
    ) -> Result<JiraWorklogResponse, String> {
        let mut body = serde_json::json!({
            "timeSpentSeconds": time_spent_seconds,
            "started": started,
        });
        if !comment.is_empty() {
            body["comment"] = serde_json::json!({
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": comment
                    }]
                }]
            });
        }

        self.client
            .post(format!(
                "{}/rest/api/3/issue/{}/worklog",
                self.base_url, issue_key
            ))
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Jira API error: {}", e))?
            .json::<JiraWorklogResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    pub async fn update_worklog(
        &self,
        issue_key: &str,
        worklog_id: &str,
        time_spent_seconds: i64,
        started: &str,
        comment: &str,
    ) -> Result<JiraWorklogResponse, String> {
        let mut body = serde_json::json!({
            "timeSpentSeconds": time_spent_seconds,
            "started": started,
        });
        if !comment.is_empty() {
            body["comment"] = serde_json::json!({
                "type": "doc",
                "version": 1,
                "content": [{
                    "type": "paragraph",
                    "content": [{
                        "type": "text",
                        "text": comment
                    }]
                }]
            });
        }

        self.client
            .put(format!(
                "{}/rest/api/3/issue/{}/worklog/{}",
                self.base_url, issue_key, worklog_id
            ))
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Jira API error: {}", e))?
            .json::<JiraWorklogResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    pub async fn delete_worklog(
        &self,
        issue_key: &str,
        worklog_id: &str,
    ) -> Result<(), String> {
        self.client
            .delete(format!(
                "{}/rest/api/3/issue/{}/worklog/{}",
                self.base_url, issue_key, worklog_id
            ))
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Jira API error: {}", e))?;
        Ok(())
    }

    pub async fn get_worklogs(
        &self,
        issue_key: &str,
        started_after: Option<i64>,
    ) -> Result<JiraWorklogListResponse, String> {
        let mut url = format!(
            "{}/rest/api/3/issue/{}/worklog",
            self.base_url, issue_key
        );
        if let Some(epoch_ms) = started_after {
            url = format!("{}?startedAfter={}", url, epoch_ms);
        }
        self.client
            .get(&url)
            .header("Authorization", &self.auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Jira API error: {}", e))?
            .json::<JiraWorklogListResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }
}

pub fn extract_adf_text(adf: &serde_json::Value) -> String {
    let mut texts = Vec::new();
    if let Some(content) = adf.get("content").and_then(|c| c.as_array()) {
        for block in content {
            if let Some(inner) = block.get("content").and_then(|c| c.as_array()) {
                for node in inner {
                    if node.get("type").and_then(|t| t.as_str()) == Some("text") {
                        if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
                            texts.push(text.to_string());
                        }
                    }
                }
            }
        }
    }
    texts.join("")
}
