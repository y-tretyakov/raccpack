---
title: CLI usage
description: Overview of racc commands — global flags and the typical workflow; each command has its own detailed page.
---

# CLI usage

`racc` is the raccpack command line. It suits everyday work, scripts, and CI. This page is a short overview: global flags, the typical workflow, and the command list; detailed pages for each command are linked below.

## Global flags

Flags may be placed before or after the subcommand.

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Config file (overrides `RACCPACK_CONFIG`) |
| `--root <PATH>` | Override `scan_root` for this run (also available as a per-command flag) |
| `--den <PATH>` | Override `den_dir` for this run (optional for `sniff`) |
| `--json` | Machine-readable JSON output instead of the human-readable table |
| `-v, --verbose` | Verbose logs to stderr (repeatable: `-v` info, `-vv` debug, `-vvv` trace) |

::: info
`--root` and `--den` override the configuration only for the current run and never change it on disk.
:::

::: tip Verbosity and logs
Without `-v`, racc is nearly silent (warn level). Logs always go to **stderr** — stdout stays clean for data and JSON, so `racc dig --json -v` is safe in scripts. The `RUST_LOG` variable, if set, takes precedence over `-v` (e.g., `RUST_LOG=raccpack_core=debug`). Secret values and passphrases never appear in logs — only paths, counters, and the passphrase source.
:::

Help and version work anywhere: `-h, --help` and `-V, --version`.

## Typical workflow

Before the first run, create a configuration with one command — [racc init](/init):

```bash
racc init --scan-root ~/DEV/PROJS
```

The full project cycle:

```bash
racc sniff
racc dig --project <PATH>
racc stash --project <PATH> --yes
racc rinse --project <PATH> --yes
racc pack --project <PATH> --yes
```

- **init** — create the starter `config.toml` (once, before first use);
- **sniff** — find projects under `scan_root`;
- **dig** — find secrets (read-only, writes nothing);
- **stash** — move secrets into an encrypted age archive in the den;
- **rinse** — remove build trash (`target`, `node_modules`, …);
- **pack** — pack the project WITHOUT secrets into `packs/`.

## Commands

### `racc sniff`

Scans `scan_root`, finds projects, and prints a table: name, stack, size, git status, path. Results are cached; `--force-refresh` ignores the cache, `--max-depth N` limits walk depth.

```text
racc sniff [--force-refresh] [--max-depth N]
```

```bash
# Full overview of the projects folder
racc sniff

# Force rescan without cache
racc sniff --force-refresh

# No deeper than 3 levels
racc sniff --max-depth 3

# Machine-readable output for scripts
racc sniff --json
```

Details: [Sniff](/sniff)

### `racc dig`

Searches for sensitive files in `scan_root` (or one project with `--project`) and returns a report with risk levels. In JSON output every finding carries the file's git status (`git_status`; `null` when it cannot be determined). The command is read-only: writes and deletes nothing. By default it exits with code `2` when Critical-or-above secrets are found; the threshold is set by `--fail-on`.

```text
racc dig [--project PATH] [--no-content] [--repeated] [--fail-on ignore|critical|high] [--max-depth N]
```

```bash
# Check all projects
racc dig

# Check one project
racc dig --project ~/DEV/PROJS/app-api

# File names only (faster, no content reading)
racc dig --no-content

# Fail the run already at High findings
racc dig --fail-on high
```

Details: [Dig](/dig)

### `racc pack`

Packs the project directory into a `tar.zst` archive and places it in the den under `packs/{yyyy}/{mm}/`, excluding secrets. Runs as **dry-run** by default and writes nothing — commit requires `--yes`.

```text
racc pack --project PATH [--den PATH] [--yes] [--dry-run] [--no-content-deny] [--zstd-level N] [--output-name NAME]
```

```bash
# Dry-run: show what would be packed (nothing written)
racc pack --project ~/DEV/PROJS/app-api

# Commit: create the archive in the den
racc pack --project ~/DEV/PROJS/app-api --yes

# Custom artifact name instead of slug__timestamp
racc pack --project ~/DEV/PROJS/app-api --yes --output-name snapshot
```

::: warning
By default `pack` runs as **dry-run** and writes nothing. Writing to the den happens only with `--yes`.
:::

Details: [Pack](/pack)

### `racc stash`

Collects the project's sensitive files into a single encrypted **age** archive and places it in the den under `secrets/{yyyy}/{mm}/`, optionally removing the originals. Runs as **dry-run** by default — commit requires `--yes`. The passphrase comes from `RACCPACK_PASSPHRASE`, interactive input, or stdin. Raw secrets are never printed or included in output.

```text
racc stash --project PATH [--den PATH] [--yes] [--dry-run] [--remove-sources] [--min-risk LEVEL] [--only PATH] [--batch-id ID]
```

```bash
# Dry-run: show what would be archived (nothing written)
racc stash --project ~/DEV/PROJS/app-api

# Commit and remove the original secret files
racc stash --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes --remove-sources
```

Commit with an env passphrase for CI (sources are not removed):

::: code-group

```bash [bash]
# bash / zsh
export RACCPACK_PASSPHRASE="$STASH_SECRET"
racc stash --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes
```

```fish [fish]
set -gx RACCPACK_PASSPHRASE $STASH_SECRET
racc stash --project $CI_PROJECT_DIR --den $DEN_PATH --yes
```

```nu [nu]
$env.RACCPACK_PASSPHRASE = $env.STASH_SECRET
racc stash --project $env.CI_PROJECT_DIR --den $env.DEN_PATH --yes
```

```powershell [pwsh]
$env:RACCPACK_PASSPHRASE = $env:STASH_SECRET
racc stash --project $env:CI_PROJECT_DIR --den $env:DEN_PATH --yes
```

:::

::: danger
The `--remove-sources` example deletes original secrets after a successful stash. Details and limitations: [Stash](/stash).
:::

::: warning
By default `stash` runs as **dry-run** and writes nothing. Commit requires `--yes`.
:::

Details: [Stash](/stash)

### `racc rinse`

Removes known build artifact directories from the project according to **strategies** (`target`, `node_modules`, `__pycache__`, …). Runs as **dry-run** by default and deletes nothing — commit requires `--yes`. Without the flag, strategies come from `config.cleanup.enabled_strategies` (default `rust`, `node`, `python`).

```text
racc rinse --project PATH [--strategy ID ...] [--yes] [--dry-run]
```

```bash
# Dry-run: show what would be removed (nothing deleted)
racc rinse --project ~/DEV/PROJS/app-api

# Commit: actually remove the found trash
racc rinse --project ~/DEV/PROJS/app-api --yes

# Node trash only (node_modules, .next, …)
racc rinse --project ~/DEV/PROJS/app-api --strategy node --yes
```

::: warning
By default `rinse` runs as **dry-run** and deletes nothing. Directory removal happens only with `--yes`.
:::

Details: [Rinse](/rinse)

### `racc raid`

Runs the whole pipeline on a project in one command: **stash → rinse → pack → move**. Default mode is **atomic**: intermediate files go to `den/staging/{id}/`, removals are deferred to commit, and a failed commit is rolled back (`rolled_back`). After a successful commit, a manifest is written to `den/manifests/{yyyy}/{mm}/`. Runs as **dry-run** by default — commit requires `--yes`.

```text
racc raid --project PATH [--den PATH] [--yes] [--dry-run] [--no-stash] [--no-rinse] [--no-pack] [--min-risk LEVEL] [--keep-sources] [--no-content-deny] [--fail-fast]
```

```bash
# Dry-run: show the whole pipeline (nothing written)
racc raid --project ~/DEV/PROJS/app-api --den ~/.raccpack/den

# Full commit (stash + rinse + pack + manifest)
export RACCPACK_PASSPHRASE="$STASH_SECRET"
racc raid --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes

# Without stash: leave secrets alone, no passphrase needed
racc raid --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes --no-stash

# Do not remove the original secrets
racc raid --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes --keep-sources
```

Exit: `0` on `success == true`, `1` on error or `success == false` (including a rolled-back commit).

::: warning
By default `raid` runs as **dry-run** and writes/removes nothing. Commit requires `--yes`.
:::

Details: [Raid](/raid)

### `racc init`

Creates a starter configuration file with a commented template (`config_version = 1`) — by default at `~/.config/raccpack/config.toml`. With `--ensure-den` it additionally creates the den skeleton (`.den-version`, `README.txt`). An existing file is overwritten only with `--force`.

```text
racc init [--force] [--scan-root PATH] [--ensure-den]
```

```bash
# Template at ~/.config/raccpack/config.toml
racc init

# Point at the projects folder and create the den right away
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den --ensure-den
```

Details: [Init](/init)

## In development

The following commands are planned for upcoming versions (see the [Roadmap](/roadmap)):

| Command | Purpose | Status |
|---------|---------|--------|
| `racc den` | Den management | Planned |

## Notes

- JSON output never contains raw secret values — only masked previews and hashes (details: [Dig](/dig)).
- Exit code `2` is used only by `dig` (the `--fail-on` policy) and means the policy triggered, not a CLI failure; `pack`, `stash`, `rinse`, and `raid` use only `0` (success) and `1` (error).
