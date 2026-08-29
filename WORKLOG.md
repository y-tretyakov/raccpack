# WORKLOG — raccpack

Журнал **текущей** вехи. Orchestrator: y-tretyakov.

| Архив | Путь |
|-------|------|
| MVP | [`docs/archive/WORKLOG_MVP.md`](docs/archive/WORKLOG_MVP.md) |
| Alpha (A1–A4) | [`docs/archive/WORKLOG_ALPHA.md`](docs/archive/WORKLOG_ALPHA.md) |
| **Detect v2 (D1–D4)** | [`docs/archive/WORKLOG_DETECT.md`](docs/archive/WORKLOG_DETECT.md) |
| Версии / roadmap | [`docs/VERSION_ROADMAP.md`](docs/VERSION_ROADMAP.md) |

---

## Текущий статус

| | |
|--|--|
| **Версия** | **`0.4.2`** |
| **Веха** | Detect v2 ✅ CLOSED · **Beta B1.2 done (0.4.2)** · Beta → 0.5.0 |
| **Этап** | **B1.3** — TUI dig screen |
| **Предыдущее** | B1.2 TUI sniff screen closed (0.4.2) |

```text
MVP 0.1.0 ✅ → Alpha 0.3.0 ✅ → Detect v2 0.4.0 ✅ → Beta 0.5.0 → RC 0.9.0 → 1.0.0
```

---

## Backlog (Beta → 0.5.0)

Кратко (детали — `docs/VERSION_ROADMAP.md` / roadmap-v1):

```
[x] B1.1 TUI skeleton (0.4.1)
[x] B1.2  TUI sniff screen (0.4.2)
[ ] B1.3  TUI dig screen
[ ] B1.4  TUI raid + progress
[ ] B1.5  TUI reveal modal
[ ] B2  Desktop (Tauri + React) + BFF + ephemeral reveal
[ ] B3  Security hardening + Safe Reveal contract
[ ] B4  Productization (den gc, parallel sniff, docs) → Beta exit 0.5.0
```

Спеки TUI: `docs/raccpack-tui-spec-*.md` (уточнять по мере B1).

---

## Открытые follow-ups (не блокеры B1)

Перенесены с Alpha/Detect; полный список и история — в архивах WORKLOG.

| ID | Суть | Горизонт |
|----|------|----------|
| F-SKIP-1 | единый skip ↔ cleanup | B3 |
| F-PACK-SIZE / F-ATOMIC-SIZE / F-TEST-SIZE / F-CLI-SIZE | файлы ≳400 строк | next touch |
| P2-7 | сужение public API | R1 |
| OS-WIN | Windows paths best-effort | R2 |

---

## Решения (живые)

| Дата | Решение |
|------|---------|
| 2026-08-19/20 | Raid default **Atomic**; FailFast = debug |
| 2026-08-20 | Manifest только после successful Atomic commit |
| 2026-08-20 | CLI raid: exit **1** при `!success` |
| 2026-08-21 | MSRV **1.85**; логи → stderr; never log passphrase/raw |
| 2026-08-22 | Detect v2 = отдельная веха **0.4.0** (закрыта) |
| 2026-08-26 | **Один продукт на PR.** Crate только под roadmap raccpack. |

---

## Инцидент (кратко)

**2026-08-26 — synthrodex-tui contamination**

В PR #103 вместе с `raccpack-tui` попал чужой crate `synthrodex-tui` (X11/Rofi/NowBar) — не продукт репозитория.  
**Fix:** crate удалён, workspace очищен (`rg synthrodex` → 0).  
**Правило:** не смешивать посторонние продукты в workspace raccpack.

---

## Этапы (Beta)

### 2026-08-29 — B1.2 — TUI sniff screen ✅ CLOSED

- **Ветка:** `b1.2-sniff-screen-fix` (PR #108 → `dev`, squash); исходный PR #107 (B1.2 sniff screen) доделан и закрыт этим фиксом
- **Версия:** 0.4.2
- **DoD:**
  - [x] Отдельный worker-поток + `WorkerMsg`/`WorkerEvent` + `TuiProgressSink` через core `ProgressSink`
  - [x] Экран проекта (loading / error / empty / table): name, language, frameworks, size, git
  - [x] Неблокирующий sniff; j/k навигация; progress %; cache indicator
  - [x] **Fix bridge:** worker `WorkerEvent` → `AppEvent::Worker` (ранее `worker_receiver` отбрасывался, события не доходили до UI)
  - [x] **Fix loading:** `set_loading(true)` до отправки `WorkerMsg::Sniff`
  - [x] Panic hook через `OnceLock`; фильтр `KeyEventKind::Press` (фиксы B1.1)
  - [x] Тесты: worker (cancel/sniff done) + event bridge + state/format_bytes + integration fixture
  - [x] `cargo test -p raccpack-tui` green (64), fmt + clippy `-D warnings` чистые
- **Файлы:** `crates/raccpack-tui/src/worker.rs`, `event.rs`, `app.rs`, `src/ui/screens/sniff.rs`, `tests/worker_bridge_test.rs`
- **Решения:** wiring через dedicated bridge thread (по образцу `event_reader`), а не select!/mux — проще при стандартном `std::sync::mpsc`.

### 2026-08-29 — B1.1 — TUI skeleton (`raccpack-tui`) ✅ CLOSED

- **Ветка:** `b1-tui-skeleton` (PR → `dev`, squash)
- **Версия:** 0.4.1
- **DoD:**
  - [x] Crate `raccpack-tui` в workspace
  - [x] Ratatui + crossterm: event loop, screens enum, theme
  - [x] Навигация: `1`–`4`, `?`, `q`; restore terminal on exit
  - [x] Stubs only (без реального sniff)
  - [x] Builds in workspace
  - [x] `cargo test -p raccpack-tui` (49 tests green)
- **Файлы:** `crates/raccpack-tui/` (new crate)

### 2026-08-26 — B1.x — TUI skeleton (in progress)

- **Ветка:** `b1-tui-skeleton` (или superseding clean branch)
- **Статус:** cleanup после инцидента; только `crates/raccpack-tui`
- **Не merge в `dev` без явного approval**

Когда этап закроется — одна короткая запись сюда (PR, DoD, version bump если нужен).
