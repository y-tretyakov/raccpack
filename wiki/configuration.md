---
title: Configuration
description: "Configuring raccpack via a TOML file and environment variables: paths, scanner, cleanup strategies, configuration errors."
---

# Configuration

raccpack is configured through a TOML file and a few environment variables.

## Where the configuration is looked up

Resolution order:

1. The **`RACCPACK_CONFIG`** environment variable — an explicit path to a file. If set, the file **must** exist.
2. The standard XDG path: `$XDG_CONFIG_HOME/raccpack/config.toml`, or `~/.config/raccpack/config.toml` when `XDG_CONFIG_HOME` is unset.
3. If no file exists anywhere — default configuration is used (paths can be provided via `--root` and `--den` flags).

The easiest way to create the file is the [`racc init`](/init) command: it writes a commented template to the standard path (or to the path from `--config`):

```bash
racc init --scan-root ~/DEV/PROJS
```

Example:

::: code-group

```bash [bash]
# bash / zsh — explicit path via environment variable
export RACCPACK_CONFIG=/path/to/raccpack.toml
racc sniff
```

```fish [fish]
set -gx RACCPACK_CONFIG /path/to/raccpack.toml
racc sniff
```

```nu [nu]
$env.RACCPACK_CONFIG = "/path/to/raccpack.toml"
racc sniff
```

```powershell [pwsh]
$env:RACCPACK_CONFIG = "/path/to/raccpack.toml"
racc sniff
```

:::

## File format

```toml
# raccpack configuration
config_version = 1

[paths]
# Directory containing your projects (input)
scan_root = "~/DEV/PROJS"
# Den storage directory (output)
den_dir = "~/.raccpack/den"

[scanner]
# Maximum tree walk depth
max_depth = 6

[cleanup]
# Default rinse strategies (when CLI passes no --strategy)
enabled_strategies = ["rust", "node", "python"]
# Opt-in if needed: "jvm", "go", "generic"
```

### The `[paths]` section

| Key | Required | Description |
|-----|----------|-------------|
| `scan_root` | Yes (for scanning) | Projects folder. Must exist |
| `den_dir` | No | Storage folder. Default `~/.raccpack/den`. Created on first write |

Paths may contain `~` and relative components — raccpack resolves them to absolute paths against the home directory.

### The `[scanner]` section

| Key | Default | Description |
|-----|---------|-------------|
| `max_depth` | `6` | Maximum walk depth. Must be ≥ 1 |

### The `[cleanup]` section

| Key | Default | Description |
|-----|---------|-------------|
| `enabled_strategies` | `["rust", "node", "python"]` | Strategy ids for `racc rinse` when no `--strategy` flag is passed |

`racc rinse` removes build artifact directories according to rule sets — **strategies**. Each strategy is a set of directory names considered trash. Registered strategies:

| Id | In defaults | Typical directories |
|----|-------------|---------------------|
| `rust` | yes | `target` |
| `node` | yes | `node_modules`, `.next`, `dist`, `.nuxt`, `coverage` |
| `python` | yes | `__pycache__`, `.venv`, `venv`, `.tox`, `.mypy_cache`, `.pytest_cache`, `*.egg-info`, `.ruff_cache` |
| `jvm` | **opt-in** | `build`, `.gradle`, `.m2` |
| `go` | **opt-in** | `vendor` |
| `generic` | **opt-in** | `.cache`, `tmp`, `temp` |

Only `rust`, `node`, and `python` are enabled by default. The reason is "cautious" names: `dist` (node) and `build` (jvm) sometimes contain real sources, `vendor` (go) may be an intentional copy of dependencies, and `tmp` / `temp` (generic) may hold user data. That's why `jvm`, `go`, and `generic` must be enabled **explicitly** — via `enabled_strategies` in config or the `--strategy` flag (see [Rinse](/rinse)).

The `--strategy` flag overrides configuration for this run:

```bash
# Instead of config.cleanup.enabled_strategies — only node and rust
racc rinse --project ~/DEV/PROJS/my-api --strategy node --strategy rust --yes
```

An unknown id in TOML is a configuration load error (see [Configuration errors](#configuration-errors)); an unknown `--strategy` on the CLI is an error with exit code `1`.

::: info
Cleanup strategy names and the skip-directory lists used during walking/packing are aligned in spirit but currently live separately. A single source of rules is planned (follow-up).
:::

### Future sections

Sections for secret groups and performance will be added to the configuration:

- `[sensitive]` — which secret groups are enabled;
- `[advanced]` — parallelism (`parallel_jobs`), zstd compression level.

::: info
Unknown keys in TOML do not break loading — future sections won't break existing configurations.
:::

## config_version and migration

The current configuration schema has version **1** (`config_version = 1`). This is exactly what [`racc init`](/init) writes into the file.

When loading configuration raccpack checks the `config_version` field:

| Value in file | Behavior |
|---------------|----------|
| Field missing or `0` | Automatic migration to v1 **in-memory**: config loads as v1 without changes on disk |
| `1` | Loaded as-is |
| Above current (e.g., `2`) | Error `incompatible config version: found N, current version is 1`, exit code `1` |

Notes:

- migration never rewrites the file — changes exist only in memory for the duration of the run;
- a config from a "future" version means the file was created by a newer raccpack; the CLI hint suggests upgrading raccpack;
- old configs (without `config_version`) keep working without manual edits.

## Environment variables

| Variable | Purpose |
|----------|---------|
| `RACCPACK_CONFIG` | Explicit path to the TOML file. If set — the file must exist |
| `RACCPACK_PASSPHRASE` | Passphrase for `racc stash` (age archive encryption). **Do not store it in TOML** — provide it via environment, interactive input, or stdin |

The passphrase is never read from the configuration file and never appears in output/reports.

## CLI overrides

Global flags override the configuration for the current run only:

```bash
# Temporarily scan another folder
racc sniff --root /tmp/other --max-depth 4

# Use a temporary den for this run
racc sniff --root ~/DEV/PROJS --den /tmp/den
```

`--root` and `--den` do not change the file on disk — they last one run. For `racc rinse` the `--den` flag is accepted but has no effect: cleaning trash never writes to the den (see [Rinse](/rinse)).

## Minimal configuration for the first run

The fastest path is [`racc init`](/init):

```bash
racc init --scan-root ~/DEV/PROJS
racc sniff
```

Manual option:

```bash
mkdir -p ~/.config/raccpack
cat > ~/.config/raccpack/config.toml <<'EOF'
[paths]
scan_root = "~/DEV/PROJS"
EOF

racc sniff
```

::: info
Without `scan_root` in configuration and without the `--root` flag, `racc` exits with an error (`missing scan_root: …`).
:::

## Configuration errors

Typical errors and hints printed by `racc`:

| Error | Cause | Hint |
|-------|-------|------|
| `missing scan_root: set paths.scan_root in config or pass --root` | `scan_root` not set | Set `scan_root` in TOML or pass `--root` |
| `scan_root does not exist: <path>` / `path not found: <path>` | Path does not exist | Check that the folder exists |
| `not a directory: <path>` | A file given instead of a folder | Provide a directory |
| `invalid max_depth: <value> (must be >= 1)` | `max_depth < 1` | Use a value ≥ 1 |
| `unknown cleanup strategy `foo`` | Unknown id in `cleanup.enabled_strategies` | Use known ids: `rust`, `node`, `python`, `jvm`, `go`, `generic` |
| `invalid configuration: unknown cleanup strategy `foo`` | Unknown `--strategy foo` on CLI | Use known ids; error, exit code `1` |
| `incompatible config version: found N, current version is 1` | Config created by a newer raccpack (`config_version` > 1) | Upgrade raccpack (see [config_version and migration](#config-version-and-migration)) |

A configuration error prints with a `hint: …` line; exit code is `1`.

## Further reading

- [Init](/init) — command that creates the starter config and den skeleton.
- [Rinse](/rinse) — cleanup strategies and the `--strategy` flag.
- [Supported catalog](/supported) — full capability catalog (markers, secrets, strategies).
- [CLI usage](/cli-usage) — all command flags.
- [Concepts](/concepts) — what the den is and how output works.
