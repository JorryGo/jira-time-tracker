import { invoke } from "@tauri-apps/api/core";

export async function settingsGet(
  key: string,
): Promise<string | null> {
  return invoke("settings_get", { key });
}

export async function settingsSet(
  key: string,
  value: string,
): Promise<void> {
  return invoke("settings_set", { key, value });
}

export async function settingsGetAll(): Promise<Record<string, string>> {
  return invoke("settings_get_all");
}

export async function saveJiraConfig(
  baseUrl: string,
  email: string,
  apiToken: string,
): Promise<void> {
  return invoke("settings_save_jira_config", {
    baseUrl,
    email,
    apiToken,
  });
}

export async function loadJiraConfig(): Promise<Record<string, string> | null> {
  return invoke("settings_load_jira_config");
}
