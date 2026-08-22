---
title: Roadmap
description: How raccpack is evolving toward version 1.0.0 — what is already available, what is in development, and what is planned.
---

# Roadmap

How raccpack is evolving toward version 1.0.0 — and what you can use today.

::: info
Current version: **0.3.2** — Alpha complete; Detect v2 in progress (D1.1 registry + D1.2 DTO closed, internal only).
Dates are approximate. "Available" status means the functionality can be used in the current version built from source.
:::

## Already available (MVP + Alpha)

- [x] Working workspace: core + CLI `racc`.
- [x] `racc sniff` — project discovery by markers, stack detection, sizes, cache.
- [x] `racc dig` — secret detection by name and content, masking, risk, exit-code policy.
- [x] `racc pack` — packing a project into `tar.zst` (dry-run by default, commit with `--yes`), excluding secrets by name and content.
- [x] `racc stash` — moving secrets into age archives (passphrase, zeroize), dry-run by default.
- [x] `racc rinse` — cleaning build trash according to strategies (dry-run by default, commit with `--yes`).
- [x] `racc raid` — the full cycle in one command (atomic: staging + WAL + rollback; JSON manifests; exit 1 on `!success`).
- [x] Den structure: `packs/{yyyy}/{mm}/` layout, `.den-version`, safe naming.
- [x] Full MVP E2E cycle: sniff → dig → pack → den.
- [x] Git status of files in the `dig` report (`git_status`).
- [x] `racc init` — starter config (`config_version = 1`) and den skeleton (`--ensure-den`); config auto-migration v0 → v1 at load time.
- [x] Secret-free tracing logs and a global `--verbose` (`-v`/`-vv`/`-vvv`, logs to stderr, `RUST_LOG` takes precedence).
- [x] Integration tests and CI (`cargo test` / fmt / clippy on every push and PR).

## Planned (Detect v2 0.4.x)

- [ ] Composite stack detectors (DAG) for monorepositories and hybrid projects.
- [ ] Stack tree in `sniff` (`--detect-mode=dag`), backward-compatible flat `stack`.
- [ ] DAG-scoped `rinse` — cleaning build trash only in relevant subtrees.

## Planned (Beta 0.5.x)

- [ ] TUI (Ratatui) — interactive terminal interface.
- [ ] Desktop (Tauri + React) — desktop application.
- [ ] Den management: `racc den list`, `staging` cleanup.
- [ ] Hard verification that "no secrets end up in logs/errors".

## Planned (RC 0.9.x → 1.0.0)

- [ ] Freezing the public API, den layout, and CLI exit codes.
- [ ] Load testing, cross-platform smoke tests.
- [ ] Shell completion, final help texts.
- [ ] Publishing binaries and 1.0 documentation.

## Milestones

| Milestone | Version | What's inside |
|-----------|---------|---------------|
| **MVP** | 0.1.x | sniff → dig → pack → den |
| **Alpha** | 0.3.x | Full `raid` (atomic: staging + WAL + rollback, JSON manifests), age-stash, rinse, git integration, CLI only |
| **Detect v2** | 0.4.x | Composite detectors / DAG for monorepositories; batch raid `racc raid --root` (planned) |
| **Beta** | 0.5.x | TUI, Desktop, security hardening |
| **RC** | 0.9.x | Contract freeze, polishing |
| **Stable** | 1.0.0 | Stable public API |

## Priorities and dependencies

- MVP packing does not require age — secrets are excluded by name.
- TUI and Desktop will appear only after the facade contract stabilizes (Alpha).
- Composite detection (Detect v2) slots in between Alpha and Beta so that TUI/Desktop immediately get a correct stack tree.
- API/den freeze comes after security hardening (Beta).
- Between milestones only releases land in `main`; all development happens in `dev`.

## Out of scope until 1.0.0

- Cloud den and remote synchronization.
- KMS/secret vaults as the primary backend.
- Multi-user HTTP server.
- Automatic "remove the secrets" PRs and "smart" editing redaction.
- Third-party rule-set plugins.

## See also

- [Quick start](/quick-start) — what you can do right now.
- [CLI usage](/cli-usage) — available commands.
