# raccpack

CLI / TUI / Desktop tool for scanning projects, finding secrets, cleaning build trash, and packing each project into a "den" — a store of age-encrypted secrets and `tar.zst` archives.

## Status

**M1 in progress** — milestone M1 (workspace scaffold + core foundation) on stage branch `m1-workspace-core`. The Cargo workspace and crate skeleton build and test green; business logic (scan / secrets / pack / den) is not implemented yet.

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

Architecture vision and roadmap live in [`docs/`](docs/). Main knowledge documents at the repo root:

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
