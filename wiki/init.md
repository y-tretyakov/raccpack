---
title: Init — starter configuration
description: The racc init command — creating a config.toml from a commented template and, optionally, a den skeleton.
---

# Init - starter configuration

Command: `racc init`  
Status: implemented.

This page describes **exactly the behavior** that `raccpack` implements right now. If a flag or path is not listed here — it does not exist in the current version.

> Back to the command overview: [CLI usage](/cli-usage).

## What it does

1. Creates a configuration file from a commented template (`config_version = 1`, sections `[paths]`, `[scanner]`, `[cleanup]`). By default — the XDG path `~/.config/raccpack/config.toml`; missing directories are created.
2. With the `--ensure-den` flag, additionally creates a den skeleton: `.den-version` and `README.txt`.

What it does **not** do:

- does not overwrite an existing config without explicit `--force`;
- does not check that `scan_root` exists — the path is only written into the template;
- migrates nothing on disk (config auto-migration v0 → v1 is performed in-memory at load time, see [Configuration](/configuration)).

## Quick start

```bash
# Create ~/.config/raccpack/config.toml with the default template
racc init

# Point to your projects folder right away and create the den skeleton
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den --ensure-den
```

## Syntax

```text
racc init [OPTIONS]
```

There are no required parameters.

## Parameters and flags

### Command flags

| Flag | Default | Description |
|------|---------|-------------|
| `--force` | off | Overwrite an existing configuration file |
| `--scan-root <PATH>` | `~/DEV/PROJS` | Prefill `paths.scan_root` in the generated template |
| `--ensure-den` | off | Create the den skeleton: `.den-version`, `README.txt` |

### Global flags

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Where to write the config (by default — the XDG path) |
| `--root <PATH>` | Alternative to `--scan-root`: prefills `paths.scan_root` |
| `--den <PATH>` | Prefills `paths.den_dir`; also the location where the den is created with `--ensure-den` |
| `--json` | JSON output instead of human-readable |

Precedence:

- if both `--scan-root` and the global `--root` are given, `--scan-root` wins;
- without `--scan-root` / `--root`, `~/DEV/PROJS` is substituted into the template;
- without `--den`, `~/.raccpack/den` is substituted into the template.

## What gets generated

An abridged view of the template (in the file it is supplemented with comments and links to the wiki):

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

The values of `scan_root` and `den_dir` are substituted from flags; the other fields are defaults. The generated file passes configuration validation.

With `--ensure-den`, the following is created in the den:

```text
{den_dir}/
├── .den-version
└── README.txt
```

For details on the file format and sections, see [Configuration](/configuration).

## Output

### Human-readable

```text
Created config file: /home/user/.config/raccpack/config.toml
Initialized den vault: /home/user/.raccpack/den
```

The second line is printed only with `--ensure-den`.

### JSON (`--json`)

```json
{
  "config_path": "/home/user/.config/raccpack/config.toml",
  "den_dir": "/home/user/.raccpack/den"
}
```

The `den_dir` field is `null` if `--ensure-den` was not passed.

## Exit codes

| Code | When |
|------|------|
| `0` | Success |
| `1` | Error: config already exists (without `--force`), an IO write error, failed to create the den |

Exit code `2` (as in `dig`) is **not** used for `init`.

## Negative scenario: config already exists

Without `--force`, the command refuses to overwrite the file:

```text
$ racc init
error: config file already exists: /home/user/.config/raccpack/config.toml
hint: Use --force to overwrite the existing configuration file.
$ echo $?
1
```

Overwriting is explicit only:

```bash
racc init --force
```

::: warning
`--force` overwrites the entire file: manual edits in `config.toml` will be lost.
:::

## Examples

```bash
# Basic: commented template into the XDG path
racc init

# With custom paths in the template
racc init --scan-root ~/DEV/PROJS --den /mnt/backup/den

# Config in a non-standard location + create the den skeleton
racc init --config ~/cfg/raccpack.toml --ensure-den

# Overwrite an existing config with a new template
racc init --force

# Machine-readable output for scripts
racc init --scan-root ~/DEV/PROJS --json
```

## Related pages

| Page | Role |
|------|------|
| [Configuration](/configuration) | The `config.toml` format, `config_version`, and migration |
| [Core concepts](/concepts) | Den and storage layout |
| [Quick start](/quick-start) | Your first run in five minutes |
| [CLI usage](/cli-usage) | Overview of all commands |

*Documentation matches the implementation; when CLI flags change, update this page in the same PR.*
