# Raccpack — roadmap до 1.0.0

Цель: стабильный **1.0.0** с ядром, CLI, TUI и Desktop (Tauri), по видению архитектуры и контракту facade/den.

Версионирование до 1.0.0 — **0.x** (ломающие изменения API допустимы).  
С **1.0.0** — semver-стабильность public API core + CLI exit codes + den layout major.

---

## Линия релизов

| Веха | Версия (ориентир) | Смысл |
|------|-------------------|--------|
| **MVP** | 0.1.x | Минимальный полезный цикл: sniff → dig → pack → den (без полного raid/TUI/Desktop) |
| **Alpha** | 0.2.x–0.3.x | Полный raid + age-stash + rinse; CLI feature-complete для headless |
| **Beta** | 0.4.x–0.5.x | TUI; Desktop skeleton; жёсткие границы безопасности и den v1 |
| **RC** | 0.9.x | Заморозка API/den; полировка; нагрузка и регрессии |
| **Stable** | **1.0.0** | Документация, политика поддержки, tag |

Ниже фазы **последовательны**. Каждая фаза = набор коротких этапов (одна строка на этап). Детализация этапов — отдельными документами позже.

---

# MVP → 0.1.0

Сквозной путь: задать root + den → увидеть проекты и секреты → упаковать проект без секретов по имени → положить архив в den.

## Фаза M1 — Каркас workspace и core

- M1.1 — Workspace Cargo: `raccpack-core`, пустой `raccpack-cli`, общая лицензия/README.
- M1.2 — Domain DTO: `Project`, `Stack`, `ScanReport`, `SensitiveRisk`, `Error` без UI-зависимостей.
- M1.3 — Config: TOML load/validate, `scan_root` / `den_dir`, strict errors.
- M1.4 — SkipPolicy и walk-хелпер с `follow_links(false)`.

## Фаза M2 — Sniff (обнаружение проектов)

- M2.1 — Marker files + skip dirs → candidates.
- M2.2 — Detect languages/frameworks → `Stack`.
- M2.3 — Facade `sniff` + простой cache (versioned).
- M2.4 — CLI: `racc sniff --root …` (text + `--json`).

## Фаза M3 — Dig (секреты, read-only)

- M3.1 — Filename patterns + risk model (severity API).
- M3.2 — Content markers (regex/prefix) + limits размера файла.
- M3.3 — Facade `dig` (masked output, без raw в report).
- M3.4 — CLI: `racc dig` + exit policy заготовка (`FailOnCritical`).

## Фаза M4 — Pack + den layout (минимум)

- M4.1 — Pack tar+zstd с deny-list по имени и SkipPolicy.
- M4.2 — Запись в `den/packs/…` + `.den-version` + README den.
- M4.3 — Facade `pack` + DryRun/Commit.
- M4.4 — CLI: `racc pack --project … --den …`; ручной E2E MVP.

**MVP exit criteria:** на реальной папке проектов CLI показывает sniff/dig и создаёт pack в den без TUI/Desktop/age/rinse.

---

# Alpha → 0.3.0

Полный headless-цикл raid: секреты в age, очистка, pack, manifest.

## Фаза A1 — Stash (age)

- A1.1 — Интеграция age (passphrase), zeroize материала ключа.
- A1.2 — Manifest записей stash без raw; удаление источников в Commit.
- A1.3 — Facade `stash` + артефакты `den/secrets/…`.
- A1.4 — CLI: `racc stash` (prompt passphrase / env для CI-теста).

## Фаза A2 — Rinse (очистка)

- A2.1 — Стратегии cleanup (rust/node/python/…) + config toggles.
- A2.2 — Facade `rinse` DryRun/Commit + подсчёт bytes freed.
- A2.3 — CLI: `racc rinse`.

## Фаза A3 — Raid orchestration

- A3.1 — Facade `raid`: stash → rinse → pack → move, fail-fast, `success`.
- A3.2 — ProgressSink + CLI progress.
- A3.3 — Manifest JSON в `den/manifests/…`.
- A3.4 — CLI: `racc raid --yes`; E2E alpha на fixture-репо.

## Фаза A4 — Git и DX alpha

- A4.1 — GitClient (process) + status sensitive files в dig.
- A4.2 — Config migrate chain + `racc init`.
- A4.3 — Логи tracing без секретов; `--verbose`.
- A4.4 — Интеграционные тесты core + CI job `cargo test`.

**Alpha exit criteria:** один командой `raid` секреты уезжают в `.age`, мусор чистится, pack в den, manifest на месте; только CLI.

---

# Beta → 0.5.0

Интерактив и Desktop; безопасность и контракты ближе к 1.0.

## Фаза B1 — TUI (Ratatui)

- B1.1 — Каркас TUI binary, навигация, theme.
- B1.2 — Экран sniff: список проектов, стек, размер.
- B1.3 — Экран dig: risk filter, masked details.
- B1.4 — Подтверждение raid + progress; вызов того же facade.

## Фаза B2 — Desktop skeleton (Tauri)

- B2.1 — Tauri app + React/Vite + Zustand stores.
- B2.2 — BFF commands: sniff/dig (DTO only, no raw).
- B2.3 — UI: выбор root/den, таблица проектов, список секретов.
- B2.4 — Raid из Desktop: passphrase dialog → events progress → result.

## Фаза B3 — Security & policy hardening

- B3.1 — Content-deny при pack; единый name-policy с dig.
- B3.2 — EnabledGroups type-safe; fingerprint/mask repeated secrets.
- B3.3 — Path containment, den `0700`/`0600` где возможно.
- B3.4 — Threat checklist + тесты «secret not in logs/errors».

## Фаза B4 — Productization beta

- B4.1 — `racc den` list/gc staging.
- B4.2 — Параллельный sniff (`parallel_jobs`).
- B4.3 — Документация пользователя (CLI+TUI), install notes.
- B4.4 — Beta tag; сбор баг-репортов по UX.

**Beta exit criteria:** CLI + TUI + Desktop прогоняют sniff/dig/raid; den v1 соблюдён; нет raw secrets в UI/logs по умолчанию.

---

# RC → 0.9.x

Заморозка контрактов, нагрузка, полировка перед 1.0.0.

## Фаза R1 — API freeze

- R1.1 — Аудит public `raccpack-core` API; убрать experimental exports.
- R1.2 — Зафиксировать den layout major=1 и schema manifest.
- R1.3 — Зафиксировать CLI flags/exit codes; changelog breaking.
- R1.4 — Semver policy документ (что считается breaking после 1.0).

## Фаза R2 — Quality

- R2.1 — Property/table tests порядка PATTERNS/CONTENT_MARKERS.
- R2.2 — Нагрузочный sniff/dig на большом дереве; профилирование hot path.
- R2.3 — Clippy `-D warnings`, fmt, MSRV в CI.
- R2.4 — Кросс-платформа smoke (Linux primary + macOS/Windows best-effort).

## Фаза R3 — UX RC

- R3.1 — Единые тексты ошибок + `suggestion()` во всех UI.
- R3.2 — TUI/Desktop empty states, cancel long ops.
- R3.3 — CLI man/help examples; shell completions.
- R3.4 — RC builds (signed/notarized по возможности).

## Фаза R4 — RC validation

- R4.1 — Чеклист E2E из архитектуры (happy path + dry-run + fail-fast).
- R4.2 — Security pass (passphrase, permissions, symlink).
- R4.3 — Bug bash; zero P0/P1 open.
- R4.4 — Tag `v0.9.0` RC; только blocker-фиксы до 1.0.

**RC exit criteria:** API/den/CLI заморожены; E2E и security checklist green; нет открытых P0/P1.

---

# Stable → 1.0.0

## Фаза S1 — Release 1.0.0

- S1.1 — Финальный CHANGELOG / MIGRATION с 0.9.
- S1.2 — Tag `v1.0.0`; publish crates / binaries / desktop artifacts.
- S1.3 — Пользовательская документация «1.0» (quickstart, den, security).
- S1.4 — Политика поддержки 1.x (support window, security advisories).

**1.0.0 exit criteria:** артефакты опубликованы; quickstart воспроизводим с нуля; public core API и den major обещаны стабильными.

---

## Сводная карта фаз

```text
MVP     M1 workspace/DTO/config  →  M2 sniff  →  M3 dig  →  M4 pack+den
Alpha   A1 stash/age  →  A2 rinse  →  A3 raid  →  A4 git+CI
Beta    B1 TUI  →  B2 Desktop  →  B3 security  →  B4 den gc + docs
RC      R1 freeze  →  R2 quality  →  R3 UX  →  R4 validation
Stable  S1 release 1.0.0
```

## Зависимости между вехами (жёсткие)

- MVP pack **не** требует age (только deny по имени).
- Alpha raid **требует** M3 dig + M4 pack layout.
- TUI/Desktop **только** после facade sniff/dig/raid (Alpha A3).
- RC freeze **после** Beta security (B3).
- 1.0.0 **после** RC validation без открытых blocker’ов.

## Вне scope до 1.0.0

- Облачный den, KMS как primary, multi-user HTTP BFF.
- Авто-PR «удали секреты», полноценный redact-engine.
- Плагины сторонних pattern-pack’ов.
- Гарантия feature-parity Windows = Linux (smoke — да, 100% — нет).

---

*Следующий шаг:* выбрать первую фазу (M1) и расписать её этапы подробно (вход/выход, файлы, критерии готовности).
