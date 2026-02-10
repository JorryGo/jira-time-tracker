import { searchIssues } from "../commands/jira";
import type { JiraIssue } from "../types/jira";
import { settingsStore } from "./settings.svelte";

class TasksStore {
  items = $state<JiraIssue[]>([]);
  isLoading = $state(false);
  error = $state<string | null>(null);

  async refresh() {
    this.isLoading = true;
    this.error = null;
    try {
      this.items = await searchIssues(settingsStore.jqlFilter, 50);
    } catch (e) {
      this.error = String(e);
    } finally {
      this.isLoading = false;
    }
  }
}

export const tasksStore = new TasksStore();
