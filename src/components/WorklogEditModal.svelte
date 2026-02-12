<script lang="ts">
  import type { Worklog } from "../lib/types/worklog";
  import { worklogsStore } from "../lib/state/worklogs.svelte";

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
  let date = $state(worklog.started_at.split("T")[0]);
  let saving = $state(false);
  let error = $state("");

  async function handleSave() {
    const totalSeconds = hours * 3600 + minutes * 60;
    if (totalSeconds <= 0) {
      error = "Duration must be > 0";
      return;
    }
    saving = true;
    error = "";
    try {
      const startedAt = new Date(date).toISOString();
      await worklogsStore.update(worklog.id, totalSeconds, description, startedAt);
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
    <h3>Edit: {worklog.issue_key}</h3>

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
      <div class="field">
        <label>Date
          <input type="date" bind:value={date} />
        </label>
      </div>
    </div>

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
    width: 320px;
    border: 1px solid var(--border);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
  }

  h3 { margin-bottom: 12px; font-size: 14px; }

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
