<script lang="ts">
  import { onDestroy } from "svelte";
  import type { Worklog } from "../lib/types/worklog";
  import type { JiraIssue } from "../lib/types/jira";
  import { worklogsStore } from "../lib/state/worklogs.svelte";
  import { settingsStore } from "../lib/state/settings.svelte";
  import { searchIssues } from "../lib/commands/jira";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let {
    worklog,
    onClose,
  }: {
    worklog: Worklog;
    onClose: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let hours = $state(Math.floor(worklog.duration_seconds / 3600));
  // svelte-ignore state_referenced_locally
  let minutes = $state(Math.floor((worklog.duration_seconds % 3600) / 60));
  // svelte-ignore state_referenced_locally
  let description = $state(worklog.description);
  // svelte-ignore state_referenced_locally
  let date = $state((() => {
    const d = new Date(worklog.started_at);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  })());
  // svelte-ignore state_referenced_locally
  let time = $state((() => {
    const d = new Date(worklog.started_at);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  })());
  // svelte-ignore state_referenced_locally
  let issueKey = $state(worklog.issue_key);
  // svelte-ignore state_referenced_locally
  let issueSummary = $state(worklog.issue_summary ?? "");

  let saving = $state(false);
  let deleting = $state(false);
  let confirmDelete = $state(false);
  let error = $state("");
  // svelte-ignore state_referenced_locally
  const isSynced = worklog.sync_status === "synced";

  // Issue search (pending/error worklogs only)
  let searchQuery = $state("");
  let searchResults = $state<JiraIssue[]>([]);
  let searching = $state(false);
  let showSearch = $state(false);
  let searchTimeout: number | null = null;

  function handleSearchInput(value: string) {
    searchQuery = value;
    if (searchTimeout) clearTimeout(searchTimeout);
    if (value.length < 2) {
      searchResults = [];
      return;
    }
    searchTimeout = window.setTimeout(async () => {
      searching = true;
      try {
        searchResults = await searchIssues(
          `key = "${value}" OR summary ~ "${value}" ORDER BY updated DESC`,
          10,
        );
      } catch {
        searchResults = [];
      } finally {
        searching = false;
      }
    }, 300);
  }

  onDestroy(() => {
    if (searchTimeout) clearTimeout(searchTimeout);
  });

  function selectIssue(issue: JiraIssue) {
    issueKey = issue.issue_key;
    issueSummary = issue.summary;
    showSearch = false;
    searchResults = [];
    searchQuery = "";
  }

  let endTime = $derived((() => {
    const start = new Date(`${date}T${time}:00`);
    const end = new Date(start.getTime() + (hours * 3600 + minutes * 60) * 1000);
    const hh = String(end.getHours()).padStart(2, "0");
    const mm = String(end.getMinutes()).padStart(2, "0");
    const dayDiff = Math.floor((end.getTime() - start.getTime()) / 86_400_000);
    return dayDiff > 0 ? `${hh}:${mm} +${dayDiff}d` : `${hh}:${mm}`;
  })());

  async function handleSave() {
    const totalSeconds = hours * 3600 + minutes * 60;
    if (totalSeconds <= 0) {
      error = "Duration must be > 0";
      return;
    }
    saving = true;
    error = "";
    try {
      const startedAt = new Date(`${date}T${time}:00`).toISOString();
      const newIssueKey = issueKey !== worklog.issue_key ? issueKey : undefined;
      if (isSynced) {
        await worklogsStore.updateAndSync(worklog.id, totalSeconds, description, startedAt);
      } else {
        await worklogsStore.update(worklog.id, newIssueKey, totalSeconds, description, startedAt);
      }
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function handleDelete() {
    deleting = true;
    error = "";
    try {
      if (isSynced) {
        await worklogsStore.removeFromJira(worklog.id);
      } else {
        await worklogsStore.remove(worklog.id);
      }
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      deleting = false;
      confirmDelete = false;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-overlay" onmousedown={onClose}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal" onmousedown={(e) => e.stopPropagation()}>
    {#if isSynced}
      <h3>Edit: <button class="issue-link" onclick={() => { if (settingsStore.jiraBaseUrl) openUrl(`${settingsStore.jiraBaseUrl}/browse/${worklog.issue_key}`); }}>{worklog.issue_key}</button></h3>
      <div class="sync-warning">Changes will be pushed to Jira</div>
    {:else if showSearch}
      <h3>Edit worklog</h3>
      <div class="field">
        <label>Task</label>
        <div class="search-row">
          <input
            type="text"
            placeholder="Search by key or text..."
            value={searchQuery}
            oninput={(e) => handleSearchInput(e.currentTarget.value)}
          />
          <button class="btn-cancel-search" onclick={() => { showSearch = false; searchQuery = ""; searchResults = []; }}>cancel</button>
        </div>
        {#if searching}
          <div class="search-hint">Searching...</div>
        {/if}
        {#if searchResults.length > 0}
          <div class="search-results">
            {#each searchResults as issue}
              <button class="search-item" onclick={() => selectIssue(issue)}>
                <span class="si-key">{issue.issue_key}</span>
                <span class="si-summary">{issue.summary}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      <h3>Edit worklog</h3>
      <div class="selected-issue">
        <button class="si-key" onclick={() => { if (settingsStore.jiraBaseUrl) openUrl(`${settingsStore.jiraBaseUrl}/browse/${issueKey}`); }}>{issueKey}</button>
        <span class="si-summary">{issueSummary}</span>
        <button class="btn-link" onclick={() => (showSearch = true)}>change</button>
      </div>
    {/if}

    <div class="field-row">
      <div class="field">
        <label>Date
          <input type="date" bind:value={date} />
        </label>
      </div>
      <div class="field">
        <label>Start time
          <input type="text" pattern="\d{2}:\d{2}" placeholder="HH:MM" bind:value={time} />
        </label>
      </div>
    </div>
    <div class="field-row">
      <div class="field">
        <label>Hours
          <input type="number" min="0" max="24" bind:value={hours} />
        </label>
      </div>
      <div class="field">
        <label>Minutes
          <input type="number" min="0" max="59" bind:value={minutes} />
        </label>
      </div>
    </div>
    <div class="end-time-hint">{time} &rarr; {endTime}</div>

    <div class="field">
      <label>Description
        <textarea rows="2" bind:value={description} placeholder="What did you work on?"></textarea>
      </label>
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <div class="actions">
      {#if confirmDelete}
        <span class="delete-confirm-text">{isSynced ? "Will also remove from Jira" : "Delete this entry?"}</span>
        <button class="btn btn-secondary" onclick={() => (confirmDelete = false)} disabled={deleting}>Cancel</button>
        <button class="btn btn-danger" onclick={handleDelete} disabled={deleting}>
          {deleting ? "Deleting..." : "Delete"}
        </button>
      {:else}
        <button class="btn btn-danger-ghost" onclick={() => (confirmDelete = true)}>Delete</button>
        <div class="actions-right">
          <button class="btn btn-secondary" onclick={onClose}>Cancel</button>
          <button class="btn btn-primary" onclick={handleSave} disabled={saving}>
            {saving ? "Saving..." : isSynced ? "Save & Push" : "Save"}
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
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

  .modal {
    background: var(--bg);
    border-radius: 12px;
    padding: 18px;
    width: 320px;
    border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
    box-shadow: var(--shadow-modal);
    animation: scaleIn 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  h3 {
    margin-bottom: 14px;
    font-size: 15px;
    font-weight: 600;
  }

  .issue-link {
    color: var(--accent);
    font-size: inherit;
    font-weight: inherit;
    padding: 0;
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  .issue-link:hover {
    text-decoration: underline;
    color: var(--accent-hover);
  }

  .field { margin-bottom: 10px; position: relative; }

  .field label {
    display: block;
    font-size: 10.5px;
    color: var(--text-secondary);
    margin-bottom: 4px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .field input, .field textarea { width: 100%; }

  .field-row { display: flex; gap: 8px; }
  .field-row .field { flex: 1; }

  .end-time-hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 10px;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .sync-warning {
    background: color-mix(in srgb, var(--warning) 10%, transparent);
    color: color-mix(in srgb, var(--warning) 80%, black);
    font-size: 11px;
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    margin-bottom: 12px;
    border: 1px solid color-mix(in srgb, var(--warning) 30%, transparent);
  }

  .error {
    color: var(--danger);
    font-size: 12px;
    margin-bottom: 8px;
    padding: 6px 8px;
    background: color-mix(in srgb, var(--danger) 6%, transparent);
    border-radius: var(--radius-sm);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  .btn {
    padding: 7px 16px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    transition: all var(--transition-fast);
  }

  .btn-primary { background: var(--accent); color: white; }
  .btn-primary:hover {
    background: var(--accent-hover);
    transform: translateY(-0.5px);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 25%, transparent);
  }
  .btn-primary:disabled { opacity: 0.5; transform: none; box-shadow: none; }
  .btn-secondary { background: var(--bg-secondary); border: 1px solid var(--border); }
  .btn-secondary:hover { background: var(--bg-tertiary); }

  .btn-danger { background: var(--danger); color: white; }
  .btn-danger:hover {
    background: color-mix(in srgb, var(--danger) 85%, black);
    transform: translateY(-0.5px);
  }
  .btn-danger:disabled { opacity: 0.5; transform: none; }

  .btn-danger-ghost {
    background: transparent;
    color: var(--danger);
    border: 1px solid transparent;
  }
  .btn-danger-ghost:hover {
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    border-color: color-mix(in srgb, var(--danger) 20%, transparent);
  }

  .actions-right {
    display: flex;
    gap: 8px;
    margin-left: auto;
  }

  .delete-confirm-text {
    font-size: 12px;
    color: var(--danger);
    font-weight: 500;
    margin-right: auto;
    align-self: center;
  }

  .search-results {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    max-height: 150px;
    overflow-y: auto;
    z-index: 10;
    box-shadow: var(--shadow-lg);
    animation: slideUp 0.15s ease;
  }

  .search-item {
    display: flex;
    gap: 6px;
    padding: 7px 10px;
    width: 100%;
    text-align: left;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
    transition: background var(--transition-fast);
  }

  .search-item:last-child { border-bottom: none; }

  .search-item:hover {
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }

  .search-hint {
    font-size: 11px;
    color: var(--text-secondary);
    padding: 4px 0;
  }

  .si-key {
    font-weight: 600;
    font-size: 11px;
    color: var(--accent);
    flex-shrink: 0;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  .si-key:hover {
    text-decoration: underline;
    color: var(--accent-hover);
  }

  .si-summary {
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .selected-issue {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 10px;
    background: color-mix(in srgb, var(--accent) 5%, var(--bg-secondary));
    border-radius: var(--radius-sm);
    margin-bottom: 10px;
    border: 1px solid color-mix(in srgb, var(--accent) 15%, transparent);
  }

  .btn-link {
    font-size: 11px;
    color: var(--accent);
    margin-left: auto;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .btn-link:hover {
    text-decoration: underline;
  }

  .search-row {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .search-row input {
    flex: 1;
  }

  .btn-cancel-search {
    background: none;
    border: none;
    font-size: 11px;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px 0;
    flex-shrink: 0;
    transition: color var(--transition-fast);
  }

  .btn-cancel-search:hover {
    color: var(--text);
  }
</style>
