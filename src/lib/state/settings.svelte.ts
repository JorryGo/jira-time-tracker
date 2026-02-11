import { loadJiraConfig, saveJiraConfig, settingsGet, settingsSet } from "../commands/settings";
import { testConnection } from "../commands/jira";

const DEFAULT_HIDDEN_STATUSES = ["Done", "Canceled"];

class SettingsStore {
  jiraBaseUrl = $state("");
  jiraEmail = $state("");
  jqlFilter = $state("assignee = currentUser() ORDER BY updated DESC");
  isConnected = $state(false);
  userName = $state("");
  hiddenStatuses = $state<string[]>(DEFAULT_HIDDEN_STATUSES);

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
  }

  async testAndSave(baseUrl: string, email: string, apiToken: string): Promise<string> {
    const user = await testConnection(baseUrl, email, apiToken);
    await saveJiraConfig(baseUrl, email, apiToken);
    this.jiraBaseUrl = baseUrl;
    this.jiraEmail = email;
    this.isConnected = true;
    this.userName = user.display_name;
    return user.display_name;
  }

  async saveJqlFilter(jql: string) {
    await settingsSet("jql_filter", jql);
    this.jqlFilter = jql;
  }

  async saveHiddenStatuses(statuses: string[]) {
    this.hiddenStatuses = statuses;
    await settingsSet("hidden_statuses", JSON.stringify(statuses));
  }
}

export const settingsStore = new SettingsStore();
