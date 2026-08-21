---
title: Supported catalog
description: Languages, frameworks, secrets, skipped directories, cleanup strategies, and packing rules — the full raccpack capability catalog.
---

# Supported catalog

This page describes **what raccpack can do right now**: how it finds projects, which secrets it looks for, which folders it skips, how it cleans build trash, and what it excludes from archives. Including Alpha capabilities: `stash` (age), `rinse` (cleanup strategies), and `raid` (full cycle).

::: info
The list matches the core's built-in rules. When a new language or secret type is added to the code, this page is updated in the same change.
:::

## Project discovery

The `racc sniff` command walks the projects folder looking for **markers** — characteristic files in the project root.

### Which projects are found

Simple rule: **such a file exists → it's such a project**.

| If the root contains… | It's a project |
|------------------------|----------------|
| `Cargo.toml` | **Rust** |
| `package.json` | **JavaScript / TypeScript** |
| `go.mod` | **Go** |
| `pyproject.toml` | **Python** (modern) |
| `setup.py` | **Python** (classic) |
| `requirements.txt` | **Python** (pip) |
| `pom.xml` | **Java** (Maven) |
| `build.gradle` | **Java** (Gradle) |
| `build.gradle.kts` | **Kotlin** (Gradle) |
| `Gemfile` | **Ruby** |
| `composer.json` | **PHP** |
| `CMakeLists.txt` | **C / C++** |
| `Makefile` | Make present (no language assigned) |
| `.git` *(directory)* | git repository (not a language) |

**14 markers** in total.

### When there are several markers

Sometimes one folder has both `package.json` and `Cargo.toml`. Then the language is chosen by **priority** — top to bottom:

| Priority | File | Language |
|----------|------|----------|
| 1 (highest) | `Cargo.toml` | Rust |
| 2 | `go.mod` | Go |
| 3 | `pom.xml`, `build.gradle`, `build.gradle.kts` | Java / Kotlin |
| 4 | `package.json` | JavaScript |
| 5 | `pyproject.toml` → `setup.py` → `requirements.txt` | Python |
| 6 | `Gemfile` | Ruby |
| 7 | `composer.json` | PHP |
| 8 | `CMakeLists.txt` | C++ |
| 9 | `Makefile` | — (no language assigned) |

`.git` has **no effect** on language choice.

### Frameworks

Detected **only by file names in the root** (no dependency reading).

| If the root contains… | Framework |
|------------------------|-----------|
| `next.config.js` / `.mjs` / `.ts` | **Next.js** |
| `nuxt.config.*` | **Nuxt** |
| `angular.json` | **Angular** |
| `vite.config.*` | **Vite** |
| `deno.json` | **Deno** |
| `manage.py` | **Django** |
| `build.sbt` | **Scala / sbt** |
| `Gemfile` **and** `config/application.rb` | **Rails** |

Go, PHP, C/C++, Make, Rust, and "pure" Git have no dedicated rules **yet** (e.g., Axum from `Cargo.toml` is not detected yet).

### Git

| Present… | In the report |
|----------|---------------|
| `.git` directory in the root | project marked as a git repository |

The walk does **not enter** `.git` itself (it's on the skip list), but its presence is noted.

---

## Secrets

The `racc dig` command searches for sensitive files **by name** and, optionally, **by content**.

Reports and JSON **never contain raw values** — only mask, hash, and length.

### Risk levels

| | Level | Meaning | Example |
|---|-------|---------|---------|
| 🔴 | **Critical** | Almost certainly a key | `.env.production`, `id_rsa`, `AKIA…` |
| 🟠 | **High** | Looks like a secret | `.env`, `*.pem`, `api_key = …` |
| 🟡 | **Medium** | Worth a look | `config.json`, `sk_test_…`, JWT |
| 🟢 | **Low** | Weak signal | (barely used in MVP) |

When multiple rules match one file, the **highest** risk wins.

### By file name

Only the **name** is checked (contents are not read). Letter case matters.

#### 📁 Environment

| File | Risk |
|------|------|
| `.env` | 🟠 High |
| `.env.local` | 🟠 High |
| `.env.production` | 🔴 Critical |
| any `.env.…` | 🟠 High |

#### 🔑 SSH and keys

| File | Risk |
|------|------|
| `id_rsa` | 🔴 Critical |
| `id_ed25519` | 🔴 Critical |
| `id_ecdsa` | 🔴 Critical |
| `*.pem` | 🟠 High |
| `*.key` | 🟠 High |
| `*.ppk` | 🟠 High |

#### 🗄️ Keystores

| File | Risk |
|------|------|
| `*.p12` | 🟠 High |
| `*.pfx` | 🟠 High |
| `*.jks` | 🟠 High |

#### 👤 Credentials

| File | Risk |
|------|------|
| `credentials` | 🟠 High |
| name contains `service-account` | 🟠 High |
| `*-sa.json` | 🟠 High |
| `.git-credentials` | 🔴 Critical |
| `.netrc` | 🟠 High |
| `.htpasswd` | 🟠 High |

#### ⚙️ Registries and Kubernetes

| File | Risk |
|------|------|
| `kubeconfig` | 🟠 High |
| `config.json` | 🟡 Medium |
| `.npmrc` | 🟠 High |
| `.pypirc` | 🟠 High |

#### 🔐 Secret files and wallets

| File | Risk |
|------|------|
| `secrets.json` | 🟠 High |
| `secrets.yaml` / `secrets.yml` | 🟠 High |
| name contains `wallet.dat` | 🔴 Critical |

**28** name rules in total.

### By content

The file is read line by line. Typical keys and assignments are searched:

| What's inside the file | Risk | What it looks like |
|-------------------------|------|--------------------|
| AWS access key | 🔴 Critical | `AKIA…` |
| AWS secret key (assignment) | 🔴 Critical | `aws_secret_access_key = …` |
| API key (assignment) | 🟠 High | `api_key = …` (long value) |
| secret / password / token | 🟠 High | `password = …` (8+ characters) |
| PEM private key | 🔴 Critical | `-----BEGIN … PRIVATE KEY-----` |
| GitHub PAT | 🔴 Critical | `ghp_…` |
| GitHub OAuth | 🔴 Critical | `gho_…` |
| Slack token | 🟠 High | `xoxb-…` |
| Stripe (live) | 🔴 Critical | `sk_live_…` |
| Stripe (test) | 🟡 Medium | `sk_test_…` |
| DB connection string | 🔴 Critical | `postgres://user:pass@…` |
| JWT-like token | 🟡 Medium | three dot-separated parts |

**12** content rules in total.

The `--no-content` flag disables content reading — only name matches remain.

### Reading limitations

| Situation | What happens |
|-----------|--------------|
| File larger than **1 MiB** | skipped |
| Binary file | skipped |
| Empty file | no findings |
| Finding in the report | only the **mask**, not the original |

### Repeats and exit code

| Flag / condition | Result |
|-------------------|--------|
| `--repeated` | same value in **2+** files |
| `--fail-on critical` *(default)* | code **2** if any Critical exists |
| `--fail-on high` | code **2** on High and above |
| `--fail-on ignore` | never fail because of findings |

Code **2** belongs to **`dig` only**. `sniff`, `pack`, `stash`, `rinse`, and `raid` use only `0` (success) and `1` (error).

---

## Skipped directories

During walking and packing these folders are **not entered**:

| Why skipped | Directories |
|-------------|-------------|
| Dependencies | `node_modules` |
| Build output | `target` · `dist` · `build` |
| Version control | `.git` · `.svn` · `.hg` |
| Python | `__pycache__` · `*.egg-info` |
| Virtual environments | `.venv` · `venv` · `.tox` |
| Caches | `.mypy_cache` · `.pytest_cache` · `.cache` |
| IDE | `.idea` · `.vscode` |
| raccpack storage | `.raccpack` |

**18** names in total.

Optionally, **all** hidden directories (dot-names) can be skipped too. This is **off** by default.

---

## Cleanup (`rinse`)

`racc rinse` removes **known build artifact directories** from the project according to **strategies** — rule sets of directory names considered trash. By default the command runs as **dry-run**; actual deletion requires `--yes`.

### Strategies

| Id | In defaults | Typical directories |
|----|-------------|---------------------|
| `rust` | yes | `target` |
| `node` | yes | `node_modules`, `.next`, `dist`, `.nuxt`, `coverage` |
| `python` | yes | `__pycache__`, `.venv`, `venv`, `.tox`, `.mypy_cache`, `.pytest_cache`, `*.egg-info`, `.ruff_cache` |
| `jvm` | opt-in | `build`, `.gradle`, `.m2` |
| `go` | opt-in | `vendor` |
| `generic` | opt-in | `.cache`, `tmp`, `temp` |

Only **`rust`**, **`node`**, and **`python`** are enabled by default. `jvm`, `go`, and `generic` are **opt-in**: their names (`build`, `vendor`, `tmp`/`temp`, partly `dist`) may turn out to be real sources or user data, so they are enabled explicitly.

Which strategies are enabled by default is set in `config.cleanup.enabled_strategies` (see [Configuration](/configuration)); the `--strategy` flag overrides config for one run. Command behavior and examples are on the [Rinse](/rinse) page.

---

## Packing (`pack`)

`racc pack` collects the project into a **tar.zst** archive and places it in the den.

### What does not get into the archive

| Rule | Included in archive? |
|------|-----------------------|
| File with 🟠 **High** or 🔴 **Critical** risk **by name** (`.env`, `id_rsa`…) | ❌ no (always) |
| File with 🔴 **Critical** content (a key in text) | ❌ no (default) |
| Same, but with `--no-content-deny` | ✅ yes (name deny remains) |
| Symlink | ❌ no |
| Service directory from the list above | ❌ no |
| Regular source / `config.json` (Medium) | ✅ yes |

### Archive structure

| Question | Answer |
|----------|--------|
| What's inside? | The contents of the project folder (`src/…`, `Cargo.toml`) without an extra wrapper |
| Format | `tar` + `zstd` compression |
| Where placed? | `packs/YYYY/MM/project-name__time.tar.zst` |
| Custom file name | `--output-name …` (without `.tar.zst`) |
| Writing to disk | only with `--yes`; otherwise dry-run |

---

## Not available yet

Currently **missing**:

| Command / capability | When |
|-----------------------|------|
| Custom markers and secrets in config | later |
| Frameworks by dependencies (Axum etc.) | later |
| Dedicated Windows optimization | later |

More in the [roadmap](/roadmap).

---

## Further reading

| Page | About |
|------|-------|
| [Concepts](/concepts) | den, risks, masking |
| [CLI usage](/cli-usage) | flags of `sniff` / `dig` / `pack` / `stash` / `rinse` / `raid` |
| [Rinse](/rinse) | cleanup strategies and examples |
| [Configuration](/configuration) | TOML configuration |
| [Facade API](/facade-api) | contract for integrations |
| [Roadmap](/roadmap) | what's coming in Alpha and beyond |
