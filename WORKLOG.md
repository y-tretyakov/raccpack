# WORKLOG — raccpack-core

Журнал статусов этапов. Orchestrator: y-tretyakov.

## Backlog

```
[x] 0.1 Inventory
[x] 0.2 Baseline (empty) — зафиксирован в M1.1
[x] M1.1 Workspace Cargo
[ ] M1.2 Domain DTO
[ ] M1.3 Config
[ ] M1.4 Skip policy + walk
```

## Этапы

### M1.1 — Workspace Cargo (CLOSED)

- **Дата:** 2026-08-05
- **Ветка:** `m1-workspace-core`
- **Статус:** done
- **Dev:** dev-1.1 · **Test:** n/a (scaffolding / design, по спеке m1.1 §Параллельность)

#### Сделано
- Cargo workspace (`resolver = 2`, members `crates/raccpack-core`, `crates/raccpack-cli`, `[workspace.package]` version 0.1.0 / edition 2021 / license MIT OR Apache-2.0 / rust-version 1.75).
- `raccpack-core` — library: `core_version()`, smoke-тест `version_is_semver_like`, **без зависимостей**.
- `raccpack-cli` — binary `racc`: печатает `raccpack {core_version}`, зависит только от core.
- Лицензии `LICENSE-MIT`, `LICENSE-APACHE` в корне.
- Корневой README: структура workspace, команды сборки, политика `Cargo.lock` (коммитится, binary workspace).
- `.gitignore`: убрано игнорирование `Cargo.lock`.
- `rust-toolchain.toml`: channel stable.

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo build --workspace` → pass
- `cargo test --workspace` → pass (1 test core)
- `cargo run -p raccpack-cli` → `raccpack 0.1.0`
- `cargo fmt --check` → pass
- `git check-ignore Cargo.lock` → exit 1 (не игнорируется)

#### Критерий готовности (DoD из m1.1)
- [x] Workspace c `members = ["crates/raccpack-core", "crates/raccpack-cli"]`
- [x] `cargo build --workspace` green
- [x] `cargo test --workspace` green
- [x] `cargo run -p raccpack-cli` печатает версию
- [x] `raccpack-core` не зависит от CLI/UI crate'ов
- [x] Лицензия в корне и в workspace.package
- [x] README: структура + сборка
- [x] `.gitignore` исключает `target/`; Cargo.lock коммитится

#### Follow-up / риски
- Следствие глобального un-ignore `Cargo.lock`: в `.agents/skills/rust-skills/checks/` стал виден pre-existing untracked `Cargo.lock` (чек-тулинг скилла). Вне scope M1.1, `.agents/` не трогаем; в коммит не попадает (stage только согласованные файлы).
- Принять решение по MSRV 1.75 позже при выборе edition/deps.

## Принятые решения

| Дата | Решение |
|------|---------|
| 2026-08-05 | License: MIT OR Apache-2.0 (workspace). `Cargo.lock` коммитится (binary workspace). Edition 2021, MSRV 1.75. |
