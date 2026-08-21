---
title: Rinse — clean build trash
description: The racc rinse command — remove known build artifact directories (target, node_modules, caches) by strategies, dry-run by default.
---

# Rinse - clean build trash

Command: `racc rinse`  
Status: implemented (Alpha).

This page describes **exactly the behavior** that `raccpack` implements today. If a flag or path is not listed here, it does not exist in the current version.

Back to the command overview: [CLI usage](/cli-usage).

## What rinse does (and does not do)

`racc rinse` removes **known build artifact directories** inside a project according to rule sets — **strategies**:

- `target` (Rust),
- `node_modules`, `.next`, `dist`, … (Node),
- `__pycache__`, `.venv`, … (Python),
- and other enabled strategies (see [Strategies (`--strategy`)](#strategies-strategy)).

By default the command runs as **dry-run**: it only shows what would be removed. Actual deletion requires `--yes`.

What rinse does **not** do:

- does not look for or touch secrets — that's `racc stash`;
- does not create archives — that's `racc pack`;
- does not delete arbitrary user files outside the strategy table;
- never uses exit code `2` (that is specific to `dig`);
- needs no passphrase and has no `--remove-sources` / `--only` flags (those are `stash` options).

## Quick start

```bash
# 1) See what would be removed (safe, nothing deleted)
racc rinse --project ~/DEV/PROJS/my-api

# 2) Remove the found trash
racc rinse --project ~/DEV/PROJS/my-api --yes
```

::: warning
By default `rinse` runs as **dry-run**: nothing is deleted. Actual directory removal happens only with `--yes`.
:::

::: info
Before `--yes`, always run a dry-run and read the list of paths: `dist`, `build`, and `vendor` are "cautious" names and are not in the default strategy set (see [Strategies (`--strategy`)](#strategies-strategy)).
:::

## Syntax

```text
racc rinse --project <PATH> [OPTIONS]
```

`--project <PATH>` is a **required** option: the project directory (or subtree) to search for build trash.

## Options and flags

### Project (required)

| Option | Description |
|--------|-------------|
| `--project <PATH>` | Project directory (or subtree) to search for trash. May be relative — e.g. `--project .` from within the project directory |

### Write mode

| Option | Behavior |
|--------|----------|
| *(default)* | **Dry-run**: report only; no directories removed |
| `--dry-run` | Explicit dry-run |
| `--yes` | **Commit**: actually delete the found directories |

**Priority:** if both `--dry-run` and `--yes` are given, dry-run wins — nothing is deleted.

### Strategies

| Option | Default | Description |
|--------|---------|-------------|
| `--strategy <ID>` | from `config.cleanup.enabled_strategies` | Repeatable strategy filter. Without the flag, strategies come from configuration |

### Output

| Option | Description |
|--------|-------------|
| `--json` | Print `RinseResult` as JSON (see [JSON (`--json`)](#json-json)) |

### Global flags

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Config file (overrides `RACCPACK_CONFIG`) |
| `--root <PATH>` | Override `scan_root` for this run; a relative `--project` resolves against it |
| `--den <PATH>` | Override `den_dir` for this run. **Not used** by `rinse` — rinse never writes to the den |
| `--json` | Machine-readable JSON output |

::: info
`--den` is accepted (it's a global flag) but has no effect on `rinse`: cleaning trash never touches the den.
:::

## Strategies (`--strategy`)

| ID | Typically removed |
|----|-------------------|
| `rust` | `target` |
| `node` | `node_modules`, `.next`, `dist`, `.nuxt`, `coverage` |
| `python` | `__pycache__`, `.venv`, `venv`, `.tox`, `.mypy_cache`, `.pytest_cache`, `*.egg-info`, `.ruff_cache` |
| `jvm` | `build`, `.gradle`, `.m2` |
| `go` | `vendor` |
| `generic` | `.cache`, `tmp`, `temp` |

`dist`, `build`, `vendor`, `tmp`/`temp` are "cautious" names: sometimes they are not trash but real sources or user data. That's why only `rust`, `node`, and `python` are enabled by default; `jvm`, `go`, and `generic` must be opted in explicitly — via `--strategy` or in configuration.

### Config (`config.toml`)

```toml
[cleanup]
enabled_strategies = ["rust", "node", "python"]
```

This is the default when no `--strategy` is passed on the CLI. An unknown id (in config or CLI) is an error, exit `1`.

## Dry-run vs Commit mode

| Mode | Flag | Filesystem |
|------|------|------------|
| Dry-run | default or `--dry-run` | Directories are **not** removed; report lists everything found |
| Commit | `--yes` | Found trash directories are removed (see [Security](#security)) |

## Output

### Human-readable (human)

Dry-run:

```text
Rinse (dry-run)
  Project: /home/user/DEV/PROJS/my-api
  Would remove 2 directories (140.2 MiB)
    node_modules  [node]  120.0 MiB
    target        [rust]   20.2 MiB
  (nothing deleted)
```

Commit:

```text
Rinse complete
  Removed 2 directories, freed 140.2 MiB
```

### JSON (`--json`)

| Field | Meaning |
|-------|---------|
| `removed` | Array of `{ path, strategy, pattern_name, size_bytes }` objects |
| `bytes_freed` | Sum of sizes (estimate in dry-run; actually freed in commit) |
| `dry_run` | `true` / `false` |

In dry-run, `removed` holds **candidates** (what would be removed), not "already removed" items.

Example:

```json
{
  "removed": [
    {
      "path": "/home/user/DEV/PROJS/my-api/node_modules",
      "strategy": "node",
      "pattern_name": "node_modules",
      "size_bytes": 125829120
    },
    {
      "path": "/home/user/DEV/PROJS/my-api/target",
      "strategy": "rust",
      "pattern_name": "target",
      "size_bytes": 21181235
    }
  ],
  "bytes_freed": 147010355,
  "dry_run": true
}
```

## Exit codes

| Code | When |
|------|------|
| 0 | Success (including dry-run) |
| 1 | Error: missing `--project` (usage), unknown strategy, IO during removal |

Code `2` (as in dig for Critical) is **not** used by rinse.

## Examples

```bash
# Locally: dry-run — show what would be removed (nothing deleted)
racc rinse --project ~/DEV/PROJS/my-api

# Explicit dry-run
racc rinse --project ~/DEV/PROJS/my-api --dry-run

# Commit: actually remove the found trash
racc rinse --project ~/DEV/PROJS/my-api --yes

# Cargo target/ only
racc rinse --project ~/DEV/PROJS/my-api --strategy rust --yes

# Node trash only (node_modules, .next, …)
racc rinse --project ~/DEV/PROJS/my-api --strategy node --yes

# Rust + Node in one pass (repeatable flag)
racc rinse --project ~/DEV/PROJS/my-api --strategy rust --strategy node --yes

# JVM build directories (off by default — explicit only)
racc rinse --project ~/DEV/PROJS/my-api --strategy jvm --yes

# Go vendor/ (off by default — explicit only)
racc rinse --project ~/DEV/PROJS/my-api --strategy go --yes

# Generic: .cache, tmp, temp (off by default — explicit only)
racc rinse --project ~/DEV/PROJS/my-api --strategy generic --yes

# --dry-run always wins over --yes: nothing is deleted
racc rinse --project ~/DEV/PROJS/my-api --yes --dry-run

# Project relative to current directory
cd ~/DEV/PROJS/my-api
racc rinse --project . --yes
```

### CI examples

```bash
# Check whether there is anything to clean (dry-run JSON)
racc rinse --project "$CI_PROJECT_DIR" --json

# Remove only node_modules on the CI agent after build
racc rinse --project "$CI_PROJECT_DIR" --strategy node --yes --json

# Count "how much would be freed" without deleting (jq)
racc rinse --project ~/DEV/PROJS/my-api --json | jq '.bytes_freed'

# List candidate paths
racc rinse --project ~/DEV/PROJS/my-api --json | jq -r '.removed[].path'
```

## Common errors

| Situation | What to do |
|-----------|------------|
| `error: invalid configuration: unknown cleanup strategy \`foo\`` | Check the strategy id: `rust`, `node`, `python`, `jvm`, `go`, `generic` |
| Nothing was removed | `--yes` (Commit) required; the strategy is not enabled (default is only `rust`, `node`, `python`); or the directory name is not in the strategy table |
| `--project` is required | Pass `--project <PATH>`; parse error exits with `1` |
| Can I get `node_modules` back? | Only by reinstalling dependencies (`npm install` etc.). Rinse keeps no backup |
| Will secrets in `.env` be deleted? | No — `.env` is not a trash directory of any strategy. For secrets use `racc stash` |

## Security

- Only directories matching **strategies** inside `--project` are removed (path containment).
- The walk does **not** follow symlinks (`follow_links(false)`); symlinks to directories are neither removed nor walked — external trees are untouched.
- Dry-run by default — review the report first.
- "Cautious" names (`dist`, `build`, `vendor`) exist in strategies but not in the default set: enable `jvm`, `go`, and `generic` explicitly.
- Rinse is not "delete everything except `src`" and not an antivirus replacement: only the strategy table applies.

## Related commands

| Command | Role |
|---------|------|
| `racc dig` | Find secrets (read-only) |
| `racc stash` | Move secrets into an `.age` archive |
| `racc pack` | Pack a project **without** secrets into `packs/` |
| `racc raid` | Full cycle in one command: stash → rinse → pack → move |

---

*This document matches the implementation; when CLI flags change, update the page in the same PR.*
