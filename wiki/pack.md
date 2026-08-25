---
title: Pack — pack into the den
description: The racc pack command — archive a project into tar.zst and store it in the den without secrets.
---

# Pack - pack into the den

Command: `racc pack`  
Status: implemented.

This page describes **exactly the behavior** that `raccpack` implements today. If a flag or path is not listed here, it does not exist in the current version.

> Back to the command overview: [CLI usage](/cli-usage).

## What it does

1. Collects the project directory into a single **tar + zstd** archive (`.tar.zst`).
2. Places the archive in the den under the `packs/{yyyy}/{mm}/{slug}__{UTC}.tar.zst` layout.
3. Excludes secrets: by name (risk ≥ `High`) — always; by content (risk ≥ `Critical`) — by default.
4. By default runs as **dry-run** and writes nothing; writing happens only with `--yes`.

What it does **not** do:

- does not encrypt the archive (unlike `racc stash`);
- does not modify or delete files of the source project;
- does not preserve symbolic links or empty directories;
- does not use the `sniff` cache.

::: warning
By default `pack` runs as **dry-run** and writes nothing. Writing to the den happens only with the `--yes` flag.
:::

## Quick start

```bash
# Dry-run: show what would be packed (nothing is written)
racc pack --project ~/DEV/PROJS/app-api

# Commit: create the archive in the den
racc pack --project ~/DEV/PROJS/app-api --yes

# Commit with a custom artifact name
racc pack --project ~/DEV/PROJS/app-api --yes --output-name snapshot
```

## Syntax

```text
racc pack --project <PATH> [OPTIONS]
```

`--project` is required.

## Options and flags

### Command flags

| Flag | Default | Description |
|------|---------|-------------|
| `--project <PATH>` | — (required) | Project directory to pack |
| `--yes` | off | Commit: write the archive into the den |
| `--dry-run` | off | Force dry-run; wins over `--yes` when both are given |
| `--no-content-deny` | off | Disable content-based secret deny (name deny stays) |
| `--zstd-level <N>` | crate default (`3`) | zstd compression level |
| `--output-name <NAME>` | `{slug}__{UTC}` | Artifact name without `.tar.zst` |

::: warning
`--no-content-deny` disables only **content** checks. Deny by file **name** (`.env`, keys, etc.) stays. The `pack` archive is **not encrypted** — use `racc stash` to store secrets.
:::

### Global flags

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Config file (overrides `RACCPACK_CONFIG`) |
| `--den <PATH>` | Override `den_dir`; without the flag — from config (`paths.den_dir`), default `~/.raccpack/den` |
| `--root <PATH>` | Base for a relative `--project` |
| `--json` | Print JSON instead of the human-readable block |

Priorities:

- the default mode is **dry-run**; `--dry-run` overrides `--yes`; commit requires `--yes` without `--dry-run`;
- `--den` overrides `den_dir` from config for this run;
- `--output-name` replaces `{slug}__{UTC}` in the file name (the year/month directories `packs/{yyyy}/{mm}` still come from current UTC).

## Behavior

- **Dry-run**: nothing is created under the den (neither `ensure_den` nor staging). Output shows the expected artifact path.
- **Commit**: the den skeleton is created, the project is packed into `den/staging/{short_id}/` and moved to `packs/{yyyy}/{mm}/`.
- **Uniqueness**: if the artifact already exists, an 8-hex-character suffix is appended to the name (to the timestamp or to `--output-name`); if the conflict persists — error.
- **Secrets**: name deny (risk ≥ `High`) is always on; content deny (risk ≥ `Critical`) is on by default and disabled by `--no-content-deny`.
- **Skip policy**: directories such as `node_modules`, `target`, `.git`, `dist`, `build` are skipped.
- **Symlinks** are neither followed nor archived.
- The archive contains the **contents** of the project folder (entries like `src/main.rs`), not the folder itself.
- Entry order is deterministic (by name) — archive bytes are reproducible.
- **CI/TTY**: the command is fully non-interactive (no prompts, no passphrase) — CI-safe.

## Output

### Human-readable - dry-run

```text
Pack (dry-run)
  Source: /tmp/projects/app
  Would write: /tmp/den/packs/2026/08/app__20260815T144410Z.tar.zst
  Content deny: on
  (no files written)
```

### Human-readable - commit

```text
Pack complete
  Source: /tmp/projects/app
  Output: /tmp/den/packs/2026/08/app__20260815T144410Z.tar.zst
  Size: 195 B
  Files: 3
  Skipped secret files: 2
```

### JSON (`--json`)

```json
{
  "source": "/tmp/projects/app",
  "output": "/tmp/den/packs/2026/08/app__20260815T144410Z.tar.zst",
  "size_bytes": 195,
  "file_count": 3,
  "skipped_secret_files": 2,
  "dry_run": false
}
```

Fields:

| Field | Meaning |
|-------|---------|
| `source` | Source project directory |
| `output` | Path to the artifact (expected path in dry-run) |
| `size_bytes` | Archive size in bytes; `0` in dry-run |
| `file_count` | Number of included files; `0` in dry-run |
| `skipped_secret_files` | Number of skipped secret files (by name and/or content); `0` in dry-run |
| `dry_run` | `true` / `false` |

The size in human-readable output is formatted with binary units (`B`, `KiB`, `MiB`). Raw secret values never appear in output.

## Exit codes

| Code | When |
|------|------|
| `0` | Success (including dry-run) |
| `1` | Error: missing project, not a directory, invalid `--output-name`, IO, name conflict, den inside the project |

Code `2` (as in `dig`) is **not** used by `pack`.

## Examples

```bash
# Dry-run: show what would be packed
racc pack --project ~/DEV/PROJS/app-api

# Dry-run in JSON
racc pack --project ~/DEV/PROJS/app-api --json

# Commit: create the archive in the den
racc pack --project ~/DEV/PROJS/app-api --yes

# Commit for CI with JSON
racc pack --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes --json

# Custom artifact name instead of slug__timestamp
racc pack --project ~/DEV/PROJS/app-api --yes --output-name snapshot

# zstd compression level
racc pack --project ~/DEV/PROJS/app-api --yes --zstd-level 19

# Disable content deny (name deny remains)
racc pack --project ~/DEV/PROJS/app-api --yes --no-content-deny

# Explicit dry-run even when --yes is passed
racc pack --project ~/DEV/PROJS/app-api --yes --dry-run

# Relative --project resolved against --root
racc pack --root ~/DEV/PROJS --project app-api --yes
```

## Common errors

| Situation | What to do |
|-----------|------------|
| `--project` missing | The flag is required — CLI rejects the run |
| "path not found" / "not a directory" | Check that `--project` exists and is a directory |
| `staging path lies inside the project tree` | The den is inside the project — use a den outside the project |
| Invalid `--output-name` | Name must not be empty, `.`, `..`, or contain `/`, `\`, `\0` |
| `pack artifact name collision under den` | Name conflict even after adding the suffix (unlikely) — rerun |
| Secrets remain in the archive with `--no-content-deny` | Content deny is off but name deny stays; inspect the project via `racc dig` |
| Secrets were not removed from sources | `pack` never edits the project — removing secrets is `racc stash`'s job |

## Security

- Dry-run by default — review output first, then commit.
- Name deny (risk ≥ `High`) cannot be turned off; content deny (risk ≥ `Critical`) is on by default.
- The archive is **not encrypted** (tar.zst) — do not use `pack` as a replacement for `stash` to store secrets.
- Never commit the den directory to git.
- Raw secret values never appear in human-readable or JSON output.

## Related commands

| Command | Role |
|---------|------|
| `racc sniff` | Find projects under `scan_root` |
| `racc dig` | Find secrets in a project before packing (read-only) |
| `racc stash` | Move secrets into an encrypted age archive in the den |
| `racc rinse` | Delete build trash by strategies |
| `racc raid` | Full cycle in one command |
| [Concepts](/concepts) | Den, layout, risks, skip policy |

*This document matches the implementation; when CLI flags change, update the page in the same PR.*
