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
  <div class="timer-display" class:running={timerStore.isRunning} class:paused={timerStore.isPaused}>
    <div class="timer-info">
      <span class="timer-key">{timerStore.current.issue_key}</span>
      <span class="timer-summary">{timerStore.issueSummary}</span>
    </div>
    <div class="timer-controls">
      <span class="timer-time" class:paused={timerStore.isPaused}>
        {formatDuration(timerStore.elapsedSeconds)}
      </span>
      {#if timerStore.isRunning}
        <button class="btn-icon" onclick={handlePause} title="Pause">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><rect x="5" y="3" width="5" height="18" rx="1"/><rect x="14" y="3" width="5" height="18" rx="1"/></svg>
        </button>
      {:else}
        <button class="btn-icon btn-resume" onclick={handleResume} title="Resume">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 3.5v17a1 1 0 0 0 1.5.86l14-8.5a1 1 0 0 0 0-1.72l-14-8.5A1 1 0 0 0 6 3.5z"/></svg>
        </button>
      {/if}
      <button class="btn-icon btn-stop" onclick={handleStop} title="Stop">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor"><rect x="3" y="3" width="18" height="18" rx="2"/></svg>
      </button>
    </div>
  </div>
{/if}

<style>
  .timer-display {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--timer-gradient);
    color: white;
    border-radius: var(--radius);
    margin: 10px 12px 6px;
    gap: 8px;
    position: relative;
    overflow: hidden;
    animation: slideDown 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    transition: background 0.4s ease, box-shadow 0.3s ease;
  }

  .timer-display.running::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      transparent 0%,
      rgba(255, 255, 255, 0.06) 50%,
      transparent 100%
    );
    background-size: 200% 100%;
    animation: shimmer 3s ease-in-out infinite;
    pointer-events: none;
  }

  .timer-display.running {
    animation: slideDown 0.25s cubic-bezier(0.4, 0, 0.2, 1), timerGlow 2.5s ease-in-out infinite;
  }

  .timer-display.paused {
    background: var(--timer-gradient-paused);
  }

  .timer-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .timer-key {
    font-weight: 700;
    font-size: 12px;
    letter-spacing: 0.2px;
  }

  .timer-summary {
    font-size: 10.5px;
    opacity: 0.8;
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
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 15px;
    letter-spacing: 0.5px;
  }

  .timer-time.paused {
    animation: pulse 1.5s ease-in-out infinite;
  }

  .btn-icon {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.18);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
    backdrop-filter: blur(4px);
  }

  .btn-icon:hover {
    background: rgba(255, 255, 255, 0.3);
    transform: scale(1.08);
  }

  .btn-icon:active {
    transform: scale(0.95);
  }

  .btn-stop:hover {
    background: rgba(255, 59, 48, 0.75);
  }

  .btn-resume {
    background: rgba(48, 209, 88, 0.3);
  }

  .btn-resume:hover {
    background: rgba(48, 209, 88, 0.5);
  }
</style>
