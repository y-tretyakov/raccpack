---
title: Raid — full cycle in one command
description: "The racc raid command — stash → rinse → pack → move in one call: secrets into an age archive, build trash cleanup, project archive and a manifest in the den."
---

# Raid - full cycle in one command

Command: `racc raid`  
Status: implemented (Alpha).

This page describes **exactly the behavior** that `raccpack` implements today. If a flag or path is not listed here, it does not exist in the current version.

Back to the command overview: [CLI usage](/cli-usage).

## What raid does

`racc raid` runs the whole pipeline for a project in one command, in a fixed order:

```text
stash  →  rinse  →  pack  →  move
```

1. **stash** — finds sensitive files (same rules as `racc dig`) and encrypts them into an age archive in `den/secrets/…`, deleting the originals by default;
2. **rinse** — removes build trash (`node_modules`, `target`, … by strategies);
3. **pack** — packs the project **without** secrets into `den/packs/…`;
4. **move (commit)** — finalizes the placement and, after success, writes the manifest.

Result of a single successful run:

```text
{den}/secrets/{year}/{month}/{slug}__{time}__secrets.age
{den}/packs/{year}/{month}/{slug}__{time}.tar.zst
{den}/manifests/{year}/{month}/{slug}__{time}__{id}.json
```

The manifest is a JSON record for auditing: stages, artifact paths (relative to the den), a raw-free stash manifest, the tool version, `success`, `dry_run`, `created_at`. It is written **only** after a successful commit and only if the artifacts were actually placed.

By default `racc raid` runs in **dry-run**: nothing is written and nothing is deleted.

::: info
The default mode is **atomic**: all intermediate files live in `den/staging/{id}/`, deletion of sources and trash is postponed to commit, and every commit step is recorded in a journal (WAL). If commit fails halfway, the placed artifacts are **rolled back** (`rolled_back`). See [Orphan green](#atomic-vs-fail-fast-orphan-green).
:::

## Quick start

```bash
# 1) Preview what will be done (nothing is written or deleted)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den

# 2) Full commit (stash + rinse + pack + manifest)
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes
```

::: warning
By default `racc raid` runs in **dry-run**: it writes nothing to the den, deletes no sources and no trash. Commit happens only with `--yes`.
:::

## Syntax

```text
racc raid --project <PATH> [OPTIONS]
```

`--project <PATH>` is **required**: the project directory the pipeline is executed against.

## Options and flags

### Project and den

| Option | Description |
|----------|----------|
| `--project <PATH>` | Project directory (required) |
| `--den <PATH>` | Den root. If omitted — taken from the config (`paths.den_dir`) |

### Write mode

| Option | Behavior |
|----------|-----------|
| *(default)* | **Dry-run**: report only, no files created or deleted |
| `--dry-run` | Explicit dry-run |
| `--yes` | **Commit**: write artifacts to the den, apply deletions |

**Precedence:** if both `--dry-run` and `--yes` are specified, dry-run wins.

### Phases

| Option | Default | Description |
|----------|--------------|----------|
| *(no flag)* | all phases enabled | stash → rinse → pack |
| `--no-stash` | — | Disable stash (no secret search/encryption, no source deletion) |
| `--no-rinse` | — | Disable rinse (no build trash cleanup) |
| `--no-pack` | — | Disable pack (no `tar.zst` is created) |
| `--fail-fast` | — | `FailFast` mode instead of atomic: stop at the first failing phase (see below) |

### Stash / pack fine-tuning

| Option | Default | Description |
|----------|--------------|----------|
| `--min-risk <LEVEL>` | `high` | Minimum risk level for stash: `low`, `medium`, `high`, `critical` |
| `--keep-sources` | off | Do not delete the original secrets after a successful stash (`remove_sources` disabled) |
| `--no-content-deny` | off | Do not exclude files with secret content from pack (name-based deny remains) |

### Output

| Option | Description |
|----------|----------|
| `--json` | Print `RaidResult` as JSON (stages, `success`, `rolled_back`, artifacts) |

### Global flags

| Flag | Description |
|------|----------|
| `-c, --config <PATH>` | Configuration file (overrides `RACCPACK_CONFIG`) |
| `--root <PATH>` | Override `scan_root` for this run |
| `--den <PATH>` | Override `den_dir` for this run |
| `--json` | Machine-readable JSON output |

## Passphrase

Needed **only** if stash is enabled **and** the run performs a Commit. With `--no-stash`, no passphrase is requested even with `--yes`.

Resolution order (same as `racc stash`):

1. The **`RACCPACK_PASSPHRASE`** environment variable — if set and non-empty.
2. Otherwise interactive input on a TTY (twice, without echoing characters).
3. If stdin is not a terminal, a **single line is read from stdin**.
4. If neither env, nor TTY, nor stdin is available — an error suggesting you set `RACCPACK_PASSPHRASE`.

::: warning
Do not commit `RACCPACK_PASSPHRASE` and do not store the passphrase in plain scripts. In CI, provide the variable through a secrets store.
:::

## Atomic vs fail-fast (orphan green)

### Atomic (default)

- All intermediate artifacts live in `den/staging/{id}/`.
- Deletion of sources (`remove_sources`) and trash (`rinse`) is postponed to **move (commit)**.
- Every commit effect is recorded in the journal **before** it is applied; a failure halfway rolls back the placed artifacts.
- On rollback the human-readable output shows `Failed` and `rolled back (N warnings)`; in JSON — `rolled_back: true`.
- **Guarantee:** a failed raid leaves no `.age` / `.tar.zst` / manifest in the den (only a temporary `staging/`, which is cleaned up).
- **Audit policy:** if the manifest write fails but the commit itself has already succeeded — this is `success: false` without a rollback (the artifacts stay in the den; there is nothing to roll back). Rollback does not restore deleted sources/trash (the effects of move with `remove_sources` are irreversible) — they end up in `rollback_warnings`.

### Fail-fast (`--fail-fast`)

- Legacy behavior: stops at the first failing phase.
- Already placed artifacts **remain** in the den (this is the documented difference from atomic — an "orphan").
- Used for debugging; atomic is preferred in normal work.

## Exit codes

| Code | When |
|-----|--------|
| 0 | `Ok` and `success == true` (including dry-run) |
| 1 | A CLI/config/phase error **or** `Ok` with `success == false` (incl. commit rollback) |

Code `2` (as dig uses for Critical) is **not** used for raid.

## Output

### Human-readable

During the run, phase lines are printed (`→ stash: …`, `→ rinse: …`, `→ pack: …`, `→ move: …`), then the summary:

```text
Success
  placed 2 artifact(s):
    /tmp/den/secrets/2026/08/my-api__20260804T155230Z__secrets.age
    /tmp/den/packs/2026/08/my-api__20260804T155230Z.tar.zst
```

On rollback:

```text
Failed
  rolled back (1 warnings)
```

### JSON (`--json`)

Fields of `RaidResult`: `stages` (name/success/message), `stash`/`rinse`/`pack` sub-results, `den_artifacts`, `success`, `dry_run`, `rolled_back`, `rollback_warnings`. There are no raw secrets in the JSON.

## Examples

```bash
# Dry-run: show the whole pipeline, write nothing
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den

# Full atomic commit
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# Without stash (leave secrets alone; no passphrase needed)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-stash

# Do not delete the original secrets
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --keep-sources

# Debug fail-fast (an orphan is possible)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --fail-fast

# JSON for CI + checking the rollback fields
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --json \
  | jq '{success, rolled_back, stages}'

# Critical-only secrets
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --min-risk critical

# Exit code: success=false → 1
racc raid --project /bad --den /tmp/den --yes ; echo $?
```

## Security

- Dry-run by default — look at the report first.
- Sources and trash are deleted only in commit (`--yes`) and **after** the artifacts have been placed successfully.
- In atomic mode a failed commit is rolled back: no artifacts are left in the den.
- The passphrase is never written to logs or JSON; key material is zeroized.
- The manifest file and `.age` files are created with `0600` permissions (best-effort on Unix).
- Do not commit the den directory to git.

## Related commands

| Command | Role |
|---------|------|
| `racc sniff` / `racc dig` | Find projects / secrets (read-only) |
| `racc stash` | Secrets only → age archive |
| `racc rinse` | Trash cleanup only |
| `racc pack` | Project archive without secrets only |

---

*The documentation matches the implementation; when CLI flags change, update this page in the same PR.*
