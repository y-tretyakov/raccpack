---
title: Quick start
description: Your first raccpack run in five minutes — configuration, project discovery, and a secret check.
---

# Quick start

In five minutes: set up raccpack, find your projects, and check them for secrets.

## 1. Make sure `racc` is installed

```bash
racc --version
```

If the command is missing, see [Installation](/installation).

## 2. Create the configuration

Specify the projects folder and the "den" folder:

```bash
mkdir -p ~/.config/raccpack
cat > ~/.config/raccpack/config.toml <<'EOF'
[paths]
scan_root = "~/DEV/PROJS"
den_dir = "~/.raccpack/den"
EOF
```

::: info
Paths may contain `~` and relative components — raccpack will resolve them to absolute paths itself.
:::

## 3. Find projects (`sniff`)

```bash
racc sniff
```

Example output:

```text
Scan root: /home/user/DEV/PROJS
Projects: 3  |  Total size: 1.2 GiB  |  210 ms  |  cache: miss

NAME        STACK                 SIZE    GIT  PATH
app-api     Rust + Axum           412.5 MiB  yes  /home/user/DEV/PROJS/app-api
web-dashboard TypeScript + Next.js 730.1 MiB  yes  /home/user/DEV/PROJS/web-dashboard
scripts     -                      1.8 MiB   no   /home/user/DEV/PROJS/scripts
```

If no projects are found — check `scan_root` and the scan depth (see [Configuration](/configuration)).

## 4. Check projects for secrets (`dig`)

```bash
racc dig
```

or one project at a time:

```bash
racc dig --project ~/DEV/PROJS/app-api
```

Example output:

```text
Dig root: /home/user/DEV/PROJS
Files scanned: 1204  |  Findings: 4  |  Repeated: 1  |  180 ms

RISK      LABEL                    PATH
Critical  AWS Access Key           /home/user/DEV/PROJS/app-api/app/config/aws.env
Critical  Private key PEM          /home/user/DEV/PROJS/app-api/certs/server.key
High      Env file                 /home/user/DEV/PROJS/app-api/app/.env
Medium    JWT-like token           /home/user/DEV/PROJS/scripts/token.txt
```

::: info
Original values never appear in the output — only masked previews and a risk level.
:::

## 5. Understand the exit code

`racc dig` returns an exit code suitable for CI:

- `0` — no errors;
- `1` — a runtime error occurred;
- `2` — secrets above the policy threshold were found (`Critical` by default).

This is handy for checks in scripts:

```bash
racc dig --fail-on high
code=$?
if [ "$code" -eq 2 ]; then
  echo "High-or-above secrets found"
fi
```

## 6. Pack a project (`pack`)

```bash
racc pack --project ~/DEV/PROJS/app-api --yes
```

By default `pack` is a **dry-run** (nothing is written); the `--yes` flag is the explicit confirmation that writes the `.tar.zst` archive into the den. Secrets are excluded from the archive automatically (by name — always; by content — by default).

## 7. Move secrets out (`stash`)

```bash
racc stash --project ~/DEV/PROJS/app-api
racc stash --project ~/DEV/PROJS/app-api --yes
```

The first run is a **dry-run** (nothing is written); the `--yes` flag moves sensitive files into an encrypted age archive under `den/secrets/`. The passphrase is provided via `RACCPACK_PASSPHRASE` or entered interactively.

## 8. Clean build trash (`rinse`)

```bash
racc rinse --project ~/DEV/PROJS/app-api
racc rinse --project ~/DEV/PROJS/app-api --yes
```

The first run is a **dry-run** (nothing is deleted); the `--yes` flag deletes build artifact directories (`target`, `node_modules`, …) according to strategies from the configuration. Which strategies are enabled by default and how to enable `jvm`, `go`, or `generic` — see [Configuration](/configuration) and [Rinse](/rinse).

## 9. What next

The CLI currently supports `sniff`, `dig`, `pack`, `stash`, `rinse`, and `raid`; on the roadmap — `den`, `init`. For a command overview see [CLI usage](/cli-usage); details on each command are on the `/sniff`, `/dig`, `/pack`, `/stash`, `/rinse`, and `/raid` pages:

- [CLI usage](/cli-usage) — full command reference.
- [Core concepts](/concepts) — what den, risks, and phases are.
- [Configuration](/configuration) — all settings.
