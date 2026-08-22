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
| **Версия** | **`0.3.1`** (Detect v2 стартовал) |
| **Веха** | **Detect v2 → 0.4.0**, фаза D1 в работе |
| **Следующий этап** | **D1.2** — Detection / StackNode DTO → bump **`0.3.2`** |
| **Предыдущий** | D1.1 StackDetector trait + registry (PR #92) |

```text
MVP 0.1.0 ✅ → Alpha 0.3.0 ✅ → Detect v2 0.4.0 ⬜ → Beta 0.5.0 → RC 0.9.0 → 1.0.0
```

---

## Backlog (Detect v2 → 0.4.0)

```
[x] D1.1 StackDetector trait + registry          → 0.3.1
[ ] D1.2 Detection / StackNode DTO                 → 0.3.2
[ ] D1.3 detect.mode config + CLI                  → 0.3.3
[ ] D2.1 WorkspaceDetector → DAG                   → 0.3.4
[ ] D2.2 conflict merge                            → 0.3.5
[ ] D2.3 flat stack + stack_tree compat            → 0.3.6
[ ] D3.1 rinse по DAG scopes                       → 0.3.7
[ ] D3.2 sniff tree output                         → 0.3.8
[ ] D3.3 fixtures + Detect v2 exit                 → 0.4.0
```

Спеки: `docs/detect/detect-v2-index.md` и `d1.*` / `d2.*` / `d3.*`.

### Follow-ups (открытые)

- [ ] hygiene: `detect/mod.rs` 435 строк — вынести инлайн unit-тесты в отдельный файл (сделать при удобном этапе D1.x/D2.x, не блокирует)

### Exit criteria Detect v2

- [ ] Monorepo: `sniff` показывает корректное `stack_tree` (composite_dag)
- [ ] `rinse` удаляет мусор только в релевантных scope
- [ ] `detect.mode = priority_table` — без регрессий (default)
- [ ] CLI `--detect-mode` / config `[detect] mode`
- [ ] Плоский `stack` всегда в JSON

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
**Следующее:** D1.1 по спеке `docs/detect/d1.1-stack-detector-trait.md` → version **0.3.1**.

---

## Шаблон записи этапа (Detect+)

```markdown
### D1.1 — StackDetector trait + registry (… )

- **Дата:**
- **Ветка / PR:**
- **Статус:**
- **Версия:** 0.3.1

#### Сделано
- …

#### Тесты
- `cargo test --workspace`

#### DoD
- [ ] …

#### Follow-up
- …
```
