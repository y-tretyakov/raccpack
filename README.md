<p align="center">
  <img src="RaccPack.webp" alt="raccpack" width="435"/>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85-orange?style=flat-square&logo=rust" alt="Rust"/></a>
  <a href="https://doc.rust-lang.org/cargo/"><img src="https://img.shields.io/badge/Cargo-workspace-blue?style=flat-square&logo=cargo" alt="Cargo"/></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/version-0.4.3-blue?style=flat-square" alt="version"/></a>
  <a href="https://github.com/y-tretyakov/raccpack/actions/workflows/wiki.yml"><img src="https://img.shields.io/badge/CI-wiki-success?style=flat-square" alt="CI"/></a>
  <a href="https://github.com/y-tretyakov/raccpack/releases"><img src="https://img.shields.io/badge/OS-Linux-success?style=flat-square" alt="Linux"/></a>
  <a href="https://clap.rs"><img src="https://img.shields.io/badge/CLI-clap-ee4b2b?style=flat-square" alt="CLI"/></a>
  <a href="https://github.com/FiloSottile/age"><img src="https://img.shields.io/badge/secrets-age--encrypted-0a0a0a?style=flat-square" alt="age"/></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue?style=flat-square" alt="License"/></a>
</p>

<p align="center">
  <a href="README.ru.md">🇷🇺 Русский</a>
</p>

# raccpack

CLI tool for scanning project directories, finding secrets, cleaning build trash, and packing projects into a secure **den** — age-encrypted secret archives and compressed project packs.

**Docs:** [https://y-tretyakov.github.io/raccpack/](https://y-tretyakov.github.io/raccpack/)

## Status

**Version `0.4.3`** — MVP `0.1.0` closed; **Alpha `0.3.0` closed** (stash / rinse / raid / git+DX). **Detect v2 `0.4.0` closed** (composite DAG detectors, scoped rinse, batch raid `racc raid --root`, wiki + E2E). **Beta B1.3: TUI dig screen done** (Findings — masked detail, risk filter, content-scan toggle; non-blocking worker). Next: Beta `0.5.0` (TUI + Desktop).

| Command | Status | Role |
|---------|--------|------|
| **sniff** | Available | Discover projects by markers, stack, sizes, versioned cache |
| **dig** | Available | Secret scan (filename + content), masked values, risk levels, exit policy, git status per finding |
| **pack** | Available | `tar.zst` into den (`packs/…`), name/content deny, DryRun default / `--yes` |
| **stash** | Available (Alpha) | Age-encrypted secret archives into den (`secrets/…`), optional source removal |
| **rinse** | Available (Alpha) | Build-trash cleanup by strategies (`rust`/`node`/`python` default, more opt-in), DryRun default / `--yes` |
| **raid** | Available (Alpha) | Orchestrated stash → rinse → pack → move in one command; atomic default (staging + WAL + rollback), manifest JSON in den, `--fail-fast` mode, exit 1 on `!success`; `--root` for batch mode across all projects |
| **init** | Available (Alpha) | Create default config (`config_version = 1`) with prefilled paths; optional den skeleton (`--ensure-den`), `--force` to overwrite |
| **TUI** | Beta (sniff + dig screens) | Ratatui project table, non-blocking worker sniff/dig, Findings with masked details, risk filter `f`, content-toggle `c`, j/k navigation (since 0.4.2, dig since 0.4.3) |
| **Desktop** | Planned (Beta) | Tauri + React |

Details and exact flags: [wiki / CLI](https://y-tretyakov.github.io/raccpack/cli-usage.html).

## Install

Download from [GitHub Releases](https://github.com/y-tretyakov/raccpack/releases/latest):

```bash
# Debian / Ubuntu
sudo dpkg -i raccpack-0.4.0-1-amd64.deb

# Fedora / RHEL / Rocky
sudo rpm -i raccpack-0.4.0-1.x86_64.rpm

# Arch Linux / Manjaro
sudo pacman -U raccpack-0.4.0-1-x86_64.pkg.tar.zst

# Any Linux (musl, universal)
tar --zstd -xf raccpack-0.4.0-linux-x86_64.tar.zst
sudo cp raccpack-0.4.0/racc /usr/local/bin/

# From source
cargo install raccpack-cli
```

ARM64 packages available for all formats.

## Quick start

```bash
# Create config
racc init

# Find projects
racc sniff

# Scan for secrets
racc dig --project ~/DEV/PROJS/my-app

# Pack a project
racc pack --project ~/DEV/PROJS/my-app --yes

# Encrypt secrets
racc stash --project ~/DEV/PROJS/my-app --yes

# Clean build trash
racc rinse --project ~/DEV/PROJS/my-app --yes
```

Add `--json` to any command for machine-readable output.

## What is supported

Full tables: [wiki / Supported](https://y-tretyakov.github.io/raccpack/supported.html)

- **14 project markers:** `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `setup.py`, `requirements.txt`, `pom.xml`, `build.gradle`, `build.gradle.kts`, `Gemfile`, `composer.json`, `CMakeLists.txt`, `Makefile`, `.git`
- **28 secret filename patterns:** `.env` family, SSH keys, keystores, credentials, registry configs, service-account JSON, etc.
- **12 content markers:** AWS, GitHub tokens, Slack, Stripe, PEM headers, connection strings, JWT, generic `api_key` / `secret`
- **6 cleanup strategies:** `rust`, `node`, `python` (default) + opt-in `jvm`, `go`, `generic`

## Den layout

```
~/.raccpack/den/
├── packs/2026/08/     # tar.zst project archives
├── secrets/2026/08/   # age-encrypted secret archives
├── manifests/2026/08/ # operation manifests
└── staging/            # temporary (safe to clean)
```

Do not commit a den to git. Keep passphrases offline.

## Build & test

```bash
cargo build
cargo test -p raccpack-core
cargo fmt --check
cargo clippy -p raccpack-core --all-targets -- -D warnings
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
