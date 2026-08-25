---
layout: home
title: raccpack
hero:
  name: raccpack
  text: A modern tool for safe project backups
  tagline: Secrets go into encrypted age archives, build trash goes out, every project becomes a clean tar.zst in the den.
  image:
    src: /RaccPack.webp
    alt: raccpack
  actions:
    - theme: brand
      text: Command pipeline
      link: /concepts
    - theme: alt
      text: Quick start
      link: /quick-start
    - theme: alt
      text: Wiki
      link: /introduction
features:
  - title: Rust
    details: A fast, reliable, safe raccpack-core — one codebase for every interface.
  - title: age
    details: Secrets are encrypted with the age standard — via a passphrase or recipient keys; raw values live in memory only for the duration of encryption.
  - title: tar.zst
    details: Every project is packed into a clean tar.zst without secrets or build trash.
  - title: CLI · TUI · Desktop
    details: One business logic, three interfaces. The CLI currently offers sniff, dig, stash, rinse, pack and raid.
  - title: Safe by default
    details: Secrets are masked in reports; destructive operations start with a dry-run.
  - title: Den — storage
    details: Project archives (tar.zst) go to packs/, encrypted secrets (age) to secrets/, JSON manifests to manifests/.
---

## Why you need this

Developers keep dozens of projects in their working folder — with `.env` files, SSH keys, and multi-gigabyte build directories. Backing up such a folder as-is means leaking secrets and shipping tons of trash. raccpack automates tidying things up before packing.

## Pipeline

<DenPipeline />

## What next

- [Installation](/installation) — build and verify `racc`.
- [Quick start](/quick-start) — your first run in five minutes.
- [Core concepts](/concepts) — den, secrets, risks, phases.
