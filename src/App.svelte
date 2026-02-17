<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import TasksView from "./views/TasksView.svelte";
  import WorklogsView from "./views/WorklogsView.svelte";
  import SettingsView from "./views/SettingsView.svelte";
  import CalendarView from "./views/CalendarView.svelte";
  import { timerStore } from "./lib/state/timer.svelte";
  import { worklogsStore } from "./lib/state/worklogs.svelte";
  import { updaterStore } from "./lib/state/updater.svelte";

  const isCalendarWindow = new URLSearchParams(window.location.search).get("view") === "calendar";

  const isLinux = navigator.userAgent.includes("Linux");

  let activeTab = $state<"tasks" | "worklogs" | "settings">("tasks");
  let showQuitConfirm = $state(false);

  function handleDrag(e: MouseEvent) {
    if ((e.target as HTMLElement).closest("button")) return;
    getCurrentWindow().startDragging();
  }

  onMount(() => {
    if (isCalendarWindow) return;
    updaterStore.check();
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        timerStore.flushPendingDescription();
        worklogsStore.flushPendingDescriptions();
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  });
</script>

{#if isCalendarWindow}
  <CalendarView />
{:else}
<div class="app">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="tabs" onmousedown={handleDrag}>
    <div class="tab-segment">
      <button class="tab" class:active={activeTab === "tasks"} onclick={() => activeTab = "tasks"}>
        Tasks
      </button>
      <button class="tab" class:active={activeTab === "worklogs"} onclick={() => activeTab = "worklogs"}>
        Work Logs
      </button>
      <button class="tab" class:active={activeTab === "settings"} onclick={() => activeTab = "settings"}>
        Settings
      </button>
    </div>
    {#if isLinux}
      <button class="quit-btn" onclick={() => getCurrentWindow().hide()} title="Hide window">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>
    {:else}
      <button class="quit-btn" onclick={() => (showQuitConfirm = true)} title="Quit">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
          <polyline points="16 17 21 12 16 7" />
          <line x1="21" y1="12" x2="9" y2="12" />
        </svg>
      </button>
    {/if}
  </div>

  <main class="content">
    {#if activeTab === "tasks"}
      <TasksView />
    {:else if activeTab === "worklogs"}
      <WorklogsView />
    {:else}
      <SettingsView />
    {/if}
  </main>
</div>

{#if showQuitConfirm}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-overlay" onmousedown={() => (showQuitConfirm = false)}>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="confirm-dialog" onmousedown={(e) => e.stopPropagation()}>
      <p>Quit app?</p>
      <div class="confirm-actions">
        <button class="confirm-cancel" onclick={() => (showQuitConfirm = false)}>Cancel</button>
        <button class="confirm-quit" onclick={() => invoke("quit_app")}>Quit</button>
      </div>
    </div>
  </div>
{/if}
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .tabs {
    display: flex;
    align-items: center;
    padding: 10px 10px 8px;
    flex-shrink: 0;
    gap: 8px;
    background: var(--bg-glass);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  }

  .tab-segment {
    display: flex;
    background: color-mix(in srgb, var(--text) 6%, transparent);
    border-radius: 8px;
    padding: 2px;
    gap: 1px;
  }

  .tab {
    padding: 5px 14px;
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-secondary);
    border-radius: 6px;
    transition: all var(--transition-normal);
  }

  .tab:hover {
    color: var(--text);
    background: color-mix(in srgb, var(--text) 4%, transparent);
  }

  .tab.active {
    color: var(--text);
    background: var(--bg);
    box-shadow: var(--shadow-sm);
    font-weight: 600;
  }

  .quit-btn {
    margin-left: auto;
    padding: 6px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
    transition: all var(--transition-fast);
  }

  .quit-btn:hover {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
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
    margin-bottom: 14px;
    font-size: 13px;
    font-weight: 500;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .confirm-cancel,
  .confirm-quit {
    padding: 6px 18px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    transition: all var(--transition-fast);
  }

  .confirm-cancel {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
  }

  .confirm-cancel:hover {
    background: var(--bg-tertiary);
  }

  .confirm-quit {
    background: var(--danger);
    color: white;
    border: 1px solid var(--danger);
  }

  .confirm-quit:hover {
    background: color-mix(in srgb, var(--danger) 85%, black);
    transform: translateY(-0.5px);
  }
</style>
