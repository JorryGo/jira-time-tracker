# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Jira Time Tracker is a macOS menubar desktop app for tracking time on Jira issues. Built with **Tauri 2** (Rust backend + Svelte 5 frontend). The app runs as a system tray accessory (no dock icon), with a popover window that appears on tray click.

## Commands

```bash
# Development (starts both Vite dev server and Tauri app with hot reload)
npm run tauri dev

# Build production app
npm run tauri build

# Frontend only (Vite dev server on port 1420)
npm run dev

# Type checking for Svelte files
npx svelte-check
```

No test runner or linter is configured.

## Architecture

```
Frontend (Svelte 5 / TypeScript)
    ↕  Tauri IPC (invoke())
Backend (Rust / Tokio async)
    ↕  SQLx
SQLite database (jira-tracker.db in app data dir)
```

### Frontend (`src/`)

- **Views** (`src/views/`) — Three tabs: TasksView (timer + issue list), WorklogsView (history + sync), SettingsView (Jira config)
- **State** (`src/lib/state/`) — Svelte 5 runes with `$state` class-based stores: `timer.svelte.ts`, `worklogs.svelte.ts`, `tasks.svelte.ts`, `settings.svelte.ts`
- **Commands** (`src/lib/commands/`) — Typed wrappers around `tauri.invoke()` calls, one file per domain (timer, jira, worklogs, settings)
- **Types** (`src/lib/types/`) — TypeScript interfaces for Jira entities, worklogs, settings

### Backend (`src-tauri/src/`)

- **commands/** — `#[tauri::command]` handlers exposed via IPC. All commands registered in `lib.rs` invoke_handler
- **jira/** — `JiraClient` HTTP wrapper using reqwest with Basic Auth (base64-encoded email:token)
- **db/migrations.rs** — SQLite schema: tables for `settings` (KV store), `issues`, `worklogs`, `active_timer`
- **state.rs** — `AppState` holds `SqlitePool` and `Mutex<Option<JiraConfig>>`

### Key patterns

- Tauri commands return `Result<T, String>` — errors are string-serialized for IPC
- Database queries use `sqlx::query`/`sqlx::query_as` with runtime string building (not compile-time checked macros)
- Timer state persists in SQLite `active_timer` table; frontend polls and drives tray icon updates
- Tray icon changes dynamically based on timer state (idle/working/paused PNGs in `src-tauri/icons/`)
- Window hides on focus loss, shows/positions on tray click via `tauri-plugin-positioner`
- macOS activation policy set to `Accessory` (hidden from dock)
