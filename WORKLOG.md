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
[x] M2.2 Detect languages/frameworks → Stack
[x] M2.3 Facade sniff + versioned cache
[x] M2.4 CLI sniff (racc sniff --root, text + --json)
[x] M3.1 Filename patterns + risk model (severity API)
[x] M3.2 Content markers (regex/prefix) + size limits + mask/fingerprint
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

### M2.1-followup — markers modularity (CLOSED)

- **Дата:** 2026-08-06
- **Ветка:** `m2.1-markers-split`
- **Статус:** done
- **Dev:** dev-m2.1-split · **Test:** test-m2.1-split (параллельно)

#### Сделано
- Принятое решение `raccpack-markers-detect-modularity.md` зафиксировано в корне рядом с `raccpack-modularity.md` (один язык/экосистема ≈ один файл; агрегация только в registry; `candidates` не знает языков; detect по файлам экосистем — с M2.2).
- `scan/markers.rs` разрезан в `scan/markers/`: `types.rs` (MarkerKind/MarkerDef/MarkerHit — определения без изменения), 10 group-файлов (`rust/node/go/python/jvm/ruby/php/cpp/make/git`), тонкий `mod.rs` — registry `default_markers() -> &'static [MarkerDef]` на `std::sync::OnceLock` (без новых deps, MSRV-safe). Порядок групп **воспроизводит эффективный порядок M2.1** (rust, node, go, python, jvm, ruby, php, cpp, make, git), а не иллюстративный python-before-go из документа — hit-порядок `find_candidates` не регрессирует.
- `candidates.rs`: `DEFAULT_MARKERS` → `default_markers()`; публичный API `find_candidates`/`CandidateOptions`/`ProjectCandidate` без изменений.
- Re-exports: `scan::default_markers` (+ additive в top-level `lib.rs`). Новый маркер/язык = новая строка в группе или новый group-файл + одна строка в `GROUPS`.

#### Файлы
- `raccpack-markers-detect-modularity.md` (created)
- `crates/raccpack-core/src/scan/markers/{mod,types,rust,node,python,go,jvm,ruby,php,cpp,make,git}.rs` (created)
- `crates/raccpack-core/src/scan/markers.rs` (deleted)
- `crates/raccpack-core/src/scan/candidates.rs` (changed)
- `crates/raccpack-core/src/scan/mod.rs` (changed)
- `crates/raccpack-core/src/lib.rs` (changed)
- `crates/raccpack-core/tests/markers_registry.rs` (created, 4 теста)

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo build --workspace` → pass
- `cargo test -p raccpack-core` → pass (107: 12 lib + 19 candidates + 22 config + 22 domain + 4 markers_registry + 27 skip_walk + 1 doctest, 0 failed)
- `cargo test -p raccpack-core --test markers_registry` → pass (4)
- `cargo test -p raccpack-core --test candidates` → pass (19, файл без правок)
- `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass
- `cargo fmt --all -- --check` → pass
- grep unwrap/expect/anyhow/Box<dyn/once_cell в изменённых файлах → чисто (только внутренний статик `DEFAULT_MARKERS` как `OnceLock` в mod.rs)

#### Критерий готовности
- [x] `markers.rs` → `markers/` registry, поведение не изменено
- [x] `default_markers()` = 14 маркеров в точном порядке M2.1 (зафиксировано инвариант-тестом)
- [x] `find_candidates` API/поведение не изменены (19 тестов candidates без правок)
- [x] build/clippy/fmt green, без unwrap/anyhow/новых deps

#### Follow-up / риски
- **Breaking (pre-1.0):** публичный статик `scan::DEFAULT_MARKERS` → `scan::default_markers()`. Внешних callers нет; пометка для CHANGES при релизе.
- `detect/` по экосистемам (`trait StackDetector` + registry в `detect/mod.rs`) — этап M2.2; резать сразу по файлам экосистем, не god-file `match language`.

### M2.2 — Detect languages/frameworks → Stack (CLOSED)

- **Дата:** 2026-08-06
- **Ветка:** `m2.2-detect-stack`
- **Статус:** done
- **Dev:** dev-m2.2 · **Test:** test-m2.2 (параллельно) · rework test-m2.2 (компиляция/фикстуры, попытка 2) · rework dev-m2.2 (doc warnings, попытка 2)

#### Сделано
- `detect/` — новый модуль по экосистемам (accepted decision `raccpack-markers-detect-modularity.md`): `trait StackDetector` (`id`/`matches`/`detect -> Result<Stack, Error>`) + таблица приоритетов языка §4.1 в `types.rs`; 10 детекторов (rust/node/go/python/jvm/ruby/php/cpp/make/git) по одному файлу; registry `all_detectors()` + оркестрация/merge в `mod.rs`. Размещение — top-level `src/detect/` (по рекомендации спеки §3 и архитектурному vision, где detect — отдельная подсистема; в modularity-документе дерево было иллюстративным).
- Public API (спека §5): `stack_from_candidate` (PURE), `detect_stack` (PathNotFound/NotADirectory/Io), `detect_stacks` (fail-fast батч), `candidate_to_project` (§6). Re-exports в `lib.rs` additive.
- Merge policy (rustdoc модуля): language — центрально по приоритету §4.1 (tie → первый hit в порядке markers; fallback на первый hit с hint для extra_markers); frameworks — union по registry-порядку с dedup (first wins); markers — сортированные уникальные имена hit'ов.
- Framework hints по именам файлов (MVP, без парсинга манифестов): `next.config.{js,mjs,ts}`→Next.js, `nuxt.config.*`→Nuxt, `angular.json`→Angular, `vite.config.*`→Vite, `deno.json`→Deno, `manage.py`→Django, `Gemfile`+`config/application.rb` (config — реальная dir через `symlink_metadata`)→Rails, `build.sbt`→Scala/sbt. `detect_stack` с пустыми markers (path-only) пробирует все детекторы.
- `scan/size.rs`: `project_size_bytes(path, policy, max_depth)` на существующем `walk_tree` (`follow_links(false)`, policy уважается). Симлинки не считаются и не следуются; unreadable-файлы skip+continue; walk-ошибки → fail-fast (`Error::Io`/`Other`). Re-export в `scan::mod.rs` и `lib.rs`.
- Парсинг `package.json`/`Cargo.toml` (next/react/vue/axum по deps) — **отложен** (не блокер MVP, спека §4.2).

#### Файлы
- `crates/raccpack-core/src/detect/{mod,types,rust,node,go,python,jvm,ruby,php,cpp,make,git}.rs` (created)
- `crates/raccpack-core/src/scan/size.rs` (created)
- `crates/raccpack-core/src/scan/mod.rs` (changed), `crates/raccpack-core/src/lib.rs` (changed)
- `crates/raccpack-core/tests/detect_stack.rs` (created, 22 теста)

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo build --workspace` → pass
- `cargo test -p raccpack-core` → pass (161, 0 failed: 12 lib + 29 unit detect/size + 22 config + 22 domain + 19 candidates + 4 markers_registry + 27 skip_walk + 1 doctest + 22 detect_stack)
- `cargo test -p raccpack-core -- detect` → pass; `-- stack` → pass
- `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass
- `cargo fmt --all -- --check` → pass
- `cargo doc -p raccpack-core --no-deps` → без warning на `detect/` (остался один pre-existing warning `markers/mod.rs:53` — вне скоупа)
- grep unwrap/expect/anyhow/Box<dyn: только `#[cfg(test)]`; `WalkDir` в новых файлах не используется (только `walk_tree`)

#### Критерий готовности (DoD из m2.2 §9)
- [x] `stack_from_candidate` / `detect_stack` заполняют `Stack` по правилам §4
- [x] Приоритет language при конфликте задокументирован (rustdoc) и покрыт тестом
- [x] Framework hints по именам файлов: Next.js + Nuxt/Angular/Vite/Deno/Django/Rails/Scala-sbt (7 экосистем)
- [x] `project_size_bytes` уважает SkipPolicy и `follow_links(false)`
- [x] Тесты §7 зелёные
- [x] `cargo test -p raccpack-core` green

#### Риски / follow-up
- Парсинг manifest-файлов для framework-deps (next/react/vue/axum) — отложен на Alpha (спека §4.2 optional).
- `trait StackDetector::detect` возвращает `Result<Stack, Error>` (а не `-> Stack`, как в иллюстрации modularity-документа): bare `Stack` не выражает `Error::Io` при shallow-read, что требует спека §5. Зафиксировано в rustdoc `detect/types.rs`.
- Pre-existing rustdoc-warning `markers/mod.rs:53` (`GROUPS` — приватный item в ссылке из публичного док-комментария) — с M2.1-followup, вне скоупа M2.2; закрыть отдельным мелким этапом.
- `project_size_bytes` fail-fast на walk-ошибках (нечитаемый каталог роняет размер). Консистентно с `candidates.rs`, но для UX можно перевести на skip+continue позже.
- `.git`-marker не влияет на language (по спека §4.1 таблица без `.git`) — покрыто тестом.

#### Follow-up review замечания (человек, 2026-08-06; PR #13) — НЕ блокеры M2.2
- **A. `detect/mod.rs` ~400+ строк — ок** (часть — `#[cfg(test)]` unit-тесты). Если разрастётся — вынести тесты в `detect/tests_unit.rs`; до M2.3 оставить как есть.
- **B. Пустые markers → probe all detectors — ПРИНЯТО** (осознанно path-only, чуть шире, чем «только matched ecosystem»; для sniff-кейсов hits обычно не пустые). Зафиксировано как решение.
- **C. Manifest deps (next/react в package.json) — отложено на Alpha** (в PR body) — корректно.
- **D. `size.rs` symlink unit-тест** — по сути `#[cfg(unix)]` (unix symlink API); primary Linux — ок.
- **E. Кэш GitHub иногда отстаёт от tip dev** — при ревью ориентироваться на merge SHA `45363aa`.

#### Идея «на вырост» — модели detect-фреймворков (ОБЯЗАТЕЛЬНО к последующей реализации, не забыть)
- Не глобальный список фреймворков, а **вложенность внутри экосистемы**:
  ```
  textdetect/
    node/
      mod.rs          # NodeDetector: matches + вызывает hints
      next.rs
      nuxt.rs
      vite.rs
      …
    python/
      mod.rs
      django.rs
    ruby/
      mod.rs
      rails.rs
    types.rs
    mod.rs            # all_detectors() без изменений снаружи
  ```
- Снаружи API тот же: `StackDetector` по экосистемам. Фреймворки — детали реализации экосистемы.
- Практический критерий (когда сплитить):
  - M2.2 / M2.3: оставить как есть.
  - Когда добавляете **4–5+ framework-правил в один файл** или конфигурируемое **«включить только Next»** — тогда split внутри экосистемы.
  - **НЕ делать** плоский `detect/frameworks/next.rs` рядом с языками: Next без Node-контекста почти бессмысленен.

### M2.3 — Facade sniff + versioned cache (CLOSED)

- **Дата:** 2026-08-06
- **Ветка:** `m2.3-sniff-cache`
- **Статус:** done
- **Dev:** dev-m2.3 · **Test:** test-m2.3 (параллельно, без rework)

#### Сделано
- `app/` — facade-слой (spec §3): `context.rs` (`WorkspacePaths{scan_root,den_dir}` serde, `RunMode`+`is_dry_run()`, `SecretExitPolicy`, `AppContext` + `from_config(config, mode) -> Result<Self, ConfigError>`, exit_policy default FailOnCritical), `progress.rs` (`OperationKind`, `ProgressEvent`, `trait ProgressSink: Send`, `NullProgress`), `sniff.rs` (`SniffOptions`, `SniffResult`, `sniff`).
- `sniff()` по алгоритму §4: `ensure_scan_root` → max_depth из opts или `config.scanner.max_depth` → policy `default_scan` + fp const `"default_scan_v1"` → cache-read (если `!force_refresh`) → `find_candidates` → `detect_stack` per candidate (enrich frameworks) + `project_size_bytes(...).unwrap_or_default()` (ошибка размера → 0, sniff не падает) → `ScanReport{schema_version: 1}` → best-effort `store_sniff_cache` (ошибка глотается) → progress-события 0/40/90/100 (`phase="scan"`, `OperationKind::Sniff`, phase_index=0, phase_count=1, overall==percent); cache-hit тоже эмитит complete `"Done (from cache)"`.
- `cache/sniff_cache.rs` — **versioned XDG cache** (решение: вариант C спеки): `$XDG_CACHE_HOME/raccpack/sniff/{hash}.json`, fallback `~/.cache/raccpack/sniff/`; никогда не пишем в scan_root. Ключ — FNV-1a 64 по `root\0max_depth\0policy_fp` (не `DefaultHasher` — он рандомизирован по процессу и сломал бы hit). Entry JSON: `cache_schema` (const 1) / `core_version` / `root` / `max_depth` / `policy_fingerprint` / `created_at` (ISO-8601 UTC, helper без chrono) / `report`. Инвалидация по любому несовпадению; любые ошибки чтения → `Ok(None)` (miss); ошибки записи → `Error::Io` (sniff глотает); нерезолвящийся XDG/HOME → cache недоступен без ошибки.
- `lib.rs`: `pub mod app;` + `pub mod cache;` + аддитивные re-exports (app: sniff/AppContext/WorkspacePaths/RunMode/SecretExitPolicy/OperationKind/ProgressEvent/ProgressSink/NullProgress/SniffOptions/SniffResult; cache: try_load_sniff_cache/store_sniff_cache) — не breaking. Существующие модули не тронуты.
- `Cargo.toml`: `serde_json` перенесён из dev-dependencies в dependencies (JSON cache).

#### Отклонение от спеки (зафиксировано в rustdoc `AppContext`)
- Поле `secret_groups_override: Option<EnabledGroups>` НЕ введено: тип `EnabledGroups` появится только с секретной фазой (M3.x). Добавится аддитивно без placeholder-типов.

#### Тесты (test-m2.3)
- `tests/sniff_cache.rs` (created, 12 тестов): все 9 обязательных кейсов §8 (empty root; 2-project fixture с размерами/языками; skip node_modules/target; cache hit; force_refresh; max_depth change → miss; progress события монотонны + последний phase_complete=100; bad root PathNotFound/NotADirectory; serde roundtrip ScanReport через cache) + 2 бонусных (miss без файла, miss при другом max_depth). Все env-зависимые `#[serial]` + `CacheEnvGuard` (capture/restore XDG_CACHE_HOME, изоляция от реального `~/.cache`), den — sibling scan_root.

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo build --workspace` → pass
- `cargo test -p raccpack-core` → pass (173: 35+19+22+31+22+4+27 unit + 12 sniff_cache + 1 doctest, 0 failed)
- `cargo test -p raccpack-core --test sniff_cache` → pass (12)
- `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass
- `cargo fmt --all -- --check` → pass
- `cargo doc -p raccpack-core --no-deps` → только pre-existing warning `markers/mod.rs:53` (M2.2-followup), новых нет
- grep `unwrap(/expect(/anyhow/Box<dyn/WalkDir::new` в `app/`+`cache/`+`lib.rs` → чисто (`unwrap_or` на Option — допустимо; DefaultHasher только в doc-комментарии почему не используется)

#### Критерий готовности (DoD из m2.3 §10)
- [x] `sniff` реализован и возвращает `SniffResult`
- [x] `ScanReport.schema_version == 1`
- [x] Cache versioned; hit/miss/force_refresh покрыты тестами
- [x] `ProgressSink` вызывается
- [x] `NullProgress` работает
- [x] Ошибка cache write не валит sniff
- [x] Тесты §8 зелёные
- [x] `cargo test -p raccpack-core` green
- [x] Краткий rustdoc на `sniff` со ссылкой на поведение cache

#### Риски / follow-up
- Cache не проверяет mtime дерева (спека §5.2): свежесть только по ключу/версиям; mtime-инвалидация — опционально позже.
- `secret_groups_override` — добавить аддитивно при M3.x вместе с `EnabledGroups` (помечено в rustdoc).
- `serde_json` стал prod-зависимостью core (оправдано cache JSON).

#### Follow-up review замечания (человек, 2026-08-06; PR #15) — НЕ блокеры, принято
- **A. `project_size_bytes(...).unwrap_or_default()`** — ошибка size → 0, sniff не падает. Для UX sniff ок; если позже нужен fail-fast — отдельная политика/опция (не менять сейчас).
- **B. `POLICY_FINGERPRINT = "default_scan_v1"`** — при смене `SkipPolicy::default_scan` обязателен bump строки; комментарий у const уже есть.
- **C. Root-сравнение без canonicalize** (как config): одинаковый путь через разные представления (`/a/b` vs `/a/../a/b`) → разные cache-ключи. Для v1 приемлемо; canonicalize — отдельное решение (не сейчас).
- **D. Progress — одна фаза `"scan"`** (phase_index 0, phase_count 1); для dig/pack мультифазность расширится позже. Оставлено как есть.

### M2.4 — CLI sniff (racc sniff --root, text + --json) (CLOSED)

- **Дата:** 2026-08-08
- **Ветка:** `m2.4-cli-sniff`
- **Статус:** done
- **Dev:** dev-m2.4 · **Test:** test-m2.4 (параллельно, без rework)

#### Сделано
- **core (additive, non-breaking):** `#[derive(Serialize, Deserialize)]` на `SniffResult` (`crates/raccpack-core/src/app/sniff.rs`) — JSON выводит весь `SniffResult` (`report` + `from_cache` + `duration_ms`), решение по спеке §5 зафиксировано. Поля/API не менялись.
- **CLI** (структура по спеке §3, сразу `Commands` enum для будущего M3.4 dig):
  - `cli.rs` — `Cli` (clap, `name="racc"`, version, about) + `GlobalOpts` (`-c/--config`, `--root`, `--den`, `--json` — все `global=true`, работают до и после подкоманды) + `Commands::Sniff` с `--force-refresh` и `--max-depth`; unit-тесты clap parse (5).
  - `commands/sniff.rs` — `run_sniff`: config (`--config`→`load_from_path`, иначе `RaccConfig::load`) → overrides `--root`/`--den` → `AppContext::from_config(…, DryRun)` → `SniffOptions` → `sniff` с `NullProgress` → печать.
  - `output.rs` — human: `Scan root`, сводка `Projects / Total size / ms / cache: hit|miss`, таблица `NAME STACK SIZE GIT PATH` со стабильным выравниванием; `human_size` (KiB/MiB/GiB/TiB, 1 знак); без ANSI; 0 проектов → заголовок+сводка, exit 0; unit-тесты (4). JSON: `serde_json::to_string_pretty(&SniffResult)`.
  - `error.rs` — `CliError` (Config/Core/Json) с `From`-конверсиями, `report()` печатает `error: …` + `hint: suggestion()` в stderr, `exit_code()` = 1. Кода 2 в sniff нет.
  - `main.rs` — тонкий: `Cli::parse()` → dispatch → `ExitCode`.
- `Cargo.toml` CLI: deps `clap`(derive) + `serde_json`; dev-deps `assert_cmd`, `predicates`, `tempfile`, `serde_json`.
- CLI **не** дублирует domain-логику — только facade `sniff`; без dig/pack subcommands; без интерактива; `--den` не требуется.

#### Файлы
- `crates/raccpack-core/src/app/sniff.rs` (changed — additive serde on SniffResult)
- `crates/raccpack-cli/Cargo.toml` (changed)
- `crates/raccpack-cli/src/main.rs` (changed), `src/cli.rs` (created), `src/error.rs` (created), `src/output.rs` (created), `src/commands/{mod,sniff}.rs` (created)
- `crates/raccpack-cli/tests/cli_sniff.rs` (created, Test-субагент)
- `Cargo.lock` (changed)

#### Тесты
- `cargo build --workspace` → pass
- `cargo test --workspace` → pass (192, 0 failed: 173 core + 19 cli [9 unit + 10 integration])
- `cargo test -p raccpack-cli` → pass (9 unit + 10 integration)
- `cargo clippy --workspace --all-targets -- -D warnings` → pass
- `cargo fmt --all -- --check` → pass
- grep unwrap/expect/anyhow/Box<dyn в CLI production `src/` → чисто (только `#[cfg(test)]`)
- Manual E2E: text-вывод, `--json` (валидный SniffResult, cache hit на 2-м прогоне `from_cache:true`), `--root /no/such` → exit 1 + `hint:` в stderr

#### Критерий готовности (DoD из m2.4 §10)
- [x] `racc sniff --root PATH` работает (text)
- [x] `racc sniff --root PATH --json` печатает валидный JSON `SniffResult`
- [x] `--force-refresh`, `--max-depth` пробрасываются в `SniffOptions` (тест max-depth исключает глубокий проект)
- [x] `--config` / `--root` override работают (тест: root wins над config scan_root)
- [x] Exit 0/1 по правилам §6 (2 — только для secrets policy, в sniff не используется)
- [x] Ошибки показывают `suggestion()` (печать `hint:`)
- [x] Тесты §8 зелёные (parse unit + 10 integration через assert_cmd/tempfile)
- [x] `cargo build -p raccpack-cli` green
- [x] `racc sniff --help` читаемый (тест)

#### Риски / follow-up
- clap-ошибки парсинга аргументов обрабатывает сам `Cli::parse()` (exit 2 — стандарт clap, к §6 не относится).
- `--max-depth 0` не валидируется на CLI — пробрасывается как есть; валидация `>= 1` остаётся за core (config `InvalidMaxDepth`), при необходимости добавить на CLI позже.
- `--den` в sniff пробрасывается в config, но не используется (sniff read-only) — это по спеке.
- М3.4 `racc dig` добавится в `Commands::Dig` рядом (структура уже готова).

#### Follow-up review замечания (человек, 2026-08-08; PR #17) — НЕ блокеры, принято
- **A. `--root` только global** — в clap нет per-command root; doc-комментарий «also per-command» в `cli.rs` не точен, по факту флаг global. Для sniff достаточно; при добавлении `dig` (M3.4) уточнить формулировку/поведение.
- **B. `Progress = NullProgress`** — CLI пока без прогресс-бара; для M2.4 ок, progress-бар для CLI/TUI — отдельный этап позже (facade уже эмитит `ProgressEvent`).
- **C. hint на missing root** — integration-тест ассертит непустой stderr и подстроки `scan_root`/`--root` (не хрупко цепляется за wording); формат `hint: <suggestion()>` покрыт через `CliError::report`/`suggestion()`.

### M3.1 — Filename patterns + risk model (severity API) (CLOSED)

- **Дата:** 2026-08-08
- **Ветка:** `m3.1-filename-patterns`
- **Статус:** done
- **Dev:** dev-m3.1 · **Test:** test-m3.1 (параллельно, без rework)

#### Сделано
- `secrets/` — новый модуль: `mod.rs` (модульный док со списком pattern categories), `filename.rs` (таблица + matching + scan), `finding.rs` (`SensitiveFinding`/`FindingSource`), `risk.rs` (severity helpers).
- `DEFAULT_FILENAME_PATTERNS` — data-driven `pub static &[FilenamePattern]`, **28 строк в точном порядке спеки §4.2** (env, keys/SSH, keystores/certs, registry/config, cloud/service-account, wallets). `aws_credentials` + `aws_credentials_path` (обе High `credentials`) сохранены как две строки с разными id — по спеке. Единственная точка агрегации: новый паттерн = одна строка.
- `NameMatchKind` (Exact/Suffix/Prefix/Contains) — plain substring-сравнение по lossy `file_name()`, case-sensitive (Linux v1), без regex/glob; `Contains` осознанно редко.
- `match_filename` (max risk; при равенстве — первая строка таблицы) / `match_filename_all` (все совпадения в порядке таблицы).
- `scan_filenames` — `ensure_scan_root` → `walk_tree` (`follow_links(false)`, max_depth, policy из opts) → только **files**, `min_risk` через `SensitiveRisk::at_least`, walk-ошибки не глотаются (`io_error()` → `Error::Io`, иначе `Error::Other`), сортировка path asc → risk desc (детерминизм). Содержимое не читается.
- `FilenameScanOptions` с `Default` (max_depth = `config::default_max_depth()`, policy = `default_scan()`, min_risk = Low).
- Severity API: `SensitiveRisk::at_least(min)` (inherent impl в `secrets/risk.rs`, domain не тронут) + `upgrade_risk(a, b) = max` — единственное санкционированное место upgrade (FINAL-правило «upgrade только через severity API»).
- `lib.rs`: `pub mod secrets;` + аддитивные re-exports (`match_filename`, `match_filename_all`, `scan_filenames`, `upgrade_risk`, `FilenamePattern`, `NameMatchKind`, `FilenameMatch`, `FilenameScanOptions`, `SensitiveFinding`, `FindingSource`, `DEFAULT_FILENAME_PATTERNS`) — additive, не breaking. Другие модули не тронуты; `Cargo.toml` не менялся (tempfile уже dev-dep).

#### Тесты
- `tests/filename_secrets.rs` (created, Test-субагент) — 23 теста: все 10 обязательных кейсов §7 (`.env`→High `env_file`, `id_rsa`→Critical, `notes.txt`→None, `foo.pem`→High suffix, dual-pattern max risk + порядок `match_filename_all`, scan не заходит в `node_modules`, `min_risk: Critical` фильтрует High, ordering Low<Medium<High<Critical, `upgrade_risk` never downgrade, детерминизм) + extra: tie-break первой строки, `credentials` basename (обе строки), `wallet.dat`/containing → Critical, директория `.env` не даёт finding, `max_depth`, PathNotFound/NotADirectory, case-sensitivity (`.ENV` не матчится), `at_least`, целостность таблицы (28 строк/уникальные id), symlink-dir не обходится (cfg unix).
- `secrets/filename.rs` unit (`#[cfg(test)]`): 28 строк, уникальные id, поведение каждого kind; `secrets/risk.rs` unit: `upgrade_risk` max, `at_least` inclusive.

#### Проверки (выполнены Orchestrator самостоятельно)
- `cargo build --workspace` → pass
- `cargo test --workspace` → pass (201 core: 40 lib + 19 candidates + 22 config + 22 domain_dto + 31 detect_stack + 23 filename_secrets + 4 markers_registry + 27 skip_walk + 12 sniff_cache + 1 doctest; 0 failed; +19 cli)
- `cargo test -p raccpack-core -- filename risk secrets` → pass (9 lib + 21 integration; 2 не попали в фильтр — имена без filename/risk/secrets, зелёные в полном прогоне)
- `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass
- `cargo fmt --all -- --check` → pass
- `cargo doc -p raccpack-core --no-deps` → только pre-existing warning `markers/mod.rs:53` (M2.2-followup), новых нет
- grep unwrap/expect/anyhow/Box<dyn в `src/secrets/` → чисто (кроме `unwrap_or`/`unwrap_or_else` на Option — допустимо; `#[cfg(test)]`)

#### Критерий готовности (DoD из m3.1 §9)
- [x] Data-driven `DEFAULT_FILENAME_PATTERNS` с минимум env/keys/certs
- [x] `match_filename` / `scan_filenames` работают
- [x] Risk upgrade через `max` / `upgrade_risk`
- [x] SkipPolicy + no symlink follow
- [x] Тесты §7 зелёные
- [x] `cargo test -p raccpack-core` green
- [x] rustdoc: список pattern categories (модульный док `secrets/mod.rs`)

#### Риски / follow-up
- `.key` suffix — осознанные false positives на High (по спеке).
- `match_filename` tie-break и порядок `match_filename_all` завязаны на порядок строк таблицы (фиксирован спекой §4.2); тесты `env_local_tie_breaks_to_first_in_table` / `env_production_dual_pattern_max_risk` и integrity-тест (28 строк) придётся обновлять при осознанной перестановке/добавлении паттернов.
- `filename.rs` ~450 строк (453) — выше мягкого предела ~400, но ~200 из них — чистая data-таблица (carve-out «pure data tables»); logic ~250 строк. При росте таблицы — вынести в `secrets/patterns.rs` (registry-паттерн сохраняется).
- `SensitiveFinding`/`FindingSource`/`FilenameMatch` пока без serde: M3.3 facade dig потребует serde-вывода — добавить аддитивно на M3.3 (сейчас по спеке не требуется).
- Замечание из спеки (аналогично M1.4/M2.1): `cargo test -p raccpack-core filename risk secrets` (несколько фильтров до `--`) — невалидный синтаксис; корректно `-- filename risk secrets`.
- M3.2 (content markers + file size limits) — следующий этап, входы: `SensitiveFinding`/`FindingSource` и `upgrade_risk`.

#### Follow-up review замечания (человек, 2026-08-08; PR #18) — НЕ блокеры, принято
- **A. Дубль `credentials` (aws_credentials / aws_credentials_path) — принято как есть.** Два Exact-ряда с одним `pattern` матчят одно и то же `file_name()`; второй ряд избыточен, пока нет path-segment matching. Работает корректно (обе строки возвращаются, risk одинаковый). Решение: оставить на M3.1 (по спеке), при введении path-context (например, `~/.aws/` для дига) — пересмотреть/схлопнуть в одну строку.
- **B. `config.json` → Medium — принято.** Много легитимных Docker/прочих config-файлов; false positives ожидаемы на Medium. Зафиксировано в PR body.
- **C. `filename.rs` ~450 строк — принято.** ~200 строк — чистая data-таблица (carve-out «pure data tables»); при росте — split в `secrets/patterns.rs` отдельным follow-up PR.
- **D. Модульность «один секрет = один файл» — подтверждено.** Для content matchers (M3.2) уместна; для статической name-таблицы один registry (`DEFAULT_FILENAME_PATTERNS`) правильнее. Текущая реализация согласована с data-driven подходом.

### M3.2 — Content markers (regex/prefix/contains) + size limits + mask/fingerprint (CLOSED)

- **Дата:** 2026-08-08
- **Ветка:** `m3.2-content-markers`
- **Статус:** done
- **Dev:** dev-m3.2 · **Test:** test-m3.2 (параллельно, без rework)

#### Сделано
- `secrets/mask.rs` (created): `MaskedValue { masked, value_hash, original_len }` (serde, единственный value-carrying DTO), `mask_secret` (≤8 байт → `"****"`; >8 → first 4 chars + `…` + last 2 chars, char-based без паники, `original_len` в байтах), `fingerprint_secret` = blake3 hex.
- `secrets/content.rs` (created): `ContentMatchKind` (Prefix/Contains/Regex), `ContentMarker`, `DEFAULT_CONTENT_MARKERS` — 12 строк в точном порядке спеки §4.2 (`telegram_bot` осознанно отложен), `ContentScanLimits` (default 1 MiB / 1 MiB / skip_binary), `ContentHit`, `scan_file_content` — best-effort: skip empty / oversize (`max_file_bytes`) / binary (null в первых 8 KiB), line-oriented lossy read до `max_read_bytes`, 1-based line numbers, по-hit на каждое вхождение, Prefix-token = alnum/`-`/`_` от вхождения, read-ошибки → `Error::Io`. Regex компилируются один раз в `OnceLock`; единственный санкционированный `.expect` в production (static-таблица, fail-at-startup по спеке §8 тест 9, задокументирован в `content.rs`).
- `secrets/finding.rs` (changed, additive): `FindingSource::Content { marker_id, masked, line }`; `SensitiveFinding` + `sources`/`labels`/`content_match`; инвариант `source == sources[0]`, `label == labels[0]` в rustdoc.
- `secrets/filename.rs` (changed): `scan_filenames` заполняет новые поля (одно место конструктора).
- `secrets/scan.rs` (created): `SecretScanOptions` (max_depth / policy / min_risk / scan_content (default true) / limits / find_repeated — placeholder для M3.3, не агрегируется), `scan_secrets` — один walk, per-path merge filename+content, risk = max через `upgrade_risk`, per-source `min_risk`-фильтр, content read-error → best-effort skip (не роняет), walk-ошибки не глотаются, сортировка path asc → risk desc.
- `mod.rs` / `lib.rs`: re-exports (additive); `Cargo.toml`: `regex = "1"`, `blake3 = "1"` (+ `regex` в dev-dependencies для integration-теста).

#### Файлы
- created: `crates/raccpack-core/src/secrets/mask.rs`, `content.rs`, `scan.rs`, `crates/raccpack-core/tests/content_secrets.rs`
- changed: `Cargo.toml`, `Cargo.lock` (regex 1.13.1, blake3 1.8.6), `src/lib.rs`, `src/secrets/{mod,finding,filename}.rs`

#### Тесты
- unit: mask.rs (7) + content.rs (9) + scan.rs (6) = 22 новых; lib unit всего 63.
- integration `tests/content_secrets.rs` — 33: все 10 обязательных кейсов §8 (AKIA Critical+masked+raw не в Debug, PEM Critical, oversize→skip content но `.env` filename жив, binary skip, `.env`+password → risk max + 2 sources, masked не содержит raw, одинаковый raw в 2 файлах → одинаковый `value_hash`, node_modules не сканируется, таблица 12 строк/порядок/уникальность/regex-компиляция, empty file без паники) + экстры: content-only finding, line numbers, per-line/per-occurrence hits, `scan_content: false`, `min_risk: Critical` фильтрует High, mask-таблица (0/4/8/9/15, char-based/byte-len), fingerprint детерминизм, сортировка path asc, unreadable skip (cfg unix, root-aware), `max_read_bytes` truncation, upgrade High→Critical, root validation PathNotFound.
- Команды: `cargo test -p raccpack-core -- content mask secret scan` → pass; `cargo test -p raccpack-core` → 257 (0 failed); `cargo test --workspace` → pass; `cargo clippy -p raccpack-core --all-targets -- -D warnings` → pass; `cargo fmt --all -- --check` → pass; `cargo doc -p raccpack-core --no-deps` → только pre-existing warning `markers/mod.rs:53`.

#### Критерий готовности (DoD из m3.2 §10)
- [x] Content markers table + scan with size/binary limits
- [x] `mask_secret` + `fingerprint_secret` стабильны и протестированы
- [x] Merge filename + content через risk upgrade
- [x] Нет raw в public serializable types (только `MaskedValue`; Debug-тесты на finding/MaskedValue)
- [x] Тесты §8 зелёные
- [x] `cargo test -p raccpack-core` green

#### Риски / follow-up
- `generic_secret_assign` / `generic_api_key_assign` шумные (`\S{8,}` / `{16,}`) — тюнинг при табличных тестах позже.
- `telegram_bot` отложен: нужен length-bound на `bot`+digits, иначе ложные срабатывания.
- `private_key_header` — Regex вместо `Contains` из иллюстрации спеки (одна `Contains`-needle не выражает AND «-----BEGIN»+«PRIVATE KEY»); поведение строже, `-----BEGIN RSA PRIVATE KEY-----` матчится.
- Prefix-семантика: матч по вхождению префикса в любой позиции строки (не только старт) — зафиксировано в rustdoc; при необходимости канонизировать отдельным решением.
- `MaskedValue` уже serde (нужно M3.3); `SensitiveFinding`/`FindingSource` serde — аддитивно на M3.3 (как замечал M3.1).
- `SensitiveFinding` получил новые поля (additive; конструктор литералом в `filename.rs` обновлён). Breaking: нет (внешних callers нет).
- Спека-команда `cargo test -p raccpack-core content mask secrets` — невалидный синтаксис (несколько фильтров до `--`); корректно `-- content mask secret scan` (аналогично M1.4/M2.1/M3.1).
- M3.3 (facade `dig`) — следующий этап, входы: `scan_secrets`/`SecretScanOptions`, `MaskedValue`, `upgrade_risk`.

#### Follow-up review замечания (человек, 2026-08-08; PR #19) — НЕ блокеры, принято
- **A. Шумные маркеры `generic_secret_assign` / `generic_api_key_assign`** — дают FP; тюнинг позже (min/max длина value, denylist). В PR отмечено; оставлено как есть для MVP.
- **B. Prefix без length bound** — `AKIA`/`ghp_` матчат любой token после префикса; для MVP ок. При желании — min/max length на `ContentMarker` (аддитивное поле позже, затронет таблицу+тесты).
- **C. `telegram_bot` отложен** — осознанно; вернуть с length constraint (не раньше введения length-поля из B).
- **D. Serde на findings** — на M3.3 (facade dig), как планировали.
- **E. Модульность** — одна data-table на content markers правильна (registry-паттерн); дробить на файлы имеет смысл только при росте правил / конфигурируемых группах (по аналогии с markers/detect по экосистемам).

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
| 2026-08-06 | M2.1-followup (`raccpack-markers-detect-modularity.md`): `markers.rs` → `markers/` по экосистемам, registry `default_markers()`; порядок групп = эффективный порядок M2.1 (behavior-preserving). `detect/` по файлам экосистем (trait + registry) — с M2.2. |
| 2026-08-06 | M2.2: `detect/` — top-level модуль (а не `scan/detect/`): спека §3 рекомендует отдельный модуль detect, architecture-vision — отдельная подсистема. Принята политика merge: language по приоритету §4.1 (tie → первый hit; fallback на первый hit с hint), frameworks union по registry-порядку с dedup, markers sorted+dedup. `StackDetector::detect -> Result<Stack, Error>` (deviation от иллюстрации modularity-документа, чтобы выразить `Error::Io`, спека §5). Парсинг manifest-deps — отложен на Alpha. |
| 2026-08-06 | M2.2 follow-up (PR #13): **пустые markers → probe all detectors — принято** (осознанно path-only, чуть шире «matched ecosystem»; sniff-кейсы обычно с непустыми hits). `detect/mod.rs` ~400+ строк — ок до M2.3 (при росте вынести тесты в `detect/tests_unit.rs`). Symlink-тест `size.rs` — по сути `#[cfg(unix)]`, primary Linux ок. |
| 2026-08-06 | M2.2 follow-up (PR #13), **идея «на вырост» (обязательно к реализации)**: фреймворки — вложенность внутри экосистемы (`detect/node/next.rs`, `detect/python/django.rs`, `detect/ruby/rails.rs` …), API снаружи без изменений (`StackDetector` по экосистемам). Сплит внутри экосистемы только при 4–5+ правил в одном файле или конфигурируемом «только Next». Плоский `detect/frameworks/` НЕ делать (Next без Node-контекста бессмысленен). |
| 2026-08-06 | M2.3: cache-локация = **XDG** (`$XDG_CACHE_HOME/raccpack/sniff/{hash}.json`, fallback `~/.cache/raccpack/sniff/`) — вариант C спеки (не писать в scan_root). Ключ = FNV-1a 64 по root+max_depth+policy_fp (НЕ DefaultHasher). `AppContext.secret_groups_override` отложен до M3.x (нет типа EnabledGroups). |
| 2026-08-08 | M2.4: JSON sniff-вывод = **весь `SniffResult`** (report + from_cache + duration_ms) — решение §5 спеки зафиксировано; для этого `SniffResult` получил `Serialize/Deserialize` (additive, non-breaking). CLI human-вывод — plain table (без ANSI), размеры binary-единицы. |
| 2026-08-08 | M3.1: severity helpers живут в `secrets/risk.rs` (inherent `SensitiveRisk::at_least` + `upgrade_risk`) — domain/risk.rs не тронут (узкий diff). `SensitiveFinding`/`FindingSource`/`FilenameMatch` без serde (по спеке M3.1); serde — аддитивно на M3.3. `filename.rs` содержит data-таблицу (~200 строк из ~450) — приемлемо по carve-out «pure data tables»; при росте — `secrets/patterns.rs`. Обе строки `aws_credentials`/`aws_credentials_path` сохранены по спеке (разные id, одинаковый risk). |
| 2026-08-08 | M3.2: content-скан line-oriented с prefix-token extraction (token = alnum/`-`/`_` от вхождения префикса в любой позиции строки, не только старт) — зафиксировано в rustdoc `content.rs`. `private_key_header` — Regex (а не `Contains` из иллюстрации спеки): одна needle не выражает AND двух подстрок; поведение строже и покрыто тестом. `telegram_bot` отложен (шум, нужен length-bound). Единственный `.expect` в production — компиляция static regex-таблицы в `OnceLock` (fail-at-startup, спека §8 тест 9). `MaskedValue` сериализуем уже сейчас (нужен M3.3); `SensitiveFinding`/`FindingSource` serde — на M3.3. |
| 2026-08-08 | M3.2 follow-up (PR #19), принято: шумные `generic_*` маркеры остаются как есть (тюнинг длины/denylist позже); prefix без length bound — ок для MVP (min/max length на `ContentMarker` — аддитивная эволюция позже, вернёт и `telegram_bot`); serde на findings — на M3.3; модульность content-markers — одна data-table + registry (не дробить до роста правил/конфигурируемых групп). |
