export interface Worklog {
  id: number;
  issue_key: string;
  started_at: string;
  duration_seconds: number;
  description: string;
  sync_status: "pending" | "synced" | "error";
  jira_worklog_id: string | null;
  sync_error: string | null;
  created_at: string;
  updated_at: string;
  issue_summary: string | null;
}

export interface WorklogFilter {
  issue_key?: string;
  sync_status?: string;
  date_from?: string;
  date_to?: string;
}

export interface StoppedWorklog {
  id: number;
  issue_key: string;
  duration_seconds: number;
}

export interface PushSummary {
  total: number;
  success: number;
  failed: number;
  errors: string[];
}
