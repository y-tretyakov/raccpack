# WORKLOG — raccpack

Журнал статусов этапов. Orchestrator: y-tretyakov.

**Alpha 0.3.0 закрыт.** Полный журнал A1–A4:
[`docs/archive/WORKLOG_ALPHA.md`](docs/archive/WORKLOG_ALPHA.md)  
(при переносе: полный dump из git → `docs/archive/WORKLOG_ALPHA_FULL.md` по желанию).

Спеки Alpha: [`docs/archive/alpha/`](docs/archive/alpha/).  
Спеки Detect v2: [`docs/detect/`](docs/detect/).  
Версии: [`docs/VERSION_ROADMAP.md`](docs/VERSION_ROADMAP.md).

---

## Текущий статус

| | |
|--|--|
| **Версия** | **`0.3.5`** (Detect v2, D2.2 закрыт) |
| **Веха** | **Detect v2 → 0.4.0**, идёт фаза D2 |
| **Следующий этап** | **D2.3** — flat stack + stack_tree compat → bump **`0.3.6`** |
| **Предыдущий** | D2.2 conflict merge (PR #96) |

```text
MVP 0.1.0 ✅ → Alpha 0.3.0 ✅ → Detect v2 0.4.0 ⬜ → Beta 0.5.0 → RC 0.9.0 → 1.0.0
```

---

## Backlog (Detect v2 → 0.4.0)

```
[x] D1.1 StackDetector trait + registry          → 0.3.1
[x] D1.2 Detection / StackNode DTO                 → 0.3.2
[x] D1.3 detect.mode config + CLI                  → 0.3.3
[x] D2.1 WorkspaceDetector → DAG                   → 0.3.4
[x] D2.2 conflict merge                            → 0.3.5
[ ] D2.3 flat stack + stack_tree compat            → 0.3.6
[ ] D3.1 rinse по DAG scopes                       → 0.3.7
[ ] D3.2 sniff tree output                         → 0.3.8
[ ] D3.3 fixtures + монорепо-тесты                 → 0.3.9
[ ] D4.1 batch raid design (--root vs --project)   → без bump
[ ] D4.2 facade raid_batch                         → 0.3.10
[ ] D4.3 CLI racc raid --root                      → 0.3.11
[ ] D4.4 wiki + E2E = Detect v2 EXIT               → 0.4.0
```

Спеки: `docs/detect/detect-v2-index.md` и `d1.*` / `d2.*` / `d3.*` / `d4.*`.

### Follow-ups (открытые)

- [x] hygiene: `detect/mod.rs` 435 строк — инлайн unit-тесты вынесены в `detect/tests.rs` (mod.rs 442→191; закрыто после D1.3)
- [ ] perf (deferred): detect merge после D2.2 — `merge_same_scope` корректен и достаточно быстр (не hot path); при реальном профиле оптимизировать сначала `extend_frameworks_union` (линейный contains, вызывается на каждый detector) и повторный `normalization_key` в `attach_draft` (на каждый child/уровень), НЕ merge. Анализ 2026-08-25: O(F²+M log M) на вызов при F≤20/M≤50 — микросекунды vs walk/IO

### Exit criteria Detect v2

- [ ] Monorepo: `sniff` показывает корректное `stack_tree` (composite_dag)
- [ ] `rinse` удаляет мусор только в релевантных scope
- [ ] `detect.mode = priority_table` — без регрессий (default)
- [ ] CLI `--detect-mode` / config `[detect] mode`
- [ ] Плоский `stack` всегда в JSON
- [ ] Multi-project: `racc raid --root` — батч по scan root (D4.3; planned, в бинарнике пока нет)

---

## Последние действия (перед архивом Alpha)

| Дата | Что |
|------|-----|
| 2026-08-20 | A3.3–A3.5: Atomic WAL/rollback, manifest, full raid CLI, wiki |
| 2026-08-21 | A4.1 GitClient + dig `git_status` → **0.2.12** |
| 2026-08-21 | A4.2 init + config migrate → **0.2.13**; F-ERR-1 closed |
| 2026-08-21 | A4.3 tracing + `-v` → **0.2.14**; no secret in logs |
| 2026-08-21 | A4.4 CI + MSRV 1.85 → **0.3.0 ALPHA EXIT** |
| 2026-08-22 | docs: Alpha specs → `docs/archive/alpha/`; scaffolds `docs/detect/`, `docs/beta/` (PR #91) |
| 2026-08-22 | **WORKLOG.md → archive WORKLOG_ALPHA**; этот файл — журнал Detect+ |
| 2026-08-22 | **D1.1** StackDetector trait → `traits.rs`, `all_detectors()` → `detector_registry()`, integration-тесты реестра → **0.3.1** (PR #92); behavior-preserving, wiki не трогали (внутренний рефакторинг без изменений CLI) |
| 2026-08-22 | **D1.2** Detection/StackNode DTO + `Project.stack_tree` (additive, serde back-compat) → **0.3.2** (PR #93); Eq снят каскадно; wiki — только версии |
| 2026-08-22 | docs: фаза **D4 batch raid** встроена в конец Detect v2 (roadmap/versions/index/wiki); exit вехи перенесён D3.3 → **D4.4 = 0.4.0**; без bump |
| 2026-08-22 | docs: спеки d4.* залиты (4be1f58); полировка формулировок (48cc26d) |
| 2026-08-22 | **D1.3** `detect.mode` config + `racc sniff --detect-mode` → **0.3.3** (PR #94); composite_dag до D2.x = явная ошибка; wiki обновлён (UX-этап) |
| 2026-08-22 | **D2.1** WorkspaceDetector → tree, composite_dag исполняется → **0.3.4** (PR #95); breaking: убран `Error::DetectPipelineUnavailable`; wiki обновлён (UX-этап) |
| 2026-08-25 | **D2.2** conflict merge policy → `detect/merge.rs` (правила 1–5 в rustdoc; клон union+dedup устранён: `extend_frameworks_union`; дубликаты scope теперь merge хитов вместо keep-first) → **0.3.5** (PR #96); 10 новых тестов, `detect::` 48 green, workspace 860 green; wiki не трогали (без изменений CLI) |

---

## Принятые решения (актуальные)

| Дата | Решение |
|------|---------|
| 2026-08-14 | Orchestrator сам squash-merge в `dev` после закрытого этапа |
| 2026-08-19/20 | Raid default **Atomic**; FailFast только debug; destructive ops в commit |
| 2026-08-20 | Manifest только на successful Atomic commit |
| 2026-08-20 | CLI raid: exit **1** при `!success` |
| 2026-08-21 | MSRV **1.85** (не даунгрейдить age/blake3) |
| 2026-08-21 | Логи → stderr; `RUST_LOG` > `-v`; never log passphrase/raw |
| 2026-08-22 | После Alpha exit журнал ведётся здесь; Alpha — только archive |
| 2026-08-22 | Detect v2 = отдельная веха **0.4.0** (не часть Beta) |

---

## Follow-up, перенесённые с Alpha (не блокеры D1)

| ID | Суть | Горизонт |
|----|------|----------|
| **F-SKIP-1** | Единый источник имён skip ↔ cleanup; `SkipPolicy::default_pack()` | B3.1 |
| **F-PACK-SIZE** | `archive/pack.rs` ≳400 строк | next pack |
| **F-ATOMIC-SIZE** | `app/raid/atomic.rs` ≳400 | next atomic |
| **F-TEST-SIZE** | `tests/raid_atomic.rs`, `cli_raid.rs` огромные | next test touch |
| **F-CLI-SIZE** | `cli.rs` ~941 | hygiene |
| **F-TRACE-RAID** | info tracing для raid/rinse/pack | optional |
| **F-DOC-LINKS** | ссылки `docs/alpha/…` в тестах → `docs/archive/alpha/…` | next touch |
| **P2-5** | `zstd_level` в `[advanced]` | later |
| **P2-6** | content-deny cost (size-cap) | later |
| **P2-7** | сужение `lib.rs` public API | R1 |
| **P2-8** | типизация `Error::Other` | R3 / UX |
| **OS-WIN** | Windows HOME/XDG best-effort | R2.2 |

---

## Инварианты, которые Detect v2 не должен ломать

1. Default detect behavior ≡ **priority_table** (flat `stack`), пока mode не сменён.
2. `follow_links(false)` на walk.
3. Raid Atomic / FailFast семантика и ORPHAN-тесты зелёные.
4. No raw secrets in JSON/logs.
5. Public API — additive preferred; breaking только с записью в CHANGELOG.

---

## Этапы

### 2026-08-22 — подготовка Detect v2: архив WORKLOG Alpha

**Задача:** закрыть журналирование Alpha; завести WORKLOG под Detect v2.

**Сделано:**
- `WORKLOG.md` (Alpha) → архив как `WORKLOG_ALPHA` (выжимка + указание на full git dump).
- Новый `WORKLOG.md`: статус 0.3.0, backlog D1–D3, follow-up, решения.
- Аудит: блокеров Detect нет; техдолг выписан.

**Версия:** без bump (docs/process only).  
**Следующее:** D1.1 по спеке `docs/detect/d1/d1.1-stack-detector-trait.md` → version **0.3.1**.

---

## Этапы (Detect+)

### 2026-08-22 — D1.1 — StackDetector trait + registry

- **Дата:** 2026-08-22
- **Ветка / PR:** `d1-stack-detector-trait` / **#92** (squash в `dev`)
- **Статус:** CLOSED
- **Версия:** 0.3.1

#### Сделано
- Trait `StackDetector` перенесён `types.rs` → `detect/traits.rs` (layout §2 спеки); в `types.rs` остались таблица приоритетов + pure-хелперы.
- `all_detectors()` → `pub fn detector_registry()` — стабильный порядок rust → node → go → python → jvm → ruby → php → cpp → make → git без изменений; re-export добавлен в `lib.rs`.
- Integration-тесты `tests/detector_registry.rs` (13 кейсов): форма реестра/stable ids, каждый экосистемный детектор стреляет на fixture, probe-all smoke.
- Behavior-preserving: merge policy, `resolve_language`, probe-all при пустых markers не тронуты.
- **Deviation (осознанный):** скетч спеки `detect(...) -> Vec<Detection>` требует DTO `Detection` из D1.2. В D1.1 сохранены `matches()` и `detect(hits, project_dir) -> Result<Stack, Error>` как bridge; миграцию сигнатуры целиком выполняет D1.2. Зафиксировано в спеке d1.1 (§6).

#### Тесты
- `cargo test -p raccpack-core --test detector_registry` — 13 pass; `--test detect_stack` — 31; `--test candidates` — 19; полный `cargo test -p raccpack-core` — green.
- `cargo fmt --check`; clippy `-D warnings` (core + cli, all-targets) — clean.

#### DoD
- [x] Trait + registry
- [x] Existing detectors implement trait
- [x] PriorityTable path behavior unchanged
- [x] Tests green; no unwrap in prod

#### Follow-up
- hygiene: `detect/mod.rs` — **закрыт** (инлайн тесты → `detect/tests.rs`, см. запись D1.3).

### 2026-08-22 — D1.2 — Detection / StackNode DTO

- **Дата:** 2026-08-22
- **Ветка / PR:** `d1-detection-dto` / **#93** (squash в `dev`)
- **Статус:** CLOSED
- **Версия:** 0.3.2

#### Сделано
- DTO `Detection` + `StackNode` (рекурсивный) в `detect/types.rs` (спека §2: зафиксировано detect/).
- `clamp_confidence(f32) -> f32`: clamp [0,1]; NaN/±inf → **0.0** (JSON-детерминизм) — задокументировано + unit-тесты.
- `Project.stack_tree: Option<StackNode>`, `#[serde(default)]` (старый JSON ⇒ None); flat `stack` всегда заполнен; 8 мест конструирования обновлены (`None` до composite_dag D2.x).
- `schema_version = 1` сохранён, решение задокументировано в `report.rs`.
- Re-exports: `raccpack_core::detect::{Detection, StackNode, clamp_confidence}` (аддитивные).

#### Semver-заметка
`Eq` снят с `Project`, `ScanReport`, `SniffResult` (каскад от f32 в `StackNode`; HashMap/BTreeSet-потребителей нет — прослежено grep'ом, компиляция доказывает).

#### Тесты
- Новый `tests/detection_dto.rs`: 7 кейсов (serde roundtrip, вложенное дерево ≥3 уровней, back-compat без поля, `"stack_tree":null` + flat stack, Some-roundtrip, clamp-края, f32-in-JSON).
- `cargo test --workspace` — green (38 suites); fmt clean; clippy core+cli all-targets `-D warnings` clean.

#### DoD
- [x] DTO public + Serialize
- [x] Project.stack_tree additive
- [x] Flat stack always present
- [x] Tests green

#### Follow-up
- Продюсеры `Some(stack_tree)` — D2.1 WorkspaceDetector (по плану).

### 2026-08-22 — docs: фаза D4 (batch raid) в roadmap/versions

- **Дата:** 2026-08-22
- **Статус:** CLOSED (docs-only, **без bump**)
- **Версия:** 0.3.2 (не менялась)

#### Сделано
- VERSION_ROADMAP: D4.1–D4.4 добавлены; D3.3 перенумерован 0.4.0 → 0.3.9 (фикстуры); exit вехи = **D4.4 → 0.4.0**; якорь/≥-таблица упомянуты batch raid.
- roadmap-v1: секция «Фаза D4», ASCII-карта, exit criteria + `racc raid --root`.
- `docs/detect/detect-v2-index.md`: строка фазы D4 (+ ссылка `d4-index.md`), линия пайплайна до D4.4, exit-criteria пункт. Спеки d4.* залиты следом (4be1f58).
- README + wiki roadmap (ru/en): «+ batch raid» как planned, без заявлений о shipped.
- WORKLOG: backlog `[ ]` D4.1–D4.4, exit criteria.

#### Решение по нумерации
Уже проставленные версии сохранены; D4.1 design — без bump; D4.2 → 0.3.10, D4.3 → 0.3.11, D4.4 → 0.4.0.

### 2026-08-22 — D1.3 — `detect.mode` config + CLI

- **Дата:** 2026-08-22
- **Ветка / PR:** `d1-detect-mode-config` / **#94** (squash в `dev`)
- **Статус:** CLOSED — **фаза D1 полностью закрыта**
- **Версия:** 0.3.3

#### Сделано
- `DetectMode` (PriorityTable default / CompositeDag) в новом `detect/mode.rs`; serde-строки строгие, алиас `dag` только на CLI.
- Секция `[detect]` в конфиге (`DetectConfig.mode`, serde default); старые TOML без секции валидны; `config_version = 1` не тронут, миграция не нужна.
- Unknown TOML mode → `ConfigError::UnknownDetectMode` + suggestion (проверка по raw-TOML до typed parse).
- CLI `racc sniff --detect-mode priority_table|composite_dag|dag`; unknown → clap possible values (exit 2).
- **composite_dag до D2.x → явная `Error::DetectPipelineUnavailable`** с hint «lands in Detect v2 (0.4.x)», fail до любых IO (exit 1). Решение Orchestrator: честный UX вместо тихого fallback; D2.1 подменит на реальный пайплайн.
- Кэш-fingerprint различает режимы (`+detect_mode=composite_dag`); default-ключ `default_scan_v1` байт-в-байт прежний (кэш существующих пользователей жив).
- `racc init`: закомментированный `[detect]`-хинт в шаблоне.

#### Тесты
- Новый `tests/detect_mode_config.rs`: 7 кейсов (default, parse обоих строк, unknown+suggestion, fail-fast до IO, CLI>config override в обе стороны, explicit==default, cache-hit default).
- `cargo test --workspace` green · fmt clean · clippy core+cli `-D warnings` clean · smoke exit-кодов 1/2 подтверждены лично.

#### DoD
- [x] Config parse + default priority_table
- [x] CLI override
- [x] Unknown mode → Error
- [x] Tests + help text

#### Follow-up
- Wiki обновлён в этом же этапе (UX-этап): cli-usage / sniff / configuration.

### 2026-08-22 — D2.1 — WorkspaceDetector → tree (composite_dag)

- **Дата:** 2026-08-22
- **Ветка / PR:** `d2-workspace-detector` / **#95** (squash в `dev`)
- **Статус:** CLOSED — фаза D2 начата
- **Версия:** 0.3.4

#### Сделано
- `detect/workspace.rs`: `WorkspaceDetector::detect_tree(project_root, markers_by_path) -> Result<StackNode>` — один узел на scope; ecosystem = первый применимый детектор по реестру (`"unknown"` если нет), frameworks = union в порядке реестра, language через `resolve_language`, confidence 1.0/0.0; корень без маркеров → placeholder unknown; привязка к ближайшему строго содержащему предку; дети отсортированы по нормализованному ключу компонентов (детерминизм). FS сам не обходит.
- Containment: scope вне project_root → `Error::Other`; переиспользованы `ensure_scan_root` + `is_path_under_root` (без клонов).
- Facade `app/sniff.rs`: guard удалён; ветка CompositeDag заполняет `project.stack_tree` per candidate через re-use `find_candidates` (те же max_depth/policy); плоский stack заполнен в обоих режимах; priority_table путь не тронут; кэш-fingerprint прежний.
- **Breaking:** убран публичный `Error::DetectPipelineUnavailable` (composite_dag теперь исполняется, experimental). Зачистка доков: detect/mod, mode, project.stack_tree, config/init шаблон.
- Re-export: `raccpack_core::detect::WorkspaceDetector`.

#### Решения Orchestrator
- Семантика sniff: nested candidates НЕ схлопываются (инвариант find_candidates «nested projects are not collapsed»); каждый кандидат получает своё дерево — collapse/multi-opinion merge → D2.2.
- Multi-ecosystem мнения на одной scope схлопываются в один узел (primary + union frameworks) — зафиксировано в rustdoc как текущее ограничение до D2.2.
- Гонка Test→Dev: ретракты Test проверены по merge-ready tip (§5.2 AGENTS) — все три сняты (тест уже перевёрнут в коммите Dev; fmt/clippy чистые).

#### Тесты
- Юниты: `detect/workspace_tests.rs` (9 кейсов); перевёрнуты `tests/detect_mode_config.rs` Case 4/5 и sniff success-path тест.
- Integration: новый `tests/workspace_detect.rs` (11 кейсов: монорепо rust+web, single project, placeholder root, containment reject, typed errors, сортировка/детерминизм, nearest-ancestor, serde roundtrip, facade composite/priority_table, symlink never a scope).
- `cargo test --workspace` green (все suite'ы 0 failed) · fmt clean · clippy core+cli all-targets `-D warnings` clean.

#### DoD
- [x] WorkspaceDetector returns StackNode
- [x] composite_dag mode uses this path
- [x] priority_table path untouched
- [x] Tests green

#### Follow-up
- `composite_stack_tree` перезапускает find_candidates для каждого кандидата (двойной обход при вложенных проектах) — производительность, рассмотреть в D3/D4.
- `tests/workspace_detect.rs` 540 строк — крупный integration-файл (в духе F-TEST-SIZE, next touch).
- Расхождение лексического linking vs канонизирующего containment на symlink-scope — задокументировано в rustdoc; вернуться при D2.2/D3.1.
