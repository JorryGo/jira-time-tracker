use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JiraUser {
    #[serde(rename = "accountId")]
    pub account_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "emailAddress", default)]
    pub email_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JiraSearchResponse {
    pub issues: Vec<JiraIssueRaw>,
}

#[derive(Debug, Deserialize)]
pub struct JiraIssueRaw {
    pub key: String,
    pub fields: JiraIssueFields,
}

#[derive(Debug, Deserialize)]
pub struct JiraIssueFields {
    pub summary: String,
    pub status: Option<JiraStatus>,
    pub issuetype: Option<JiraIssueType>,
    pub project: Option<JiraProject>,
}

#[derive(Debug, Deserialize)]
pub struct JiraStatus {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraIssueType {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraProject {
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JiraIssue {
    pub issue_key: String,
    pub summary: String,
    pub project_key: String,
    pub status: Option<String>,
    pub issue_type: Option<String>,
}

impl From<JiraIssueRaw> for JiraIssue {
    fn from(raw: JiraIssueRaw) -> Self {
        Self {
            issue_key: raw.key,
            summary: raw.fields.summary,
            project_key: raw.fields.project.map(|p| p.key).unwrap_or_default(),
            status: raw.fields.status.map(|s| s.name),
            issue_type: raw.fields.issuetype.map(|t| t.name),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JiraWorklogResponse {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct JiraWorklogListResponse {
    pub worklogs: Vec<JiraWorklogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct JiraWorklogEntry {
    pub id: String,
    pub started: String,
    pub updated: String,
    #[serde(rename = "timeSpentSeconds")]
    pub time_spent_seconds: i64,
    pub comment: Option<serde_json::Value>,
    pub author: JiraWorklogAuthor,
}

#[derive(Debug, Deserialize)]
pub struct JiraWorklogAuthor {
    #[serde(rename = "accountId")]
    pub account_id: String,
}
