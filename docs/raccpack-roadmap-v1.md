# Raccpack — roadmap до 1.0.0

**Статус:** обновлено с учётом атомарных откатов, композитных детекторов и эфемерной верификации секретов.  
**Текущая точка:** MVP 0.1.0 закрыт; Alpha A1–A3 закрыты (stash/rinse/raid доступны), идёт A4 (git/DX): A4.1–A4.2 закрыты.  
**Текущая версия:** `0.2.14` — сверять `docs/VERSION_ROADMAP.md`.

Цель: стабильный **1.0.0** с ядром, CLI, TUI и Desktop (Tauri), по видению архитектуры и контракту facade/den.

Версионирование до 1.0.0 — **0.x** (ломающие изменения API допустимы).  
С **1.0.0** — semver-стабильность public API core + CLI exit codes + den layout major.

---

## Линия релизов

| Веха | Версия (ориентир) | Смысл |
|------|-------------------|--------|
| **MVP** | 0.1.x | Минимальный полезный цикл: sniff → dig → pack → den (закрыт) |
| **Alpha** | 0.2.x–0.3.x | Полный raid (атомарный) + age-stash + rinse; CLI feature-complete для headless |
| **Detect v2** | 0.4.x | Композитные детекторы / DAG для монорепо (между Alpha и Beta) |
| **Beta** | 0.5.x | TUI; Desktop; ephemeral reveal; hardening безопасности |
| **RC** | 0.9.x | Заморозка API/den; полировка; нагрузка и регрессии |
| **Stable** | **1.0.0** | Документация, политика поддержки, tag |

Ниже фазы **последовательны**. Каждая фаза = набор коротких этапов. Детализация этапов — отдельными спеками в `docs/`.

---

# MVP → 0.1.0 (закрыто)

Сквозной путь: задать root + den → увидеть проекты и секреты → упаковать проект без секретов по имени → положить архив в den.

## Фаза M1 — Каркас workspace и core ✅

- M1.1 — Workspace Cargo: `raccpack-core`, пустой `raccpack-cli`, общая лицензия/README.
- M1.2 — Domain DTO: `Project`, `Stack`, `ScanReport`, `SensitiveRisk`, `Error` без UI-зависимостей.
- M1.3 — Config: TOML load/validate, `scan_root` / `den_dir`, strict errors.
- M1.4 — SkipPolicy и walk-хелпер с `follow_links(false)`.

## Фаза M2 — Sniff (обнаружение проектов) ✅

- M2.1 — Marker files + skip dirs → candidates.
- M2.2 — Detect languages/frameworks → `Stack`.
- M2.3 — Facade `sniff` + простой cache (versioned).
- M2.4 — CLI: `racc sniff --root …` (text + `--json`).

## Фаза M3 — Dig (секреты, read-only) ✅

- M3.1 — Filename patterns + risk model (без raw в API).
- M3.2 — Content markers (regex/prefix) + limits размера файла.
- M3.3 — Facade `dig` (masked output, без raw в report).
- M3.4 — CLI: `racc dig` + exit policy заготовка (`FailOnCritical`).

## Фаза M4 — Pack + den layout (минимум) ✅

- M4.1 — Pack tar+zstd с deny-list по имени и SkipPolicy.
- M4.2 — Запись в `den/packs/…` + `.den-version` + README den.
- M4.3 — Facade `pack` + DryRun/Commit.
- M4.4 — CLI: `racc pack --project … --den …`; ручной E2E MVP.

**MVP exit criteria:** на реальной папке проектов CLI показывает sniff/dig и создаёт pack в den без TUI/Desktop/age/rinse. ✅

---

# Alpha → 0.3.0

Полный headless-цикл raid: секреты в age, очистка, pack, **атомарный** commit, manifest.

## Фаза A1 — Stash (age) ✅

- A1.1 — Интеграция age (passphrase), zeroize материала ключа.
- A1.2 — Manifest записей stash без raw; удаление источников в Commit.
- A1.3 — Facade `stash` + артефакты `den/secrets/…`.
- A1.4 — CLI: `racc stash` (prompt passphrase / env для CI-теста).

## Фаза A2 — Rinse (очистка) ✅

- A2.1 — Стратегии cleanup (rust/node/python/…) + config toggles.
- A2.2 — Facade `rinse` DryRun/Commit + подсчёт bytes freed.
- A2.3 — CLI: `racc rinse`.

## Фаза A3 — Raid orchestration (атомарная)

> **Ключевое изменение относительно исходного roadmap:** вместо чистого fail-fast внедряется атомарность по умолчанию (staging + WAL + rollback). Fail-fast остаётся как debug-флаг.

- A3.0 — Подготовка: orphan-фикстуры и regression-тесты «частичный успех оставляет артефакты» (сейчас красные — ожидаемо).
- A3.1 — Единый `staging/{raid_id}/` на весь raid; все промежуточные артефакты только туда.
- A3.2 — Write-Ahead Log (WAL): append-only журнал каждого Create/Rename/Delete **до** побочного эффекта.
- A3.3 — Rollback-движок: при Err читаем WAL назад, откатываем, удаляем staging.
- A3.4 — Facade `raid`: stash → rinse → pack → atomic commit (rename в den). Режимы `Atomic` (default) / `FailFast` (`--fail-fast`).
- A3.5 — ProgressSink + CLI progress; в отчёте `rolled_back`, список откатанных путей (без секретов).
- A3.6 — Manifest JSON в `den/manifests/…` только после успешного commit.
- A3.7 — CLI: `racc raid --yes`; E2E alpha на fixture-репо + orphan regression green.

## Фаза A4 — Git и DX alpha

- A4.1 — GitClient (process) + status sensitive files в dig. ✅
- A4.2 — Config migrate chain + `racc init`. ✅
- A4.3 — Логи tracing без секретов; `--verbose`. ✅
- A4.4 — Интеграционные тесты core + CI job `cargo test`.

**Alpha exit criteria:** одной командой `raid` секреты уезжают в `.age`, мусор чистится, pack в den, manifest на месте; при любой ошибке фазы — полный откат, нет orphan; только CLI.

---

# Detect v2 → 0.4.x

Композитные детекторы и DAG стека для монорепозиториев и гибридных проектов.  
Вклинивается **после** стабильного raid (Alpha) и **до** полноценного Beta UI, чтобы TUI/Desktop сразу получили корректное дерево стека.

## Фаза D1 — Реестр и контракт детекторов

- D1.1 — Trait `StackDetector` + внутренний реестр модулей (сохранить текущую модульность «один язык ≈ один модуль»).
- D1.2 — `Detection` / `StackNode` DTO (markers, confidence, scope).
- D1.3 — Config / CLI: `detect.mode = priority_table | composite_dag` (default пока `priority_table`).

## Фаза D2 — Workspace / Composite detector

- D2.1 — `WorkspaceDetector`: опрашивает все модули, строит направленный граф (DAG) технологий.
- D2.2 — Фаза разрешения конфликтов: слияние экспертных мнений в богатое дерево проекта (не «один победитель»).
- D2.3 — Обратная совместимость: плоский `stack: String` остаётся в JSON; добавляется `stack_tree`.

## Фаза D3 — Влияние на rinse / pack / sniff

- D3.1 — `rinse` использует DAG: чистит `target/` только в Rust-поддеревьях, `node_modules/` — в Node и т.д.
- D3.2 — `sniff` выводит дерево/DAG при `--detect-mode=dag` или в JSON.
- D3.3 — Фикстуры монорепо (Rust+Node, Python+JS …) + тесты.

**Detect v2 exit criteria:** на типичном monorepo `sniff` показывает корректное дерево; `rinse` удаляет только релевантный мусор; legacy PriorityTable продолжает работать.

---

# Beta → 0.5.0

Интерактив и Desktop; ephemeral reveal; безопасность и контракты ближе к 1.0.

## Фаза B1 — TUI (Ratatui)

- B1.1 — Каркас TUI binary, навигация, theme.
- B1.2 — Экран sniff: список проектов, стек (и дерево при DAG), размер.
- B1.3 — Экран dig: risk filter, masked details.
- B1.4 — Подтверждение raid + progress; вызов того же facade.
- B1.5 — Безопасный modal reveal (opt-in).

## Фаза B2 — Desktop skeleton (Tauri)

- B2.1 — Tauri app + React/Vite + Zustand stores.
- B2.2 — BFF commands: sniff/dig/raid (DTO only, no raw).
- B2.3 — UI: выбор root/den, таблица проектов, список секретов (masked).
- B2.4 — Raid из Desktop: passphrase dialog → events progress → result.
- B2.5 — `reveal_secret_ephemeral`: IPC → изолированный React-компонент (минуя Zustand) → zeroize при закрытии.

## Фаза B3 — Security & policy hardening + Safe Reveal

- B3.1 — Content-deny при pack; единый name-policy с dig.
- B3.2 — EnabledGroups type-safe; fingerprint/mask repeated secrets.
- B3.3 — Path containment, den `0700`/`0600` где возможно.
- B3.4 — Контракт `EphemeralSecret` в core (Drop + zeroize, не serde в отчёты).
- B3.5 — CLI interactive reveal (защищённый терминальный ввод, стирание из history).
- B3.6 — Threat checklist + тесты «secret not in logs/errors/store».
- B3.7 — Опциональный audit-log факта reveal (без значения).

## Фаза B4 — Productization beta

- B4.1 — `racc den` list/gc staging.
- B4.2 — Параллельный sniff (`parallel_jobs`).
- B4.3 — Документация пользователя (CLI+TUI+Desktop), install notes; разделы про атомарность и reveal.
- B4.4 — Beta tag; сбор баг-репортов по UX.

**Beta exit criteria:** CLI + TUI + Desktop прогоняют sniff/dig/raid; den v1 соблюдён; нет raw secrets в UI/logs/store по умолчанию; ephemeral reveal работает безопасно; DAG-детект доступен.

---

# RC → 0.9.x

Заморозка контрактов, нагрузка, полировка перед 1.0.0.

## Фаза R1 — API freeze

- R1.1 — Аудит public `raccpack-core` API; убрать experimental exports.
- R1.2 — Зафиксировать den layout major=1 и schema manifest.
- R1.3 — Зафиксировать CLI flags/exit codes (включая rollback/reveal); changelog breaking.
- R1.4 — Semver policy документ (что считается breaking после 1.0).

## Фаза R2 — Quality

- R2.1 — Property/table tests порядка PATTERNS/CONTENT_MARKERS + rollback invariants.
- R2.2 — Нагрузочный sniff/dig/raid на большом дереве; профилирование hot path.
- R2.3 — Clippy `-D warnings`, fmt, MSRV в CI.
- R2.4 — Кросс-платформа smoke (Linux primary + macOS/Windows best-effort).

## Фаза R3 — UX RC

- R3.1 — Единые тексты ошибок + `suggestion()` во всех UI.
- R3.2 — TUI/Desktop empty states, cancel long ops.
- R3.3 — CLI man/help examples; shell completions.
- R3.4 — RC builds (signed/notarized по возможности).

## Фаза R4 — RC validation

- R4.1 — Чеклист E2E из архитектуры (happy path + dry-run + atomic rollback + reveal).
- R4.2 — Security pass (passphrase, permissions, symlink, no-leak).
- R4.3 — Bug bash; zero P0/P1 open.
- R4.4 — Tag `v0.9.0` RC; только blocker-фиксы до 1.0.

**RC exit criteria:** API/den/CLI заморожены; E2E и security checklist green; нет открытых P0/P1.

---

# Stable → 1.0.0

## Фаза S1 — Release 1.0.0

- S1.1 — Финальный CHANGELOG / MIGRATION с 0.9.
- S1.2 — Tag `v1.0.0`; publish crates / binaries / desktop artifacts.
- S1.3 — Пользовательская документация «1.0» (quickstart, den, security, atomicity, reveal).
- S1.4 — Политика поддержки 1.x (support window, security advisories).

**1.0.0 exit criteria:** артефакты опубликованы; quickstart воспроизводим с нуля; public core API и den major обещаны стабильными.

---

## Сводная карта фаз

```text
MVP        M1 workspace/DTO/config  →  M2 sniff  →  M3 dig  →  M4 pack+den          ✅ 0.1.0
Alpha      A1 stash/age ✅  →  A2 rinse ✅  →  A3 raid+atomic  →  A4 git+CI          → 0.3.0
Detect v2  D1 registry  →  D2 composite DAG  →  D3 rinse/sniff impact               → 0.4.x
Beta       B1 TUI  →  B2 Desktop+reveal  →  B3 security+reveal  →  B4 den gc + docs → 0.5.0
RC         R1 freeze  →  R2 quality  →  R3 UX  →  R4 validation                     → 0.9.x
Stable     S1 release 1.0.0
```

## Куда вклинились нововведения из разбора

| Нововведение | Куда в roadmap | Почему |
|--------------|----------------|--------|
| **Атомарный откат / WAL для raid** | A3 (расширена) | Raid ещё не реализован — идеальное место внедрить атомарность сразу, а не чинить fail-fast потом. |
| **Композитные детекторы / DAG** | Новая веха 0.4.x (Detect v2) | Sniff уже стабилен; UI ещё нет — можно улучшить модель стека до того, как TUI/Desktop зафиксируют UX. |
| **Эфемерный reveal секретов** | B2 + B3 (Beta) | Нужны UI-поверхности (CLI interactive + Desktop modal); логично вместе с security hardening. |

## Зависимости между вехами (жёсткие)

- MVP pack **не** требует age (только deny по имени). ✅
- Alpha raid **требует** dig + pack layout + stash + rinse.
- Атомарность **внутри** A3 (не отдельная веха).
- Detect v2 **после** Alpha (raid стабилен), **до** Beta UI.
- TUI/Desktop **только** после facade sniff/dig/raid (Alpha A3).
- Ephemeral reveal **в** Beta (нужен UI + hardened core).
- RC freeze **после** Beta security (B3).
- 1.0.0 **после** RC validation без открытых blocker’ов.

## Вне scope до 1.0.0

- Облачный den, KMS как primary, multi-user HTTP BFF.
- Авто-PR «удали секреты», полноценный redact-engine.
- Плагины сторонних pattern-pack’ов.
- Resume после сбоя raid (WAL позволяет добавить позже).
- Гарантия feature-parity Windows = Linux (smoke — да, 100% — нет).

---

*Следующий шаг:* взять **A3.0–A3.3** (orphan-тесты + staging + WAL + rollback) и расписать отдельными спеками в `docs/alpha/`.
