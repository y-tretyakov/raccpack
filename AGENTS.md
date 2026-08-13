# AGENTS.md — рабочая памятка агента для raccpack

Краткая карта знаний. Этот файл — быстрая навигация и жёсткие правила.
Полные ТЗ этапов Alpha — в `docs/alpha/` (по явной ссылке перед этапом).

## Что это за проект

`raccpack` — CLI / TUI / Desktop инструмент: сканирует папку с проектами, находит
секреты, выносит их в age-архивы, чистит мусор сборки, пакует каждый проект в
`tar.zst` в «den». Ядро — `raccpack-core` (Rust). Клиенты: CLI (`racc`), TUI
(ratatui), Desktop (Tauri + React).

**Текущая веха:** Alpha → `v0.3.0` (stash / rinse / raid / git+DX).
**Закрыто:** MVP `0.1.0` — sniff, dig, pack + den layout (см. `docs/archive/WORKLOG_MVP.md`).

## Роль агента — Orchestrator (ОБЯЗАТЕЛЬНО)

`raccpack-agent-workflow.md` — **обязателен к выполнению**. Ты — **главный агент
(Orchestrator)**, а не исполнитель:

- **Не** пишешь сам: исследование, production-код, тесты, пользовательскую docs.
- **Делаешь только:** план этапа → делегирование Dev / Test (параллельно) →
  **строгая приёмка** по чеклисту критерия готовности или rework-билет →
  ведение `WORKLOG.md`.
- Docs-субагент — **только после** зелёного FINAL этапа / вехи.
- Не делегируй несколько этапов одной задачей. Не принимай этап с красными тестами.

Анти-паттерны (запрещено): писать продакшн-код «чтобы быстрее»; «сделай A1–A3
целиком»; Docs до FINAL; «тесты потом».

## Карта документов

| Файл | Что даёт | Когда читать |
|------|----------|--------------|
| `raccpack-agent-workflow.md` | Orchestrator / Dev / Test / Docs, шаблоны, rework, анти-паттерны | перед делегированием и приёмкой |
| `raccpack-roadmap-v1.md` | MVP→1.0.0, фазы M/A/B/R/S, жёсткие зависимости вех | приоритеты, границы вехи |
| `raccpack-architecture-vision.md` | Слои core / facade / UI; потоки; границы доверия; DTO | архитектурные решения |
| `raccpack-facade-and-den.md` | Сигнатуры facade, den layout, manifest JSON | use-cases, pack/stash/raid |
| `raccpack-modularity.md` | Secrets matchers + archive backends: один вид = один `*.rs` + registry | dig/stash, backends |
| `raccpack-markers-detect-modularity.md` | Markers/detect по экосистемам | scan/detect |
| `docs/alpha/*` | Детальные спеки этапов Alpha | **только по явной ссылке** перед этапом |
| `docs/archive/WORKLOG_MVP.md` | Журнал закрытого MVP (M1–M4 + docs) | справка, не править |
| `docs/archive/mvp/` | Спеки закрытых этапов M1–M4 | справка |
| `WORKLOG.md` | Текущий журнал (Alpha и далее) | после каждого этапа обновлять |
| `wiki/` | Пользовательская документация (VitePress, RU-first) | UX-тексты; **не** dev-спеки |

## Жёсткие правила (не нарушать)

- **Один этап = одна узкая задача.** Не смешивай рефакторинг + тесты + документацию.
- `docs/alpha/` и `docs/archive/` не читать целиком «на всякий случай» — только
  документ, на который человек дал ссылку перед этапом.
- Код **компилируется в рамках этапа** (или явный `TODO` только для промежуточного каркаса).
- Не менять семантику first-match таблиц `PATTERNS` / `CONTENT_MARKERS` без тестов-инвариантов.
- **Без `unwrap()` / `expect()` в production-коде** (исключение: static OnceLock init
  при старте, уже принятое в MVP).
- **Без `anyhow::Error` / `Box<dyn Error>` в public API**; только `Error` / `ConfigError`.
- Секреты (raw values, passphrase) **не** в `Display` ошибок, логах, отчётах по умолчанию.
  Masked по умолчанию; reveal только по явному opt-in.
- Стиль: rustfmt-совместимо, как окружающий код. Без лишних комментариев.
- Публичный API меняешь → обнови re-exports в `lib.rs` + отметь breaking в отчёте.
- `WalkDir` в production → с `follow_links(false)`. Pack walker — explicit DFS (см. M4.3).
- SensitiveRisk меняется только через severity API (`at_least` / `upgrade_risk`).
- После каждого этапа — отчёт по шаблону. Критерий не выполнен → следующий этап не начинать.

## Правила модульности Rust (ОБЯЗАТЕЛЬНО в поручении Dev)

Включать в задачу Dev при любом изменении Rust-кода (полная версия — в
`raccpack-modularity.md` и исторически в agent-prompt):

```text
Rust modularity rules for this repo:

- No giant `.rs` files. Aim 150–300 lines of logic; soft max ~400. Split earlier if multiple concerns.
- One concept per file (one secret matcher, one encryption backend, one policy, etc.).
- Use module directories + thin `mod.rs` (API + re-exports + registry only).
- Extensibility via registry: implementations in separate files; register in one place.
- Types in `types.rs` (or domain modules); algorithms in engine/service modules.
- Do not put all secrets, all archive backends, or full pipeline logic in a single file.
- Adding a feature = new file + one registry line, not growing a monolith.
- Keep business logic in `raccpack-core`; UI crates only call the facade.

Follow `raccpack-modularity.md`, `raccpack-markers-detect-modularity.md`, and the
existing `src/` tree. If a file can’t be summarized in one sentence, split it.
```

## Текущее состояние (после MVP 0.1.0)

- Workspace: `crates/raccpack-core` + `crates/raccpack-cli` (`racc`).
- Реализовано: **sniff**, **dig**, **pack** (tar.zst + den layout, DryRun/Commit).
- CLI: `racc sniff|dig|pack` (text + `--json`).
- Не реализовано: **stash** (age), **rinse**, **raid**, полноценный GitClient в dig.
- Wiki: VitePress в `wiki/`, RU-first, Pages на `dev`.
- Архив MVP: `docs/archive/WORKLOG_MVP.md`, `docs/archive/mvp/`.

## Alpha backlog (порядок)

```
A1.1 age + zeroize passphrase
A1.2 stash manifest (без raw) + remove sources в Commit
A1.3 facade stash + den/secrets/…
A1.4 CLI racc stash
→ A2.1 cleanup strategies + config
A2.2 facade rinse
A2.3 CLI racc rinse
→ A3.1 facade raid (fail-fast)
A3.2 ProgressSink + CLI progress
A3.3 manifest JSON в den/manifests/
A3.4 CLI racc raid --yes; E2E alpha
→ A4.1 GitClient + status в dig
A4.2 config migrate chain + racc init
A4.3 tracing без секретов; --verbose
A4.4 integration tests + CI cargo test
```

Не начинать A3, пока A1 (stash) и A2 (rinse) не стабильны по контракту.
Параллель допустима только если этапы не правят один и тот же файл
(см. `raccpack-agent-workflow.md`).

## Формат отчёта этапа

```markdown
## Этап X.Y — <название>
### Сделано
- ...
### Файлы
- path (changed|created)
### Тесты
- command: ... ; result: pass/fail
### Риски / follow-up
- ...
### Критерий готовности
- [x]/[ ] <текст из спеки>
```

## Архитектура (кратко)

- **core** — config, scan, detect, secrets, clean, archive, git (за `GitClient`),
  cache, report, policy/skip. Не знает про ratatui/tauri/react.
- **facade**: `sniff` → `dig` → `stash`(age) → `rinse` → `pack` → `raid`.
  Вход: `AppContext`; прогресс: `ProgressSink`; dry-run: `RunMode`.
- **Данные**: report DTO serde-friendly, masked secrets; ошибки — `Error` +
  `ConfigError` с `suggestion()`.
- **Den**: `manifests/`, `secrets/`, `packs/`, `staging/`, `.den-version`.
  Имена: `{slug}__{utc_timestamp}[__{short_id}]`. Paths в manifest — relative to den.
- **Exit codes CLI**: 0 ok, 1 ошибка, 2 CRITICAL (политика). `--json` → serde result.

## Git workflow

- `main` — только релизы вех (PR + review, squash, no force push).
- `dev` — основная рабочая ветка.
- Stage-ветки от `dev`: `{phase}-{short-slug}` (`a1-stash-age`, `a2-rinse`, …).
- После **каждого** закрытого этапа: PR stage → `dev` (squash) → удалить stage.
- Merge `dev → main` + tag + GitHub Release **только** на вехах:
  MVP `v0.1.0`, Alpha `v0.3.0`, Beta `v0.5.0`, RC `v0.9.0`, Stable `v1.0.0`.
- Детали — в `README.md` (раздел Git workflow).

## Команды проверки

```bash
cargo test -p raccpack-core
cargo test --workspace
cargo fmt --check
cargo clippy -p raccpack-core --all-targets -- -D warnings
pnpm run wiki:build   # если трогали wiki/
```

## Wiki (пользовательская docs)

- Источник: `wiki/` (VitePress). Primary locale — русский (root), EN — skeleton.
- Тон: спокойный, практичный, без маркетингового шума; callouts через
  `::: info|tip|warning|danger|details`.
- Не переносить dev-спеки (`docs/`, корневые architecture MD) в wiki.
- После изменений UX-текстов — `pnpm run wiki:build` без ошибок.
