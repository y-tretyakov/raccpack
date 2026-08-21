---
title: Troubleshooting
description: Common problems when working with raccpack — installation, configuration, scanning, den, and performance.
---

# Troubleshooting

Frequently asked questions and problems when working with raccpack.

## Installation and running

### `racc: command not found`

The binary is not in `PATH`. Build and install it (see [Installation](/installation)):

```bash
cargo build --release
install -m 0755 target/release/racc ~/.local/bin/racc
```

Check that `~/.local/bin` is in your `PATH`.

### Build fails due to the Rust version

The workspace requires **Rust 1.75+**. Check the version:

```bash
rustc --version
```

If necessary, update the toolchain:

```bash
rustup update stable
```

## Configuration

### `MissingScanRoot`

`scan_root` is set neither in the configuration nor via a flag. Set one of the options:

```toml
# ~/.config/raccpack/config.toml
[paths]
scan_root = "~/DEV/PROJS"
```

or:

```bash
racc sniff --root ~/DEV/PROJS
```

### `path not found` / `ScanRootMissing`

The specified folder does not exist or is not accessible. Check the path:

```bash
ls -ld ~/DEV/PROJS
```

### `NotADirectory`

A file, not a directory, is specified in `scan_root`. Specify a folder.

### `invalid configuration: max_depth`

`scanner.max_depth` is less than 1. Set it to ≥ 1:

```toml
[scanner]
max_depth = 6
```

### The configuration file cannot be read

- If `RACCPACK_CONFIG` is set, the file **must** exist — check the variable:

```bash
echo "$RACCPACK_CONFIG"
```

- The default file: `~/.config/raccpack/config.toml`. Create it if necessary.
- The syntax is TOML. Check that sections and keys are written correctly.

## Scanning and secret detection

### `sniff` finds no projects

Possible causes:

- `scan_root` is wrong or empty.
- Projects are deeper than `scanner.max_depth` (6 by default). Increase the depth:

```bash
racc sniff --max-depth 10
```

or in the configuration:

```toml
[scanner]
max_depth = 10
```

- The projects contain no recognizable markers (`package.json`, `Cargo.toml`, `go.mod`, etc.).

### `sniff` results are stale

Results are cached. Force a rescan:

```bash
racc sniff --force-refresh
```

### `dig` did not find a secret I expected

- Files larger than **1 MiB** and **binary files** are skipped during content scanning.
- Content scanning can be disabled (`--no-content`) — then findings come from file names only.
- A finding's risk level may be below the threshold. By default everything is shown; the threshold only affects exit-code policy.
- If a value looks like a secret but is not covered by built-in markers — add a check to your own process. The rule set grows as the project evolves.

### `dig` returned exit code 2

This is **not** a runtime error: secrets above the policy threshold were found (`Critical` by default). Look at the report, fix the findings, or relax the policy:

```bash
racc dig --fail-on ignore
```

Exit codes:

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime error |
| `2` | Secrets above the policy threshold were found |

### No "Repeated secrets" in the output

The repeated secrets block is printed only with the `--repeated` flag:

```bash
racc dig --repeated
```

## Den and packing

### `incompatible den version`

The `den_dir` contains a `.den-version` of an incompatible version. Options:

- Point to another (empty) `den_dir` for new artifacts.
- When a migration tool appears (`racc den migrate`) — use it.

::: warning
Do not delete `.den-version` manually — you will lose information about the storage format.
:::

### Files remain in `staging/` after an interrupted raid

This is expected: temporary files can outlive a crash. For now you can delete them manually. A `racc den gc` command (cleaning staging older than N days) is planned.

## Performance

### Scanning a large tree takes long

- Reduce the traversal depth (`--max-depth` / `scanner.max_depth`).
- For `dig`, enable name-only mode (`--no-content`) as a first pass.
- Parallel scanning (`advanced.parallel_jobs`) is planned for Beta.

## Reporting an issue

Project repository: [github.com/y-tretyakov/raccpack](https://github.com/y-tretyakov/raccpack). When describing a problem, attach:

- the output of `racc --version`;
- platform and Rust version;
- the command that fails and its output (without secrets);
- a minimal example reproducing the problem.
