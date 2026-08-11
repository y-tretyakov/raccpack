# raccpack

<p align="center">
  <img src="RaccPack.webp" alt="raccpack" width="435"/>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.75-orange?style=flat-square&logo=rust" alt="Rust"/></a>
  <a href="https://doc.rust-lang.org/cargo/"><img src="https://img.shields.io/badge/Cargo-workspace-blue?style=flat-square&logo=cargo" alt="Cargo"/></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/version-0.1.0-blue?style=flat-square" alt="version"/></a>
  <a href="https://github.com/y-tretyakov/raccpack/actions/workflows/wiki.yml"><img src="https://github.com/y-tretyakov/raccpack/actions/workflows/wiki.yml/badge.svg?style=flat-square" alt="CI"/></a>
  <a href="https://github.com/y-tretyakov/raccpack"><img src="https://img.shields.io/badge/OS-Windows%20%7C%20Linux%20%7C%20macOS-success?style=flat-square" alt="Windows | Linux | macOS"/></a>
  <a href="https://tauri.app"><img src="https://img.shields.io/badge/Tauri-Desktop-purple?style=flat-square&logo=tauri" alt="Tauri"/></a>
  <a href="https://clap.rs"><img src="https://img.shields.io/badge/CLI-clap-ee4b2b?style=flat-square" alt="CLI"/></a>
  <a href="https://ratatui.rs"><img src="https://img.shields.io/badge/TUI-ratatui-4f8?style=flat-square" alt="TUI"/></a>
  <a href="https://github.com/FiloSottile/age"><img src="https://img.shields.io/badge/secrets-age--encrypted-0a0a0a?style=flat-square" alt="age"/></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue?style=flat-square" alt="License"/></a>
</p>

CLI / TUI / Desktop tool for scanning projects, finding secrets, cleaning build trash, and packing each project into a "den" — a store of age-encrypted secrets and `tar.zst` archives.

## Status

**M1 done** — milestone M1 (workspace scaffold + core foundation: domain DTOs, config, SkipPolicy + safe walk) is closed. Next milestone: **M2 — sniff** (project discovery by markers). Business logic for scan / secrets / pack / den is not implemented yet; `racc` is a stub that prints the version.

## Workspace structure

```
raccpack/
  Cargo.toml                  # workspace manifest (resolver 2)
  crates/
    raccpack-core/            # library: domain + use-cases, no UI/CLI deps
    raccpack-cli/             # binary: `racc` stub, links raccpack-core
  LICENSE-MIT                 # MIT license
  LICENSE-APACHE              # Apache-2.0 license
```

The workspace is dual-licensed **MIT OR Apache-2.0**. `Cargo.lock` is committed (binary workspace policy) so builds are reproducible.

## Build

```bash
cargo build
cargo test
cargo run -p raccpack-cli
```

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Docs

**User wiki** lives in [`wiki/`](wiki/) and is built with [VitePress](https://vitepress.dev). The site is published to GitHub Pages at <https://y-tretyakov.github.io/raccpack/>.

- Serve locally: `pnpm install && pnpm run wiki:dev`
- Build: `pnpm run wiki:build` (output in `wiki/.vitepress/dist/`)
- Preview a build: `pnpm run wiki:preview`
- Deploy is handled by the `.github/workflows/wiki.yml` workflow.
- The wiki is RU-first (root locale) with an English skeleton under `wiki/en/`.

**Development docs** live in [`docs/`](docs/) — these are dev specs, not part of the published wiki. Main knowledge documents at the repo root:

- `raccpack-agent-prompt.md` — main spec (phases 0–11, stage criteria)
- `raccpack-architecture-vision.md` — layers, trust boundaries, DTO contracts
- `raccpack-facade-and-den.md` — facade use-cases and den layout
- `raccpack-roadmap-v1.md` — MVP → 1.0.0 milestones

## Git workflow

Branches:

- `main` — **protected**. Milestone releases only (see tags below).
- `dev` — main working branch. All development merges here.
- Stage/feature branches — short-lived, created **from `dev`**.

Rules:

1. Development happens **only** in short-lived branches off `dev`.
2. Stage branch names follow the roadmap as `{phase}-{short-slug}` in kebab-case, e.g. `m1-workspace-core`, `m2-sniff`, `m3-dig`, `m4-pack-den`, `a1-stash-age`, `a2-rinse`, `a3-raid`.
3. On stage completion: open a PR **into `dev`**; after review/merge, delete the stage branch.
4. Merge `dev` → `main` plus `git tag` and a GitHub Release **only** on milestones:
   - MVP → `v0.1.0`
   - Alpha → `v0.3.0`
   - Beta → `v0.5.0`
   - RC → `v0.9.0`
   - Stable → `v1.0.0`
5. Between milestones nothing is merged into `main`. Intermediate stages live only in `dev`.
6. Hotfix/blocker after a milestone release: branch from `main` (or the tag), PR into `main`, then backport into `dev`.

Merge method: **squash** (fixed for stage branches and for `dev → main`). Stage branches are deleted on merge.

**Branch protection (status):** enforced via GitHub rulesets (repo is public). Both rulesets restrict merge method to **squash**.
- `main`: PR required + 1 approval, no force push, no deletions (bypass: maintainers/admins).
- `dev`: PR required, no force push, no deletions (bypass: maintainers/admins).
