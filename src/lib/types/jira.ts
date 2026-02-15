export interface JiraUser {
  accountId: string;
  displayName: string;
  emailAddress?: string;
}

export interface JiraIssue {
  issue_key: string;
  summary: string;
  project_key: string;
  status: string | null;
  issue_type: string | null;
}
