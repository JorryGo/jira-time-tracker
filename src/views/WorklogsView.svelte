<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { worklogsStore } from "../lib/state/worklogs.svelte";
  import WorklogEditModal from "../components/WorklogEditModal.svelte";
  import AddWorklogModal from "../components/AddWorklogModal.svelte";
  import { formatDurationShort, formatTimeRange, formatTimeOpen } from "../lib/utils/format";
  import { settingsStore } from "../lib/state/settings.svelte";
  import { timerStore } from "../lib/state/timer.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { invoke } from "@tauri-apps/api/core";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import type { Worklog, WorklogFilter } from "../lib/types/worklog";

  function toLocalDateStr(date: Date): string {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, "0");
    const d = String(date.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }

  let statusFilter = $state("all");
  let selectedDate = $state(toLocalDateStr(new Date()));
  let isToday = $derived(selectedDate === toLocalDateStr(new Date()));
  let showInProgress = $derived(isToday && timerStore.current !== null);
  let editingWorklog = $state<Worklog | null>(null);
  let showAddModal = $state(false);
  let pushingAll = $state(false);
  let pushingSelected = $state(false);
  let toast = $state("");

  let displayDate = $derived.by(() => {
    const [y, m, d] = selectedDate.split("-");
    return `${d}/${m}/${y}`;
  });

  const statusRank = (s: string) => s === "pending" ? 0 : s === "error" ? 1 : 2;

  let sortedWorklogs = $derived(
    [...worklogsStore.items].sort((a, b) => {
      const statusDiff = statusRank(a.sync_status) - statusRank(b.sync_status);
      if (statusDiff !== 0) return statusDiff;
      return b.started_at.localeCompare(a.started_at);
    })
  );

  let totalSeconds = $derived(
    worklogsStore.items.reduce((sum, wl) => sum + wl.duration_seconds, 0)
    + (showInProgress ? timerStore.elapsedSeconds : 0)
  );

  function buildFilter(): WorklogFilter {
    const filter: WorklogFilter = {};
    const startOfDay = new Date(selectedDate + "T00:00:00");
    const nextDay = new Date(startOfDay);
    nextDay.setDate(nextDay.getDate() + 1);
    filter.date_from = startOfDay.toISOString();
    filter.date_to = nextDay.toISOString();
    if (statusFilter !== "all") {
      filter.sync_status = statusFilter;
    }
    return filter;
  }

  async function refreshWorklogs() {
    await worklogsStore.refresh(buildFilter());
  }

  let syncInterval: number | undefined;

  onMount(async () => {
    await refreshWorklogs();
    backgroundSync();
    syncInterval = window.setInterval(backgroundSync, 60_000);
  });

  onDestroy(() => {
    if (syncInterval !== undefined) clearInterval(syncInterval);
  });

  async function handleFilterChange() {
    worklogsStore.clearSelection();
    await refreshWorklogs();
  }

  async function changeDate(offset: number) {
    const d = new Date(selectedDate + "T12:00:00");
    d.setDate(d.getDate() + offset);
    selectedDate = toLocalDateStr(d);
    worklogsStore.clearSelection();
    await refreshWorklogs();
    backgroundSync();
  }

  async function goToToday() {
    selectedDate = toLocalDateStr(new Date());
    worklogsStore.clearSelection();
    await refreshWorklogs();
    backgroundSync();
  }

  let showReport = $state(false);
  let reportText = $state("");
  let copied = $state(false);

  function generateReport(): string {
    const grouped = new Map<string, { summary: string | null; totalSeconds: number; descriptions: { text: string; seconds: number }[] }>();

    if (showInProgress && timerStore.current) {
      const desc = timerStore.description
        ? `${timerStore.description} (in progress)`
        : "(in progress)";
      grouped.set(timerStore.current.issue_key, {
        summary: timerStore.issueSummary || null,
        totalSeconds: timerStore.elapsedSeconds,
        descriptions: [{ text: desc, seconds: timerStore.elapsedSeconds }],
      });
    }

    for (const wl of sortedWorklogs) {
      let group = grouped.get(wl.issue_key);
      if (!group) {
        group = { summary: wl.issue_summary, totalSeconds: 0, descriptions: [] };
        grouped.set(wl.issue_key, group);
      }
      group.totalSeconds += wl.duration_seconds;
      group.descriptions.push({
        text: wl.description || "",
        seconds: wl.duration_seconds,
      });
    }

    const lines: string[] = [];
    let first = true;
    for (const [key, group] of grouped) {
      if (!first) lines.push("");
      first = false;
      const header = group.summary ? `${key} - ${group.summary}` : key;
      lines.push(`${header} (${formatDurationShort(group.totalSeconds)})`);
      const descs = group.descriptions.filter(d => d.text);
      for (const d of descs) {
        lines.push(`  - ${d.text}`);
      }
    }
    return lines.join("\n");
  }

  function openReport() {
    reportText = generateReport();
    copied = false;
    showReport = true;
  }

  async function copyReport() {
    await navigator.clipboard.writeText(reportText);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  let syncing = $state(false);
  let confirmDeleteId = $state<number | null>(null);
  let confirmDeleteSynced = $state(false);

  async function handleDelete(id: number, isSynced: boolean) {
    confirmDeleteId = id;
    confirmDeleteSynced = isSynced;
  }

  async function confirmDelete() {
    if (confirmDeleteId === null) return;
    try {
      if (confirmDeleteSynced) {
        await worklogsStore.removeFromJira(confirmDeleteId);
      } else {
        await worklogsStore.remove(confirmDeleteId);
      }
    } catch (e) {
      toast = String(e);
      setTimeout(() => (toast = ""), 3000);
    } finally {
      confirmDeleteId = null;
      confirmDeleteSynced = false;
    }
  }

  async function backgroundSync() {
    if (syncing || !settingsStore.isConnected) return;
    syncing = true;
    try {
      await worklogsStore.importFromJira(selectedDate);
    } catch (e) {
      console.warn("Background sync failed:", e);
    } finally {
      syncing = false;
    }
  }

  async function handlePushAll() {
    pushingAll = true;
    try {
      const result = await worklogsStore.pushAll(selectedDate);
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

  async function openCalendar() {
    try {
      await invoke("set_dock_visible", { visible: true });
      const found: boolean = await invoke("focus_calendar");
      if (found) return;
      new WebviewWindow("calendar", {
        url: "index.html?view=calendar",
        title: "Calendar — Jira Time Tracker",
        width: 900,
        height: 600,
        decorations: true,
        alwaysOnTop: false,
        resizable: true,
        skipTaskbar: false,
        visible: true,
        center: true,
      });
    } catch (e) {
      console.error("Failed to open calendar:", e);
    }
  }
</script>

<div class="worklogs-view">
  <div class="date-bar">
    <button class="date-calendar" onclick={openCalendar} title="Calendar view">
      <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
        <line x1="16" y1="2" x2="16" y2="6"></line>
        <line x1="8" y1="2" x2="8" y2="6"></line>
        <line x1="3" y1="10" x2="21" y2="10"></line>
      </svg>
    </button>
    <button class="btn btn-sm btn-nav" onclick={() => changeDate(-1)} title="Previous day">&#8592;</button>
    <input
      type="date"
      class="date-picker"
      bind:value={selectedDate}
      onchange={handleFilterChange}
    />
    <button class="btn btn-sm btn-nav" onclick={() => changeDate(1)} title="Next day">&#8594;</button>
    {#if selectedDate !== toLocalDateStr(new Date())}
      <button class="btn btn-sm date-today" onclick={goToToday}>Today</button>
    {/if}
  </div>

  <div class="toolbar">
    <div class="toolbar-left">
      <select bind:value={statusFilter} onchange={handleFilterChange}>
        <option value="all">All</option>
        <option value="pending">Pending</option>
        <option value="synced">Synced</option>
        <option value="error">Error</option>
      </select>
      <button class="btn btn-sm" onclick={() => (showAddModal = true)}>+ Add</button>
      {#if sortedWorklogs.length > 0 || showInProgress}
        <button class="btn btn-sm" onclick={openReport}>Report</button>
      {/if}
    </div>
    <div class="toolbar-right">
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

  <div class="summary-bar">
    <span class="total-label">Total:</span>
    <span class="total-value">{formatDurationShort(totalSeconds)}</span>
  </div>

  {#if toast}
    <div class="toast">{toast}</div>
  {/if}

  <div class="worklog-list">
    {#if showInProgress && timerStore.current}
      <div class="worklog-row in-progress-row">
        <div class="checkbox-spacer"></div>
        <div class="wl-info">
          <div class="wl-header">
            <button class="wl-key-link" onclick={() => openUrl(`${settingsStore.jiraBaseUrl}/browse/${timerStore.current?.issue_key}`)}>{timerStore.current?.issue_key}</button>
            <span class="wl-duration">{formatDurationShort(timerStore.elapsedSeconds)}</span>
            <span class="badge badge-in-progress">
              <span class="badge-dot"></span>
              {timerStore.isPaused ? "paused" : "in progress"}
            </span>
          </div>
          {#if timerStore.issueSummary}
            <div class="wl-summary">{timerStore.issueSummary}</div>
          {/if}
          <div class="wl-meta">
            <span>{formatTimeOpen(timerStore.elapsedSeconds)}</span>
          </div>
          <input
            class="wl-desc-input"
            type="text"
            placeholder="What I'm working on..."
            value={timerStore.description}
            oninput={(e) => timerStore.updateDescription(e.currentTarget.value)}
          />
        </div>
        <div class="wl-actions"></div>
      </div>
    {/if}
    {#each sortedWorklogs as wl (wl.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
      <div class="worklog-row" onclick={(e) => { if (!(e.target as HTMLElement).closest('button, input, a')) editingWorklog = wl; }}>
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
            <button class="wl-key-link" onclick={() => openUrl(`${settingsStore.jiraBaseUrl}/browse/${wl.issue_key}`)}>{wl.issue_key}</button>
            <span class="wl-duration">{formatDurationShort(wl.duration_seconds)}</span>
            <span class="badge {statusClass(wl.sync_status)}" title={wl.sync_error ?? ""}>
              <span class="badge-dot"></span>
              {wl.sync_status}
            </span>
          </div>
          {#if wl.issue_summary}
            <div class="wl-summary">{wl.issue_summary}</div>
          {/if}
          <div class="wl-meta">
            <span>{formatTimeRange(wl.started_at, wl.duration_seconds)}</span>
          </div>
          {#if wl.sync_status === "pending"}
            <input
              class="wl-desc-input"
              type="text"
              placeholder="What I worked on..."
              value={wl.description}
              oninput={(e) => worklogsStore.updateDescription(wl.id, e.currentTarget.value)}
            />
          {:else if wl.description}
            <div class="wl-desc">{wl.description}</div>
          {/if}
        </div>
        <div class="wl-actions">
          <button class="btn-icon-sm btn-edit" onclick={() => (editingWorklog = wl)} title="Edit">✎</button>
          <button class="btn-icon-sm btn-danger" onclick={() => handleDelete(wl.id, wl.sync_status === "synced")} title="Delete">✕</button>
        </div>
      </div>
    {:else}
      {#if !showInProgress}
        <div class="empty-state">No work logs yet</div>
      {/if}
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
  <AddWorklogModal selectedDate={selectedDate} onClose={() => (showAddModal = false)} />
{/if}

{#if showReport}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onmousedown={() => (showReport = false)}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="report-modal" onmousedown={(e) => e.stopPropagation()}>
      <div class="report-header">
        <span class="report-title">Report — {displayDate}</span>
      </div>
      <textarea class="report-textarea" bind:value={reportText}></textarea>
      <div class="report-actions">
        <button class="btn btn-sm" onclick={() => (showReport = false)}>Close</button>
        <button class="btn btn-sm btn-primary" onclick={copyReport}>
          {copied ? "Copied!" : "Copy"}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if confirmDeleteId !== null}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onmousedown={() => (confirmDeleteId = null)}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="confirm-dialog" onmousedown={(e) => e.stopPropagation()}>
      {#if confirmDeleteSynced}
        <p>Delete this entry from Jira and locally?</p>
        <p class="confirm-warning">This will also remove the worklog from Jira.</p>
      {:else}
        <p>Delete this entry?</p>
      {/if}
      <div class="confirm-actions">
        <button class="btn btn-sm" onclick={() => (confirmDeleteId = null)}>Cancel</button>
        <button class="btn btn-sm btn-danger" onclick={confirmDelete}>Delete</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .worklogs-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .date-bar {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 8px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
    gap: 6px;
    flex-shrink: 0;
    background: var(--bg-glass);
    backdrop-filter: blur(12px) saturate(150%);
    -webkit-backdrop-filter: blur(12px) saturate(150%);
  }

  .date-calendar {
    position: absolute;
    left: 12px;
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .date-calendar:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .date-calendar svg {
    display: block;
    flex-shrink: 0;
  }

  .date-today {
    position: absolute;
    right: 12px;
  }

  .date-picker {
    font-size: 13px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    color: var(--text);
    transition: background var(--transition-fast);
  }

  .date-picker:hover {
    background: var(--bg-secondary);
  }

  .toolbar {
    display: flex;
    align-items: center;
    padding: 6px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
    gap: 8px;
    flex-shrink: 0;
    background: var(--bg-glass);
    backdrop-filter: blur(12px) saturate(150%);
    -webkit-backdrop-filter: blur(12px) saturate(150%);
  }

  .toolbar select {
    padding: 3px 8px;
    font-size: 11px;
    height: 26px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    color: var(--text);
  }

  .toolbar-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .toolbar-right {
    display: flex;
    gap: 6px;
    margin-left: auto;
    align-items: center;
  }

  .btn-nav {
    width: 26px;
    height: 26px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    font-weight: 600;
    border-radius: 50%;
  }

  .btn-nav:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .summary-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    background: var(--accent-gradient);
    color: white;
    font-size: 12px;
    flex-shrink: 0;
  }

  .total-label {
    opacity: 0.8;
  }

  .total-value {
    font-weight: 700;
    font-family: var(--font-mono);
    letter-spacing: 0.3px;
  }

  .btn {
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    font-size: 11px;
    font-weight: 500;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    transition: all var(--transition-fast);
  }

  .btn:hover {
    background: var(--bg-tertiary);
  }

  .btn-sm { font-size: 11px; }

  .btn-primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-primary:hover {
    background: var(--accent-hover);
    transform: translateY(-0.5px);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .btn-accent {
    background: var(--success);
    color: white;
    border-color: var(--success);
  }

  .btn-accent:hover {
    background: color-mix(in srgb, var(--success) 85%, white);
    transform: translateY(-0.5px);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--success) 25%, transparent);
  }

  .btn:disabled { opacity: 0.5; }

  .toast {
    padding: 8px 12px;
    background: color-mix(in srgb, var(--accent) 10%, transparent);
    font-size: 12px;
    text-align: center;
    border-bottom: 1px solid color-mix(in srgb, var(--accent) 15%, transparent);
    color: var(--accent);
    font-weight: 500;
    animation: slideDown 0.2s ease;
  }

  .worklog-list {
    flex: 1;
    overflow-y: auto;
  }

  .worklog-row {
    display: flex;
    align-items: flex-start;
    padding: 10px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
    gap: 8px;
    transition: all var(--transition-fast);
    animation: slideUp 0.2s ease both;
    cursor: pointer;
  }

  .worklog-row:hover {
    background: color-mix(in srgb, var(--bg-secondary) 60%, transparent);
  }

  .worklog-row input[type="checkbox"] {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    cursor: pointer;
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

  .wl-key-link {
    font-weight: 600;
    font-size: 12px;
    color: var(--accent);
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: none;
    transition: color var(--transition-fast);
  }

  .wl-key-link:hover {
    text-decoration: underline;
    color: var(--accent-hover);
  }

  .wl-summary {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .wl-duration {
    font-size: 12px;
    font-weight: 600;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .badge {
    font-size: 9px;
    padding: 1.5px 7px;
    border-radius: 10px;
    text-transform: uppercase;
    font-weight: 600;
    letter-spacing: 0.3px;
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .badge-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: currentColor;
  }

  .badge-pending { background: color-mix(in srgb, var(--warning) 12%, transparent); color: var(--warning); }
  .badge-success { background: color-mix(in srgb, var(--success) 12%, transparent); color: var(--success); }
  .badge-error { background: color-mix(in srgb, var(--danger) 12%, transparent); color: var(--danger); }
  .badge-in-progress { background: color-mix(in srgb, var(--accent) 12%, transparent); color: var(--accent); }

  .in-progress-row {
    border-left: 3px solid var(--accent);
    padding-left: 9px;
    background: color-mix(in srgb, var(--accent) 3%, transparent);
    animation: borderPulse 2s ease-in-out infinite;
  }

  .wl-meta {
    font-size: 11px;
    color: var(--text-secondary);
    display: flex;
    gap: 8px;
  }

  .wl-desc {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 6px;
    padding: 4px 8px;
    background: color-mix(in srgb, var(--bg-secondary) 40%, transparent);
    border-radius: var(--radius-sm);
    border-left: 2px solid color-mix(in srgb, var(--border) 60%, transparent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-style: italic;
  }

  .wl-desc-input {
    font-size: 11px;
    color: var(--text-secondary) !important;
    margin-top: 6px;
    padding: 4px 8px !important;
    background: transparent !important;
    border: 1px dashed color-mix(in srgb, var(--border) 50%, transparent) !important;
    border-radius: var(--radius-sm);
    box-shadow: none !important;
    width: 100%;
    transition: all 0.2s ease;
  }

  .wl-desc-input::placeholder {
    color: var(--text-secondary);
    opacity: 0.4;
  }

  .wl-desc-input:focus {
    color: var(--text) !important;
    background: color-mix(in srgb, var(--bg-secondary) 60%, transparent) !important;
    border-style: solid !important;
    border-color: color-mix(in srgb, var(--accent) 50%, transparent) !important;
  }

  .wl-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .btn-icon-sm {
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: transparent;
    border: 1px solid transparent;
    font-size: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: all var(--transition-fast);
  }

  .worklog-row:hover .btn-icon-sm {
    opacity: 1;
  }

  .btn-edit {
    color: var(--accent);
  }

  .btn-icon-sm:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-danger {
    color: var(--danger);
  }

  .btn-danger:hover {
    background: var(--danger);
    border-color: var(--danger);
    color: white;
  }

  .empty-state {
    text-align: center;
    padding: 48px 16px;
    color: var(--text-secondary);
    animation: fadeIn 0.3s ease;
    font-weight: 500;
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.3);
    backdrop-filter: blur(8px) saturate(150%);
    -webkit-backdrop-filter: blur(8px) saturate(150%);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    animation: fadeIn 0.15s ease;
  }

  .confirm-dialog {
    background: var(--bg);
    border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
    border-radius: 12px;
    padding: 20px;
    box-shadow: var(--shadow-modal);
    text-align: center;
    animation: scaleIn 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .confirm-dialog p {
    margin-bottom: 12px;
    font-size: 13px;
    font-weight: 500;
  }

  .confirm-warning {
    font-size: 11px;
    color: var(--danger);
    margin-top: -8px;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .confirm-actions .btn-danger {
    background: var(--danger);
    color: white;
    border-color: var(--danger);
  }

  .confirm-actions .btn-danger:hover {
    background: color-mix(in srgb, var(--danger) 85%, black);
    transform: translateY(-0.5px);
  }

  .report-modal {
    background: var(--bg);
    border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
    border-radius: 12px;
    padding: 18px;
    width: 340px;
    box-shadow: var(--shadow-modal);
    display: flex;
    flex-direction: column;
    gap: 12px;
    animation: scaleIn 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .report-header {
    display: flex;
    align-items: center;
  }

  .report-title {
    font-size: 13px;
    font-weight: 600;
  }

  .report-textarea {
    width: 100%;
    min-height: 200px;
    max-height: 350px;
    padding: 10px;
    font-size: 11px;
    font-family: var(--font-mono);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    color: var(--text);
    resize: vertical;
    line-height: 1.5;
  }

  .report-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
</style>
