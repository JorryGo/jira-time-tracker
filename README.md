# Jira Time Tracker

A desktop app for tracking time on Jira issues. Lives in the system tray — click the icon to open a popover, log your work, and push worklogs to Jira. Runs on macOS, Windows, and Linux.

## Features

- **Timer** — Start, pause, resume, and stop a timer on any Jira issue. Elapsed time shows in the menu bar.
- **Work Logs** — Browse, create, edit, and delete worklogs. Filter by date and sync status.
- **Push to Jira** — Push individual or all pending worklogs to Jira with one click.
- **Dynamic Tray Icon** — Icon changes based on timer state (idle / working / paused).
- **Search, Sort & Pin** — Filter tasks by key or summary, sort by status priority, pin important issues to the top.
- **Report** — Generate a text report grouped by issue with total time and descriptions, copy to clipboard.
- **Custom JQL** — Configure which issues appear via a custom JQL filter.
- **Offline-first** — All data stored locally in SQLite; sync to Jira when ready.

## Tech Stack

Tauri 2 · Svelte 5 · TypeScript · Rust · SQLite

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) 22+
- [Rust](https://rustup.rs/) stable
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

### Development

```bash
npm install
npm run tauri dev
```

This starts both the Vite dev server (with hot reload) and the Tauri app.

### Build

```bash
npm run tauri build
```

Produces a platform-specific bundle in `src-tauri/target/release/bundle/`.

## Project Structure

```
src/                        # Frontend (Svelte 5 + TypeScript)
├── views/                  # TasksView, WorklogsView, SettingsView
├── components/             # TimerDisplay, TaskItem, modals
└── lib/
    ├── state/              # Svelte 5 rune-based stores
    ├── commands/           # Typed wrappers for Tauri invoke()
    └── types/              # TypeScript interfaces

src-tauri/                  # Backend (Rust)
├── src/
│   ├── commands/           # Tauri IPC command handlers
│   ├── jira/               # HTTP client (reqwest + Basic Auth)
│   ├── db/                 # SQLite schema & migrations
│   └── lib.rs              # App setup, tray, window management
├── icons/                  # App and tray icons
└── tauri.conf.json         # Tauri configuration
```
