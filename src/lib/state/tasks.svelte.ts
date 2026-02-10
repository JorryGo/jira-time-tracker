import { searchIssues } from "../commands/jira";
import { settingsGet, settingsSet } from "../commands/settings";
import type { JiraIssue } from "../types/jira";
import { settingsStore } from "./settings.svelte";

const DEFAULT_STATUS_ORDER = ["In Progress", "Reopened", "Ready for Develop"];

class TasksStore {
  items = $state<JiraIssue[]>([]);
  isLoading = $state(false);
  error = $state<string | null>(null);
  searchQuery = $state("");
  pinnedKeys = $state<Set<string>>(new Set());
  statusOrder = $state<string[]>(DEFAULT_STATUS_ORDER);

  get sortedItems(): JiraIssue[] {
    let filtered = this.items;

    const q = this.searchQuery.trim().toLowerCase();
    if (q) {
      filtered = filtered.filter(
        (i) =>
          i.issue_key.toLowerCase().includes(q) ||
          i.summary.toLowerCase().includes(q),
      );
    }

    const order = this.statusOrder;
    const pinned = this.pinnedKeys;

    const statusRank = (status: string | null): number => {
      if (!status) return order.length;
      const idx = order.findIndex((s) => s.toLowerCase() === status.toLowerCase());
      return idx >= 0 ? idx : order.length;
    };

    const pinnedItems = filtered.filter((i) => pinned.has(i.issue_key));
    const unpinnedItems = filtered.filter((i) => !pinned.has(i.issue_key));

    pinnedItems.sort((a, b) => statusRank(a.status) - statusRank(b.status));
    unpinnedItems.sort((a, b) => statusRank(a.status) - statusRank(b.status));

    return [...pinnedItems, ...unpinnedItems];
  }

  async init() {
    const orderJson = await settingsGet("status_sort_order");
    if (orderJson) {
      try {
        this.statusOrder = JSON.parse(orderJson);
      } catch {
        // keep default
      }
    }

    const pinnedJson = await settingsGet("pinned_issues");
    if (pinnedJson) {
      try {
        this.pinnedKeys = new Set(JSON.parse(pinnedJson));
      } catch {
        // keep default
      }
    }
  }

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

  async togglePin(issueKey: string) {
    const next = new Set(this.pinnedKeys);
    if (next.has(issueKey)) {
      next.delete(issueKey);
    } else {
      next.add(issueKey);
    }
    this.pinnedKeys = next;
    await settingsSet("pinned_issues", JSON.stringify([...next]));
  }

  async saveStatusOrder(order: string[]) {
    this.statusOrder = order;
    await settingsSet("status_sort_order", JSON.stringify(order));
  }
}

export const tasksStore = new TasksStore();
