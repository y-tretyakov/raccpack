---
title: Dig — find secrets
description: The racc dig command — search for secrets and assess risks in a project (read-only).
---

# Dig - find secrets

Command: `racc dig`  
Status: implemented.

This page describes **exactly the behavior** that `raccpack` implements today. If a flag or path is not listed here, it does not exist in the current version.

> Back to the command overview: [CLI usage](/cli-usage).

## What it does

1. Walks `scan_root` (or a single project with `--project`) and finds sensitive files.
2. Classifies each finding by risk level: `Low`, `Medium`, `High`, `Critical`.
3. By default also checks file contents; `--no-content` limits the search to file names.
4. On request (`--repeated`) finds values that repeat across two or more files.
5. Determines the git status of each finding (`git_status`, best-effort).
6. Exits with code `2` when findings exceed the `--fail-on` policy threshold.

What it does **not** do:

- writes and deletes nothing (read-only) — neither in the den nor on disk;
- does not use the `sniff` cache and never touches the den;
- never prints raw secret values — only masked previews, the blake3 hash, and the length.

::: info
Exit code **2** means the `--fail-on` policy triggered (by default — Critical findings), not a CLI failure. Code **1** is an execution error (paths, IO, config).
:::

## Quick start

```bash
# All projects under scan_root
racc dig

# A single project
racc dig --project ~/DEV/PROJS/my-api

# JSON for scripts and CI
racc dig --project ~/DEV/PROJS/my-api --json
```

## Syntax

```text
racc dig [OPTIONS]
```

`--project` is optional: without it, `scan_root` is scanned.

## Options and flags

### Command flags

| Flag | Default | Description |
|------|---------|-------------|
| `--project <PATH>` | `scan_root` | Limit scanning to one directory (may live outside `scan_root`) |
| `--no-content` | off | Do not read contents — match file names only |
| `--repeated` | off | Find values repeated across ≥ 2 files (grouped by blake3 hash) |
| `--fail-on <POLICY>` | `critical` | Exit policy: `ignore` — never fail because of findings; `critical` — fail only on Critical; `high` — fail on High and above |
| `--max-depth <N>` | from config (`scanner.max_depth`, default `6`) | Limit the walk depth |

### Global flags

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Config file (overrides `RACCPACK_CONFIG`) |
| `--root <PATH>` | Override `scan_root` for this run |
| `--den <PATH>` | Override `den_dir` — accepted as a global flag, but has no effect on `dig` results |
| `--json` | Print JSON instead of the human-readable report |

Priorities:

- `--project` takes precedence over `scan_root`/`--root`;
- `--max-depth` takes precedence over `scanner.max_depth` from config;
- the exit threshold is set by `--fail-on` (default `critical`).

## Behavior

- **Read-only**: `dig` always runs as dry-run and never creates/deletes files.
- **Cache**: the `sniff` cache is not used — scanning is always fresh.
- **Contents**: files larger than 1 MiB and binary files (null byte within the first 8 KiB) are skipped during content scanning — see [Concepts](/concepts) for details.
- **Skip policy**: directories such as `node_modules`, `target`, `.git`, `dist`, `build` are not walked.
- **Findings**: include all risk levels starting from `Low`.
- **Sorting**: the findings table sorts by risk descending, then by path ascending.
- **`--repeated`**: grouping uses the stable blake3 hash; only values found in at least two files make it into the report.
- **CI/TTY**: the command is fully non-interactive — safe in scripts and CI.
- With a non-zero exit code in non-JSON mode, stderr prints `Sensitive findings triggered exit policy (…)`.

## Output

### Human-readable

```text
Dig root: /tmp/projects
Files scanned: 5  |  Findings: 2  |  Repeated: 0  |  14 ms

RISK      LABEL                  PATH
Critical  SSH private key (RSA)  /tmp/projects/app/id_rsa
High      Environment file       /tmp/projects/app/.env
```

The `Repeated secrets:` block prints only with `--repeated`, and only when repeats exist:

```text
Repeated secrets:
  hash=abcd…  risk=High  count=2
    /tmp/projects/app/.env
    /tmp/projects/app/.env.backup
```

### JSON (`--json`)

```json
{
  "root": "/tmp/projects",
  "files": [
    { "path": "...", "risk": "High", "labels": ["Environment file", "Secret assignment"], "content_match": { "masked": "PASS…et", "value_hash": "d917…bfe", "original_len": 20 }, "git_status": "untracked" }
  ],
  "repeated": [],
  "duration_ms": 18,
  "files_scanned": 5
}
```

Fields:

| Field | Meaning |
|-------|---------|
| `root` | Scanned directory |
| `files` | Array of findings |
| `files[].path` | Path to the file |
| `files[].risk` | Risk level: `Low` / `Medium` / `High` / `Critical` |
| `files[].labels` | Labels: the name-based rule and/or content-based rule |
| `files[].content_match` | `{ masked, value_hash, original_len }` or `null` |
| `files[].content_ref` | `null` or `{ path, marker_id, line, value_hash }` — a stable reference to the content match (internal use in interfaces; `path` and `marker_id` identify the finding, `line` is 1-based, `value_hash` matches `content_match.value_hash`). Never carries the raw value. |
| `files[].git_status` | Git status of the file: `"tracked"` / `"untracked"` / `"ignored"` / `"modified"` / `"staged"` / `"deleted"` / `"unknown"`, or `null` — see [below](#git-status) |
| `repeated` | Repeated values (populated only with `--repeated`) |
| `repeated[].value_hash` | blake3 hash of the value (never the value itself) |
| `repeated[].masked` | Masked preview |
| `repeated[].risk` | Highest risk among occurrences |
| `repeated[].paths` | Files containing the value |
| `repeated[].count` | How many files contain the value |
| `duration_ms` | Scan duration, ms |
| `files_scanned` | Total files walked (with or without findings) |

Masking rules: value ≤ 8 bytes → `"****"`; longer → first 4 characters + `…` + last 2. Raw values never appear in output.

::: tip
Both human and JSON `dig` output never contain raw secrets — only a masked preview, the blake3 hash, and the length.
:::

`content_match` is `null` when only the file name matched (for example, with `--no-content`). `content_ref` is `null` in the same case — a raw reveal is therefore only possible via the interfaces (TUI `v`), never through the CLI.

### git_status

The `files[].git_status` field is the file's state in git at scan time. Values are stable snake_case strings:

| Value | Meaning |
|-------|---------|
| `tracked` | File is tracked by git, no changes |
| `untracked` | File is not tracked by git |
| `ignored` | File matches ignore rules (`.gitignore` etc.) |
| `modified` | File is tracked and modified (in the working tree or index) |
| `staged` | File changes added to the index (new, renamed, or copied) |
| `deleted` | File deleted |
| `unknown` | Status could not be determined |

The field is `null` when there is no status: the project is outside a git repository, `git` is not installed, or git failed or timed out. Git status is best-effort: a git failure does **not** affect the `dig` result — report and exit code stay the same.

::: tip
In CI it is convenient to look only at path, risk, and status of findings:

```bash
racc dig --project ~/DEV/PROJS/my-api --json | jq '.files[] | {path, risk, git_status}'
```
:::

## Exit codes

| Code | When |
|------|------|
| `0` | No findings above the policy threshold |
| `1` | Execution error: missing `scan_root`, directory missing/not a directory, IO error |
| `2` | Findings above the `--fail-on` threshold (default — Critical) |

Code `2` fires in JSON mode too (in CI, check the exit code); with `--json` no extra message is printed to stderr.

## Examples

```bash
# Full check of all projects under scan_root
racc dig

# A single project
racc dig --project ~/DEV/PROJS/app-api

# File names only (faster, no content reading)
racc dig --project ~/DEV/PROJS/app-api --no-content

# Secrets repeated across files
racc dig --project ~/DEV/PROJS/app-api --repeated

# JSON for CI
racc dig --project "$CI_PROJECT_DIR" --json

# Never fail because of findings
racc dig --project ~/DEV/PROJS/app-api --fail-on ignore

# Fail already at High
racc dig --project ~/DEV/PROJS/app-api --fail-on high

# Limit walk depth
racc dig --project ~/DEV/PROJS/app-api --max-depth 3

# Override scan_root via global flag
racc dig --root ~/DEV/PROJS
```

## Common errors

| Situation | What to do |
|-----------|------------|
| "no scan_root" / directory error | Set `scan_root` in config or use `--project`/`--root` |
| Unexpected exit code `2` | That's the `--fail-on` policy (default Critical), not a failure; if needed `--fail-on ignore` or `--fail-on high` |
| A file has a secret but content scan missed it | The file is larger than 1 MiB or binary — content scanning skips it; it may still be found by file name |
| `--repeated` shows nothing | Requires a content match with identical blake3 hash in at least two files |
| Directory is not walked | Check the skip policy (`node_modules`, `target`, `dist`, …) and `--max-depth` |

## Security

- The command is read-only — safe to run at any time.
- Raw secret values are neither printed nor included in JSON: only masked preview, stable blake3 hash, and length.
- In CI, prefer `--json` + exit code as a gate instead of dumping unmasked findings into logs.

## Related commands

| Command | Role |
|---------|------|
| `racc sniff` | Find projects under `scan_root` |
| `racc stash` | Move discovered secrets into an encrypted age archive in the den |
| `racc rinse` | Delete build trash by strategies |
| `racc pack` | Pack a project **without** secrets into `packs/` |
| `racc raid` | Full cycle in one command |
| [Supported catalog](/supported) | Full list of name-based and content-based rules |
| [Concepts](/concepts) | Risks, masking, skip policy |

*This document matches the implementation; when CLI flags change, update the page in the same PR.*
