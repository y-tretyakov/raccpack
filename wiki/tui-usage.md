---
title: TUI (terminal interface)
description: Target behavior of racc-tui — an interactive terminal interface built on Ratatui (planned).
---

# TUI (terminal interface)

::: warning
The TUI is in development and scheduled for release in Beta (0.5.x). This section describes the target behavior based on vision documents; command examples may change.
:::

## What it is

`racc-tui` is an interactive terminal interface built on Ratatui. It lets you work with projects and secrets without memorizing commands: tree navigation, filters, confirmations.

## Target capabilities

- **sniff screen** — list of projects, stack, size, git-repository flag; keys to open a project.
- **dig screen** — secret findings with risk-level filters and masked details.
- **raid confirmation** — a wizard that shows what will happen at each phase (stash → rinse → pack), with interactive progress.
- **Dry-run** — a mode where all changes are visible before they are applied.

## How it will work

The TUI uses the same public contract of the core as the CLI, so results and policies (risks, masking, dry-run) are identical across all interfaces.

```
key press  →  TUI screen state  →  facade call
           ←  progress events + report ←  core
           →  panel updates
```

## Progress of long operations

Long operations (a deep `dig`, a full `raid`) send progress events: phase, percentage, message. The TUI redraws the screen on every event without blocking the interface.

## When it will arrive

The TUI is expected in phase **B1** (Beta, 0.5.x), right after the facade contract stabilizes (Alpha). Once released, this page will gain a section on installation and the full key map.

## See also

- [Desktop](/desktop-usage) — the graphical interface.
- [Facade API](/facade-api) — the contract used by all interfaces.
