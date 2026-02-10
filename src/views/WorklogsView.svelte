<script lang="ts">
  import { onMount } from "svelte";
  import { worklogsStore } from "../lib/state/worklogs.svelte";
  import WorklogEditModal from "../components/WorklogEditModal.svelte";
  import AddWorklogModal from "../components/AddWorklogModal.svelte";
  import { formatDurationShort, formatDateTime } from "../lib/utils/format";
  import type { Worklog } from "../lib/types/worklog";

  let statusFilter = $state("all");
  let editingWorklog = $state<Worklog | null>(null);
  let showAddModal = $state(false);
  let pushingAll = $state(false);
  let pushingSelected = $state(false);
  let toast = $state("");

  onMount(() => {
    worklogsStore.refresh();
  });

  async function handleFilterChange() {
    const filter = statusFilter === "all" ? undefined : { sync_status: statusFilter };
    await worklogsStore.refresh(filter);
  }

  async function handleDelete(id: number) {
    try {
      await worklogsStore.remove(id);
    } catch (e) {
      toast = String(e);
      setTimeout(() => (toast = ""), 3000);
    }
  }

  async function handlePushAll() {
    pushingAll = true;
    try {
      const result = await worklogsStore.pushAll();
      toast = `Pushed ${result.success}/${result.total}${result.failed > 0 ? `, ${result.failed} failed` : ""}`;
    } catch (e) {
      toast = String(e);
    } finally {
      pushingAll = false;
      setTimeout(() => (toast = ""), 4000);
    }
  }

  async function handlePushSelected() {
    pushingSelected = true;
    try {
      const result = await worklogsStore.pushSelected();
      toast = `Pushed ${result.success}/${result.total}${result.failed > 0 ? `, ${result.failed} failed` : ""}`;
    } catch (e) {
      toast = String(e);
    } finally {
      pushingSelected = false;
      setTimeout(() => (toast = ""), 4000);
    }
  }

  function statusClass(status: string): string {
    if (status === "synced") return "badge-success";
    if (status === "error") return "badge-error";
    return "badge-pending";
  }
</script>

<div class="worklogs-view">
  <div class="toolbar">
    <select bind:value={statusFilter} onchange={handleFilterChange}>
      <option value="all">All</option>
      <option value="pending">Pending</option>
      <option value="synced">Synced</option>
      <option value="error">Error</option>
    </select>
    <div class="toolbar-right">
      <button class="btn btn-sm" onclick={() => (showAddModal = true)}>+ Add</button>
      {#if worklogsStore.selectedCount > 0}
        <button
          class="btn btn-sm btn-primary"
          onclick={handlePushSelected}
          disabled={pushingSelected}
        >
          Push {worklogsStore.selectedCount}
        </button>
      {/if}
      {#if worklogsStore.pendingCount > 0}
        <button
          class="btn btn-sm btn-accent"
          onclick={handlePushAll}
          disabled={pushingAll}
        >
          {pushingAll ? "Pushing..." : `Push All (${worklogsStore.pendingCount})`}
        </button>
      {/if}
    </div>
  </div>

  {#if toast}
    <div class="toast">{toast}</div>
  {/if}

  <div class="worklog-list">
    {#each worklogsStore.items as wl (wl.id)}
      <div class="worklog-row">
        {#if wl.sync_status === "pending"}
          <input
            type="checkbox"
            checked={worklogsStore.selectedIds.has(wl.id)}
            onchange={() => worklogsStore.toggleSelect(wl.id)}
          />
        {:else}
          <div class="checkbox-spacer"></div>
        {/if}
        <div class="wl-info">
          <div class="wl-header">
            <span class="wl-key">{wl.issue_key}</span>
            <span class="wl-duration">{formatDurationShort(wl.duration_seconds)}</span>
            <span class="badge {statusClass(wl.sync_status)}" title={wl.sync_error ?? ""}>
              {wl.sync_status}
            </span>
          </div>
          <div class="wl-meta">
            <span>{formatDateTime(wl.started_at)}</span>
            {#if wl.description}
              <span class="wl-desc">{wl.description}</span>
            {/if}
          </div>
        </div>
        <div class="wl-actions">
          {#if wl.sync_status !== "synced"}
            <button class="btn-icon-sm" onclick={() => (editingWorklog = wl)} title="Edit">✎</button>
            <button class="btn-icon-sm btn-danger" onclick={() => handleDelete(wl.id)} title="Delete">✕</button>
          {/if}
        </div>
      </div>
    {:else}
      <div class="empty-state">No work logs yet</div>
    {/each}
  </div>
</div>

{#if editingWorklog}
  <WorklogEditModal
    worklog={editingWorklog}
    onClose={() => (editingWorklog = null)}
  />
{/if}

{#if showAddModal}
  <AddWorklogModal onClose={() => (showAddModal = false)} />
{/if}

<style>
  .worklogs-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    gap: 8px;
    flex-shrink: 0;
  }

  .toolbar select {
    padding: 4px 8px;
    font-size: 11px;
  }

  .toolbar-right {
    display: flex;
    gap: 6px;
    margin-left: auto;
  }

  .btn {
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-weight: 500;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .btn-sm { font-size: 11px; }

  .btn-primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-accent {
    background: var(--success);
    color: white;
    border-color: var(--success);
  }

  .btn:disabled { opacity: 0.5; }

  .toast {
    padding: 6px 12px;
    background: var(--bg-secondary);
    font-size: 12px;
    text-align: center;
    border-bottom: 1px solid var(--border);
  }

  .worklog-list {
    flex: 1;
    overflow-y: auto;
  }

  .worklog-row {
    display: flex;
    align-items: flex-start;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    gap: 8px;
  }

  .worklog-row:hover {
    background: var(--bg-secondary);
  }

  .checkbox-spacer {
    width: 16px;
    flex-shrink: 0;
  }

  .wl-info {
    flex: 1;
    min-width: 0;
  }

  .wl-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 2px;
  }

  .wl-key {
    font-weight: 600;
    font-size: 12px;
    color: var(--accent);
  }

  .wl-duration {
    font-size: 12px;
    font-weight: 500;
  }

  .badge {
    font-size: 9px;
    padding: 1px 5px;
    border-radius: 3px;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.3px;
  }

  .badge-pending { background: color-mix(in srgb, var(--warning) 15%, transparent); color: var(--warning); }
  .badge-success { background: color-mix(in srgb, var(--success) 15%, transparent); color: var(--success); }
  .badge-error { background: color-mix(in srgb, var(--danger) 15%, transparent); color: var(--danger); }

  .wl-meta {
    font-size: 11px;
    color: var(--text-secondary);
    display: flex;
    gap: 8px;
  }

  .wl-desc {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .wl-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .btn-icon-sm {
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .btn-icon-sm:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-danger:hover {
    background: var(--danger);
    border-color: var(--danger);
  }

  .empty-state {
    text-align: center;
    padding: 40px 16px;
    color: var(--text-secondary);
  }
</style>
