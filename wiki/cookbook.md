---
title: Usage cookbook — scenarios and scripts
description: "Practical raccpack scenarios: onboarding, dry-run safety, a full raid, raiding every project from config, jq JSON pipelines, debugging without leaking secrets — with scripts for bash, fish, Nushell and PowerShell."
---

# Usage cookbook — scenarios and scripts

Status: accurate for **raccpack 0.4.5** (Detect v2 complete: DAG composite detectors, batch raid `racc raid --root`; Beta B1.4 TUI raid done; Visual System 2.0 TUI done).

Ready-made recipes on top of the command surface. If a flag isn't described here or on
the command's page, it doesn't exist in the current version.

Command overview: [CLI usage](/cli-usage) ·
Individual commands: [Sniff](/sniff) · [Dig](/dig) · [Pack](/pack) ·
[Stash](/stash) · [Rinse](/rinse) · [Raid](/raid) · [Init](/init) ·
Config: [Configuration](/configuration) · [Git, init and DX](/git-and-dx)

::: warning Dry-run by default
`pack`, `stash`, `rinse` and `raid` **write and delete nothing** without `--yes`.
If both `--yes` and `--dry-run` are passed — `--dry-run` wins.
:::

## 1. Onboarding: init → sniff → dig

First run: create a config, see what was found, check for leaks.

```bash
racc init --scan-root ~/DEV/PROJS --ensure-den   # config + den in one step
racc sniff                                        # project table (from cache)
racc sniff --force-refresh                        # rescan, bypassing the cache
racc dig                                          # sensitive files across the whole scan_root
```

Expected result: `init` writes `~/.config/raccpack/config.toml` (paths can be
overridden) and creates the den with `--ensure-den`. `sniff` shows projects,
stack and size; `dig` — a list of findings with risks Critical…Low.

More details: [Init](/init), [Sniff](/sniff), [Dig](/dig).

## 2. Dry-run safety

Any dangerous operation first runs as a rehearsal — this is the default behavior:

```bash
racc pack  --project ~/DEV/PROJS/my-api     # shows the archive plan, writes nothing
racc rinse --project ~/DEV/PROJS/my-api     # shows what it would delete
racc raid  --project ~/DEV/PROJS/my-api     # a full run with no consequences
```

Expected result: a report on stdout, the den unchanged, files in the project intact.
Committing happens only with an explicit `--yes`.

## 3. Full raid of a single project

A passphrase is needed only if stash is enabled (it is by default) and a Commit runs.
From a TTY it is asked for twice with confirmation; in scripts — via the
`RACCPACK_PASSPHRASE` env var.

```bash
export RACCPACK_PASSPHRASE='…'   # placeholder — substitute your own secret
racc raid --project ~/DEV/PROJS/my-api --yes
unset RACCPACK_PASSPHRASE
```

Phase order: stash → rinse → pack → move. Any failed phase rolls back the entire
operation (atomic by default; `--fail-fast` is a debugging mode).

::: danger Passphrase
The passphrase cannot be recovered. Lose it — and the age secret archives become unreadable.
`racc` never logs or stores it; the examples above use a placeholder.
:::

## 4. Raid of all projects from scan_root

The simplest way to raid every project under a directory is `racc raid --root`:

```bash
# Dry-run first (the default)
racc raid --root ~/DEV/PROJS

# Commit for real
racc raid --root ~/DEV/PROJS --yes
```

`--root` discovers projects under the given directory (using the same markers as
`sniff`), then raids each one sequentially. Combine with `--only` and `--limit`
to narrow the list:

```bash
# Only Rust projects
racc raid --root ~/DEV/PROJS --only rust --yes

# First 5 projects, stop on first error
racc raid --root ~/DEV/PROJS --limit 5 --stop-on-error --yes
```

::: tip Without stash
If none of the projects contain secrets, skip stash to avoid the passphrase prompt:
`racc raid --root ~/DEV/PROJS --yes --no-stash`.
:::

::: details Advanced: custom filter with a shell script
When you need a filter that `--only` can't express (e.g. exclude a specific
project, or use external metadata), loop over `racc raid --project` yourself.

The scripts below read projects from `racc sniff --json --force-refresh`, loop
over them, and raid each one. Set `EXTRA_RAID_ARGS="--no-stash"` if you don't
need the stash phase.

Environment variables:

| Variable | Meaning | Default |
|----------|---------|---------|
| `RACCPACK_PASSPHRASE` | passphrase for the stash phase | empty (required unless `--no-stash`) |
| `DRY_RUN=1` | run without `--yes` | `0` (a real Commit with `--yes`) |
| `EXTRA_RAID_ARGS` | extra raid arguments, e.g. `--no-stash --keep-sources` | empty |
| `CONTINUE_ON_ERROR=1` | don't stop at the first failing project | `1` |

### bash

```bash
#!/usr/bin/env bash
# raid-all.sh — raid across all projects from sniff (scan_root/den from config)
set -u

command -v racc >/dev/null || { echo "need: racc" >&2; exit 1; }
command -v jq    >/dev/null || { echo "need: jq" >&2; exit 1; }

DRY_RUN="${DRY_RUN:-0}"
CONTINUE_ON_ERROR="${CONTINUE_ON_ERROR:-1}"
EXTRA_RAID_ARGS="${EXTRA_RAID_ARGS:-}"

if [ -z "${RACCPACK_PASSPHRASE:-}" ] && [[ "$EXTRA_RAID_ARGS" != *--no-stash* ]]; then
  echo "ERROR: set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS=--no-stash)" >&2
  exit 1
fi
export RACCPACK_PASSPHRASE

mapfile -t PROJECTS < <(racc sniff --json --force-refresh | jq -r '.report.projects[].path')
[ "${#PROJECTS[@]}" -eq 0 ] && { echo "No projects found."; exit 0; }

echo "Found ${#PROJECTS[@]} project(s):"
printf '  - %s\n' "${PROJECTS[@]}"

ok=0; fail=0; failed=()
for proj in "${PROJECTS[@]}"; do
  echo "==> raid: $proj"
  mode=(--yes); [ "$DRY_RUN" = "1" ] && mode=(--dry-run)
  # shellcheck disable=SC2086
  if racc raid --project "$proj" "${mode[@]}" $EXTRA_RAID_ARGS; then
    ok=$((ok+1))
  else
    echo "FAIL: $proj" >&2; fail=$((fail+1)); failed+=("$proj")
    [ "$CONTINUE_ON_ERROR" != "1" ] && exit 1
  fi
done

echo "Done. ok=$ok fail=$fail total=${#PROJECTS[@]}"
[ "$fail" -gt 0 ] && { printf 'Failed: %s\n' "${failed[@]}"; exit 1; }
```

### fish

```fish
#!/usr/bin/env fish
# raid-all.fish — raid across all projects from sniff (scan_root/den from config)

set -q DRY_RUN; or set DRY_RUN 0
set -q CONTINUE_ON_ERROR; or set CONTINUE_ON_ERROR 1
set -q EXTRA_RAID_ARGS; or set EXTRA_RAID_ARGS ""

if not command -q racc
    echo "need: racc" >&2
    exit 1
end
if not command -q jq
    echo "need: jq" >&2
    exit 1
end

set -q RACCPACK_PASSPHRASE; or set RACCPACK_PASSPHRASE ""
if test -z "$RACCPACK_PASSPHRASE"; and not string match -q '*--no-stash*' -- $EXTRA_RAID_ARGS
    echo "ERROR: set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS=--no-stash)" >&2
    exit 1
end
set -gx RACCPACK_PASSPHRASE $RACCPACK_PASSPHRASE

echo "==> sniff --json --force-refresh"
set PROJECTS (racc sniff --json --force-refresh | jq -r '.report.projects[].path')

if test (count $PROJECTS) -eq 0
    echo "No projects found."
    exit 0
end

echo "Found "(count $PROJECTS)" project(s):"
for p in $PROJECTS
    echo "  - $p"
end

set ok 0
set fail 0
set failed_list

for proj in $PROJECTS
    echo "==> raid: $proj"
    set RAID_ARGS raid --project $proj
    if test "$DRY_RUN" = "1"
        set -a RAID_ARGS --dry-run
    else
        set -a RAID_ARGS --yes
    end
    if test -n "$EXTRA_RAID_ARGS"
        set -a RAID_ARGS (string split ' ' -- $EXTRA_RAID_ARGS)
    end

    if racc $RAID_ARGS
        echo "OK: $proj"
        set ok (math $ok + 1)
    else
        echo "FAIL: $proj" >&2
        set fail (math $fail + 1)
        set -a failed_list $proj
        if test "$CONTINUE_ON_ERROR" != "1"
            echo "Stopping on first error." >&2
            exit 1
        end
    end
end

echo "Done. ok=$ok fail=$fail total="(count $PROJECTS)
if test $fail -gt 0
    for p in $failed_list
        echo "  - $p"
    end
    exit 1
end
```

### Nushell

```nu
#!/usr/bin/env nu
# raid-all.nu — raid across all projects from sniff (scan_root/den from config)
# Requirements: racc in PATH; JSON is parsed with built-in nu tooling (no jq needed)

def main [
  --dry-run   # run without --yes (equivalent of DRY_RUN=1)
] {
  # CONTINUE_ON_ERROR=0 stops the loop at the first error (the default is to continue)
  let stop_on_error = (($env | get -i CONTINUE_ON_ERROR | default '1') == '0')
  let extra = ($env | get -i EXTRA_RAID_ARGS | default '')
  let pass  = ($env | get -i RACCPACK_PASSPHRASE | default '')
  if ($pass | is-empty) and not ($extra | str contains '--no-stash') {
    error make {msg: "set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS='--no-stash')"}
  }

  let projects = (racc sniff --json --force-refresh
    | from json
    | get report.projects.path)

  if ($projects | is-empty) {
    print 'No projects found.'
    return
  }
  print $"Found ($projects | length) project(s):"
  for p in $projects { print $"  - ($p)" }

  mut ok = 0
  mut fail = 0
  mut failed = []
  for proj in $projects {
    print $"==> raid: ($proj)"
    let mode = if $dry_run { '--dry-run' } else { '--yes' }
    let extra_args = ($extra | split row ' ' | where {|x| $x != ''})
    let outcome = (do -i { ^racc raid --project $proj $mode ...$extra_args } | complete)
    if $outcome.exit_code == 0 {
      print $"OK: ($proj)"
      $ok += 1
    } else {
      print -e $"FAIL: ($proj)"
      $fail += 1
      $failed = ($failed | append $proj)
      if $stop_on_error {
        error make {msg: 'stopping on first error'}
      }
    }
  }

  print $"Done. ok=($ok) fail=($fail) total=($projects | length)"
  if $fail > 0 {
    for p in $failed { print $"  - ($p)" }
    exit 1
  }
}
```

### PowerShell 7+

```powershell
#!/usr/bin/env pwsh
# raid-all.ps1 — raid across all projects from sniff (scan_root/den from config)

$ErrorActionPreference = 'Continue'

if (-not (Get-Command racc -ErrorAction SilentlyContinue)) { Write-Error 'need: racc'; exit 1 }
if (-not (Get-Command jq    -ErrorAction SilentlyContinue)) { Write-Error 'need: jq';    exit 1 }

$DRY_RUN            = if ($env:DRY_RUN)            { $env:DRY_RUN }            else { '0' }
$CONTINUE_ON_ERROR  = if ($env:CONTINUE_ON_ERROR)  { $env:CONTINUE_ON_ERROR }  else { '1' }
$EXTRA_RAID_ARGS    = if ($env:EXTRA_RAID_ARGS)    { $env:EXTRA_RAID_ARGS }    else { '' }

if (-not $env:RACCPACK_PASSPHRASE -and $EXTRA_RAID_ARGS -notmatch '--no-stash') {
  Write-Error 'ERROR: set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS=--no-stash)'
  exit 1
}

$projects = (racc sniff --json --force-refresh | jq -r '.report.projects[].path')
if (-not $projects) { Write-Host 'No projects found.'; exit 0 }

$projects = @($projects)
Write-Host "Found $($projects.Count) project(s):"
$projects | ForEach-Object { Write-Host "  - $_" }

$ok = 0; $fail = 0; $failed = @()
foreach ($proj in $projects) {
  Write-Host "==> raid: $proj"
  $mode = if ($DRY_RUN -eq '1') { '--dry-run' } else { '--yes' }
  $extra = @($EXTRA_RAID_ARGS -split ' ' | Where-Object { $_ })
  racc raid --project $proj $mode @extra
  if ($LASTEXITCODE -eq 0) {
    $ok++
  } else {
    Write-Error "FAIL: $proj"
    $fail++; $failed += $proj
    if ($CONTINUE_ON_ERROR -ne '1') { exit 1 }
  }
}

Write-Host "Done. ok=$ok fail=$fail total=$($projects.Count)"
if ($fail -gt 0) { $failed | ForEach-Object { Write-Host "  - $_" }; exit 1 }
```

::: info On the identical env vars across all variants
The scripts deliberately repeat one semantics: sniff from config → raid loop →
summary. Only the shell syntax differs; in nu the built-in `from json` replaces
`jq`.
:::
:::

## 5. Targeted operations: stash only / rinse only / pack only

The raid phases are available individually too — when you need just one effect:

```bash
# Secrets into the age archive only (+ remove sources after a successful commit)
racc stash --project ~/DEV/PROJS/my-api --yes --remove-sources

# Build trash cleanup only
racc rinse --project ~/DEV/PROJS/my-api --yes

# Project archive into the den only
racc pack --project ~/DEV/PROJS/my-api --yes
```

Handy stash qualifiers: `--min-risk critical` (take critical items only),
`--only path/to/file` (a specific file, repeatable), `--batch-id release-x`
(an artifact name instead of the timestamp). Rinse qualifiers: `--strategy ID`
(repeatable; defaults to the strategies from config).

Pages: [Stash](/stash) · [Rinse](/rinse) · [Pack](/pack).

## 6. Raid without stash (--no-stash), when there is no passphrase

Archiving and cleanup work without encrypting secrets:

```bash
racc raid --project ~/DEV/PROJS/my-api --yes --no-stash
```

No passphrase is set at all — the stash phase is switched off. An option for "cold"
projects without sensitive files, or when the secrets phase will be a separate pass.

## 7. JSON pipelines

`--json` on every command; the structure is stable (`schema_version`).

```bash
# Paths of Critical findings only
racc dig --project "$PROJ" --json | jq -r '.files[] | select(.risk=="Critical") | .path'

# High+ findings with git status (git_status exists only in JSON)
racc dig --project "$PROJ" --json \
  | jq '.files[] | select(.risk=="Critical" or .risk=="High") | {path, risk, git_status}'

# Repeated secrets (the same value in several files)
racc dig --project "$PROJ" --repeated --json | jq '.repeated'

# Projects larger than 100 MiB
racc sniff --json | jq '.report.projects[] | select(.size_bytes > 104857600) | .path'

# Git repositories with no language
racc sniff --json | jq '.report.projects[] | select(.is_git_repo and (.stack.language == null)) | .name'
```

Exit codes: `dig` returns `2` when the `--fail-on critical|high` policy triggers —
handy for CI.

## 8. Debugging without leaking secrets

Logs (`tracing`) always go to **stderr**, machine output (`--json`) to **stdout**,
so pipes don't get mixed and logs never end up inside the JSON:

```bash
racc dig --project "$PROJ" --json 2>dig.log          # stdout is pure JSON, logs go to the file
racc raid --project "$PROJ" -vv --yes                # debug logs to the terminal
racc pack --project "$PROJ" -v                       # info logs
```

Levels: `-v` info · `-vv` debug · `-vvv` trace. Logs contain no raw secrets,
passphrases or file contents — that is a product invariant.

## 9. Custom config / den / root

```bash
# One-off override of paths
racc sniff --root ~/other/projects --den /mnt/vault/den

# A fully alternative config
RACCPACK_CONFIG=~/.config/raccpack/work.toml racc raid --project "$PROJ" --yes
racc --config ~/.config/raccpack/work.toml sniff
```

Paths with `~` and relative paths are resolved to absolute ones when the config loads.
What goes inside the config — [Configuration](/configuration); config version migration —
[Git, init and DX](/git-and-dx).

## 10. Monorepo awareness

`sniff` may show both the monorepo root and nested packages (each with its own
markers). Before a mass raid, narrow the list down to the "leaves" so you don't
pack the same thing twice.

::: tip Simpler with `--root`
`racc raid --root ~/path/to/monorepo --only subpkg` is usually enough to raid
specific nested packages without a shell loop. Use the scripts below only when
you need a custom exclude filter (e.g. skip a specific subfolder by path).
:::

```bash
# Show the candidate tree
racc sniff --json | jq '.report.projects[] | {name, path}'

# Raid only the nested packages, excluding the root
racc sniff --json \
  | jq -r '.report.projects[].path' \
  | grep -v '/monorepo-root$' \
  | while read -r p; do racc raid --project "$p" --yes --no-stash; done
```

::: warning Double packing of a monorepo
Raiding the root and its subfolders yields overlapping archives. Decide in advance
what the "backup unit" level is: usually the leaves (packages/services) or the root,
but not both.
:::

## 11. Checking the den after a raid

```text
~/.raccpack/den/
├── packs/{yyyy}/{mm}/      # {slug}__{UTC}__.tar.zst
├── secrets/{yyyy}/{mm}/    # {slug}__{UTC}__.age
└── manifests/{yyyy}/{mm}/  # raid JSON manifests
```

Quick check:

```bash
ls -lh ~/.raccpack/den/packs/*/* | tail
ls -lh ~/.raccpack/den/secrets/*/* | tail
jq '{project, success, phases}' ~/.raccpack/den/manifests/*/*.json | tail -40
```

Manifests contain operation metadata (paths relative to the den, phases, counters)
— no raw secrets. Layout and naming conventions: [Concepts](/concepts).

## 12. Checksum verification and installing the binary from a Release

```bash
# Download the tarball and the checksum signature (see GitHub Release v0.3.0)
curl -LO https://github.com/y-tretyakov/raccpack/releases/download/v0.3.0/raccpack-0.3.0-linux-x86_64.tar.gz
curl -LO https://github.com/y-tretyakov/raccpack/releases/download/v0.3.0/raccpack-0.3.0-linux-x86_64.tar.gz.sha256

sha256sum -c raccpack-0.3.0-linux-x86_64.tar.gz.sha256   # OK
tar xzf raccpack-0.3.0-linux-x86_64.tar.gz               # inside: racc (0755)
./racc --version                                         # racc 0.3.0
install -m 0755 racc ~/.local/bin/racc                   # or ~/.cargo/bin
racc init --scan-root ~/DEV/PROJS --ensure-den
```

For ARM64/Raspberry Pi/Graviton take `linux-aarch64`; for Alpine — the `-musl`
suffix (if that build is present in the release).
