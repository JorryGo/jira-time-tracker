import { checkForUpdate } from "../commands/updater";

class UpdaterStore {
  updateAvailable = $state(false);
  latestVersion = $state("");
  releaseUrl = $state("");
  checking = $state(false);

  async check(force = false) {
    if (this.checking) return;
    this.checking = true;
    try {
      const result = await checkForUpdate(force);
      this.updateAvailable = result.update_available;
      this.latestVersion = result.latest_version;
      this.releaseUrl = result.release_url;
    } catch (e) {
      console.error("Update check failed:", e);
    } finally {
      this.checking = false;
    }
  }
}

export const updaterStore = new UpdaterStore();
