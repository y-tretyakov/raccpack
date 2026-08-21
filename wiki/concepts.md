---
title: Core concepts
description: Key raccpack terms — the den, its structure and naming, secrets, risks, masking, and operation phases.
---

# Core concepts

This page gathers the key raccpack terms: what den, secrets, and risks are, and how scanning and packing work.

## Den

**Den** (English *den* — a lair, a storage place) is the directory where raccpack puts its results. It is your local protected storage, not a working copy of your projects.

The den should never be committed to git or synced to the cloud.

### Den structure

```
{den_dir}/
├── README.txt                 # brief description of the directory
├── .den-version               # format version ("1")
├── manifests/{yyyy}/{mm}/     # JSON manifests of every raid
├── secrets/{yyyy}/{mm}/       # encrypted secret archives (.age)
├── packs/{yyyy}/{mm}/         # project archives (.tar.zst)
└── staging/{short_id}/        # temporary files (safe to clean)
```

### Naming conventions

Every artifact gets a deterministic name:

| Token | Rule |
|-------|------|
| `project_slug` | The project folder name stripped of special characters: `[a-zA-Z0-9._-]`, spaces → `-`, length ≤ 80 |
| `utc_timestamp` | `YYYYMMDDThhmmssZ` (UTC) |
| `short_id` | 8 hexadecimal characters for uniqueness |

Examples:

```text
secrets/2026/08/my-api__20260804T155230Z__secrets.age
packs/2026/08/my-api__20260804T155230Z.tar.zst
manifests/2026/08/my-api__20260804T155230Z__a1b2c3d4.json
```

A single den serves any number of projects — they differ only by project name and time.

### Format version

A `.den-version` file (currently `1`) lives at the root of the den. If the format ever changes in an incompatible way, raccpack will refuse to write into an old den and will offer a migration.

## Secrets

raccpack finds secrets in two ways.

### By file name

Only the file name is checked — contents are not read. Example rules:

- `.env`, `.env.local`, `.env.production` — environment files;
- `id_rsa`, `id_ed25519`, `*.pem`, `*.key`, `*.ppk`, `*.p12`, `*.jks` — keys and keystore files;
- `credentials`, `.netrc`, `.npmrc`, `.pypirc`, `.git-credentials` — credential files;
- `kubeconfig`, `secrets.json`, `secrets.yaml`, `service-account`, `wallet.dat`.

> → full list of what is supported by file name: [Supported](/supported)

### By content

The file is read (within limits) and checked against built-in markers:

- AWS access key (`AKIA…`) and `aws_secret_access_key=…` assignments;
- `-----BEGIN … PRIVATE KEY-----` headers;
- GitHub tokens (`ghp_`, `gho_`);
- Slack (`xoxb-`), Stripe (`sk_live_`, `sk_test_`);
- database connection strings (`postgres://user:pass@…`, `mysql://`, `mongodb://`);
- JWT-like tokens;
- generic assignments like `api_key=…`, `password=…`, `secret=…`.

Content scanning has limits: files larger than 1 MiB and binary files are skipped.

> → full list of what is supported by content: [Supported](/supported)

### Risks

Each finding gets a risk level:

| Level | Meaning |
|-------|---------|
| `Low` | Informational, low confidence |
| `Medium` | Worth checking |
| `High` | Probably a secret |
| `Critical` | Almost certainly a key/credential |

Risk is used for filters, the minimum threshold when moving secrets out, and the exit-code policy.

### Masking

In reports, logs, and JSON, **masked previews**, a stable blake3 hash, and the value's length are shown instead of secret values. Raw values never leave the core and do not appear in output by default.

## Scanning (sniff)

`sniff` finds projects under `scan_root`. A project is identified by **markers** — characteristic files:

- Rust — `Cargo.toml`;
- Node.js — `package.json`;
- Go — `go.mod`;
- Python — `pyproject.toml`, `setup.py`, `requirements.txt`;
- JVM — `pom.xml`, `build.gradle`, `build.gradle.kts`;
- Ruby — `Gemfile`;
- PHP — `composer.json`;
- C/C++ — `CMakeLists.txt`;
- Make — `Makefile`;
- Git — `.git`.

For each project, the stack (language + frameworks), size, and git-repository flag are determined. Results are cached; on a repeated run without changes, `sniff` reads from the cache (`cache: hit`).

> → full list of markers and frameworks: [Supported](/supported)

## Packing (pack)

`pack` creates a project archive in **tar + zstd** (`tar.zst`) format. When packing:

- secrets are excluded by name (risk ≥ `High`) — always; file-content checks are enabled by default (risk ≥ `Critical`) and can be disabled with the `--no-content-deny` flag;
- service directories are skipped according to skip-policy rules;
- symbolic links are not preserved;
- the archive contains the contents of the project folder, not the folder itself.

::: warning
Secrets are not "fixed" in the original files — they are moved into an encrypted archive in a separate step (`stash`), and the source can then be deleted if desired.
:::

## Cleaning (rinse)

`rinse` removes build artifact directories from a project according to **strategies** — sets of names (`target`, `node_modules`, `__pycache__`, …). Which strategies are active by default is set in `config.cleanup.enabled_strategies`; the `--strategy` flag overrides this for a single run.

By default, `rust`, `node`, and `python` are enabled; `jvm`, `go`, and `generic` are opt-in (cautious names such as `build`, `vendor`, `tmp`). By default the command runs as a dry-run; deletion only happens with `--yes`.

> → list of strategies: [Supported](/supported) · configuration: [Configuration](/configuration) · command: [Rinse](/rinse)

## Full cycle (raid)

`raid` performs everything in one action, strictly phase by phase:

1. **stash** — move secrets into `den/secrets/…/*.age` (age encryption);
2. **rinse** — delete build trash;
3. **pack** — pack the project into `den/packs/…/*.tar.zst`;
4. **move** — finalize artifacts and write a JSON manifest.

By default raid runs **atomically**: artifacts are written to a temporary `den/staging/{id}/` and only moved into the den during the commit phase; if commit fails, the effect is rolled back (`rolled_back: true` in the report). The `--fail-fast` flag enables the old mode: stop at the first failed phase, leaving already-written artifacts in the den. The manifest is written **only** after a successful commit and only if artifacts were actually placed in the den — it lists artifacts and phase statuses, with no raw secrets.

## CLI exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime error |
| `2` | Secrets above the policy threshold were found — **only for `dig`**. `pack`, `stash`, `rinse`, and `raid` use only `0`/`1` |

## Further reading

- [Architecture concepts](/architecture) — how raccpack is structured inside.
- [Facade API](/facade-api) — the public contract of the core.
- [Roadmap](/roadmap) — what exists already and what is planned.
