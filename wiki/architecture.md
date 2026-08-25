---
title: Architecture (high level)
description: "How raccpack is structured: core and facade layers, interfaces, trust boundaries, and the data flow."
---

# Architecture (high level)

This section explains how raccpack is structured "top down", without developer internals. You don't need to know this to use the tool, but it helps to understand why behavior is identical across all interfaces and where the security guarantees come from.

## Overall diagram

![Overall diagram: interfaces → facade → core](/architecture.webp)

## The main rule

All business logic — secret detection, skip rules, report formats, den structure — lives **in the core** `raccpack-core`. CLI, TUI, and Desktop are only "wrappers" that call the same public contract (the facade) and display the result. That is why output, risks, and policies match across all three interfaces.

## What each layer does

### Core (raccpack-core)

- **config** — loading TOML, validating and migrating configuration, paths.
- **scan** — tree traversal, skip rules, project discovery by markers.
- **detect** — determining a project's language and frameworks.
- **secrets** — finding secrets by name and content, risk model, masking, hashes.
- **archive** — packing into `tar.zst`, moving secrets into `age` archives.
- **den** — storage layout, artifact naming, file placement.
- **cache** — cache of scan results.
- **report** — stable DTOs (data structures) for reports, JSON-friendly.
- **policy** — unified rules for "what not to traverse / what is forbidden when packing".

The core knows nothing about interfaces: no Ratatui, no React, no interactive prompts. Progress of long operations is reported outward through events that an interface subscribes to.

### Facade (use-case layer)

One public contract for all interfaces:

| Operation | Purpose |
|-----------|---------|
| `sniff` | Find projects, stacks, sizes |
| `dig` | Find secrets (read-only) |
| `stash` | Move secrets into an `age` archive |
| `rinse` | Delete build trash |
| `pack` | Pack a project without secrets or trash |
| `raid` | All together: stash → rinse → pack → finalize into the den |

### Interfaces

- **CLI** — command-line arguments, human-readable or JSON output, exit codes.
- **TUI** — interactive tree, filters, progress.
- **Desktop** — React interface, Tauri commands as a middle layer (BFF) to the core.

## Trust boundaries and security

| Zone | Rule |
|------|------|
| **Core** | The only place where a raw secret may exist in memory; after use the memory is zeroed |
| **CLI / TUI** | May request showing a secret explicitly; by default — masked |
| **Desktop (React)** | Never receives raw secrets — only DTOs with masked values |
| **Den on disk** | age files; access permissions are configured by the user |
| **CI** | JSON report + policy-driven failure; usually without showing secrets |

## Data flow (happy path)

![Data flow (happy path): sniff → dig → raid](/happy-path.webp)

## Extensibility

- New languages — by adding markers and detection rules in the core.
- New secret types — by rule groups in the core tables.
- Another encryption algorithm — by a new backend behind a common interface.
- A new interface — just another frontend on the same facade.

## Further reading

- [Facade API](/facade-api) — concrete signatures of the public contract.
- [Core concepts](/concepts) — den, risks, phases.
