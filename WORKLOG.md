# WORKLOG — raccpack-core

Журнал статусов этапов. Orchestrator: y-tretyakov.

## Backlog

```
[x] 0.1 Inventory
[x] 0.2 Baseline (empty) — зафиксирован в M1.1
[x] M1.1 Workspace Cargo
[x] M1.2 Domain DTO
[x] M1.3 Config
[x] M1.4 Skip policy + walk
[x] M2.1 Marker files + skip dirs → candidates
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

### M1.4 — Skip policy + walk (CLOSED)

- **Дата:** 2026-08-05
- **Ветка:** `m1-skip-walk`
- **Статус:** done
- **Dev:** dev-1.4 · **Test:** test-1.4 (параллельно) · rework dev-1.4 (doctest, попытка 2)

#### Сделано
- `scan/` модуль: тонкий `mod.rs` (re-exports), `skip.rs` (SkipPolicy/SkipReason), `walk.rs` (WalkOptions/walk_tree/ensure_scan_root).
- `SkipPolicy`: `default_scan()` (18 имён в фиксированном порядке, включая `*.egg-info` как суффикс-паттерн), `empty()`, `with_custom_dir_names`, `with_skip_hidden_dirs` (default off), `should_skip_dir`/`skip_reason_dir` (детерминированный порядок DefaultDirName → CustomPattern → Hidden). Правило: паттерн с ведущим `*` = suffix match, иначе exact по lossy `file_name()`.
- `WalkOptions` Default = { max_depth: 6 (из `config::default_max_depth()`), policy: default_scan(), include_root: false }.
- `walk_tree`: `WalkDir` всегда `follow_links(false)` + `max_depth`; `filter_entry` пропускает только директории по policy; `include_root=false` не выдаёт root, `Err`-элементы не проглатываются.
- `ensure_scan_root`: `Error::PathNotFound` / `Error::NotADirectory` (domain Error, без unwrap).
- Rustdoc: инвариант «symlinks are never followed» в модульном доке и у `walk_tree`; `.DS_Store`/file-skip и `is_under_root`/path-containment задокументированы как follow-up; hidden-whitelist осознанно отсутствует (M1.4).
- Wiring: `lib.rs` — `pub mod scan;` + additive re-exports; `Cargo.toml` — `walkdir = "2"` (только dependencies).
- Rework 1: doc-блок в `walk.rs` был 4-space-indent и компилировался как невалидный doctest (E0425/E0433) → переписан в fenced `no_run` doctest с `#`-скрытыми setup-строками.

#### Тесты (test-1.4)
- `tests/skip_walk.rs` — 27 тестов: symlink isolation (наружу + cycle), skip node_modules/target/custom/suffix `*.egg-info`, max_depth (0/1/N), root validation (PathNotFound/NotADirectory/ok), default_scan состав, пустое дерево (0 и только-root), symlink не следует как dir, детерминизм, классификация причин (Default/Custom/Hidden/None), hidden-флаг off по умолчанию. Имена содержат `skip`|`walk` (см. замечание про команду ниже).

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo build --workspace` → pass
- `cargo test -p raccpack-core` → pass (84: 12 lib + 22 config + 22 domain + 27 skip_walk + 1 doctest, 0 failed)
- `cargo test -p raccpack-core -- skip walk` → pass (27 + 1 doctest)
- `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass
- `cargo fmt --all -- --check` → pass
- grep unwrap/anyhow/Box<dyn>: только pre-existing `#[cfg(test)]` в domain (M1.2); в `src/scan/` чисто
- WalkDir: только `walk.rs:76` с `follow_links(false)`

#### Критерий готовности (DoD из m1.4 §9)
- [x] `SkipPolicy::default_scan()` существует, включает `node_modules`, `target` и типичные cache/venv-имена
- [x] `should_skip_dir` / `skip_reason_dir` детерминированы
- [x] Walk-хелпер всегда создаёт `WalkDir` с `follow_links(false)`
- [x] `max_depth` соблюдается
- [x] Root validation через domain `Error` (PathNotFound/NotADirectory)
- [x] Тесты §7 зелёные, включая symlink isolation
- [x] `cargo test -p raccpack-core` green
- [x] rustdoc: явный инвариант «symlinks are not followed»

#### Follow-up / риски
- Команда из спеки `cargo test -p raccpack-core skip walk` — невалидный синтаксис cargo (допустим только один TESTNAME до `--`). Корректно: `cargo test -p raccpack-core -- skip walk`. В спеку m1.4 §7 не правил (docs/ не трогаем без ссылки); зафиксировано здесь.
- Hidden-флаг `with_skip_hidden_dirs(true)` применяется и к самому root: `tempfile::TempDir` (dot-dir) с включённым флагом даёт пустой walk — поведение по спеку, покрыто тестом через не-hidden subdir. В UX CLI/TUI при включённом hidden-режиме стоит явно предупреждать о скрытом root.
- `.DS_Store`/file-skip — вне скоупа M1.4 (файлы не матчатся). Для dig/pack нужна **отдельная file-policy** — не смешивать с `SkipPolicy` для директорий.
- `is_under_root`/path-containment — обязательный follow-up перед pack/stash (symlink / `..` / escape из root).
- M1.2-замечание «Config variant временный»: merge `ConfigError` ↔ `domain::Error` (единый enum или `From<ConfigError>`) отложен до facade-фазы / `AppContext`, чтобы UI не ветвился по двум типам ошибок.
- Cargo.lock: после каждого merge сверять, что raw dev даёт актуальную версию (человек: «raw dev отдавал старую версию»). На актуальном SHA: toml, walkdir; dev: tempfile, serial_test — ок.
- Windows: HOME/XDG-резолв Unix-центричен. Для v1 primary Linux — осознанно ок; на Windows позже (USERPROFILE / crate `directories`).

### M2.1 — Marker files + skip dirs → candidates (CLOSED)

- **Дата:** 2026-08-05
- **Ветка:** `m2-sniff-candidates`
- **Статус:** done
- **Dev:** dev-m2.1 · **Test:** test-m2.1 (параллельно)

#### Сделано
- `scan/markers.rs` (created): `MarkerKind` (FileName/DirName), `MarkerDef`, `MarkerHit`, `DEFAULT_MARKERS` — 14 маркеров (Rust `Cargo.toml`, JS `package.json`, Go `go.mod`, Python `pyproject.toml`/`setup.py`/`requirements.txt`, JVM `pom.xml`/`build.gradle`/`build.gradle.kts`, Ruby `Gemfile`, PHP `composer.json`, C++ `CMakeLists.txt`, generic `Makefile`) + `.git` (DirName). Реестр-таблица: добавление маркера = одна строка (registry pattern).
- `scan/candidates.rs` (created): `ProjectCandidate`, `CandidateOptions` (Default = depth 6 / `default_scan()` / no extras / `accept_git_only=true`), `find_candidates`. Алгоритм: `ensure_scan_root` → `walk_tree` (M1.4, `follow_links(false)`, max_depth, policy) + `inspect_dir` для каждой посещённой директории **и корня**; имена читаются одним `read_dir` в `HashSet<OsString>`, затем маркерная таблица перебирается в фиксированном порядке (детерминированные hits). `.git` не обходится walker'ом (skip-policy) и детектится через `read_dir` родителя — skip-политика не менялась. Симлинки не читаются (`symlink_metadata().is_symlink()`). Git-only фильтр через `accept_git_only`. Стабильная сортировка по `path`; nested-проекты не схлопываются.
- Ошибки: `read_dir` → `Error::Io{path,source}`; walkdir-ошибки → `Error::Io` (если io-источник) или `Error::Other` (loop detection). Без `unwrap()`/`anyhow`/`Box<dyn Error>`.
- `scan/mod.rs` + `lib.rs`: re-exports `MarkerKind/MarkerDef/MarkerHit/ProjectCandidate/CandidateOptions/find_candidates` (+ `DEFAULT_MARKERS` в scan) — additive, не breaking.

#### Тесты (test-m2.1)
- `tests/candidates.rs` (created) — 15 тестов: все кейсы §7 спеки (fixture app-rust/app-node/node_modules/nested/deep/only-git/empty-dir/target; находит 4 кандидата, node_modules/target исключены, max_depth, PathNotFound/NotADirectory, детерминизм, symlink не входит) + `accept_git_only=false`, root-as-candidate, `extra_markers` с language_hint, поля MarkerHit (Rust/JS/Go/.git), пустой root, поле `name`.

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo test -p raccpack-core` → pass (99: 12 lib + 15 candidates + 22 config + 22 domain + 27 skip_walk + 1 doctest, 0 failed)
- `cargo test -p raccpack-core --test candidates` → pass (15)
- `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass
- `cargo fmt --all -- --check` → pass
- `cargo doc -p raccpack-core --no-deps` → без warning/error (отчёт dev)
- grep unwrap/expect/anyhow/Box<dyn в `src/scan/markers.rs` + `src/scan/candidates.rs` → чисто

#### Критерий готовности (DoD из m2.1 §9)
- [x] Таблица `DEFAULT_MARKERS` покрывает минимум Rust/Node/Go/Python/JVM + `.git`
- [x] `find_candidates` возвращает `Vec<ProjectCandidate>` с markers и `is_git_repo`
- [x] SkipPolicy соблюдается (нет кандидатов из `node_modules`/`target`)
- [x] `follow_links(false)` сохранён
- [x] Тесты §7 зелёные
- [x] `cargo test -p raccpack-core` green
- [x] rustdoc на `find_candidates` и `ProjectCandidate`

#### Follow-up / риски
- M2.2 (detect languages/frameworks → `Stack`) — следующий этап, вход `ProjectCandidate` (+ опционально уточнение name).
- Extension-pattern маркеры (`*.csproj`/`*.sln`) осознанно отложены (спека: опционально в MVP); при необходимости — расширение `MarkerKind` отдельным этапом.
- Команда из спеки `cargo test -p raccpack-core candidates markers` — невалидный синтаксис (несколько фильтров до `--`); корректно: `cargo test -p raccpack-core --test candidates` (аналогично замечанию M1.4).
- `inspect_dir` повторяет `read_dir` для каждой посещённой директории (спека §5 — один read_dir на dir); при большом дереве возможна оптимизация через один проход walker'а, но это нарушает «маркеры по имени из entries» и отложено.

#### Follow-up review замечания (человек, 2026-08-05; PR #11)
- **A. Kind-aware matching — FIXED.** `inspect_dir` собирает `HashMap<OsString, bool>` (name → is_dir из `DirEntry::file_type()`, ошибки → `Error::Io`); `MarkerKind::FileName` матчит только не-директории, `DirName` — только директории. Файл `.git` и директория `Cargo.toml` больше не дают ложных hits. `file_type()` не следует симлинкам → симлинк не матчит `DirName`.
- **B. Nested projects — TEST ADDED.** `nested_projects_are_not_collapsed` (parent + child оба с `Cargo.toml` → два кандидата). Плюс регрессия kind: `file_named_git_is_not_a_git_marker`, `directory_named_cargo_toml_is_not_a_marker`, `git_dir_still_detected_as_dir_marker`. candidates-тесты: 15 → 19.
- **C. Case sensitivity — ЗАФИКСИРОВАНО.** Exact, case-sensitive match по `file_name()`; на macOS/Windows (case-insensitive FS) потребует политики — v1 primary Linux, осознанно ок (rustdoc `MarkerDef` уже отмечает).
- **D. `extra_markers: Vec<MarkerDef>` с `&'static str` — НЕ блокер.** Owned-вариант (String) для конфига/CLI — отложено до M2.2+/facade (см. Решения).
- **E. Двойной обход (walk + read_dir per dir) — принято** для ясности; оптимизация на больших деревьях позже.
- **F. WORKLOG backlog** — уже синхронизирован на SHA M2.1 (`[x] M2.1`).

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
| 2026-08-05 | M1.4: М1 закрыт (m1.4 merged #7). README Status обновлён: «M1 done, next M2 sniff». Замечания человека после приёмки (не блокеры, зафиксированы в follow-up M1.4): Cargo.lock сверять после merge; ConfigError↔Error на facade; is_under_root перед pack/stash; warning в UX про hidden root; отдельная file-policy; Windows HOME/XDG — позже. |
| 2026-08-05 | M2.1 review: exact case-sensitive match маркеров (Linux v1); macOS/Windows case-insensitive FS — отдельная политика позже. `MarkerDef.extra_markers` пока `&'static str`; owned-вариант (String) — когда понадобится конфиг/CLI (M2.2+/facade). |
