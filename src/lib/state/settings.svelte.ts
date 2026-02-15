import { loadJiraConfig, saveJiraConfig, settingsGet, settingsSet } from "../commands/settings";
import { testConnection } from "../commands/jira";
import { timerUpdateTray } from "../commands/timer";

const DEFAULT_HIDDEN_STATUSES = ["Done", "Canceled"];

class SettingsStore {
  jiraBaseUrl = $state("");
  jiraEmail = $state("");
  jqlFilter = $state("assignee = currentUser() ORDER BY updated DESC");
  isConnected = $state(false);
  userName = $state("");
  hiddenStatuses = $state<string[]>(DEFAULT_HIDDEN_STATUSES);
  showTrayTitle = $state(false);
  theme = $state<"system" | "light" | "dark">("system");

  async init() {
    const config = await loadJiraConfig();
    if (config) {
      this.jiraBaseUrl = config["jira_base_url"] ?? "";
      this.jiraEmail = config["jira_email"] ?? "";
      this.isConnected = true;
    }
    const jql = await settingsGet("jql_filter");
    if (jql) this.jqlFilter = jql;

    const hiddenJson = await settingsGet("hidden_statuses");
    if (hiddenJson) {
      try {
        this.hiddenStatuses = JSON.parse(hiddenJson);
      } catch {
        // keep default
      }
    }

    const trayTitle = await settingsGet("show_tray_title");
    if (trayTitle !== null) this.showTrayTitle = trayTitle !== "false";

    const theme = await settingsGet("theme");
    if (theme === "light" || theme === "dark") this.theme = theme;
    this.applyTheme();
  }

  applyTheme() {
    if (this.theme === "system") {
      document.documentElement.removeAttribute("data-theme");
    } else {
      document.documentElement.setAttribute("data-theme", this.theme);
    }
  }

  async saveTheme(value: "system" | "light" | "dark") {
    this.theme = value;
    this.applyTheme();
    await settingsSet("theme", value);
  }

  async testAndSave(baseUrl: string, email: string, apiToken: string): Promise<string> {
    const user = await testConnection(baseUrl, email, apiToken);
    await saveJiraConfig(baseUrl, email, apiToken);
    this.jiraBaseUrl = baseUrl;
    this.jiraEmail = email;
    this.isConnected = true;
    this.userName = user.displayName;
    return user.displayName;
  }

  async saveJqlFilter(jql: string) {
    await settingsSet("jql_filter", jql);
    this.jqlFilter = jql;
  }

  async saveHiddenStatuses(statuses: string[]) {
    this.hiddenStatuses = statuses;
    await settingsSet("hidden_statuses", JSON.stringify(statuses));
  }

  async toggleTrayTitle(enabled: boolean) {
    this.showTrayTitle = enabled;
    await settingsSet("show_tray_title", String(enabled));
    if (!enabled) await timerUpdateTray("");
  }
}

export const settingsStore = new SettingsStore();
