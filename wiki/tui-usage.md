---
title: TUI (terminal interface)
description: racc-tui — an interactive terminal interface built on Ratatui. The sniff, dig, raid, and pack screens work, plus an opt-in reveal modal (since 0.4.5).
---

# TUI (terminal interface)

::: info
The TUI is being built in Beta (0.5.x). As of **0.4.6** the **sniff**, **dig**, **raid**, and **pack** screens work, plus an **opt-in reveal modal** for secret values.
:::

## What it is

`racc-tui` is an interactive terminal interface built on Ratatui. It lets you work with projects and secrets without memorizing commands: navigation, the project table, and confirmations.

## Shell

- **Sidebar**: brand `◈ RACCPACK` + workspace root, navigation, live badges — `Projects` shows the real count of scanned projects, `Findings` counts the findings of the last dig (amber when > 0, muted 0, `·` until the first dig). The renderer version sits at the bottom.
- **Header**: brand, scan root, and version; hotkey hints on the right (collapsed on narrow terminals).
- **Footer**: status counters.

## Sniff screen (available since 0.4.2)

- Lists projects from the scan root in a table: name, language, frameworks, size, git-repository flag.
- Runs `sniff` in a background worker thread, so the UI stays responsive (shows a progress indicator) while the scan runs.
- Navigation: `j`/`k` (or arrows) move the selection; `r` triggers a refresh; `o` is reserved for changing the scan root (not yet implemented); `Enter` opens a project (dig, since 0.4.3).

## Dig screen (available since 0.4.3)

- Opens with `Enter` on a project in the Sniff screen and runs the secret scan in a background worker thread.
- Findings table: risk, path, kind, git status — **masked only**, never raw secret values.
- Filter by minimum risk: `f` cycles all → critical → high+ → medium+.
- `c` toggles content scanning (re-runs dig with/without content matched values); `r` re-digs; `Esc` returns to the Sniff screen.
- Detailed strip under the table shows the selected finding's meta; selection: `j`/`k`, first/last `g`/`G`.

## Reveal modal (available since 0.4.5)

- With a finding selected in the **dig** screen (i.e. it was found in file contents, not by filename), press `v` to open an opt-in reveal.
- A confirm step asks before anything is decrypted/read; `y` (or `Enter`) proceeds, `n`/`Esc` aborts.
- The value is then loaded from the file in a background worker and shown **once** in the modal. It lives only in short-lived memory (`Zeroizing`), never in TUI app state, logs, or debug output, and is wiped when the modal closes (any key).
- If the file changed since the dig (the value no longer matches), the modal reports an error instead of showing a stale value.
- Filename-only findings (no content match) have no reveal — `v` is a no-op there.

## Raid flow (available since 0.4.4)

- Press `R` on a selected project in the Sniff screen to open the raid wizard as a modal overlay.
- **Preview** shows first: project, mode badge (`ATOMIC` / `FAIL-FAST`), the phases that will run (stash → rinse → pack), and the dry-run note — **nothing is written yet**.
- Confirm with `y` (or `Enter`), cancel with `n`/`Esc`. Toggles: `K` keep sources, `S` skip stash, `m` mode.
- If the stash phase is enabled, a **passphrase modal** collects it twice (`•` masked); `RACCPACK_PASSPHRASE` env var skips the prompt. The passphrase never appears in TUI state, logs, or debug output.
- While **running**, the modal shows the phase pipeline (`✓` done, `→` current, `○` pending), an overall progress bar, and the phase message — all from real core events. `Esc` does not cancel a running raid (core has no cancel).
- The **result** is honest: success, rolled back (with rollback-warning count), or failed; in `FAIL-FAST` mode a note shows that already-placed artifacts may remain. Placed artifacts are listed relative to the den. `Enter`/`Esc` closes.

## Pack flow (available since 0.4.6)

- Reachable from the **Operations** screen (hotkey `p`) with a project selected from `sniff`; opened as a modal overlay.
- **Preview** shows first: the project, the dry-run note, and the target output — **nothing is written to the den yet**.
- Options available before confirming with `y`/`Enter` (cancel with `n`/`Esc`): `c` toggles content deny (deny files whose contents contain secrets), `z` cycles the zstd compression level, `o` opens the inline editor for a custom output name (`Enter` confirms, `Esc` aborts, empty falls back to the auto name).
- **Progress** mirrors core events; the **result** lists the placed `tar.zst` path relative to the den. `Enter`/`Esc` closes.

## Target capabilities

- **raid confirmation** — done (since 0.4.4): wizard with phase-by-phase progress and honest result (see above).
- **Dry-run** — built into the raid wizard and the pack flow: the preview never writes to the den.
- **reveal modal** — done (since 0.4.5): safe opt-in reveal, value shown once then zeroized (see above).
- **pack flow** — done (since 0.4.6): preview → confirm → progress → result with content-deny, zstd level, and custom output-name options (see above).

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

The TUI color semantics come from design tokens in **DTCG** format (`docs/design-tokens/raccpack.tokens.json`) — a single source for both the TUI and the future Desktop. The current palette is **graphite + orange** (brand/focus orange; teal is no longer used) defined via primitive → semantic token mapping. Screens use semantic names (`bg`, `fg`, `accent`, `muted`, `danger`, `selection`, etc.), and sizes (sidebar width, panel heights) use **terminal cells**. A visual re-theme is a one-file change, not a grep across widgets.

## See also

- [Desktop](/desktop-usage) — the graphical interface.
- [Facade API](/facade-api) — the contract used by all interfaces.
