# raccpack

CLI / TUI / Desktop tool for scanning projects, finding secrets, cleaning build trash, and packing each project into a "den" — a store of age-encrypted secrets and `tar.zst` archives.

## Status

**pre-MVP / greenfield** — repository scaffold only, no code yet. Implementation starts with milestone **M1** on stage branch `m1-workspace-core`.

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
