<script lang="ts">
  import { settingsStore } from "../lib/state/settings.svelte";
  import { tasksStore } from "../lib/state/tasks.svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";

  const isMac = navigator.platform.toUpperCase().includes("MAC");

  let baseUrl = $state(settingsStore.jiraBaseUrl);
  let email = $state(settingsStore.jiraEmail);
  let apiToken = $state("");
  let jqlFilter = $state(settingsStore.jqlFilter);
  let testing = $state(false);
  let saving = $state(false);
  let message = $state("");
  let messageType = $state<"success" | "error">("success");

  function showMessage(text: string, type: "success" | "error") {
    message = text;
    messageType = type;
    setTimeout(() => (message = ""), 4000);
  }

  async function handleTestAndSave() {
    if (!baseUrl || !email || !apiToken) {
      showMessage("Fill in all fields", "error");
      return;
    }
    testing = true;
    message = "";
    try {
      const userName = await settingsStore.testAndSave(
        baseUrl.trim(),
        email.trim(),
        apiToken.trim(),
      );
      showMessage(`Connected as ${userName}`, "success");
      apiToken = "";
    } catch (e) {
      showMessage(String(e), "error");
    } finally {
      testing = false;
    }
  }

  async function handleSaveJql() {
    saving = true;
    try {
      await settingsStore.saveJqlFilter(jqlFilter);
      showMessage("JQL filter saved", "success");
    } catch (e) {
      showMessage(String(e), "error");
    } finally {
      saving = false;
    }
  }

  function handleResetJql() {
    jqlFilter = "assignee = currentUser() ORDER BY updated DESC";
  }

  let newStatus = $state("");
  let newHiddenStatus = $state("");

  function moveStatus(index: number, direction: -1 | 1) {
    const arr = [...tasksStore.statusOrder];
    const target = index + direction;
    if (target < 0 || target >= arr.length) return;
    [arr[index], arr[target]] = [arr[target], arr[index]];
    tasksStore.saveStatusOrder(arr);
  }

  function removeStatus(index: number) {
    const arr = tasksStore.statusOrder.filter((_, i) => i !== index);
    tasksStore.saveStatusOrder(arr);
  }

  function addStatus() {
    const name = newStatus.trim();
    if (!name) return;
    if (tasksStore.statusOrder.some((s) => s.toLowerCase() === name.toLowerCase())) return;
    tasksStore.saveStatusOrder([...tasksStore.statusOrder, name]);
    newStatus = "";
  }

  function resetStatusOrder() {
    tasksStore.saveStatusOrder(["In Progress", "Reopened", "Ready for Develop"]);
  }

  function removeHiddenStatus(index: number) {
    const arr = settingsStore.hiddenStatuses.filter((_, i) => i !== index);
    settingsStore.saveHiddenStatuses(arr);
  }

  function addHiddenStatus() {
    const name = newHiddenStatus.trim();
    if (!name) return;
    if (settingsStore.hiddenStatuses.some((s) => s.toLowerCase() === name.toLowerCase())) return;
    settingsStore.saveHiddenStatuses([...settingsStore.hiddenStatuses, name]);
    newHiddenStatus = "";
  }

  function resetHiddenStatuses() {
    settingsStore.saveHiddenStatuses(["Done", "Canceled"]);
  }
</script>

<div class="settings-view">
  <section>
    <h3>Jira Connection</h3>
    {#if settingsStore.isConnected}
      <div class="connected-badge">
        <span class="connected-dot"></span>
        Connected to {settingsStore.jiraBaseUrl}
      </div>
    {/if}
    <div class="field">
      <label>Base URL
        <input type="text" bind:value={baseUrl} placeholder="https://yoursite.atlassian.net" />
      </label>
    </div>
    <div class="field">
      <label>Email
        <input type="email" bind:value={email} placeholder="you@company.com" />
      </label>
    </div>
    <div class="field">
      <label>API Token
        <input type="password" bind:value={apiToken} placeholder="Your Jira API token" />
      </label>
      <span class="field-hint">
        <button class="btn-link" onclick={() => openUrl("https://id.atlassian.com/manage-profile/security/api-tokens")}>
          Get token
        </button>
      </span>
    </div>
    <button class="btn btn-primary" onclick={handleTestAndSave} disabled={testing}>
      {testing ? "Testing..." : "Test & Save"}
    </button>
  </section>

  <section>
    <h3>JQL Filter</h3>
    <div class="field">
      <textarea rows="3" bind:value={jqlFilter}></textarea>
    </div>
    <div class="btn-row">
      <button class="btn btn-secondary" onclick={handleResetJql}>Reset to Default</button>
      <button class="btn btn-primary" onclick={handleSaveJql} disabled={saving}>
        {saving ? "Saving..." : "Save Filter"}
      </button>
    </div>
  </section>

  <section>
    <h3>Status Sort Order</h3>
    <p class="section-hint">Tasks are sorted by status in this order. Statuses not in the list appear at the bottom.</p>
    <div class="status-list">
      {#each tasksStore.statusOrder as status, i (status)}
        <div class="status-row">
          <span class="status-rank">{i + 1}.</span>
          <span class="status-name">{status}</span>
          <div class="status-actions">
            <button class="btn-sm" onclick={() => moveStatus(i, -1)} disabled={i === 0} title="Move up">&uarr;</button>
            <button class="btn-sm" onclick={() => moveStatus(i, 1)} disabled={i === tasksStore.statusOrder.length - 1} title="Move down">&darr;</button>
            <button class="btn-sm btn-danger-sm" onclick={() => removeStatus(i)} title="Remove">&times;</button>
          </div>
        </div>
      {/each}
    </div>
    <div class="add-status-row">
      <input
        type="text"
        class="add-status-input"
        placeholder="New status name..."
        bind:value={newStatus}
        onkeydown={(e) => e.key === "Enter" && addStatus()}
      />
      <button class="btn btn-secondary" onclick={addStatus}>Add</button>
    </div>
    <div class="btn-row">
      <button class="btn btn-secondary" onclick={resetStatusOrder}>Reset to Default</button>
    </div>
  </section>

  <section>
    <h3>Hidden Statuses</h3>
    <p class="section-hint">Tasks with these statuses will be hidden from the list.</p>
    <div class="status-list">
      {#each settingsStore.hiddenStatuses as status, i (status)}
        <div class="status-row">
          <span class="status-name">{status}</span>
          <div class="status-actions">
            <button class="btn-sm btn-danger-sm" onclick={() => removeHiddenStatus(i)} title="Remove">&times;</button>
          </div>
        </div>
      {/each}
    </div>
    <div class="add-status-row">
      <input
        type="text"
        class="add-status-input"
        placeholder="Status name..."
        bind:value={newHiddenStatus}
        onkeydown={(e) => e.key === "Enter" && addHiddenStatus()}
      />
      <button class="btn btn-secondary" onclick={addHiddenStatus}>Add</button>
    </div>
    <div class="btn-row">
      <button class="btn btn-secondary" onclick={resetHiddenStatuses}>Reset to Default</button>
    </div>
  </section>

  <section>
    <h3>Appearance</h3>
    <div class="theme-row">
      <button
        class="theme-option"
        class:active={settingsStore.theme === "system"}
        onclick={() => settingsStore.saveTheme("system")}
      >System</button>
      <button
        class="theme-option"
        class:active={settingsStore.theme === "light"}
        onclick={() => settingsStore.saveTheme("light")}
      >Light</button>
      <button
        class="theme-option"
        class:active={settingsStore.theme === "dark"}
        onclick={() => settingsStore.saveTheme("dark")}
      >Dark</button>
    </div>
    {#if isMac}
      <label class="checkbox-row">
        <input
          type="checkbox"
          checked={settingsStore.showTrayTitle}
          onchange={(e) => settingsStore.toggleTrayTitle((e.target as HTMLInputElement).checked)}
        />
        Show task in menu bar
      </label>
    {/if}
  </section>

  {#if message}
    <div class="message" class:error={messageType === "error"} class:success={messageType === "success"}>
      {message}
    </div>
  {/if}

  <section class="about">
    <p>Jira Time Tracker v0.1.2</p>
  </section>
</div>

<style>
  .settings-view {
    padding: 14px;
    overflow-y: auto;
    height: 100%;
  }

  section {
    margin-bottom: 18px;
    padding: 14px;
    background: var(--bg-secondary);
    border-radius: var(--radius);
    border: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
  }

  h3 {
    font-size: 13px;
    margin-bottom: 10px;
    padding-bottom: 6px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
    font-weight: 600;
  }

  .connected-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text);
    margin-bottom: 10px;
    padding: 6px 10px;
    background: color-mix(in srgb, var(--success) 8%, transparent);
    border-radius: var(--radius-sm);
    border: 1px solid color-mix(in srgb, var(--success) 15%, transparent);
  }

  .connected-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--success);
    animation: pulse 2s ease-in-out infinite;
    flex-shrink: 0;
  }

  .field {
    margin-bottom: 8px;
  }

  .field label {
    display: block;
    font-size: 10.5px;
    color: var(--text-secondary);
    margin-bottom: 4px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .field input,
  .field textarea {
    width: 100%;
    background: var(--bg);
  }

  .field-hint {
    font-size: 10px;
    margin-top: 2px;
    display: block;
  }

  .btn-link {
    color: var(--accent);
    text-decoration: none;
    font-size: inherit;
    padding: 0;
  }

  .btn-link:hover {
    text-decoration: underline;
  }

  .btn-row {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .btn {
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    transition: all var(--transition-fast);
  }

  .btn-primary {
    background: var(--accent);
    color: white;
  }

  .btn-primary:hover {
    background: var(--accent-hover);
    transform: translateY(-0.5px);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 25%, transparent);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    transform: none;
    box-shadow: none;
  }

  .btn-secondary {
    background: var(--bg);
    border: 1px solid var(--border);
  }

  .btn-secondary:hover {
    background: var(--bg-tertiary);
  }

  .message {
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    margin-bottom: 12px;
    animation: slideUp 0.2s ease;
  }

  .message.success {
    background: color-mix(in srgb, var(--success) 10%, transparent);
    color: var(--success);
  }

  .message.error {
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    color: var(--danger);
  }

  .about {
    text-align: center;
    color: var(--text-secondary);
    font-size: 11px;
    background: transparent;
    border: none;
    padding: 0;
  }

  .section-hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .status-list {
    margin-bottom: 8px;
  }

  .status-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 0;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 30%, transparent);
    transition: background var(--transition-fast);
  }

  .status-row:hover {
    background: color-mix(in srgb, var(--bg) 50%, transparent);
    border-radius: var(--radius-sm);
  }

  .status-rank {
    font-size: 11px;
    color: var(--text-secondary);
    width: 18px;
    flex-shrink: 0;
  }

  .status-name {
    flex: 1;
    font-size: 12px;
  }

  .status-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }

  .btn-sm {
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    background: var(--bg);
    border: 1px solid var(--border);
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    transition: all var(--transition-fast);
  }

  .btn-sm:hover:not(:disabled) {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    transform: scale(1.05);
  }

  .btn-sm:disabled {
    opacity: 0.3;
  }

  .btn-danger-sm:hover:not(:disabled) {
    background: var(--danger);
    border-color: var(--danger);
  }

  .add-status-row {
    display: flex;
    gap: 6px;
    margin-bottom: 8px;
  }

  .add-status-input {
    flex: 1;
    font-size: 12px;
    padding: 4px 8px;
    background: var(--bg);
  }

  .theme-row {
    display: flex;
    background: color-mix(in srgb, var(--text) 6%, transparent);
    border-radius: 8px;
    padding: 2px;
    gap: 2px;
    margin-bottom: 10px;
  }

  .theme-option {
    flex: 1;
    padding: 6px 0;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .theme-option:hover {
    color: var(--text);
  }

  .theme-option.active {
    background: var(--bg);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    cursor: pointer;
  }

  .checkbox-row input[type="checkbox"] {
    width: auto;
    margin: 0;
    cursor: pointer;
  }
</style>
