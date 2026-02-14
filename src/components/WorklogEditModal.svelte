<script lang="ts">
  import type { Worklog } from "../lib/types/worklog";
  import { worklogsStore } from "../lib/state/worklogs.svelte";
  import { settingsStore } from "../lib/state/settings.svelte";
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
  let saving = $state(false);
  let error = $state("");
  // svelte-ignore state_referenced_locally
  const isSynced = worklog.sync_status === "synced";

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
      if (isSynced) {
        await worklogsStore.updateAndSync(worklog.id, totalSeconds, description, startedAt);
      } else {
        await worklogsStore.update(worklog.id, totalSeconds, description, startedAt);
      }
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
    <h3>Edit: <button class="issue-link" onclick={() => { if (settingsStore.jiraBaseUrl) openUrl(`${settingsStore.jiraBaseUrl}/browse/${worklog.issue_key}`); }}>{worklog.issue_key}</button></h3>

    {#if isSynced}
      <div class="sync-warning">Changes will be pushed to Jira</div>
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
        {saving ? "Saving..." : isSynced ? "Save & Push" : "Save"}
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
    width: 320px;
    border: 1px solid var(--border);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  }

  h3 { margin-bottom: 12px; font-size: 14px; }

  .issue-link {
    color: var(--accent);
    font-size: inherit;
    font-weight: inherit;
    padding: 0;
    cursor: pointer;
  }

  .issue-link:hover {
    text-decoration: underline;
  }

  .field { margin-bottom: 10px; }

  .field label {
    display: block;
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 3px;
  }

  .field input, .field textarea { width: 100%; }

  .field-row { display: flex; gap: 8px; }
  .field-row .field { flex: 1; }

  .end-time-hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 10px;
  }

  .sync-warning {
    background: rgba(255, 170, 0, 0.12);
    color: #b8860b;
    font-size: 11px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    margin-bottom: 10px;
  }

  .error { color: var(--danger); font-size: 12px; margin-bottom: 8px; }

  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px; }

  .btn {
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
  }

  .btn-primary { background: var(--accent); color: white; }
  .btn-primary:hover { background: var(--accent-hover); }
  .btn-primary:disabled { opacity: 0.5; }
  .btn-secondary { background: var(--bg-secondary); border: 1px solid var(--border); }
</style>
