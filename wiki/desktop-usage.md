---
title: Desktop (desktop application)
description: Target behavior of the raccpack Desktop application on Tauri + React (planned).
---

# Desktop (desktop application)

::: warning
The Desktop application is in development and scheduled for release in Beta (0.5.x). This section describes the target behavior based on vision documents; interface details may change.
:::

## What it is

A desktop application built on **Tauri + React** (state — Zustand). It provides the same functionality as the CLI and TUI, but in a graphical interface: folder pickers, project tables, secret lists, a raid wizard.

## How it is structured

The application is split into two parts:

- **React (UI)** — the interface and screen state. Contains no secret logic and does not read files from disk directly.
- **BFF (Tauri Rust commands)** — a thin layer between React and the core: path validation, DTO mapping, launching long operations, and progress streaming.

```
React (UI)  →  Zustand  →  Tauri command (BFF)  →  raccpack-core
                  ↑← progress events / result ←┘
```

## Target capabilities

- **Folder selection** — `scan_root` and `den_dir` via system dialogs.
- **Project table** — the result of `sniff` with filters by stack and size.
- **Secret list** — the result of `dig` with risk filters and masked values.
- **Raid wizard** — confirmation of stash/rinse/pack phases, passphrase entry in a secure dialog, event-driven progress.
- **Security** — React receives only DTOs with masked secrets; the passphrase is never stored as a long-lived string in state.

## How it will work

```
action in React  →  Zustand → invoke("raid", { root, den, dryRun })
                 →  Tauri command (BFF) → core: raid + progress events
                 →  Zustand updates the UI (masked data only)
```

## When it will arrive

Desktop is expected in phase **B2** (Beta, 0.5.x), after the facade contract stabilizes. Once released, installation instructions and screen descriptions will appear here.

## See also

- [TUI](/tui-usage) — the terminal interface.
- [Facade API](/facade-api) — the contract used by all interfaces.
- [Core concepts](/concepts) — risks and masking.
