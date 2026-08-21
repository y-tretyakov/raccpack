---
title: Git, init & DX
description: Alpha phase A4 capabilities — git status on dig findings, racc init, verbose --verbose logs without secrets.
---

# Git, init & DX

**For CLI users.**
Capabilities of the **A4** phase: git status in dig, `racc init`, `--verbose`.

If it's not on this page, it's not in the current version.

---

## What A4 brought

| Capability | Why |
|------------|-----|
| Git status on dig findings | Understand whether a secret is tracked / untracked / ignored |
| [racc init](/init) | Quickly create `config.toml` and optionally the den |
| `-v` / `-vv` / `-vvv` | Verbose logs **without** raw secrets or passphrases |
| CI / tests | Stable headless CLI (Alpha) |

---

## CLI command examples

### dig + git status

```bash
# Plain dig — human output shows risk/label/path; git status is NOT shown there
racc dig --project ~/DEV/PROJS/my-api

# Git status is available in JSON only: the git_status field per file
racc dig --project ~/DEV/PROJS/my-api --json | jq '.files[] | {path, risk, git_status}'

# Critical only + status
racc dig --project ~/DEV/PROJS/my-api --json \
  | jq '[.files[] | select(.risk=="Critical") | {path, git_status}]'

# Not a git repository — dig still works, git_status is usually null
racc dig --project /tmp/not-a-git-project --json
```

Possible `git_status` values (JSON strings):

`tracked`, `untracked`, `ignored`, `modified`, `staged`, `deleted`, `unknown`,
or a missing field / `null` when git is unavailable.

```bash
# sniff: projects with is_git_repo
racc sniff --root ~/DEV/PROJS --json | jq '.report.projects[] | {name, is_git_repo}'
```

---

### racc init

```bash
# Default config: ~/.config/raccpack/config.toml
racc init

# With paths
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den

# Also create the den skeleton (.den-version, README, secrets/, packs/, …)
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den --ensure-den

# Overwrite an existing config
racc init --force --scan-root ~/DEV/PROJS --den ~/.raccpack/den

# Custom config file
racc init --config /tmp/my-racc.toml --scan-root /tmp/proj --den /tmp/den

# JSON: path to the created file
racc init --scan-root ~/DEV/PROJS --json

# Typical onboarding
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den --ensure-den
racc sniff --root ~/DEV/PROJS
racc dig --project ~/DEV/PROJS/my-api
```

**Errors:**

```bash
# Second init without --force → "already exists" error
racc init
racc init
```

Example contents after init:

```toml
config_version = 1

[paths]
scan_root = "~/DEV/PROJS"
den_dir = "~/.raccpack/den"

[scanner]
max_depth = 6

[cleanup]
enabled_strategies = ["rust", "node", "python"]
```

---

### Verbose / logs

```bash
# Quiet mode (default)
racc sniff --root ~/DEV/PROJS

# Info
racc sniff --root ~/DEV/PROJS -v
racc dig --project ~/DEV/PROJS/my-api -v
racc rinse --project ~/DEV/PROJS/my-api --yes -v

# Debug
racc pack --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes -vv
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes -vv

# Trace
racc dig --project ~/DEV/PROJS/my-api -vvv

# RUST_LOG (takes precedence when set)
RUST_LOG=raccpack_core=debug racc sniff --root ~/DEV/PROJS

# JSON to stdout, logs to stderr
racc dig --project ~/DEV/PROJS/my-api --json -v 2>dig-verbose.log

# All main commands with -v
racc init --scan-root ~/DEV/PROJS -v
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes -v
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --json -v
```

::: tip Log safety
Secret values and passphrases never appear in logs — only paths,
counters, and the passphrase source ("env" / "tty" / "stdin"). Verify like this:
:::

```bash
RACCPACK_PASSPHRASE='unique-pass-xyz' \
  racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes -vv 2>&1 \
  | grep -F 'unique-pass-xyz' && echo 'LEAK — a bug' || echo 'OK — no passphrase in logs'
```

---

### Full Alpha scenario in one session

```bash
export RACCPACK_PASSPHRASE='your-strong-passphrase'
PROJ=~/DEV/PROJS/my-api
DEN=~/.raccpack/den

racc init --scan-root ~/DEV/PROJS --den "$DEN" --ensure-den --force
racc sniff --root ~/DEV/PROJS -v
racc dig --project "$PROJ" --json | jq '.files[] | {path, risk, git_status}'
racc raid --project "$PROJ" --den "$DEN" --yes -v

# Artifacts
find "$DEN/secrets" -name '*.age'
find "$DEN/packs" -name '*.tar.zst'
find "$DEN/manifests" -name '*.json'
```

::: warning Careful with init --force
`racc init --force` overwrites the config at the given path. In scripts and CI,
isolate the environment: `RACCPACK_CONFIG=/tmp/my-racc.toml`.
:::

---

### Local "as CI" check

```bash
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Syntax: `racc init`

```text
racc init [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--config <PATH>` | Where to write the config (default XDG) |
| `--force` | Overwrite existing |
| `--scan-root <PATH>` | Fill in `paths.scan_root` |
| `--den <PATH>` | Fill in `paths.den_dir` |
| `--ensure-den` | Create the den skeleton |
| `--json` | Print the path to the config |

## Global: verbose

| Flag | Level |
|------|-------|
| *(none)* | warn |
| `-v` | info |
| `-vv` | debug |
| `-vvv` | trace |

Also: `RUST_LOG=…`.

---

## Related pages

- [Stash](/stash) · [Rinse](/rinse) · [Raid](/raid) · [Init](/init)

---

*Matches Alpha A4 (a4.1–a4.4). Changing flags? Update the wiki in the same PR.*
