# WORKLOG — raccpack-core

Журнал статусов этапов. Orchestrator: y-tretyakov.

## Backlog

```
[x] 0.1 Inventory
[x] 0.2 Baseline (empty) — зафиксирован в M1.1
[x] M1.1 Workspace Cargo
[x] M1.2 Domain DTO
[x] M1.3 Config
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

### M1.2 — Domain DTO (CLOSED)

- **Дата:** 2026-08-05
- **Ветка:** `m1-domain-dto`
- **Статус:** done
- **Dev:** dev-1.2 · **Test:** test-1.2 (параллельно)

#### Сделано
- `SensitiveRisk` (Low/Medium/High/Critical, Ord Critical>High>Medium>Low, serde PascalCase, as_str/from_str_ignore_case).
- `Stack` (language/frameworks/markers, Default), `Project` (path/name/stack/size_bytes/is_git_repo), `ScanReport` (root/projects/total_size_bytes/schema_version=1).
- `Error` (thiserror: PathNotFound/NotADirectory/Io/Config/Other) + `suggestion()` + `Result<T>`. Без anyhow/Box<dyn Error>.
- Re-exports в lib.rs; rustdoc на всех public items.
- Deps core: serde(derive), thiserror; serde_json только dev.

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo build -p raccpack-core` → pass
- `cargo test -p raccpack-core` → pass (12 unit + 22 integration, 0 failed)
- `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass
- `cargo fmt --check` → pass
- grep unwrap/anyhow/Box<dyn: только #[cfg(test)] и doc-комментарий

#### Критерий готовности (DoD из m1.2)
- [x] Типы существуют и публично доступны
- [x] Все типы (кроме Error) Serialize+Deserialize
- [x] SensitiveRisk: Ord
- [x] Error: std::error::Error + suggestion()
- [x] Нет UI/CLI зависимостей в core
- [x] Unit-тесты §7 зелёные
- [x] build + test green
- [x] rustdoc на каждом public type

#### Follow-up / риски
- `Error::Io` не PartialEq (io::Error) — сравнения через matches!/dyn-cast.
- Config variant временный; на M1.3 решить ConfigError.

### M1.3 — Config (CLOSED)

- **Дата:** 2026-08-05
- **Ветка:** `m1-config`
- **Статус:** done
- **Dev:** dev-1.3 · **Test:** test-1.3 (параллельно)

#### Сделано
- `RaccConfig` / `PathsConfig` / `ScannerConfig` — sections-style TOML (`[paths]`, `[scanner]`), `serde(default)`, `deny_unknown_fields` off (будущие секции не ломают парсинг). `max_depth` default = 6.
- `load()` (RACCPACK_CONFIG → XDG default → `Default`), `load_from_path()` (FileNotFound / Read / Parse + validate), `scan_root_dir()` / `den_dir()` (default `~/.raccpack/den`), builder `with_scan_root` / `with_den_dir`.
- `ConfigError` (thiserror, отдельный от `domain::Error`, без `From` на этом этапе) + `suggestion()` на ключевых вариантах.
- Резолв путей: `~`→HOME (нет HOME → PathResolve, не silent `/`), relative→cwd, empty→missing, scan_root требует существующую директорию, без canonicalize. Правила зафиксированы в rustdoc `paths.rs`.
- `docs/config.example.toml`; re-exports в `lib.rs` (additive, не breaking).
- Модульность (по `raccpack-modularity.md`): пустые `secrets/`/`archive/` скелеты НЕ созданы — не блокируют config, задеплоено на M1.4/M3.

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo build -p raccpack-core` → pass
- `cargo test -p raccpack-core` → pass (22 unit + 22 domain integration + 22 config integration, 0 failed)
- `cargo test -p raccpack-core config` → pass (22)
- `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass
- `cargo fmt --check` → pass
- grep unwrap/anyhow/Box<dyn Error>: только pre-existing `#[cfg(test)]` в domain (M1.2) и doc-комментарий

#### Критерий готовности (DoD из m1.3)
- [x] RaccConfig десериализуется из TOML
- [x] load() и load_from_path() работают по правилам §5
- [x] scan_root_dir()/den_dir() возвращают PathBuf или строгую ошибку
- [x] ConfigError с Display + suggestion() на ключевых вариантах
- [x] Нет anyhow / Box<dyn Error> в public API
- [x] Тесты §9 зелёные (покрыты §9 + дополнительные кейсы 10–16)
- [x] cargo test -p raccpack-core green
- [x] docs/config.example.toml создан

#### Follow-up / риски
- Merge `ConfigError` ↔ `domain::Error` (с `From`) — отложен до facade-фазы.
- XDG резолвится вручную через env (`directories` не добавлен).
- Скелеты `secrets/`/`archive/` — при старте M1.4/M3 (modularity §4).

## Принятые решения

| Дата | Решение |
|------|---------|
| 2026-08-05 | License: MIT OR Apache-2.0 (workspace). `Cargo.lock` коммитится (binary workspace). Edition 2021, MSRV 1.75. |
| 2026-08-05 | M1.2: SensitiveRisk serde = PascalCase (стабильно, breaking при смене). schema_version = 1. |
| 2026-08-05 | Репозиторий переведён в PUBLIC. Включены rulesets: `main` (PR + 1 approval, no force push, no deletions) и `dev` (PR, no force push, no deletions). Bypass: maintain/admin. |
| 2026-08-05 | Rulesets `main`/`dev`: allowed_merge_methods ограничены до `["squash"]` — приведено в соответствие с политикой README (было merge/squash/rebase). |
| 2026-08-05 | Документация: спеки живут в `docs/` (mvp + roadmap/vision/facade) и дублируются ссылками из корня. Решение отложено: перенести спеки в `docs/`, оставить в корне README + AGENTS + WORKLOG (TODO позже, не сейчас). |
| 2026-08-05 | Разрез фаз: AGENTS описывает фазы 0–11 (Group enum, WalkSession…), roadmap — M1.1–M1.4. Orchestrator следует текущему backlog в WORKLOG/docs/mvp; НЕ прыгать в «фазу 7 walk session» вместо M1.3 Config. |
| 2026-08-05 | AGENTS.md: обновлена строка «текущее состояние» (было «нет дерева crate», стало — workspace развёрнут, M1.1 done). |
| 2026-08-05 | M1.3: config-стиль = секции `[paths]`/`[scanner]`; relative paths резолвятся от `current_dir()` (не от dir файла); `den_dir` default = `~/.raccpack/den`; `deny_unknown_fields` off; без canonicalize; `ConfigError` отдельный от `domain::Error` до facade-фазы. |
