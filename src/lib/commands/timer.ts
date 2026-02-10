import { invoke } from "@tauri-apps/api/core";
import type { TimerState } from "../types/settings";
import type { StoppedWorklog } from "../types/worklog";

export async function timerGetState(): Promise<TimerState | null> {
  return invoke("timer_get_state");
}

export async function timerStart(issueKey: string): Promise<TimerState> {
  return invoke("timer_start", { issueKey });
}

export async function timerPause(): Promise<TimerState> {
  return invoke("timer_pause");
}

export async function timerResume(): Promise<TimerState> {
  return invoke("timer_resume");
}

export async function timerStop(): Promise<StoppedWorklog> {
  return invoke("timer_stop");
}

export async function timerUpdateTray(displayText: string): Promise<void> {
  return invoke("timer_update_tray", { displayText });
}
