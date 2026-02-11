import * as cmd from "../commands/timer";
import { formatDuration } from "../utils/format";
import type { TimerState } from "../types/settings";

class TimerStore {
  current = $state<TimerState | null>(null);
  issueSummary = $state("");
  elapsedSeconds = $state(0);
  private intervalId: number | null = null;

  get isRunning(): boolean {
    return this.current !== null && !this.current.is_paused;
  }

  get isPaused(): boolean {
    return this.current !== null && this.current.is_paused;
  }

  get displayText(): string {
    if (!this.current) return "";
    return `${this.current.issue_key} ${formatDuration(this.elapsedSeconds)}`;
  }

  async init() {
    const state = await cmd.timerGetState();
    if (state) {
      this.current = state;
      this.recalcElapsed();
      if (state.is_paused) {
        cmd.timerSetTrayIcon("paused");
      } else {
        cmd.timerSetTrayIcon("working");
        this.startTicking();
      }
    }
  }

  async start(issueKey: string, summary: string) {
    if (this.current) {
      await this.stop();
    }
    this.current = await cmd.timerStart(issueKey);
    this.issueSummary = summary;
    this.elapsedSeconds = 0;
    this.startTicking();
    cmd.timerSetTrayIcon("working");
  }

  async pause() {
    this.current = await cmd.timerPause();
    this.stopTicking();
    this.recalcElapsed();
    await cmd.timerUpdateTray(`${this.current.issue_key} ${formatDuration(this.elapsedSeconds)} ⏸`);
    cmd.timerSetTrayIcon("paused");
  }

  async resume() {
    this.current = await cmd.timerResume();
    this.startTicking();
    cmd.timerSetTrayIcon("working");
  }

  async stop() {
    const result = await cmd.timerStop();
    this.current = null;
    this.issueSummary = "";
    this.elapsedSeconds = 0;
    this.stopTicking();
    await cmd.timerUpdateTray("");
    cmd.timerSetTrayIcon("idle");
    return result;
  }

  private startTicking() {
    this.stopTicking();
    this.intervalId = window.setInterval(() => {
      this.recalcElapsed();
      if (this.current) {
        cmd.timerUpdateTray(this.displayText);
      }
    }, 1000);
  }

  private stopTicking() {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
  }

  private recalcElapsed() {
    if (!this.current) return;
    if (this.current.is_paused) {
      // accumulated_secs already includes all elapsed time up to pause
      this.elapsedSeconds = this.current.accumulated_secs;
    } else {
      const nowMs =
        Date.now() - new Date(this.current.started_at).getTime();
      this.elapsedSeconds =
        this.current.accumulated_secs + Math.floor(nowMs / 1000);
    }
  }
}

export const timerStore = new TimerStore();
