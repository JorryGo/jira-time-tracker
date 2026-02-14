<script lang="ts">
  import { onDestroy } from "svelte";
  import type { JiraIssue } from "../lib/types/jira";
  import { worklogsStore } from "../lib/state/worklogs.svelte";
  import { searchIssues } from "../lib/commands/jira";

  function toLocalDateStr(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${y}-${m}-${day}`;
  }

  let {
    preselectedIssue = null,
    selectedDate = null,
    selectedTime = null,
    onClose,
  }: {
    preselectedIssue?: JiraIssue | null;
    selectedDate?: string | null;
    selectedTime?: string | null;
    onClose: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let issueKey = $state(preselectedIssue?.issue_key ?? "");
  // svelte-ignore state_referenced_locally
  let issueSummary = $state(preselectedIssue?.summary ?? "");
  let hours = $state(0);
  let minutes = $state(30);
  let description = $state("");
  // svelte-ignore state_referenced_locally
  let date = $state(selectedDate ?? toLocalDateStr(new Date()));
  // svelte-ignore state_referenced_locally
  let time = $state(selectedTime ?? (() => {
    const now = new Date();
    return `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
  })());

  let saving = $state(false);
  let error = $state("");

  let endTime = $derived((() => {
    const start = new Date(`${date}T${time}:00`);
    const end = new Date(start.getTime() + (hours * 3600 + minutes * 60) * 1000);
    const hh = String(end.getHours()).padStart(2, "0");
    const mm = String(end.getMinutes()).padStart(2, "0");
    const dayDiff = Math.floor((end.getTime() - start.getTime()) / 86_400_000);
    return dayDiff > 0 ? `${hh}:${mm} +${dayDiff}d` : `${hh}:${mm}`;
  })());

  // Search
  let searchQuery = $state("");
  let searchResults = $state<JiraIssue[]>([]);
  let searching = $state(false);
  // svelte-ignore state_referenced_locally
  let showSearch = $state(!preselectedIssue);
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

  async function handleSave() {
    if (!issueKey) {
      error = "Select a task";
      return;
    }
    const totalSeconds = hours * 3600 + minutes * 60;
    if (totalSeconds <= 0) {
      error = "Duration must be > 0";
      return;
    }
    saving = true;
    error = "";
    try {
      const startedAt = new Date(`${date}T${time}:00`).toISOString();
      await worklogsStore.create(issueKey, startedAt, totalSeconds, description);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="modal-overlay" onmousedown={onClose}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal" onmousedown={(e) => e.stopPropagation()}>
    <h3>Add Time Entry</h3>

    {#if showSearch}
      <div class="field">
        <label>Task
          <input
            type="text"
            placeholder="Search by key or text..."
            value={searchQuery}
            oninput={(e) => handleSearchInput(e.currentTarget.value)}
          />
        </label>
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
      <div class="selected-issue">
        <span class="si-key">{issueKey}</span>
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
      <button class="btn btn-secondary" onclick={onClose}>Cancel</button>
      <button class="btn btn-primary" onclick={handleSave} disabled={saving}>
        {saving ? "Saving..." : "Save"}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: var(--bg);
    border-radius: var(--radius);
    padding: 16px;
    width: 340px;
    max-height: 90vh;
    overflow-y: auto;
    border: 1px solid var(--border);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  }

  h3 {
    margin-bottom: 12px;
    font-size: 14px;
  }

  .field {
    margin-bottom: 10px;
    position: relative;
  }

  .field label {
    display: block;
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 3px;
  }

  .field input,
  .field textarea {
    width: 100%;
  }

  .field-row {
    display: flex;
    gap: 8px;
  }

  .field-row .field {
    flex: 1;
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
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .search-item {
    display: flex;
    gap: 6px;
    padding: 6px 8px;
    width: 100%;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  .search-item:hover {
    background: var(--bg-secondary);
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
    padding: 8px;
    background: var(--bg-secondary);
    border-radius: var(--radius-sm);
    margin-bottom: 10px;
  }

  .btn-link {
    font-size: 11px;
    color: var(--accent);
    margin-left: auto;
  }

  .end-time-hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 10px;
  }

  .error {
    color: var(--danger);
    font-size: 12px;
    margin-bottom: 8px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }

  .btn {
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
  }

  .btn-primary {
    background: var(--accent);
    color: white;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
  }

  .btn-primary:disabled {
    opacity: 0.5;
  }

  .btn-secondary {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
  }

</style>
