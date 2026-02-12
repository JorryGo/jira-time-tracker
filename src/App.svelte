<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import TasksView from "./views/TasksView.svelte";
  import WorklogsView from "./views/WorklogsView.svelte";
  import SettingsView from "./views/SettingsView.svelte";

  let activeTab = $state<"tasks" | "worklogs" | "settings">("tasks");
  let showQuitConfirm = $state(false);
</script>

<div class="app">
  <nav class="tabs">
    <button class="tab" class:active={activeTab === "tasks"} onclick={() => activeTab = "tasks"}>
      Tasks
    </button>
    <button class="tab" class:active={activeTab === "worklogs"} onclick={() => activeTab = "worklogs"}>
      Work Logs
    </button>
    <button class="tab" class:active={activeTab === "settings"} onclick={() => activeTab = "settings"}>
      Settings
    </button>
    <button class="quit-btn" onclick={() => (showQuitConfirm = true)} title="Quit">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
        <polyline points="16 17 21 12 16 7" />
        <line x1="21" y1="12" x2="9" y2="12" />
      </svg>
    </button>
  </nav>

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

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    padding: 0 8px;
    flex-shrink: 0;
  }

  .tab {
    padding: 10px 16px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
  }

  .tab:hover {
    color: var(--text);
  }

  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .quit-btn {
    margin-left: auto;
    padding: 8px;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
    transition: color 0.15s;
  }

  .quit-btn:hover {
    color: #e74c3c;
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .confirm-dialog {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.2);
    text-align: center;
  }

  .confirm-dialog p {
    margin-bottom: 12px;
    font-size: 13px;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: center;
  }

  .confirm-cancel,
  .confirm-quit {
    padding: 5px 16px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    transition: all 0.15s;
  }

  .confirm-cancel {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
  }

  .confirm-cancel:hover {
    background: var(--border);
  }

  .confirm-quit {
    background: var(--danger);
    color: white;
    border: 1px solid var(--danger);
  }

  .confirm-quit:hover {
    background: color-mix(in srgb, var(--danger) 85%, black);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }
</style>
