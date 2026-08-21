---
title: Stash — move secrets into an encrypted archive
description: The racc stash command — collect a project's sensitive files into an encrypted age archive in the den, optionally removing the originals.
---

# Stash - move secrets into an encrypted archive (age)

Command: `racc stash`  
Status: implemented (Alpha).

This page describes **exactly the behavior** that `raccpack` implements today. If a flag or path is not listed here, it does not exist in the current version.

Back to the command overview: [CLI usage](/cli-usage).

## What stash does (and does not do)

`racc stash` moves a project's sensitive files into an encrypted archive:

1. Finds sensitive files in the project (by name and, optionally, by content — the same rules as `racc dig`).
2. Packs them into a single **tar** archive, then encrypts it with **age** using a **passphrase**.
3. Places the file in the **den**:

   ```text
   {den}/secrets/{year}/{month}/{slug}__{UTC-time}__secrets.age
   ```

4. Optionally **deletes** the original files from disk (only after the archive is written successfully, and only with an explicit flag).

Raw secrets are **not** printed to the terminal and **do not** appear in the JSON report.

What stash does **not** do:

- without `--yes` nothing is written or deleted (default is **dry-run**);
- does not decrypt archives — use `age` outside of `racc` for that (see [Manual decryption (`age -d`)](#manual-decryption-age-d));
- never uses exit code `2` (that is specific to `dig`).

## Quick start

```bash
# 1) See what would happen (nothing is written or deleted)
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den

# 2) Actually create the .age in the den (sources are NOT removed)
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# 3) Create the .age and remove the original secret files
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --remove-sources
```

::: warning
By default `stash` runs as **dry-run**: no `.age` is created, the den is untouched, no files are deleted. Writing to the den happens only with `--yes`.
:::

::: danger
`--remove-sources` **deletes** the original secret files from disk after a **successful** Commit (`--yes`). Run a dry-run without `--yes` first. In CI, do not pass `--remove-sources` while the job still needs those files.
:::

Interactive (no env): run with `--yes` in a terminal — the CLI asks for the passphrase twice (input hidden).

## Syntax

```text
racc stash --project <PATH> [OPTIONS]
```

`--project <PATH>` is a **required** option: the project directory (or subtree) to search for secrets.

## Options and flags

### Project (required)

| Option | Description |
|--------|-------------|
| `--project <PATH>` | Project directory (or subtree) to search for secrets |

### Den

| Option | Description |
|--------|-------------|
| `--den <PATH>` | Den root. If omitted — taken from config (`paths.den_dir`), usually `~/.raccpack/den` |

### Write mode

| Option | Behavior |
|--------|----------|
| *(default)* | **Dry-run**: report only; no files created or deleted |
| `--dry-run` | Explicit dry-run |
| `--yes` | **Commit**: write the `.age` into the den |

**Priority:** if both `--dry-run` and `--yes` are given, dry-run wins — nothing is written or deleted.

### Secrets and removal

| Option | Default | Description |
|--------|---------|-------------|
| `--min-risk <LEVEL>` | `high` | Minimum risk level: `low`, `medium`, `high`, `critical` |
| `--remove-sources` | off | After a **successful** Commit, delete the source files |
| `--only <PATH>` | all found | Repeatable: archive only the listed files (must live inside `--project`) |
| `--batch-id <ID>` | none | Replaces the UTC time token in the file name: `{slug}__{ID}__secrets.age` |

`--remove-sources` is **ignored** in dry-run (nothing will be deleted).

### Output

| Option | Description |
|--------|-------------|
| `--json` | Print `StashResult` as JSON (paths, counters, manifest **without** secret contents) |

### Global flags

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Config file (overrides `RACCPACK_CONFIG`) |
| `--root <PATH>` | Override `scan_root` for this run |
| `--den <PATH>` | Override `den_dir` for this run |
| `--json` | Machine-readable JSON output |

## Behavior

### Modes: dry-run and commit

- The default is **dry-run**: nothing is created or deleted; `archive_path` in the report is the expected path.
- **Commit** requires `--yes`. Fail-safe operation order: encryption → placement in den → (optionally) source removal. An encryption or placement error **never** removes sources.

### Passphrase

Needed only in **Commit** mode; never asked for in dry-run. Resolution order:

1. The **`RACCPACK_PASSPHRASE`** environment variable — if set and non-empty.
2. Otherwise interactive input on a TTY (twice for confirmation, input hidden).
3. If stdin is not a terminal (e.g., piped in CI), a **single line from stdin** is used.
4. With no env, no TTY, and no stdin — error suggesting you set `RACCPACK_PASSPHRASE`.

Recommendations:

::: warning
Never commit `RACCPACK_PASSPHRASE` or store the passphrase in plain scripts. In CI, provide it via a secrets store.
:::

- After the command, the process does not need to retain the password; core zeroizes the key material. The value is never logged or included in JSON.

### Den structure after stash

```text
~/.raccpack/den/
├── .den-version          # 1
├── README.txt
├── secrets/
│   └── 2026/
│       └── 08/
│           └── my-api__20260804T155230Z__secrets.age
├── packs/                # from racc pack
├── staging/              # temporary; cleaned after success
└── …
```

File name:

```text
{project_slug}__{YYYYMMDDThhmmssZ}__secrets.age
```

- `project_slug` — the project folder name, safe characters `[a-zA-Z0-9._-]`, spaces → `-`, length ≤ 80.
- Time is **UTC**.
- With `--batch-id <ID>` the time token in the name is replaced by `ID`: `{slug}__{ID}__secrets.age`. The year/month directories (`secrets/{yyyy}/{mm}`) still come from current UTC time.

### What goes into the archive

- Files that `racc dig` would find with risk **≥ `--min-risk`** (by default High and Critical).
- Typical name examples: `.env`, `.env.*`, `id_rsa`, `*.pem`, `.npmrc`, `credentials`, …  
  The exact set matches the secrets engine's filename/content rules (see dig).

Excluded:

- directories like `node_modules`, `target` (skip policy);
- files below the risk threshold;
- with `--only` — everything not listed.

## Output

### Human-readable (human)

Dry-run:

```text
Stash (dry-run)
  Would archive: 1 files → /tmp/den/secrets/2026/08/app__20260815T141227Z__secrets.age
  Would remove sources: no (--remove-sources not set)
  (nothing written or deleted)
```

With `--remove-sources` set, the second line becomes `Would remove sources: yes (requires --yes)`.

Commit:

```text
Stash complete
  Archive: /tmp/den/secrets/2026/08/app__20260815T141227Z__secrets.age
  Files: 1  (21 B plaintext)
  Removed sources: 0
```

### JSON (`--json`)

| Field | Meaning |
|-------|---------|
| `archive_path` | Path to the `.age` (expected path in dry-run) |
| `files_archived` | Number of files |
| `bytes_archived` | Total plaintext size |
| `removed_sources` | How many sources were removed (0 in dry-run) |
| `dry_run` | `true` / `false` |
| `manifest` | List of `{ original_path, risk, size_bytes }` **without** file contents |

Example:

```json
{
  "archive_path": "/tmp/den/secrets/2026/08/app__20260815T141227Z__secrets.age",
  "files_archived": 2,
  "bytes_archived": 91,
  "removed_sources": 0,
  "dry_run": false,
  "manifest": [
    { "original_path": "/tmp/app/.env", "risk": "High", "size_bytes": 21 }
  ]
}
```

## Exit codes

| Code | When |
|------|------|
| 0 | Success (including dry-run) |
| 1 | Error: missing project/den, empty passphrase, nothing to archive, IO, encrypt |

Code `2` (as in dig for Critical) is **not** used by stash.

## Examples

```bash
# Locally: dry-run — show what would be archived (nothing written)
racc stash --project ~/DEV/PROJS/my-api

# Commit: create the .age in the den (sources are not removed)
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# Commit and remove the original secret files
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --remove-sources

# JSON for scripts and CI
racc stash --project ~/DEV/PROJS/my-api --yes --json

# Archive only specific files (repeatable flag)
racc stash --project ~/DEV/PROJS/my-api --yes --only ~/DEV/PROJS/my-api/.env --only ~/DEV/PROJS/my-api/id_rsa

# Custom artifact name instead of timestamp
racc stash --project ~/DEV/PROJS/my-api --yes --batch-id release-42
# → …/secrets/2026/08/my-api__release-42__secrets.age

# Lower the risk threshold (archive Medium too)
racc stash --project ~/DEV/PROJS/my-api --min-risk medium --dry-run

# --dry-run always wins over --yes: nothing is written
racc stash --project ~/DEV/PROJS/my-api --yes --dry-run
```

### CI examples

::: code-group

```bash [bash]
# bash / zsh
export RACCPACK_PASSPHRASE="$STASH_SECRET"   # from CI secrets
racc stash --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes --json
# CI agents usually keep their sources:
# don't pass --remove-sources while the job still needs its artifacts
```

```fish [fish]
set -gx RACCPACK_PASSPHRASE $STASH_SECRET   # from CI secrets
racc stash --project $CI_PROJECT_DIR --den $DEN_PATH --yes --json
```

```nu [nu]
$env.RACCPACK_PASSPHRASE = $env.STASH_SECRET   # from CI secrets
racc stash --project $env.CI_PROJECT_DIR --den $env.DEN_PATH --yes --json
```

```powershell [pwsh]
$env:RACCPACK_PASSPHRASE = $env:STASH_SECRET   # from CI secrets
racc stash --project $env:CI_PROJECT_DIR --den $env:DEN_PATH --yes --json
```

:::

### Manual decryption (`age -d`)

Decrypting archives via `racc` is **not** part of the CLI in Alpha A1 (use the official [age](https://github.com/FiloSottile/age) tool when needed). Inside the archive after decryption is a **tar** with relative file paths.

```bash
age -d -o secrets.tar /path/to/…__secrets.age
tar -tf secrets.tar
tar -xf secrets.tar -C /safe/restore/dir
```

## Common errors

| Situation | What to do |
|-----------|------------|
| `nothing to stash: no files matched the current min-risk threshold` | Lower `--min-risk` or check `racc dig --project …` |
| No passphrase | Set `RACCPACK_PASSPHRASE` or run in an interactive terminal |
| `--remove-sources` did not remove anything | Requires `--yes` (Commit) and a successful completion without errors |
| Path outside project for `--only` | Provide paths strictly inside `--project` |

## Security

- Dry-run by default — review the report first.
- Sources are removed only with `--yes --remove-sources`, and strictly **after** the archive has been placed successfully in the den.
- The passphrase is never logged or included in JSON; core zeroizes the key material.
- The `.age` file is created with **`0600`** permissions (best-effort on Unix).
- Recommended den permissions: `0700`.
- Never commit the den directory to git.

## Related commands

| Command | Role |
|---------|------|
| `racc dig` | Find secrets (read-only) |
| `racc sniff` | Find projects under `scan_root` |
| `racc pack` | Pack a project **without** secrets into `packs/` |
| `racc raid` | Full cycle in one command: stash → rinse → pack → move |

---

*This document matches the implementation; when CLI flags change, update the page in the same PR.*
