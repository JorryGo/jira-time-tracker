<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getWorklogs } from "../lib/commands/worklogs";
  import { formatDurationShort } from "../lib/utils/format";
  import { settingsStore } from "../lib/state/settings.svelte";
  import WorklogEditModal from "../components/WorklogEditModal.svelte";
  import AddWorklogModal from "../components/AddWorklogModal.svelte";
  import { importWorklogs, pushAllPending } from "../lib/commands/jira";
  import type { Worklog, WorklogFilter, ImportSummary, PushSummary } from "../lib/types/worklog";

  function toLocalDateStr(date: Date): string {
    const y = date.getFullYear();
    const m = String(date.getMonth() + 1).padStart(2, "0");
    const d = String(date.getDate()).padStart(2, "0");
    return `${y}-${m}-${d}`;
  }

  function getMonday(d: Date): Date {
    const date = new Date(d);
    const day = date.getDay();
    const diff = date.getDate() - day + (day === 0 ? -6 : 1);
    date.setDate(diff);
    return date;
  }

  function getSunday(d: Date): Date {
    const date = new Date(d);
    const day = date.getDay();
    const diff = date.getDate() + (day === 0 ? 0 : 7 - day);
    date.setDate(diff);
    return date;
  }

  function getFirstOfMonth(d: Date): Date {
    return new Date(d.getFullYear(), d.getMonth(), 1);
  }

  function getLastOfMonth(d: Date): Date {
    return new Date(d.getFullYear(), d.getMonth() + 1, 0);
  }

  function getDaysInRange(start: string, end: string): string[] {
    const result: string[] = [];
    const d = new Date(start + "T12:00:00");
    const endD = new Date(end + "T12:00:00");
    while (d <= endD) {
      result.push(toLocalDateStr(d));
      d.setDate(d.getDate() + 1);
    }
    return result;
  }

  // State
  let viewMode = $state<"week" | "month">("week");
  let startDate = $state(toLocalDateStr(getMonday(new Date())));
  let endDate = $state(toLocalDateStr(getSunday(new Date())));
  let issueFilter = $state<string[]>([]);
  let statusFilter = $state("all");
  let worklogs = $state<Worklog[]>([]);
  let loading = $state(false);
  let hoveredWorklog = $state<Worklog | null>(null);
  let tooltipPos = $state({ x: 0, y: 0 });
  let issueSearchQuery = $state("");
  let showIssueDropdown = $state(false);
  let error = $state("");
  let syncing = $state(false);
  let pushing = $state(false);
  let toast = $state("");
  let toastTimeout: ReturnType<typeof setTimeout> | undefined;
  const todayStr = toLocalDateStr(new Date());

  // Derived
  let availableIssues = $derived(
    [...new Set(worklogs.map(w => w.issue_key))].sort()
  );

  let filteredIssueOptions = $derived(
    availableIssues.filter(k =>
      !issueFilter.includes(k) &&
      k.toLowerCase().includes(issueSearchQuery.toLowerCase())
    )
  );

  let filteredWorklogs = $derived(
    worklogs.filter(w => {
      if (issueFilter.length > 0 && !issueFilter.includes(w.issue_key)) return false;
      if (statusFilter !== "all" && w.sync_status !== statusFilter) return false;
      return true;
    })
  );

  let pendingCount = $derived(
    worklogs.filter(w => w.sync_status === "pending").length
  );

  let days = $derived(getDaysInRange(startDate, endDate));

  let worklogsByDay = $derived.by(() => {
    const map = new Map<string, Worklog[]>();
    for (const day of days) map.set(day, []);
    for (const wl of filteredWorklogs) {
      const d = new Date(wl.started_at);
      const dayStr = toLocalDateStr(d);
      const list = map.get(dayStr);
      if (list) list.push(wl);
    }
    return map;
  });

  let timeRange = $derived.by(() => {
    let min = 9, max = 19;
    for (const wl of filteredWorklogs) {
      const start = new Date(wl.started_at);
      const end = new Date(start.getTime() + wl.duration_seconds * 1000);
      const startH = start.getHours() + start.getMinutes() / 60;
      // If worklog crosses midnight, clamp end to 24 (end of day)
      const crossesMidnight = toLocalDateStr(end) !== toLocalDateStr(start);
      const endH = crossesMidnight ? 24 : end.getHours() + end.getMinutes() / 60;
      if (startH < min) min = Math.floor(startH);
      if (endH > max) max = Math.ceil(endH);
    }
    return { startHour: min, endHour: Math.min(max, 24) };
  });

  let totalHours = $derived(timeRange.endHour - timeRange.startHour);

  let editingWorklog = $state<Worklog | null>(null);
  let addingWorklog = $state<{ date: string; time: string } | null>(null);

  function handleTrackClick(day: string, e: MouseEvent) {
    const track = (e.currentTarget as HTMLElement);
    const rect = track.getBoundingClientRect();
    const xRatio = (e.clientX - rect.left) / rect.width;
    const totalMinutes = xRatio * totalHours * 60;
    const clickHour = timeRange.startHour + Math.floor(totalMinutes / 60);
    const clickMin = Math.floor(totalMinutes % 60 / 15) * 15;
    const timeStr = `${String(clickHour).padStart(2, "0")}:${String(clickMin).padStart(2, "0")}`;
    addingWorklog = { date: day, time: timeStr };
  }

  // Lane assignment for overlapping worklogs
  function assignLanes(wls: Worklog[]): { lanes: Map<number, number>; laneCount: number } {
    const sorted = [...wls].sort((a, b) => new Date(a.started_at).getTime() - new Date(b.started_at).getTime());
    const lanes = new Map<number, number>();
    const laneEnds: number[] = []; // end timestamp per lane
    for (const wl of sorted) {
      const start = new Date(wl.started_at).getTime();
      let placed = false;
      for (let i = 0; i < laneEnds.length; i++) {
        if (start >= laneEnds[i]) {
          lanes.set(wl.id, i);
          laneEnds[i] = start + wl.duration_seconds * 1000;
          placed = true;
          break;
        }
      }
      if (!placed) {
        lanes.set(wl.id, laneEnds.length);
        laneEnds.push(start + wl.duration_seconds * 1000);
      }
    }
    return { lanes, laneCount: Math.max(laneEnds.length, 1) };
  }

  let worklogLanes = $derived.by(() => {
    const result = new Map<string, { lanes: Map<number, number>; laneCount: number }>();
    for (const [day, wls] of worklogsByDay) {
      result.set(day, assignLanes(wls));
    }
    return result;
  });

  // Color from issue key hash
  function issueHue(key: string): number {
    let hash = 0;
    for (let i = 0; i < key.length; i++) {
      hash = key.charCodeAt(i) + ((hash << 5) - hash);
    }
    return Math.abs(hash) % 360;
  }

  const LANE_HEIGHT = 28;
  const LANE_GAP = 4;
  const LANE_PAD = 6;

  function blockStyle(wl: Worklog, lane: number, laneCount: number): string {
    const start = new Date(wl.started_at);
    const startMinutes = (start.getHours() - timeRange.startHour) * 60 + start.getMinutes();
    const totalMinutes = totalHours * 60;
    const left = (startMinutes / totalMinutes) * 100;
    // Clamp duration to end of day (midnight) if worklog crosses midnight
    const maxDurationSeconds = (24 - start.getHours()) * 3600 - start.getMinutes() * 60 - start.getSeconds();
    const clampedDuration = Math.min(wl.duration_seconds, maxDurationSeconds);
    const width = (clampedDuration / 60 / totalMinutes) * 100;
    const hue = issueHue(wl.issue_key);
    const top = LANE_PAD + lane * (LANE_HEIGHT + LANE_GAP);
    return `left: ${left}%; width: ${Math.max(width, 0.5)}%; top: ${top}px; height: ${LANE_HEIGHT}px; --block-hue: ${hue};`;
  }

  function trackHeight(laneCount: number): number {
    return LANE_PAD * 2 + laneCount * LANE_HEIGHT + (laneCount - 1) * LANE_GAP;
  }

  function formatDayLabel(dateStr: string): string {
    const d = new Date(dateStr + "T12:00:00");
    const dayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const day = d.getDate();
    const month = d.toLocaleString(undefined, { month: "short" });
    return `${dayNames[d.getDay()]} ${day} ${month}`;
  }

  function formatRangeLabel(dateStr: string): string {
    const d = new Date(dateStr + "T12:00:00");
    const day = d.getDate();
    const month = d.toLocaleString(undefined, { month: "short" });
    return `${day} ${month}`;
  }

  function dayTotalSeconds(dateStr: string): number {
    return (worklogsByDay.get(dateStr) ?? []).reduce((s, w) => s + w.duration_seconds, 0);
  }

  function formatTime(d: Date): string {
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }

  function endTimeDate(wl: Worklog): Date {
    return new Date(new Date(wl.started_at).getTime() + wl.duration_seconds * 1000);
  }

  async function loadWorklogs() {
    loading = true;
    error = "";
    try {
      const dateFrom = new Date(startDate + "T00:00:00");
      const dateTo = new Date(endDate + "T00:00:00");
      dateTo.setDate(dateTo.getDate() + 1);
      const filter: WorklogFilter = {
        date_from: dateFrom.toISOString(),
        date_to: dateTo.toISOString(),
      };
      worklogs = await getWorklogs(filter);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function showToast(message: string, durationMs = 5000) {
    if (toastTimeout) clearTimeout(toastTimeout);
    toast = message;
    toastTimeout = setTimeout(() => { toast = ""; }, durationMs);
  }

  async function handleSync() {
    syncing = true;
    const allDays = getDaysInRange(startDate, endDate);
    const totals = { imported: 0, updated: 0, deleted: 0 };
    const errors: string[] = [];

    for (const day of allDays) {
      try {
        const result = await importWorklogs(day);
        totals.imported += result.imported;
        totals.updated += result.updated;
        totals.deleted += result.deleted;
      } catch (e) {
        errors.push(`${day}: ${String(e)}`);
      }
    }

    await loadWorklogs();
    syncing = false;

    let msg: string;
    if (totals.imported === 0 && totals.updated === 0 && totals.deleted === 0 && errors.length === 0) {
      msg = "Already up to date";
    } else {
      msg = `Imported: ${totals.imported}, Updated: ${totals.updated}, Deleted: ${totals.deleted}`;
      if (errors.length > 0) {
        msg += ` (${errors.length} day${errors.length > 1 ? "s" : ""} failed)`;
      }
    }
    showToast(msg);
  }

  async function handlePushAll() {
    pushing = true;
    const allDays = getDaysInRange(startDate, endDate);
    const totals = { total: 0, success: 0, failed: 0 };
    const errors: string[] = [];

    for (const day of allDays) {
      try {
        const result = await pushAllPending(day);
        totals.total += result.total;
        totals.success += result.success;
        totals.failed += result.failed;
        errors.push(...result.errors);
      } catch (e) {
        errors.push(`${day}: ${String(e)}`);
      }
    }

    await loadWorklogs();
    pushing = false;

    let msg = `Pushed ${totals.success}/${totals.total}`;
    if (totals.failed > 0) {
      msg += `, ${totals.failed} failed`;
    }
    showToast(msg);
  }

  async function shiftPeriod(offset: number) {
    if (viewMode === "month") {
      const s = new Date(startDate + "T12:00:00");
      s.setMonth(s.getMonth() + offset, 1);
      startDate = toLocalDateStr(getFirstOfMonth(s));
      endDate = toLocalDateStr(getLastOfMonth(s));
    } else {
      const s = new Date(startDate + "T12:00:00");
      s.setDate(s.getDate() + offset * 7);
      startDate = toLocalDateStr(s);
      const e = new Date(endDate + "T12:00:00");
      e.setDate(e.getDate() + offset * 7);
      endDate = toLocalDateStr(e);
    }
    await loadWorklogs();
  }

  async function goThisWeek() {
    viewMode = "week";
    startDate = toLocalDateStr(getMonday(new Date()));
    endDate = toLocalDateStr(getSunday(new Date()));
    await loadWorklogs();
  }

  async function goThisMonth() {
    viewMode = "month";
    startDate = toLocalDateStr(getFirstOfMonth(new Date()));
    endDate = toLocalDateStr(getLastOfMonth(new Date()));
    await loadWorklogs();
  }

  function addIssueFilter(key: string) {
    issueFilter = [...issueFilter, key];
    issueSearchQuery = "";
    showIssueDropdown = false;
  }

  function removeIssueFilter(key: string) {
    issueFilter = issueFilter.filter(k => k !== key);
  }

  function clampTooltip(x: number, y: number): { x: number; y: number } {
    const pad = 12;
    const tooltipW = 280;
    const tooltipH = 100;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let tx = x + pad;
    let ty = y - 10;
    if (tx + tooltipW > vw) tx = x - tooltipW - pad;
    if (ty + tooltipH > vh) ty = vh - tooltipH - pad;
    if (ty < pad) ty = pad;
    return { x: tx, y: ty };
  }

  function handleBlockHover(wl: Worklog, e: MouseEvent) {
    hoveredWorklog = wl;
    tooltipPos = clampTooltip(e.clientX, e.clientY);
  }

  function handleBlockLeave() {
    hoveredWorklog = null;
  }

  function isWeekend(dateStr: string): boolean {
    const d = new Date(dateStr + "T12:00:00");
    return d.getDay() === 0 || d.getDay() === 6;
  }

  function handleWindowFocus() {
    if (!syncing && !pushing) loadWorklogs();
  }

  onMount(async () => {
    await settingsStore.init();
    await loadWorklogs();
    window.addEventListener("focus", handleWindowFocus);
  });

  onDestroy(() => {
    window.removeEventListener("focus", handleWindowFocus);
    if (toastTimeout) clearTimeout(toastTimeout);
  });
</script>

<div class="calendar-view">
  <div class="cal-header">
    <div class="cal-nav">
      <button class="cal-btn" onclick={() => shiftPeriod(-1)} title={viewMode === "month" ? "Previous month" : "Previous week"}>&#8592;</button>
      <span class="cal-range">{formatRangeLabel(startDate)} &mdash; {formatRangeLabel(endDate)}</span>
      <button class="cal-btn" onclick={() => shiftPeriod(1)} title={viewMode === "month" ? "Next month" : "Next week"}>&#8594;</button>
      <button class="cal-btn cal-today-btn" class:cal-today-btn-active={viewMode === "week"} onclick={goThisWeek}>This Week</button>
      <button class="cal-btn cal-today-btn" class:cal-today-btn-active={viewMode === "month"} onclick={goThisMonth}>This Month</button>
      <div class="cal-nav-spacer"></div>
      <span class="cal-sync-wrap">
        <button
          class="cal-btn cal-sync-btn"
          onclick={handleSync}
          disabled={syncing || pushing}
          aria-label="Sync worklogs from Jira"
        >
          <svg class="sync-icon" class:spinning={syncing} xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21.5 2v6h-6"/>
            <path d="M2.5 22v-6h6"/>
            <path d="M2.5 11.5a10 10 0 0 1 17.3-5.7L21.5 8"/>
            <path d="M21.5 12.5a10 10 0 0 1-17.3 5.7L2.5 16"/>
          </svg>
        </button>
        <span class="cal-sync-tip">Sync worklogs from Jira for the displayed period. Does not push pending local worklogs.</span>
      </span>
      {#if pendingCount > 0}
        <button
          class="cal-btn cal-push-btn"
          onclick={handlePushAll}
          disabled={pushing || syncing}
        >
          {pushing ? "Pushing..." : `Push ${pendingCount} pending`}
        </button>
      {/if}
    </div>
    <div class="cal-filters">
      <div class="cal-filter-group filter-issues">
        <label for="cal-issue-filter">Tasks</label>
        <div class="issue-filter-box">
          {#each issueFilter as key}
            <span class="issue-tag" style="--tag-hue: {issueHue(key)}">
              {key}
              <button class="tag-remove" onclick={() => removeIssueFilter(key)}>&times;</button>
            </span>
          {/each}
          <input
            id="cal-issue-filter"
            type="text"
            class="issue-filter-input"
            placeholder={issueFilter.length ? "" : "Filter issues..."}
            bind:value={issueSearchQuery}
            onfocus={() => (showIssueDropdown = true)}
            onblur={() => setTimeout(() => (showIssueDropdown = false), 200)}
          />
          {#if showIssueDropdown && filteredIssueOptions.length > 0}
            <div class="issue-dropdown">
              {#each filteredIssueOptions as key}
                <button class="issue-option" onmousedown={() => addIssueFilter(key)}>
                  <span class="issue-option-dot" style="background: hsl({issueHue(key)}, 60%, 50%)"></span>
                  {key}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
      <div class="cal-filter-group">
        <label for="cal-status-filter">Status</label>
        <select id="cal-status-filter" bind:value={statusFilter}>
          <option value="all">All</option>
          <option value="pending">Pending</option>
          <option value="synced">Synced</option>
          <option value="error">Error</option>
        </select>
      </div>
      <div class="cal-filter-group">
        <label for="cal-date-from">From</label>
        <input id="cal-date-from" type="date" bind:value={startDate} onchange={() => loadWorklogs()} />
      </div>
      <div class="cal-filter-group">
        <label for="cal-date-to">To</label>
        <input id="cal-date-to" type="date" bind:value={endDate} onchange={() => loadWorklogs()} />
      </div>
    </div>
  </div>

  {#if error}
    <div class="cal-error">{error}</div>
  {/if}

  {#if toast}
    <div class="cal-toast">{toast}</div>
  {/if}

  <div class="cal-timeline">
    <div class="cal-row cal-hour-header">
      <div class="cal-day-label"></div>
      <div class="cal-track">
        {#each Array(totalHours + 1) as _, i}
          <span class="hour-mark" style="left: {(i / totalHours) * 100}%">
            {String(timeRange.startHour + i).padStart(2, "0")}
          </span>
        {/each}
      </div>
      <div class="cal-day-total"></div>
    </div>

    {#each days as day}
      {@const dayWls = worklogsByDay.get(day) ?? []}
      {@const dayLanes = worklogLanes.get(day) ?? { lanes: new Map(), laneCount: 1 }}
      {@const today = day === todayStr}
      {@const weekend = isWeekend(day)}
      <div class="cal-row" class:cal-row-today={today} class:cal-row-weekend={weekend}>
        <div class="cal-day-label" class:today-label={today}>
          {formatDayLabel(day)}
        </div>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="cal-track" style="min-height: {trackHeight(dayLanes.laneCount)}px" onclick={(e) => { if (e.target === e.currentTarget) handleTrackClick(day, e); }}>
          {#each Array(totalHours + 1) as _, i}
            <div class="grid-line" style="left: {(i / totalHours) * 100}%"></div>
          {/each}
          {#each dayWls as wl}
            <button
              class="wl-block"
              style={blockStyle(wl, dayLanes.lanes.get(wl.id) ?? 0, dayLanes.laneCount)}
              onmouseenter={(e) => handleBlockHover(wl, e)}
              onmousemove={(e) => { tooltipPos = clampTooltip(e.clientX, e.clientY); }}
              onmouseleave={handleBlockLeave}
              onclick={() => { editingWorklog = wl; }}
            >
              <span class="block-key">{wl.issue_key}</span>{#if wl.issue_summary}<span class="block-summary">{wl.issue_summary}</span>{/if}
            </button>
          {/each}
        </div>
        <div class="cal-day-total">
          {#if dayTotalSeconds(day) > 0}
            {formatDurationShort(dayTotalSeconds(day))}
          {/if}
        </div>
      </div>
    {/each}
  </div>

  {#if hoveredWorklog}
    <div class="cal-tooltip" style="left: {tooltipPos.x}px; top: {tooltipPos.y}px; --tt-hue: {issueHue(hoveredWorklog.issue_key)}">
      <div class="tt-key">
        {hoveredWorklog.issue_key}
      </div>
      {#if hoveredWorklog.issue_summary}
        <div class="tt-summary">{hoveredWorklog.issue_summary}</div>
      {/if}
      <div class="tt-time">
        {formatTime(new Date(hoveredWorklog.started_at))} &ndash; {formatTime(endTimeDate(hoveredWorklog))}
        &nbsp;&middot;&nbsp;
        {formatDurationShort(hoveredWorklog.duration_seconds)}
      </div>
      {#if hoveredWorklog.description}
        <div class="tt-desc">{hoveredWorklog.description}</div>
      {/if}
    </div>
  {/if}

  {#if loading}
    <div class="cal-loading">Loading...</div>
  {/if}

  {#if editingWorklog}
    <WorklogEditModal
      worklog={editingWorklog}
      onClose={async () => {
        editingWorklog = null;
        await loadWorklogs();
      }}
    />
  {/if}

  {#if addingWorklog}
    <AddWorklogModal
      selectedDate={addingWorklog.date}
      selectedTime={addingWorklog.time}
      onClose={async () => {
        addingWorklog = null;
        await loadWorklogs();
      }}
    />
  {/if}
</div>

<style>
  .calendar-view {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg);
    color: var(--text);
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'Helvetica Neue', sans-serif;
  }

  /* Header */
  .cal-header {
    padding: 14px 20px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
    flex-shrink: 0;
    background: var(--bg-glass);
    backdrop-filter: blur(16px) saturate(180%);
    -webkit-backdrop-filter: blur(16px) saturate(180%);
  }

  .cal-nav {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }

  .cal-range {
    font-size: 15px;
    font-weight: 700;
    min-width: 120px;
    text-align: center;
    letter-spacing: -0.2px;
  }

  .cal-btn {
    padding: 5px 11px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: var(--text);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .cal-btn:hover {
    background: var(--bg-tertiary);
    transform: translateY(-0.5px);
  }

  .cal-today-btn {
    margin-left: 4px;
    color: var(--accent);
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  .cal-today-btn:hover {
    background: color-mix(in srgb, var(--accent) 16%, transparent);
  }

  .cal-today-btn-active {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }

  .cal-filters {
    display: flex;
    align-items: flex-end;
    gap: 14px;
    flex-wrap: wrap;
  }

  .cal-filter-group {
    display: flex;
    flex-direction: column;
    gap: 3px;
    position: relative;
  }

  .cal-filter-group label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .cal-filter-group select,
  .cal-filter-group input[type="date"] {
    padding: 3px 8px;
    font-size: 11px;
    height: 26px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    color: var(--text);
  }

  .filter-issues {
    flex: 1;
    min-width: 180px;
  }

  /* Issue filter */
  .issue-filter-box {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    padding: 3px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-secondary);
    position: relative;
    min-height: 26px;
  }

  .issue-tag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    padding: 1px 7px;
    border-radius: 10px;
    font-size: 10px;
    font-weight: 600;
    background: hsl(var(--tag-hue), 55%, 92%);
    color: hsl(var(--tag-hue), 55%, 35%);
    transition: all var(--transition-fast);
  }

  .issue-tag:hover {
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.1);
  }

  @media (prefers-color-scheme: dark) {
    .issue-tag {
      background: hsl(var(--tag-hue), 40%, 22%);
      color: hsl(var(--tag-hue), 55%, 75%);
    }
  }

  .tag-remove {
    font-size: 12px;
    cursor: pointer;
    opacity: 0.5;
    background: none;
    border: none;
    padding: 0;
    color: inherit;
    line-height: 1;
  }

  .tag-remove:hover {
    opacity: 1;
  }

  .issue-filter-input {
    border: none !important;
    background: transparent !important;
    outline: none !important;
    box-shadow: none !important;
    padding: 2px 4px !important;
    font-size: 11px;
    flex: 1;
    min-width: 80px;
    color: var(--text);
  }

  .issue-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 2px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    max-height: 160px;
    overflow-y: auto;
    z-index: 20;
    box-shadow: var(--shadow-lg);
    animation: slideUp 0.15s ease;
  }

  .issue-option {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    text-align: left;
    padding: 6px 10px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text);
    background: none;
    border: none;
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .issue-option:hover {
    background: color-mix(in srgb, var(--accent) 6%, transparent);
  }

  .issue-option-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  /* Timeline */
  .cal-timeline {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .cal-row {
    display: flex;
    align-items: stretch;
    min-height: 44px;
    border-bottom: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
    padding: 0 20px;
    transition: background var(--transition-fast);
  }

  .cal-row-today {
    background: color-mix(in srgb, var(--accent) 4%, transparent);
    border-left: 3px solid var(--accent);
  }

  .cal-row-weekend {
    background: color-mix(in srgb, var(--bg-secondary) 30%, transparent);
  }

  .cal-row-today.cal-row-weekend {
    background: color-mix(in srgb, var(--accent) 4%, transparent);
  }

  .cal-hour-header {
    position: sticky;
    top: 0;
    background: var(--bg-glass);
    backdrop-filter: blur(12px) saturate(150%);
    -webkit-backdrop-filter: blur(12px) saturate(150%);
    z-index: 5;
    min-height: 28px;
    border-bottom: 1px solid var(--border);
  }

  .cal-day-label {
    width: 100px;
    flex-shrink: 0;
    padding: 10px 12px 10px 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    display: flex;
    align-items: center;
  }

  .today-label {
    color: var(--accent);
    font-weight: 700;
  }

  .cal-track {
    flex: 1;
    position: relative;
    min-height: 40px;
    cursor: cell;
  }

  .cal-day-total {
    width: 56px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding-right: 2px;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }

  .hour-mark {
    position: absolute;
    bottom: 4px;
    font-size: 9px;
    color: var(--text-secondary);
    transform: translateX(-50%);
    white-space: nowrap;
    opacity: 0.7;
  }

  .grid-line {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 1px;
    background: var(--border);
    opacity: 0.2;
  }

  /* Worklog blocks */
  .wl-block {
    position: absolute;
    border: none;
    border-radius: 5px;
    cursor: pointer;
    overflow: hidden;
    display: flex;
    align-items: center;
    padding: 0 6px;
    background: hsl(var(--block-hue), 55%, 52%);
    color: white;
    transition: all 0.1s ease;
    z-index: 1;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12), inset 0 1px 0 rgba(255, 255, 255, 0.1);
    font: inherit;
    text-align: left;
  }

  .wl-block:hover {
    filter: brightness(1.1);
    box-shadow: 0 3px 12px rgba(0, 0, 0, 0.2), inset 0 1px 0 rgba(255, 255, 255, 0.15);
    z-index: 3;
    transform: translateY(-1px);
  }

  @media (prefers-color-scheme: dark) {
    .wl-block {
      background: hsl(var(--block-hue), 48%, 40%);
      box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25), inset 0 1px 0 rgba(255, 255, 255, 0.05);
    }
  }

  .block-key {
    font-size: 9px;
    font-weight: 700;
    white-space: nowrap;
    flex-shrink: 0;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  .block-summary {
    font-size: 9px;
    font-weight: 400;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0.8;
    margin-left: 4px;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }

  /* Tooltip */
  .cal-tooltip {
    position: fixed;
    background: var(--bg-glass);
    backdrop-filter: blur(20px) saturate(180%);
    -webkit-backdrop-filter: blur(20px) saturate(180%);
    border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
    border-radius: var(--radius);
    padding: 10px 14px;
    box-shadow: var(--shadow-lg);
    z-index: 100;
    pointer-events: none;
    max-width: 280px;
    animation: fadeIn 0.1s ease;
  }

  .tt-key {
    font-weight: 700;
    font-size: 13px;
    margin-bottom: 3px;
    color: hsl(var(--tt-hue), 60%, 40%);
  }

  @media (prefers-color-scheme: dark) {
    .tt-key {
      color: hsl(var(--tt-hue), 60%, 70%);
    }
  }

  .tt-summary {
    font-size: 11px;
    color: var(--text);
    margin-bottom: 4px;
    line-height: 1.3;
    overflow-wrap: break-word;
    word-break: break-word;
  }

  .tt-time {
    font-size: 11px;
    color: var(--text-secondary);
    font-weight: 500;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .tt-desc {
    font-size: 10px;
    color: var(--text-secondary);
    font-style: italic;
    margin-top: 4px;
    line-height: 1.3;
    overflow-wrap: break-word;
    word-break: break-word;
  }

  .cal-error {
    padding: 8px 20px;
    font-size: 11px;
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 8%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--danger) 15%, transparent);
    flex-shrink: 0;
    animation: slideDown 0.2s ease;
  }

  /* Sync & Push buttons */
  .cal-nav-spacer {
    flex: 1;
  }

  .cal-sync-wrap {
    position: relative;
    display: inline-flex;
  }

  .cal-sync-tip {
    display: none;
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    width: 220px;
    padding: 8px 12px;
    font-size: 11px;
    line-height: 1.4;
    color: var(--text);
    background: var(--bg-glass);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-md);
    z-index: 30;
    pointer-events: none;
  }

  .cal-sync-wrap:hover .cal-sync-tip {
    display: block;
  }

  .cal-sync-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px 8px;
  }

  .cal-sync-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .sync-icon {
    display: block;
  }

  .sync-icon.spinning {
    animation: spin 0.8s linear infinite;
  }

  .cal-push-btn {
    background: var(--success);
    color: white;
    border-color: var(--success);
    font-weight: 600;
  }

  .cal-push-btn:hover {
    background: color-mix(in srgb, var(--success) 85%, white);
    transform: translateY(-0.5px);
    box-shadow: 0 2px 8px color-mix(in srgb, var(--success) 25%, transparent);
  }

  .cal-push-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .cal-toast {
    padding: 8px 20px;
    font-size: 11px;
    text-align: center;
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    color: var(--accent);
    border-bottom: 1px solid color-mix(in srgb, var(--accent) 15%, transparent);
    flex-shrink: 0;
    font-weight: 500;
    animation: slideDown 0.2s ease;
  }

  /* Loading */
  .cal-loading {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 13px;
    color: var(--text-secondary);
    background: var(--bg-glass);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    padding: 10px 20px;
    border-radius: var(--radius);
    box-shadow: var(--shadow-md);
  }
</style>
