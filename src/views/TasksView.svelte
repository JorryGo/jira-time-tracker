<script lang="ts">
  import { onMount } from "svelte";
  import TimerDisplay from "../components/TimerDisplay.svelte";
  import TaskItem from "../components/TaskItem.svelte";
  import AddWorklogModal from "../components/AddWorklogModal.svelte";
  import { tasksStore } from "../lib/state/tasks.svelte";
  import { timerStore } from "../lib/state/timer.svelte";
  import { settingsStore } from "../lib/state/settings.svelte";
  import type { JiraIssue } from "../lib/types/jira";

  let addManualIssue = $state<JiraIssue | null>(null);
  let showAddModal = $state(false);

  onMount(async () => {
    await settingsStore.init();
    await timerStore.init();
    if (settingsStore.isConnected) {
      await tasksStore.refresh();
    }
  });

  function handleAddManual(issue: JiraIssue) {
    addManualIssue = issue;
    showAddModal = true;
  }

  function handleCloseModal() {
    showAddModal = false;
    addManualIssue = null;
  }

  async function handleRefresh() {
    await tasksStore.refresh();
  }
</script>

<div class="tasks-view">
  <TimerDisplay />

  {#if !settingsStore.isConnected}
    <div class="empty-state">
      <p>Jira not configured</p>
      <p class="hint">Go to Settings to connect</p>
    </div>
  {:else}
    <div class="toolbar">
      <span class="filter-label" title={settingsStore.jqlFilter}>
        {settingsStore.jqlFilter.length > 35
          ? settingsStore.jqlFilter.slice(0, 35) + "..."
          : settingsStore.jqlFilter}
      </span>
      <button class="btn-refresh" onclick={handleRefresh} disabled={tasksStore.isLoading}>
        {tasksStore.isLoading ? "..." : "↻"}
      </button>
    </div>

    {#if tasksStore.error}
      <div class="error-banner">{tasksStore.error}</div>
    {/if}

    <div class="task-list">
      {#each tasksStore.items as issue (issue.issue_key)}
        <TaskItem {issue} onAddManual={handleAddManual} />
      {:else}
        {#if !tasksStore.isLoading}
          <div class="empty-state">
            <p>No tasks found</p>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

{#if showAddModal}
  <AddWorklogModal preselectedIssue={addManualIssue} onClose={handleCloseModal} />
{/if}

<style>
  .tasks-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    align-items: center;
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    gap: 8px;
  }

  .filter-label {
    font-size: 11px;
    color: var(--text-secondary);
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .btn-refresh {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .btn-refresh:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-refresh:disabled {
    opacity: 0.5;
  }

  .task-list {
    flex: 1;
    overflow-y: auto;
  }

  .empty-state {
    text-align: center;
    padding: 40px 16px;
    color: var(--text-secondary);
  }

  .empty-state p {
    margin-bottom: 4px;
  }

  .hint {
    font-size: 11px;
  }

  .error-banner {
    padding: 8px 12px;
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    color: var(--danger);
    font-size: 12px;
    border-bottom: 1px solid var(--border);
  }
</style>
