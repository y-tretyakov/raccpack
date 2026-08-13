# MVP 0.1.0 — Фаза M1: Каркас workspace и core

Индекс подробных спецификаций этапов. Каждый этап — отдельный документ.

| Этап | Файл | Кратко |
|------|------|--------|
| **M1.1** | [m1.1-workspace-cargo.md](m1.1-workspace-cargo.md) | Cargo workspace, `raccpack-core`, пустой `raccpack-cli`, лицензия, README |
| **M1.2** | [m1.2-domain-dto.md](m1.2-domain-dto.md) | `Project`, `Stack`, `ScanReport`, `SensitiveRisk`, `Error` |
| **M1.3** | [m1.3-config.md](m1.3-config.md) | TOML load/validate, `scan_root` / `den_dir`, strict errors |
| **M1.4** | [m1.4-skip-policy-walk.md](m1.4-skip-policy-walk.md) | `SkipPolicy` + walk с `follow_links(false)` |

## Порядок выполнения

```text
M1.1 → M1.2 → M1.3 → M1.4
```

- M1.1 блокирует всё остальное (нужен crate).
- M1.2 желателен перед M1.3 (`Error` / типы).
- M1.3 желателен перед M1.4 (`max_depth`).
- Параллельные Dev+Test: начиная с M1.2.

## Exit criteria фазы M1

- `cargo build --workspace` и `cargo test --workspace` green.
- Public domain DTO + `Error` в core.
- Config загружается из TOML, пути резолвятся строго.
- Walk никогда не следует symlink’ам; default skip для `node_modules` / `target` / caches.
- Нет UI-зависимостей в `raccpack-core`.

После M1 → **M2 Sniff**.

## Связанные документы проекта

- `raccpack-roadmap-v1.md` — вехи MVP/Alpha/Beta/RC/1.0
- `raccpack-architecture-vision.md` — границы core / UI
- `raccpack-facade-and-den.md` — сигнатуры facade и layout den
- `raccpack-agent-workflow.md` — процесс Orchestrator / Dev / Test
