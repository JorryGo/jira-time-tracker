export interface TimerState {
  issue_key: string;
  started_at: string;
  accumulated_secs: number;
  is_paused: boolean;
  paused_at: string | null;
  description: string;
}
