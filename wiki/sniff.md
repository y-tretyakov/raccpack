---
title: Sniff — discover projects
description: The racc sniff command — find projects under scan_root, detect their stack, size, and git status. A cached read-only walk.
---

# Sniff - discover projects

Command: `racc sniff`  
Status: implemented.

This page describes **exactly the behavior** that `raccpack` implements today. If a flag or path is not listed here, it does not exist in the current version.

Back to the command overview: [CLI usage](/cli-usage).

## What sniff does

`racc sniff` walks `scan_root` and finds projects by characteristic **markers** — files in the project root (`Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `pom.xml`, `Gemfile`, `composer.json`, `CMakeLists.txt`, `Makefile`, `.git`, etc. — see [Supported catalog](/supported) for the full list and priorities). For each project it determines:

- **stack** — language + frameworks;
- **size** in bytes;
- whether it is a **git repository**.

What sniff does **not** do:

- writes and deletes nothing — the command is **read-only**;
- does not read the contents of secret files (unlike `racc dig`);
- does not enter or write to the den;
- never returns exit code `2` (that is specific to `dig`).

## Quick start

```bash
# 1) Plain run — table of projects under scan_root
racc sniff

# 2) Force rescan without cache
racc sniff --force-refresh

# 3) Machine-readable output for scripts and CI
racc sniff --json
```

## Syntax

```text
racc sniff [OPTIONS]
```

There are no positional arguments.

## Options and flags

### Command options

| Flag | Default | Description |
|------|---------|-------------|
| `--force-refresh` | off | Ignore the sniff cache and rescan from scratch |
| `--max-depth <N>` | from config (`scanner.max_depth`, default `6`) | Override the walk depth for this run |

Depth priority: `--max-depth` for this run → `scanner.max_depth` in config → built-in default of `6`.

### Global flags

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Config file (overrides `RACCPACK_CONFIG`) |
| `--root <PATH>` | Override `scan_root` for this run |
| `--den <PATH>` | Override `den_dir` for this run (optional for sniff) |
| `--json` | Print JSON instead of the human-readable table |

::: info
`--root` and `--den` override the configuration only for the current run and never change it on disk.
:::

## Behavior

### Cache

Results are cached in `$XDG_CACHE_HOME/raccpack/sniff/{hash}.json` (or `~/.cache/raccpack/sniff/…` when `XDG_CACHE_HOME` is unset). The file name hashes the absolute `scan_root`, the `max_depth`, and the version of the skip-directory policy; the binary version is checked when reading the cache — on mismatch the cache is considered stale. Re-running without changes does **not rescan** — the report comes from the cache:

- `cache: hit` — report read from cache;
- `cache: miss` — full walk performed (the cache is rewritten afterwards).

`--force-refresh` always performs a full walk and rewrites the cache. Changing `--max-depth` changes the cache key, so such a run is also a "miss". Cache read errors are treated as "miss"; cache write errors do not abort the run.

::: tip
If `sniff` "does not see" a new project — the cache has probably kicked in: run with `--force-refresh` to rescan from scratch.
:::

### Misc

- Run mode is always **read-only**: dry-run/commit do not apply.
- Missing `scan_root` is an error (see [Common errors](#common-errors)).

## Output

### Human-readable (human)

A summary line and a project table. Columns: `NAME`, `STACK`, `SIZE`, `GIT`, `PATH`.

```text
Scan root: /tmp/projects
Projects: 1  |  Total size: 137 B  |  0 ms  |  cache: hit

NAME  STACK  SIZE   GIT  PATH
app   Rust   137 B  no   /tmp/projects/app
```

- `STACK` — `Language` or `Language + Frame1 + Frame2`; `-` when no language was detected.
- `GIT` — `yes` if the project root contains a `.git` directory, otherwise `no`.
- `SIZE` — human-readable size with binary units (`B`, `KiB`, `MiB`, `GiB`, `TiB`).

### JSON (`--json`)

Top-level fields:

| Field | Type | Meaning |
|-------|------|---------|
| `from_cache` | bool | Whether the report came from cache (`hit`) or was built fresh (`miss`) |
| `duration_ms` | number | Run duration in milliseconds |
| `report` | object | The `ScanReport` itself |

`report` fields:

| Field | Type | Meaning |
|-------|------|---------|
| `root` | string | Scanned root (absolute path) |
| `projects` | array | Discovered projects |
| `total_size_bytes` | number | Sum of all project sizes |
| `schema_version` | number | Schema version (currently always `1`) |

Fields of each `projects[]` entry:

| Field | Type | Meaning |
|-------|------|---------|
| `path` | string | Path to the project root |
| `name` | string | Name (usually the folder name) |
| `stack.language` | string/null | Language, if detected |
| `stack.frameworks` | array | Detected frameworks |
| `stack.markers` | array | Markers matched during detection |
| `size_bytes` | number | Project size in bytes |
| `is_git_repo` | bool | Whether a `.git` directory exists in the root |

Example:

```json
{
  "report": {
    "root": "/tmp/projects",
    "projects": [
      {
        "path": "/tmp/projects/app",
        "name": "app",
        "stack": { "language": "Rust", "frameworks": [], "markers": ["Cargo.toml"] },
        "size_bytes": 137,
        "is_git_repo": false
      }
    ],
    "total_size_bytes": 137,
    "schema_version": 1
  },
  "from_cache": false,
  "duration_ms": 0
}
```

## Exit codes

| Code | When |
|------|------|
| 0 | Success |
| 1 | Error: missing/inaccessible `scan_root`, unreadable config, etc. |

Sniff has **no** code `2` — it is used only by `dig` (the `--fail-on` policy).

## Examples

```bash
# Locally: full overview of the projects folder
racc sniff

# Just one root (without editing config)
racc sniff --root ~/DEV/PROJS

# No deeper than 3 levels of nesting
racc sniff --max-depth 3

# Ignore cache and rescan
racc sniff --force-refresh

# With all overrides at once
racc sniff --root ~/DEV/PROJS --max-depth 2 --force-refresh

# JSON for scripts and CI
racc sniff --json

# JSON with an overridden root — machine parsing
racc sniff --root "$CI_PROJECT_DIR/../" --json
```

## Common errors

| Situation | What to do |
|-----------|------------|
| `scan_root` not set (no config) | Set `paths.scan_root` in the config or pass `--root <PATH>` |
| `scan_root` missing / inaccessible | Check the path; error exits with code `1` |
| "I don't see a new project" | The cache has probably kicked in — run with `--force-refresh` |
| Project deeper than expected | Increase `--max-depth` (default is `6`) |
| `cache: miss` every time | Normal when `--max-depth` or the binary version changed |

## Security

- The command is **read-only**: it neither writes to the den nor deletes files.
- File contents never appear in output — only names, stacks, and sizes.
- Never prints or logs passwords/secrets — finding those is `racc dig`'s job, not sniff's.

## Related commands

| Command | Role |
|---------|------|
| `racc dig` | Find and classify secrets (read-only) |
| `racc stash` | Move secrets into an encrypted age archive in the den |
| `racc rinse` | Delete build trash by strategies |
| `racc pack` | Pack a project without secrets into `packs/` |
| `racc raid` | Full cycle in one command |
| [Supported catalog](/supported) | Full list of markers, priorities, and frameworks |

---

*This document matches the implementation; when CLI flags change, update the page in the same PR.*
