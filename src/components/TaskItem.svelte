<script lang="ts">
  import type { JiraIssue } from "../lib/types/jira";
  import { timerStore } from "../lib/state/timer.svelte";
  import { tasksStore } from "../lib/state/tasks.svelte";
  import pinIcon from "../assets/pin.png";

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
      <span class="task-key">{issue.issue_key}</span>
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
      <img class="pin-icon" src={pinIcon} alt="pin" />
    </button>
    {#if isTimerOnThis}
      {#if timerStore.isRunning}
        <button class="btn-icon" onclick={handlePause} title="Pause">⏸</button>
      {:else}
        <button class="btn-icon btn-play" onclick={handleResume} title="Resume">▶</button>
      {/if}
    {:else}
      <button class="btn-icon btn-play" onclick={handlePlay} title="Start timer">▶</button>
    {/if}
    <button class="btn-icon" onclick={() => onAddManual(issue)} title="Add time manually">+</button>
  </div>
</div>

<style>
  .task-item {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    gap: 8px;
    transition: background 0.1s;
  }

  .task-item:hover {
    background: var(--bg-secondary);
  }

  .task-item.active {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
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

  .task-key {
    font-weight: 600;
    font-size: 12px;
    color: var(--accent);
    flex-shrink: 0;
  }

  .task-status {
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 3px;
    background: var(--bg-secondary);
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .task-summary {
    font-size: 12px;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }

  .task-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .btn-icon {
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--bg-secondary);
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    border: 1px solid var(--border);
    transition: all 0.15s;
  }

  .btn-icon:hover {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .btn-play {
    color: var(--success);
  }

  .pin-icon {
    width: 14px;
    height: 14px;
    object-fit: contain;
  }

  .btn-pin {
    opacity: 0;
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
