---
title: TUI (terminal interface)
description: racc-tui — an interactive terminal interface built on Ratatui. The Overview dashboard, sniff (Projects), dig, and raid screens work; the reveal modal is planned (Beta).
---

# TUI (terminal interface)

::: info
The TUI is being built in Beta (0.5.x). As of **0.4.5** the **Overview**, **Projects** (sniff), **dig**, and **raid** screens work with the **Visual System 2.0** look and feel; the reveal modal is planned.
:::

## What it is

`racc-tui` is an interactive terminal interface built on Ratatui. It lets you work with projects and secrets without memorizing commands: an overview dashboard, project cards, and confirmations.

## Shell (since 0.4.5)

- **Sidebar**: `◈ RACCPACK` brand, workspace path, navigation, live badges — `Projects` shows the real scanned-project count, `Findings` counts findings from the last dig (amber when > 0, hidden until dig has run). The current version is shown at the bottom.
- **Header**: brand + scan root + version; keyboard hints on the right (collapsed on narrow terminals).
- **Overview** (start screen): a KPI strip (projects / Rust / JS-TS / total size / git repos), a **workspace health** line (`✓ detection READY`, `(cache)` when loaded from the cache, or hints before the first scan), and recent project cards.
- **Activity stream** (wide terminals, content ≥ 120 cols): live, user-meaningful events (`✔` ok, `!` warn, `✖` error, `·` info) from real sniff/dig/raid runs — distinct from the debug log (`l`).

## Projects screen (Sniff) (since 0.4.2, Cards since 0.4.5)

- Lists projects from the scan root. **Cards are the default view**; press `v` to cycle **Cards → Table → Tree → Cards** (Tree is a stub until the full nested view lands).
- Card hierarchy: name → language · size → git state (`●` clean) → path. The selected card has an orange border.
- Runs `sniff` in a background worker thread, so the UI stays responsive (shows a progress indicator) while the scan runs.
- Navigation: `j`/`k` (or arrows) move the selection; `r` triggers a refresh; `o` is reserved for changing the scan root (not yet implemented); `R` opens the raid flow for the selected project; `Enter` opens a project (dig, since 0.4.3).

## Dig screen (available since 0.4.3)

- Opens with `Enter` on a project in the Sniff screen and runs the secret scan in a background worker thread.
- Findings table: risk, path, kind, git status — **masked only**, never raw secret values.
- Filter by minimum risk: `f` cycles all → critical → high+ → medium+.
- `c` toggles content scanning (re-runs dig with/without content matched values); `r` re-digs; `Esc` returns to the Sniff screen.
- Detailed strip under the table shows the selected finding's meta; selection: `j`/`k`, first/last `g`/`G`.

## Raid flow (available since 0.4.4)

- Press `R` on a selected project in the Sniff screen to open the raid wizard as a modal overlay.
- **Preview** shows first: project, mode badge (`ATOMIC` / `FAIL-FAST`), the phases that will run (stash → rinse → pack), and the dry-run note — **nothing is written yet**.
- Confirm with `y` (or `Enter`), cancel with `n`/`Esc`. Toggles: `K` keep sources, `S` skip stash, `m` mode.
- If the stash phase is enabled, a **passphrase modal** collects it twice (`•` masked); `RACCPACK_PASSPHRASE` env var skips the prompt. The passphrase never appears in TUI state, logs, or debug output.
- While **running**, the modal shows the phase pipeline (`✓` done, `→` current, `○` pending), an overall progress bar, and the phase message — all from real core events. `Esc` does not cancel a running raid (core has no cancel).
- The **result** is honest: success, rolled back (with rollback-warning count), or failed; in `FAIL-FAST` mode a note shows that already-placed artifacts may remain. Placed artifacts are listed relative to the den. `Enter`/`Esc` closes.

## Target capabilities

- **raid confirmation** — done (since 0.4.4): wizard with phase-by-phase progress and honest result (see above).
- **Dry-run** — built into the raid wizard: the preview never writes to the den.
- **reveal modal** — safe opt-in reveal of a secret's masked/short value (planned, B1.5).

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

The TUI color semantics come from design tokens in **DTCG** format (`docs/design-tokens/raccpack.tokens.json`) — a single source for both the TUI and the future Desktop. Since 0.4.5 the TUI uses **Visual System 2.0**: graphite surfaces with an orange brand/focus accent (teal is no longer used). Tests assert the palette contract (e.g. focus == brand orange), and sizes (sidebar width, panel heights) use **terminal cells**. A visual re-theme is a one-file change, not a grep across widgets.

## See also

- [Desktop](/desktop-usage) — the graphical interface.
- [Facade API](/facade-api) — the contract used by all interfaces.
