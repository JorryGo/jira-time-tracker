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
    await tasksStore.init();
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
      <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
      <p>Jira not configured</p>
      <p class="hint">Go to Settings to connect</p>
    </div>
  {:else}
    <div class="toolbar">
      <div class="search-wrapper">
        <svg class="search-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        <input
          type="text"
          class="search-input"
          placeholder="Search tasks..."
          bind:value={tasksStore.searchQuery}
        />
      </div>
      <button class="btn-refresh" class:spinning={tasksStore.isLoading} onclick={handleRefresh} disabled={tasksStore.isLoading} title="Refresh">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21.5 2v6h-6"/><path d="M2.5 22v-6h6"/><path d="M2.5 11.5a10 10 0 0 1 17.3-5.7L21.5 8"/><path d="M21.5 12.5a10 10 0 0 1-17.3 5.7L2.5 16"/></svg>
      </button>
    </div>

    {#if tasksStore.error}
      <div class="error-banner">{tasksStore.error}</div>
    {/if}

    <div class="task-list">
      {#each tasksStore.sortedItems as issue (issue.issue_key)}
        <TaskItem {issue} isPinned={tasksStore.pinnedKeys.has(issue.issue_key)} onAddManual={handleAddManual} />
      {:else}
        {#if !tasksStore.isLoading}
          <div class="empty-state">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" opacity="0.3"><rect x="3" y="3" width="18" height="18" rx="2"/><line x1="9" y1="9" x2="15" y2="9"/><line x1="9" y1="13" x2="13" y2="13"/></svg>
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
    padding: 8px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
    gap: 8px;
    background: var(--bg-glass);
    backdrop-filter: blur(12px) saturate(150%);
    -webkit-backdrop-filter: blur(12px) saturate(150%);
  }

  .search-wrapper {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 8px;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    font-size: 12px;
    padding: 5px 8px 5px 28px;
    border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
    border-radius: 8px;
    background: color-mix(in srgb, var(--bg-secondary) 70%, transparent);
    color: var(--text);
    min-width: 0;
    transition: all var(--transition-fast);
  }

  .search-input::placeholder {
    color: var(--text-secondary);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent);
    background: var(--bg-secondary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .btn-refresh {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: transparent;
    border: 1px solid transparent;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--text-secondary);
    transition: all var(--transition-fast);
  }

  .btn-refresh:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-refresh:disabled {
    opacity: 0.5;
  }

  .btn-refresh.spinning svg {
    animation: spin 0.8s linear infinite;
  }

  .task-list {
    flex: 1;
    overflow-y: auto;
  }

  .empty-state {
    text-align: center;
    padding: 48px 16px;
    color: var(--text-secondary);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    animation: fadeIn 0.3s ease;
  }

  .empty-state p {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
  }

  .hint {
    font-size: 11px;
    opacity: 0.7;
  }

  .error-banner {
    padding: 8px 12px;
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    color: var(--danger);
    font-size: 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--danger) 15%, transparent);
    animation: slideDown 0.2s ease;
  }
</style>
