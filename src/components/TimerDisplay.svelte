<script lang="ts">
  import { timerStore } from "../lib/state/timer.svelte";
  import { formatDuration } from "../lib/utils/format";

  async function handlePause() {
    await timerStore.pause();
  }

  async function handleResume() {
    await timerStore.resume();
  }

  async function handleStop() {
    await timerStore.stop();
  }
</script>

{#if timerStore.current}
  <div class="timer-display">
    <div class="timer-info">
      <span class="timer-key">{timerStore.current.issue_key}</span>
      <span class="timer-summary">{timerStore.issueSummary}</span>
    </div>
    <div class="timer-controls">
      <span class="timer-time" class:paused={timerStore.isPaused}>
        {formatDuration(timerStore.elapsedSeconds)}
      </span>
      {#if timerStore.isRunning}
        <button class="btn-icon" onclick={handlePause} title="Pause">⏸</button>
      {:else}
        <button class="btn-icon" onclick={handleResume} title="Resume">▶</button>
      {/if}
      <button class="btn-icon btn-stop" onclick={handleStop} title="Stop">⏹</button>
    </div>
  </div>
{/if}

<style>
  .timer-display {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--accent);
    color: white;
    border-radius: var(--radius);
    margin: 8px 12px;
    gap: 8px;
  }

  .timer-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .timer-key {
    font-weight: 600;
    font-size: 12px;
  }

  .timer-summary {
    font-size: 11px;
    opacity: 0.85;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .timer-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .timer-time {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 14px;
  }

  .timer-time.paused {
    opacity: 0.6;
  }

  .btn-icon {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.2);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 12px;
    transition: background 0.15s;
  }

  .btn-icon:hover {
    background: rgba(255, 255, 255, 0.35);
  }

  .btn-stop:hover {
    background: rgba(255, 59, 48, 0.8);
  }
</style>
