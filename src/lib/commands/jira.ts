import { invoke } from "@tauri-apps/api/core";
import type { JiraIssue, JiraUser } from "../types/jira";
import type { PushSummary, ImportSummary, ExternalWorklog } from "../types/worklog";

export async function testConnection(
  baseUrl: string,
  email: string,
  apiToken: string,
): Promise<JiraUser> {
  return invoke("jira_test_connection", {
    baseUrl,
    email,
    apiToken,
  });
}

export async function searchIssues(
  jql: string,
  maxResults?: number,
): Promise<JiraIssue[]> {
  return invoke("jira_search_issues", { jql, maxResults });
}

export async function pushWorklog(worklogId: number): Promise<void> {
  return invoke("jira_push_worklog", { worklogId });
}

export async function pushAllPending(date: string): Promise<PushSummary> {
  return invoke("jira_push_all_pending", { date });
}

export async function updateJiraWorklog(
  worklogId: number,
  durationSeconds?: number,
  description?: string,
  startedAt?: string,
): Promise<void> {
  return invoke("jira_update_worklog", {
    worklogId,
    durationSeconds,
    description,
    startedAt,
  });
}

export async function deleteJiraWorklog(worklogId: number): Promise<void> {
  return invoke("jira_delete_worklog", { worklogId });
}

export async function importWorklogs(date: string): Promise<ImportSummary> {
  return invoke("jira_import_worklogs", { date });
}

export async function getMyself(): Promise<JiraUser> {
  return invoke("jira_get_myself");
}

export async function searchUsers(query: string): Promise<JiraUser[]> {
  return invoke("jira_search_users", { query });
}

export async function fetchUserWorklogs(
  accountId: string,
  authorName: string,
  dateFrom: string,
  dateTo: string,
): Promise<ExternalWorklog[]> {
  return invoke("jira_fetch_user_worklogs", {
    accountId,
    authorName,
    dateFrom,
    dateTo,
  });
}

export async function fetchIssueWorklogs(
  issueKeys: string[],
  dateFrom: string,
  dateTo: string,
): Promise<ExternalWorklog[]> {
  return invoke("jira_fetch_issue_worklogs", {
    issueKeys,
    dateFrom,
    dateTo,
  });
}
