# AGENTS.md — рабочая памятка агента для raccpack-core

Краткая карта знаний по проекту. Полные ТЗ — в документах ниже; этот файл — быстрая навигация и жёсткие правила.

## Что это за проект

`raccpack` — инструмент: сканирует папку с проектами, находит секреты (по имени файла, content-markers, heuristics), выносит их в зашифрованные age-архивы, чистит мусор сборки и пакует каждый проект в `tar.zst` в «den» (хранилище). Ядро — библиотека `raccpack-core` (Rust), без UI. Клиенты: CLI (clap), TUI (ratatui), Desktop (Tauri + React). Сейчас задача — реализовать `raccpack-core` с нуля по фазам 0–11 из `raccpack-agent-prompt.md`.

## Роль агента — Orchestrator (ОБЯЗАТЕЛЬНО)

`raccpack-agent-workflow.md` — **обязателен к выполнению**. Ты — **главный агент (Orchestrator)**, а не исполнитель:

- **Не выполняешь сам**: исследование/инвентаризацию, написание кода, тестов, документации.
- **Делаешь только**: детальный план работ → делегирование задач субагентам (Dev / Test / Docs) → **строгая приёмка** по чеклисту критерия готовности этапа либо rework-билет с конкретными замечаниями → ведение `WORKLOG.md`.
- На каждый этап: **Dev** (реализация) + **Test** (тесты) параллельно; **Docs** — только после зелёного FINAL.
- Не делегируй несколько этапов одной задачей, не принимай «на глаз» и не принимай этап с красными тестами.
- Анти-паттерны (запрещено): писать продакшн-код самому «чтобы быстрее»; «сделай фазы 1–3 целиком»; Docs до FINAL; «тесты потом».

## Карта документов (знания)

| Файл | Что даёт | Когда читать |
|------|----------|--------------|
| `raccpack-agent-prompt.md` | **Главный ТЗ**: роли, жёсткие правила, фазы 0–11 с этапами, критерии готовности, формат отчёта | всегда, это source of truth задач |
| `raccpack-agent-workflow.md` | **ОБЯЗАТЕЛЕН к выполнению**: как организована работа — Orchestrator (планирует/делегирует/принимает), Dev / Test / Docs субагенты, шаблоны поручений, rework-билеты, анти-паттерны | перед делегированием/приёмкой |
| `raccpack-architecture-vision.md` | Слои: core / facade / UI; потоки данных; границы доверия; контракты DTO | для решений по архитектуре |
| `raccpack-facade-and-den.md` | Конкретные сигнатуры facade (`sniff/dig/stash/rinse/pack/raid`), типы (`AppContext`, `ProgressSink`, `*Options`, `*Result`), структура den, manifest JSON | при работе с use-cases и отчётами |
| `raccpack-roadmap-v1.md` | Версии MVP→1.0.0, фазы M/A/B/R/S, жёсткие зависимости вех | контекст приоритетов |
| `docs/mvp/m{1..4}/*` | Детальные спекуляции этапов MVP (по файлу на этап) | **только по явной ссылке от человека** (правило «docs/ не читать без ссылки») |
| `WORKLOG.md` | Журнал статусов этапов (создаётся в фазе 0.1) | после каждого этапа обновлять |

## Жёсткие правила (не нарушать)

- **Один этап = одна узкая задача.** Не смешивай рефакторинг + тесты + документацию.
- `docs/` не читать, кроме документа, на который человек дал ссылку перед этапом.
- Код **компилируется в рамках этапа** (или `TODO` только для промежуточного каркаса).
- Не менять семантику first-match таблиц `PATTERNS`/`CONTENT_MARKERS` без тестов-инвариантов.
- **Без `unwrap()` в production-коде.**
- **Без `anyhow::Error` / `Box<dyn Error>` в public API**; только `Error`/`ConfigError` (строгие типы).
- Секреты (raw values, passphrase) **не** в `Display` ошибок, логах, отчётах по умолчанию. Masked по умолчанию; reveal только по явному opt-in.
- Стиль: rustfmt-совместимо, как окружающий код. Без лишних комментариев.
- Публичный API меняешь → обнови re-exports в `lib.rs` + отметь breaking change в отчёте.
- `WalkDir` в production → с `follow_links(false)`.
- SensitiveRisk меняется только через severity API.
- После каждого этапа — отчёт по шаблону (см. ниже). Критерий не выполнен → следующий этап не начинать.

## Стартовые условия

Код удалён — пишем **с нуля**. Наследуемого «уже сделано» нет; этапы 1–11 реализуются заново, каждый с компиляцией и тестами по своему критерию. Инварианты PATTERNS / CONTENT_MARKERS фиксируются тестами по мере появления кода.

## Фазы и этапы (порядок обязателен)

```
0.1 Inventory → 0.2 Baseline
→ 1.1 Group enum → 1.2 EnabledGroups (enumset/bitflags) → 1.3 config groups
→ 2.1 fingerprint (blake3/siphash, НЕ DefaultHasher/fnv) → 2.2 masked_value (без длинного prefix)
→ 3.1 единый deny/allow helper по именам → 3.2 опциональный content-scan при pack
→ 4.1 trait GitClient + ProcessGitClient → 4.2 MockGitClient
→ 5.1 thread pool из advanced.parallel_jobs
→ 6.1 цепочка migrate_vN_to_V → 6.2 расширить validate()
→ 7.1 WalkEvent/WalkVisitor design → 7.2 WalkSession минимальный → 7.3 cache через session → 7.4 sensitive через session → (7.5 scanner, опционально)
→ 8.1 тесты порядка whitelist-имён → 8.2 тесты shadowing content-markers
→ 9.1 аудит pub use → 9.2 missing_docs
→ 10.1 zeroize passphrase
→ 11.1 cargo test → 11.2 fmt+clippy → 11.3 CHANGES + MIGRATION
```

Не перескакивать фазу 0. Не начинать фазу 7, пока 1–3 не стабильны.
Можно параллелить (из workflow): 1.x ∥ 2.x · 2.x ∥ 3.x · 4.x ∥ 5.1 · 7.x ∥ 8.x. Если оба этапа правят один файл — строго последовательно.

## Формат отчёта этапа (обязательный)

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
- [x]/[ ] <текст из промпта>
```

## Архитектура (кратко)

- **core** — вся бизнес-логика: config, scan, detect, secrets, clean, archive, git (за `GitClient`), cache, report, policy/skip. Не знает про ratatui/tauri/react.
- **facade** (use-cases): `sniff` → `dig` → `stash`(age) → `rinse` → `pack` → `raid`(оркестрация). Вход: `AppContext{config, paths, mode, exit_policy}`; прогресс через `ProgressSink`; dry-run через `RunMode`.
- **Данные**: report DTO serde-дружелюбные, masked secrets; ошибки — один `Error` + `ConfigError` с `suggestion()`. UI не парсит тексты ошибок.
- **Границы доверия**: raw secret только в core и только на время encrypt; CLI/TUI могут просить reveal; React — только DTO; den — age-файлы, perms `0700`/`0600`.
- **Den layout**: `manifests/{yyyy}/{mm}/…json`, `secrets/…/*.age`, `packs/…/*.tar.zst`, `staging/{short_id}`, `.den-version`. Имя: `{project_slug}__{utc_timestamp}[__{short_id}]`. Manifest paths — relative to den root.
- **Exit codes CLI**: 0 ok, 1 ошибка, 2 найдены CRITICAL (политика). `--json` печатает serde-результат.

## Воркфлоу (роли)

- **Orchestrator** (главный агент, ты): читает ТЗ, строит план этапов, на каждый этап делегирует **Dev** и **Test** параллельно по шаблонам из `raccpack-agent-workflow.md`, строго принимает по чеклисту критерия готовности или возвращает rework-билет с конкретными замечаниями (лимит 3 попытки), ведёт `WORKLOG.md`, после зелёного FINAL делегирует **Docs**. **Не** пишет продакшн-код/тесты/документацию сам. **После каждого принятого этапа**: PR stage-ветки **в `dev`** → merge (squash) → удалить stage-ветку → стартовать следующий этап новой веткой от обновлённого `dev`.
- **Dev** → реализация этапа (отчёт по формату «Этап X.Y»). **Test** → тесты того же этапа **параллельно**, по спецификации ТЗ.
- **Docs** → `CHANGES.md` / `MIGRATION.md` и остальное **только после** зелёного FINAL checklist.
- Приёмка строгая: критерий из промпта, нет запрещённых паттернов, только согласованные файлы, отчёт заполнен, breaking-пометка при смене public API.
- Анти-паттерны (запрещено): закрывать этап с «тесты потом», делегировать «сделай фазы 1–3 целиком», Docs до зелёного FINAL, принимать этап с красными тестами.
- FINAL checklist (делает Orchestrator сам): сборка зелёная, запреты не нарушены, sensitive-тесты зелёные, WORKLOG полный.

## Команды проверки

```bash
cargo test -p raccpack-core    # или cargo test — если crate есть; иначе зафиксировать в WORKLOG
cargo fmt
cargo clippy -- -D warnings
```

Текущее состояние: в этой папке пока **нет дерева crate** (только документы: 5 корневых md + спекуляции `docs/mvp/`) — код удалён, пишем с чистого листа. Фаза 0.1 должна зафиксировать пустой baseline и определить, где разворачивать crate (`raccpack-core`).

## Git workflow (обязательный)

- `main` — защищённая, только релизы вех (PR + review 1, без force push, без deletions).
- `dev` — основная рабочая ветка, вся разработка мержится сюда (PR required, без force push, без удаления ветки).
- Stage/feature-ветки — короткоживущие, **от `dev`**. Имя по roadmap: `{phase}-{short-slug}` в kebab-case (`m1-workspace-core`, `m2-sniff`, `m3-dig`, `m4-pack-den`, `a1-stash-age`, `a2-rinse`, `a3-raid`).
- **После КАЖДОГО закрытого этапа (M1.1, M1.2, …)** Orchestrator делает PR stage-ветки **в `dev`** → merge (squash) → **удаляет** stage-ветку. Не копить несколько этапов в одной ветке; следующий этап стартует новой stage-веткой от обновлённого `dev`.
- Merge `dev → main` + `git tag` + GitHub Release — **только** на вехах: MVP `v0.1.0`, Alpha `v0.3.0`, Beta `v0.5.0`, RC `v0.9.0`, Stable `v1.0.0`. Между вехами в `main` ничего не мержить.
- Hotfix/blocker после релиза: ветка от `main`/tag → PR в `main` → backport в `dev`.
- Merge method фиксирован: **squash**; stage-ветки удаляются при merge.
- Branch protection на GitHub **пока не включена** (приватные репозитории требуют GitHub Pro) — правила выше соблюдаются вручную через PR. После апгрейда настроить `main` (PR + 1 approval, no force push, no deletions) и `dev` (PR, no force push, no deletions).
- Детали — в `README.md` (раздел Git workflow).

## Полезные ссылки на API (из спекуляций)

- `SensitiveGroup` enum (1.1) из `KNOWN_SENSITIVE_GROUPS`; `from_str`/`as_str` 1:1.
- `EnabledGroups`: `all()`, `from_config(&SensitiveConfig)`, `is_enabled(Group)`/`is_enabled_str(&str)`.
- `GitClient` trait: `available`, `find_repo_root`, `classify_file`→`GitFileStatus`, `analyze`→`Option<GitState>`.
- `WalkEvent`/`WalkVisitor`/`WalkControl` в `walk_session.rs` (7.x); `WalkSession::run(root, policy, max_depth, visitor)`.
- Zeroize: passphrase как `SecretString`/`Zeroizing<String>` на время encrypt/decrypt.
