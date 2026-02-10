export interface JiraUser {
  account_id: string;
  display_name: string;
  email_address?: string;
}

export interface JiraIssue {
  issue_key: string;
  summary: string;
  project_key: string;
  status: string | null;
  issue_type: string | null;
}
