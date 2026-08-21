---
title: Installation
description: Requirements, building from source, and environment verification for raccpack.
---

# Installation

## Requirements

- **Linux** — the primary supported platform (macOS and Windows work through the same Rust mechanisms but are tested on a best-effort basis).
- **Rust toolchain** version **1.75+** for building from source.
- To build: the `cargo` compiler and `rustc`.

::: info
Building from source is currently the only installation method: release binaries and system packages will appear at milestone 1.0.0.
:::

## Building from source

Clone the repository and build the workspace:

```bash
git clone https://github.com/y-tretyakov/raccpack.git
cd raccpack

# Build the whole workspace (core + CLI)
cargo build --release
```

The binary will appear at `target/release/racc`. You can install it into a system directory:

```bash
install -m 0755 target/release/racc ~/.local/bin/racc
```

Verify the installation:

```bash
racc --help
racc --version
```

If the command is not found — make sure the installation directory (`~/.local/bin`) is in your `PATH`.

## Interface versions

raccpack ships with three interfaces. Currently only the CLI is available.

| Interface | Binary | Status |
|-----------|--------|--------|
| CLI | `racc` | Available (MVP) |
| TUI | `racc-tui` | Planned (Beta, 0.5.x) |
| Desktop | `raccpack` (Tauri) | Planned (Beta, 0.5.x) |

::: info
TUI and Desktop are in development. Their installation will be described here once the first builds appear. The target behavior of the interfaces is described in the [TUI](/tui-usage) and [Desktop](/desktop-usage) sections.
:::

## Verifying the environment

Create a minimal configuration and make sure `racc` can see your projects:

```bash
mkdir -p ~/.config/raccpack
cat > ~/.config/raccpack/config.toml <<'EOF'
[paths]
scan_root = "~/DEV/PROJS"
den_dir = "~/.raccpack/den"
EOF

racc sniff
```

For more on settings, see [Configuration](/configuration).

## Next steps

- [Quick start](/quick-start) — your first run in five minutes.
