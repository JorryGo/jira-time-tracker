<script lang="ts">
  import TasksView from "./views/TasksView.svelte";
  import WorklogsView from "./views/WorklogsView.svelte";
  import SettingsView from "./views/SettingsView.svelte";

  let activeTab = $state<"tasks" | "worklogs" | "settings">("tasks");
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

  .content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }
</style>
