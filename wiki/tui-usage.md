---
title: TUI (terminal interface)
description: racc-tui — an interactive terminal interface built on Ratatui. The sniff screen works; dig/raid screens are planned (Beta).
---

# TUI (terminal interface)

::: info
The TUI is being built in Beta (0.5.x). As of **0.4.2** the **sniff screen** works (project table, non-blocking worker sniff); dig/raid screens and the reveal modal are planned.
:::

## What it is

`racc-tui` is an interactive terminal interface built on Ratatui. It lets you work with projects and secrets without memorizing commands: navigation, the project table, and confirmations.

## Sniff screen (available since 0.4.2)

- Lists projects from the scan root in a table: name, language, frameworks, size, git-repository flag.
- Runs `sniff` in a background worker thread, so the UI stays responsive (shows a progress indicator) while the scan runs.
- Navigation: `j`/`k` (or arrows) move the selection; `r` triggers a refresh; `o` is reserved for changing the scan root (not yet implemented); `Enter` opens a project (dig — planned).

## Target capabilities

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

## Visual system (design tokens)

The TUI color semantics come from design tokens in **DTCG** format (`docs/design-tokens/raccpack.tokens.json`) — a single source for both the TUI and the future Desktop. Screens use semantic names (`bg`, `fg`, `accent`, `muted`, `danger`, `selection`, etc.), and sizes (sidebar width, panel heights) use **terminal cells**. A visual re-theme is a one-file change, not a grep across widgets.

## See also

- [Desktop](/desktop-usage) — the graphical interface.
- [Facade API](/facade-api) — the contract used by all interfaces.
