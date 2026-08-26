<p align="center">
  <img src="RaccPack.webp" alt="raccpack" width="435"/>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85-orange?style=flat-square&logo=rust" alt="Rust"/></a>
  <a href="https://doc.rust-lang.org/cargo/"><img src="https://img.shields.io/badge/Cargo-workspace-blue?style=flat-square&logo=cargo" alt="Cargo"/></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/version-0.4.0-blue?style=flat-square" alt="version"/></a>
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

---

## Install

Download the latest release for your system from [GitHub Releases](https://github.com/y-tretyakov/raccpack/releases/latest).

**Debian / Ubuntu:**
```bash
sudo dpkg -i raccpack-0.4.0-1-amd64.deb
```

**Fedora / RHEL / Rocky:**
```bash
sudo rpm -i raccpack-0.4.0-1.x86_64.rpm
```

**Arch Linux / Manjaro:**
```bash
sudo pacman -U raccpack-0.4.0-1-x86_64.pkg.tar.zst
```

**Any Linux (musl, universal):**
```bash
tar --zstd -xf raccpack-0.4.0-linux-x86_64.tar.zst
sudo cp raccpack-0.4.0/racc /usr/local/bin/
```

**From source:**
```bash
cargo install raccpack-cli
```

ARM64 packages are available for all formats.

---

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

---

## Commands

| Command | What it does |
|---------|-------------|
| `racc sniff` | Discover projects by language markers and frameworks |
| `racc dig` | Scan for secrets (filename + content patterns) |
| `racc pack` | Archive a project into `tar.zst` in your den |
| `racc stash` | Encrypt secrets with age and store in den |
| `racc rinse` | Remove build artifact directories |
| `racc raid` | Run stash → rinse → pack in one go |
| `racc init` | Create default configuration |

Full reference: [wiki / CLI usage](https://y-tretyakov.github.io/raccpack/cli-usage.html)

---

## Supported

- **14 project markers:** `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `setup.py`, `requirements.txt`, `pom.xml`, `build.gradle`, `build.gradle.kts`, `Gemfile`, `composer.json`, `CMakeLists.txt`, `Makefile`, `.git`
- **28 secret filename patterns:** `.env` family, SSH keys, keystores, credentials, registry configs, service-account JSON, etc.
- **12 content markers:** AWS keys, GitHub tokens, Slack/Stripe tokens, PEM headers, connection strings, JWT, generic `api_key` / `secret`
- **6 cleanup strategies:** `rust`, `node`, `python` (default) + opt-in `jvm`, `go`, `generic`

Full table: [wiki / Supported](https://y-tretyakov.github.io/raccpack/supported.html)

---

## Den layout

```
~/.raccpack/den/
├── packs/2026/08/     # tar.zst project archives
├── secrets/2026/08/   # age-encrypted secret archives
├── manifests/2026/08/ # operation manifests
└── staging/            # temporary (safe to clean)
```

Do not commit a den to git. Keep passphrases offline.

---

## Build & test

```bash
cargo build
cargo test -p raccpack-core
cargo fmt --check
cargo clippy -p raccpack-core --all-targets -- -D warnings
```

---

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
