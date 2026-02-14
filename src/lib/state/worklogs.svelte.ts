import * as cmd from "../commands/worklogs";
import { pushWorklog, pushAllPending, updateJiraWorklog, deleteJiraWorklog, importWorklogs } from "../commands/jira";
import type { Worklog, WorklogFilter, PushSummary, ImportSummary } from "../types/worklog";

class WorklogsStore {
  items = $state<Worklog[]>([]);
  isLoading = $state(false);
  selectedIds = $state<Set<number>>(new Set());
  private lastFilter: WorklogFilter | undefined = undefined;

  get pendingCount(): number {
    return this.items.filter((w) => w.sync_status === "pending").length;
  }

  get selectedCount(): number {
    return this.selectedIds.size;
  }

  async refresh(filter?: WorklogFilter) {
    if (filter !== undefined) {
      this.lastFilter = filter;
    }
    this.isLoading = true;
    try {
      this.items = await cmd.getWorklogs(this.lastFilter);
    } finally {
      this.isLoading = false;
    }
  }

  async create(
    issueKey: string,
    startedAt: string,
    durationSeconds: number,
    description?: string,
  ) {
    await cmd.createWorklog(issueKey, startedAt, durationSeconds, description);
    await this.refresh();
  }

  async update(
    id: number,
    durationSeconds?: number,
    description?: string,
    startedAt?: string,
  ) {
    await cmd.updateWorklog(id, durationSeconds, description, startedAt);
    await this.refresh();
  }

  async updateAndSync(
    id: number,
    durationSeconds?: number,
    description?: string,
    startedAt?: string,
  ) {
    await updateJiraWorklog(id, durationSeconds, description, startedAt);
    await this.refresh();
  }

  async remove(id: number) {
    await cmd.deleteWorklog(id);
    this.selectedIds.delete(id);
    await this.refresh();
  }

  async removeFromJira(id: number) {
    await deleteJiraWorklog(id);
    this.selectedIds.delete(id);
    await this.refresh();
  }

  async importFromJira(date: string): Promise<ImportSummary> {
    const result = await importWorklogs(date);
    await this.refresh();
    return result;
  }

  async pushSelected(): Promise<PushSummary> {
    const ids = [...this.selectedIds];
    let success = 0;
    let errors: string[] = [];
    for (const id of ids) {
      try {
        await pushWorklog(id);
        success++;
      } catch (e) {
        errors.push(String(e));
      }
    }
    this.selectedIds = new Set();
    await this.refresh();
    return { total: ids.length, success, failed: ids.length - success, errors };
  }

  async pushAll(date: string): Promise<PushSummary> {
    const result = await pushAllPending(date);
    this.selectedIds = new Set();
    await this.refresh();
    return result;
  }

  toggleSelect(id: number) {
    const newSet = new Set(this.selectedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    this.selectedIds = newSet;
  }

  selectAllPending() {
    this.selectedIds = new Set(
      this.items.filter((w) => w.sync_status === "pending").map((w) => w.id),
    );
  }

  clearSelection() {
    this.selectedIds = new Set();
  }
}

export const worklogsStore = new WorklogsStore();
