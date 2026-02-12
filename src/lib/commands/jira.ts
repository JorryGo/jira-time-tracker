import { invoke } from "@tauri-apps/api/core";
import type { JiraIssue, JiraUser } from "../types/jira";
import type { PushSummary } from "../types/worklog";

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
