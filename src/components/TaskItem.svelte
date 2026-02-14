<script lang="ts">
  import type { JiraIssue } from "../lib/types/jira";
  import { timerStore } from "../lib/state/timer.svelte";
  import { tasksStore } from "../lib/state/tasks.svelte";
  import { settingsStore } from "../lib/state/settings.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  let { issue, isPinned, onAddManual }: { issue: JiraIssue; isPinned: boolean; onAddManual: (issue: JiraIssue) => void } = $props();

  let isTimerOnThis = $derived(timerStore.current?.issue_key === issue.issue_key);

  async function handlePlay() {
    await timerStore.start(issue.issue_key, issue.summary);
  }

  async function handlePause() {
    await timerStore.pause();
  }

  async function handleResume() {
    await timerStore.resume();
  }
</script>

<div class="task-item" class:active={isTimerOnThis}>
  <div class="task-info">
    <div class="task-header">
      <button class="task-key-link" onclick={() => openUrl(`${settingsStore.jiraBaseUrl}/browse/${issue.issue_key}`)}>{issue.issue_key}</button>
      {#if issue.status}
        <span class="task-status">{issue.status}</span>
      {/if}
    </div>
    <span class="task-summary">{issue.summary}</span>
  </div>
  <div class="task-actions">
    <button
      class="btn-icon btn-pin"
      class:pinned={isPinned}
      onclick={() => tasksStore.togglePin(issue.issue_key)}
      title={isPinned ? "Unpin" : "Pin to top"}
    >
      <svg class="pin-icon" viewBox="0 0 24 24" fill="currentColor"><path d="M16 9V4h1c.55 0 1-.45 1-1s-.45-1-1-1H7c-.55 0-1 .45-1 1s.45 1 1 1h1v5c0 1.66-1.34 3-3 3v2h5.97v7l1 1 1-1v-7H19v-2c-1.66 0-3-1.34-3-3z"/></svg>
    </button>
    {#if isTimerOnThis}
      {#if timerStore.isRunning}
        <button class="btn-icon" onclick={handlePause} title="Pause">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><rect x="5" y="3" width="5" height="18" rx="1"/><rect x="14" y="3" width="5" height="18" rx="1"/></svg>
        </button>
      {:else}
        <button class="btn-icon btn-play" onclick={handleResume} title="Resume">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><path d="M6 3.5v17a1 1 0 0 0 1.5.86l14-8.5a1 1 0 0 0 0-1.72l-14-8.5A1 1 0 0 0 6 3.5z"/></svg>
        </button>
      {/if}
    {:else}
      <button class="btn-icon btn-play" onclick={handlePlay} title="Start timer">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><path d="M6 3.5v17a1 1 0 0 0 1.5.86l14-8.5a1 1 0 0 0 0-1.72l-14-8.5A1 1 0 0 0 6 3.5z"/></svg>
      </button>
    {/if}
    <button class="btn-icon" onclick={() => onAddManual(issue)} title="Add time manually">+</button>
  </div>
</div>

<style>
  .task-item {
    display: flex;
    align-items: center;
    padding: 9px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
    gap: 8px;
    transition: all var(--transition-fast);
    animation: slideUp 0.2s ease both;
    position: relative;
  }

  .task-item:hover {
    background: color-mix(in srgb, var(--bg-secondary) 70%, transparent);
    box-shadow: var(--shadow-sm);
  }

  .task-item.active {
    background: color-mix(in srgb, var(--accent) 6%, transparent);
    border-left: 3px solid var(--accent);
    padding-left: 9px;
  }

  .task-info {
    flex: 1;
    min-width: 0;
  }

  .task-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 2px;
  }

  .task-key-link {
    font-weight: 600;
    font-size: 12px;
    color: var(--accent);
    flex-shrink: 0;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: none;
    transition: color var(--transition-fast);
  }

  .task-key-link:hover {
    text-decoration: underline;
    color: var(--accent-hover);
  }

  .task-status {
    font-size: 9.5px;
    padding: 1px 7px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--text-secondary) 10%, transparent);
    color: var(--text-secondary);
    flex-shrink: 0;
    font-weight: 500;
    letter-spacing: 0.2px;
  }

  .task-summary {
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    opacity: 0.85;
  }

  .task-actions {
    display: flex;
    gap: 3px;
    flex-shrink: 0;
  }

  .btn-icon {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: transparent;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    border: 1px solid transparent;
    transition: all var(--transition-fast);
  }

  .btn-icon:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    transform: scale(1.08);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
  }

  .btn-icon:active {
    transform: scale(0.95);
  }

  .btn-play {
    color: var(--success);
  }

  .btn-play:hover {
    background: var(--success);
    border-color: var(--success);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--success) 30%, transparent);
  }

  .pin-icon {
    width: 14px;
    height: 14px;
  }

  .btn-pin {
    opacity: 0;
    transition: opacity var(--transition-fast), background var(--transition-fast), transform var(--transition-fast);
  }

  .task-item:hover .btn-pin,
  .btn-pin.pinned {
    opacity: 1;
  }

  .btn-pin.pinned {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
</style>
