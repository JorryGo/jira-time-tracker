import { invoke } from "@tauri-apps/api/core";
import type { Worklog, WorklogFilter } from "../types/worklog";

export async function getWorklogs(
  filter?: WorklogFilter,
): Promise<Worklog[]> {
  return invoke("get_worklogs", { filter: filter ?? null });
}

export async function createWorklog(
  issueKey: string,
  startedAt: string,
  durationSeconds: number,
  description?: string,
): Promise<Worklog> {
  return invoke("create_worklog", {
    issueKey,
    startedAt,
    durationSeconds,
    description,
  });
}

export async function updateWorklog(
  id: number,
  issueKey?: string,
  durationSeconds?: number,
  description?: string,
  startedAt?: string,
): Promise<Worklog> {
  return invoke("update_worklog", {
    id,
    issueKey,
    durationSeconds,
    description,
    startedAt,
  });
}

export async function deleteWorklog(id: number): Promise<void> {
  return invoke("delete_worklog", { id });
}
