import { invoke } from "@tauri-apps/api/core";

export interface UpdateCheckResult {
  update_available: boolean;
  latest_version: string;
  current_version: string;
  release_url: string;
  release_notes: string;
}

export async function checkForUpdate(
  force: boolean = false,
): Promise<UpdateCheckResult> {
  return invoke("check_for_update", { force });
}
