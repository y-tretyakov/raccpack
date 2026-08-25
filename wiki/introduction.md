---
title: Welcome to raccpack
description: An overview of raccpack — a tool for finding secrets and packing projects into the den storage.
---

# Welcome to raccpack

raccpack is a tool for keeping your projects folder in order. It helps you:

- **Find secrets** — keys, tokens, passwords, and credential files that ended up in working copies by accident.
- **Pack projects** — each project is packed into a separate `tar.zst` archive without secrets or trash and placed into the "den".
- **Move secrets out** — into encrypted [age](https://age-encryption.org)-standard archives in the "den".
- **Clean build trash** — `node_modules`, `target`, caches — according to strategies.
- **Run the whole cycle with one command** — `racc raid` (stash → rinse → pack → move).

The den is your local protected storage (vault): clean project archives, encrypted secrets, and JSON manifests of every operation.

## Why you need this

Developers often keep dozens of projects in their working folder. Among them you will almost certainly find:

- `.env` files with real keys;
- SSH keys and certificates;
- AWS/GitHub/Stripe credential files;
- build directories tens of gigabytes in size.

Copying such a folder "as is" to a backup or the cloud means leaking secrets and shipping tons of trash. raccpack automates tidying things up before packing.

## How it works (in short)

![raccpack pipeline: sniff → dig → stash → rinse → pack → raid](/how-it-works.webp)

You specify two folders:

- **`scan_root`** — where your projects live (input);
- **`den_dir`** — where archives are stored (output).

Everything else raccpack does based on configuration and built-in rules.

## Available now / Coming soon

What you can use today, and what follows next:

**Available in the CLI (`racc`):**

- `racc init` — starter configuration with a single command;
- `racc sniff` — an overview of projects, their stacks, and sizes;
- `racc dig` — secret detection with masking, risk scoring, and per-file git status;
- `racc stash` — moving secrets into encrypted age archives;
- `racc rinse` — cleaning build trash according to strategies;
- `racc pack` — packing a project into a `tar.zst` without secrets or trash, into the den;
- `racc raid` — the full cycle in one command (stash → rinse → pack → move); `--root` for batch mode across all projects;
- `--detect-mode` — composite DAG stack detection for monorepositories;
- `-v/--verbose` — detailed logs without secrets.

The full catalog of supported markers, secrets, and deny rules is on the [Supported](/supported) page.

**Coming soon (Beta 0.5.x):**

- TUI (Ratatui) — interactive terminal interface;
- Desktop (Tauri + React) — desktop application.

## Key security principles

- **Secrets are hidden by default.** Reports, logs, and JSON output show masked previews and hashes instead of values. Showing the original value requires an explicit action.
- **Dry-run by default.** Destructive operations (deleting sources, cleaning trash) first show what will happen and require explicit confirmation (`pack --yes`).
- **One codebase for every interface.** All logic lives in the `raccpack-core`; CLI, TUI, and Desktop merely call the same public contract.
- **Raw secrets never leave the core.** During `stash`/`raid`, a value lives in core memory only while it is being encrypted; afterwards the memory is zeroed.

## Interfaces

| Interface | Status | Description |
|-----------|--------|-------------|
| **CLI** (`racc`) | Available | Command line, suitable for scripts and CI |
| **TUI** | Planned | Terminal interface with interactive navigation |
| **Desktop** | Planned | Desktop application on Tauri + React |

::: info
Currently the CLI supports `sniff`, `dig`, `pack`, `stash`, `rinse`, and `raid`, and `racc init` creates a starter configuration; next up — `racc den`. See the [Roadmap](/roadmap).
:::

## What next

- [Installation](/installation) — build and verify `racc`.
- [Quick start](/quick-start) — your first run in five minutes.
- [Core concepts](/concepts) — den, secrets, risks, phases.
