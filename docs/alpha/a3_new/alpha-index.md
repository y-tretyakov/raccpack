# Alpha → 0.3.0 — актуальная позиция

**Источник истины по коду:** `WORKLOG.md` (2026-08-18).  
**Vision/roadmap** вводят атомарный raid (WAL); код A3.1/A3.2 уже в `dev` как **fail-fast**. Спеки ниже стыкуют оба факта.

## Статус по факту

| Этап | Статус | Факт в коде |
|------|--------|-------------|
| **A1.1–A1.4** | ✅ CLOSED | stash + age + CLI |
| **A2.1–A2.3** | ✅ CLOSED | rinse strategies + CLI |
| **A3.1** | ✅ CLOSED | `raid()` fail-fast, `Ok(success:false)`, без WAL |
| **A3.2** | ✅ CLOSED | Raid progress + **минимальный** `racc raid` |
| **A3.3** | ⬜ NEXT | **Atomic upgrade** (staging + WAL + rollback) |
| **A3.4** | ⬜ | Manifest JSON только после successful commit |
| **A3.5** | ⬜ | Полный CLI (toggles, exit 1), E2E, orphan green, wiki |
| **A4.1–A4.4** | ⬜ | Git, init/migrate, tracing, CI |

```text
DONE:  A1 → A2 → A3.1 (fail-fast) → A3.2 (progress + thin CLI)
NEXT:  A3.3 atomic → A3.4 manifest → A3.5 CLI/E2E/wiki → A4 → 0.3.0
```

## Почему A3.3 = atomic, а не «просто manifest»

Roadmap/vision требуют Atomic **до** Alpha exit. Manifest (бывший A3.3 в старом backlog WORKLOG) логично писать **только после** успешного atomic commit → сдвиг:

| Старый backlog WORKLOG | Новая нумерация спек |
|------------------------|----------------------|
| A3.3 manifest | → **A3.4** |
| A3.4 CLI + E2E | → **A3.5** (+ orphan + wiki) |
| *(не было)* | → **A3.3** Atomic upgrade |

## Документы

| | |
|--|--|
| A1 | [a1-index.md](a1-index.md) (archive-style closed) |
| A2 | [a2-index.md](a2-index.md) |
| A3 | [a3-index.md](a3-index.md) **← главная точка входа сейчас** |
| A4 | [a4-index.md](a4-index.md) |

## Alpha exit (не менять)

Одной командой `racc raid --yes`: secrets→age, rinse, pack, manifest; при ошибке фазы — **полный откат, нет orphan**; headless CLI.
