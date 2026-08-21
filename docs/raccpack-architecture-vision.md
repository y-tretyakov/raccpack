# Raccpack — видение архитектуры

**Статус:** vision / target architecture (обновлено с учётом атомарной оркестрации, композитных детекторов и эфемерной верификации секретов)  
**Контекст:** развитие поверх текущего `dev` (MVP 0.1.0 закрыт; Alpha: stash + rinse доступны, raid в работе).  
**Не цель документа:** API каждой функции или UI-макеты. Цель — **что за системы, границы, потоки данных и кто чем владеет**.

---

## 1. Продукт в одном абзаце

Пользователь указывает **корневую папку с проектами** и **папку вывода (den)**. Система обходит дерево, определяет стек каждого проекта (включая гибридные и монорепозитории), находит секреты (ключи, токены, connection strings, credential-файлы), **убирает их из рабочих копий** в зашифрованные age-архивы, **чистит** мусор сборки по релевантным поддеревьям, **упаковывает** каждый проект отдельно и складывает артефакты в den. Управление — через CLI, терминальный TUI или Desktop (Tauri).

**Инварианты безопасности:**

- Сырые секреты не пишутся в логи, отчёты по умолчанию и IPC без явного opt-in.
- Шифрование секретов — **age** (passphrase или recipients); 7z и прочее — опциональные backend’ы, не ядро доверия.
- «Очистка» и «упаковка» по умолчанию **dry-run / confirm**, пока пользователь не подтвердит destructive-режим.
- Оркестрация `raid` по умолчанию **атомарна**: либо полный успех, либо откат к исходному состоянию (нет orphan-артефактов).
- Сырые значения секретов могут быть показаны только через **эфемерный reveal** (opt-in, zeroize, без persistence).

---

## 2. Высокоуровневая схема

```
┌─────────────────────────────────────────────────────────────────┐
│                        Presentation                              │
│  ┌──────────┐   ┌──────────────┐   ┌──────────────────────────┐ │
│  │   CLI    │   │  TUI         │   │  Desktop (Tauri)         │ │
│  │  clap    │   │  Ratatui     │   │  React + Zustand         │ │
│  │          │   │  (crossterm) │   │       │                  │ │
│  │          │   │              │   │       ▼                  │ │
│  │          │   │              │   │  BFF (Rust, in-process   │ │
│  │          │   │              │   │   or sidecar commands)   │ │
│  └────┬─────┘   └──────┬───────┘   └──────────┬───────────────┘ │
│       │                │                      │                 │
│       └────────────────┼──────────────────────┘                 │
│                        ▼                                        │
│              Application services (facade)                        │
│   sniff / dig / stash / rinse / pack / raid / report / reveal    │
└────────────────────────┬────────────────────────────────────────┘
                         ▼
┌────────────────────────────────────────────────────────────────┐
│                     raccpack-core (library)                      │
│  domain │ scan │ detect │ secrets │ clean │ archive │ config     │
│  policy │ git  │ cache  │ report  │ error │ skip │ orchestration │
│  wal    │ rollback                                                │
└────────┬───────────────┬─────────────────────┬──────────────────┘
         ▼               ▼                     ▼
   filesystem         git (opt)            age / tar+zstd
   (walk, IO)         subprocess           (encrypt, pack)
```

**Правило:** вся **бизнес-логика** живёт в `raccpack-core`. CLI / TUI / Desktop **не** дублируют эвристики секретов, правила skip, логику детекции стека и формат отчётов — только adapt UI ↔ core API.

---

## 3. Слои и ответственность

### 3.1. `raccpack-core` — ядро

Чистая library-crate (workspace-crate без binary).

| Подсистема | Ответственность |
|------------|-----------------|
| **config** | Схема TOML, load/validate/migrate, пути den / scan root, feature flags (groups секретов, dry-run, parallel_jobs, detect_mode, orchestration_mode). |
| **scan** | Обход дерева, политики skip, кандидаты проектов (markers, `.git`, wrapper-логика). |
| **detect** | Языки и фреймворки → `Stack` / `Project`. Поддерживает **PriorityTable** (legacy) и **CompositeDag** (монорепо / гибриды). |
| **secrets** | Filename patterns, content markers, heuristics, risk model, repeated secrets, git status файла. Masked-by-default + ephemeral reveal. |
| **clean (rinse)** | Известный «мусор» (`node_modules`, `target`, caches…) по стратегиям; учитывает DAG стека (чистит только релевантные поддеревья). |
| **archive** | Pack проекта (tar+zstd), stash секретов (age), перемещение в den. |
| **orchestration** | `raid`: фазы, progress, **WAL**, атомарный commit через rename, rollback. |
| **git** | Опционально: dirty state, tracked/untracked/ignored для sensitive files. За интерфейсом `GitClient`. |
| **cache** | Кэш результатов sniff (версия схемы, инвалидация). |
| **report** | Стабильные DTO: `ScanReport`, `SensitiveFile`, `RaidResult`… serde-friendly. |
| **policy / skip** | Единые правила «что не обходить / что hard-deny в pack». |

**Ядро не знает** про Ratatui, Tauri, React, stdin prompts (кроме возврата данных для UI). Прогресс длинных операций — через callback / channel / `Iterator` событий, который UI подписывает.

### 3.2. Application facade (часть core или тонкий `raccpack-app`)

Операции уровня use-case:

| Операция | Смысл |
|----------|--------|
| **sniff** | Найти проекты + стек (плоский или дерево/DAG) + размеры (+ git summary). |
| **dig** | Найти секреты в root или в одном проекте (masked). |
| **stash** | Вынести секреты в age-архив, опционально удалить/заменить в дереве. |
| **rinse** | Удалить trash dirs по стратегиям (с учётом DAG). |
| **pack** | Упаковать проект без секретов/мусора в архив. |
| **raid** | Оркестрация: stash → rinse → pack → commit в den. По умолчанию **атомарно** (WAL + rollback). |
| **reveal** | Эфемерный просмотр сырого значения конкретной находки (opt-in, zeroize). |
| **report** | Экспорт JSON/текст для CI. |

Facade принимает `RaccConfig` + paths + `dry_run` + progress sink + orchestration/detect mode. Возвращает typed `Result`.

### 3.3. CLI

- Парсинг args (`clap`), глобальные флаги: `--config`, `--dry-run`, `--json`, `--den`, `--root`.
- Вызов facade, human или JSON output.
- Exit codes: 0 ok, 1 ошибка (в т.ч. после успешного rollback), 2 найдены CRITICAL секреты (политика настраивается).
- Без собственного «ума» про паттерны секретов.
- Интерактивный reveal через защищённый терминальный ввод (не пишет в history).

### 3.4. TUI (Ratatui)

- Интерактивный обзор дерева проектов, фильтры risk, подтверждение raid.
- Подписка на progress events от `raid` / long scan.
- Те же facade-вызовы, что CLI; состояние экрана — локально в TUI.
- Не ходит в FS в обход core.
- Reveal — модальный безопасный просмотр с немедленным стиранием.

### 3.5. Desktop (Tauri + React + Zustand + BFF)

```
┌──────────── React (UI) ────────────┐
│  Zustand stores: scan, secrets,    │
│  raid progress, settings           │
│  (только masked DTO)               │
└──────────────┬─────────────────────┘
               │ invoke / events
┌──────────────▼─────────────────────┐
│  Tauri commands = BFF layer        │
│  (Rust, рядом с core)              │
│  - validate paths                  │
│  - map DTO ↔ frontend types        │
│  - spawn long jobs, emit events    │
│  - reveal_secret_ephemeral         │
│  - never expose raw secrets by def │
└──────────────┬─────────────────────┘
               │
               ▼
         raccpack-core
```

**BFF (Backend-for-Frontend)** — **Rust-команды Tauri**:

- Тонкая адаптация: пути, ошибки → UI-friendly messages, streaming progress через Tauri events.
- React **не** содержит эвристик секретов и не читает произвольные файлы с диска напрямую.
- Zustand хранит **уже отфильтрованные** отчёты (masked secrets), не raw values.
- Reveal: сырая строка передаётся напрямую в изолированный React-компонент, минуя глобальный store; при закрытии модалки — zeroize на стороне Rust.

---

## 4. Главный пользовательский поток (happy path)

```
1. Выбрать scan_root + den_dir
2. sniff(scan_root)  →  список Project + Stack (или дерево/DAG) + size
3. dig(scan_root | project)  →  SensitiveFile[] + RepeatedSecret[] (masked)
4. [UI] пользователь смотрит risk, при необходимости reveal конкретной находки
5. raid(project, den, dry_run=false, mode=Atomic):
     a. создать staging/{raid_id}/ + WAL
     b. stash  →  age archive во staging
     c. rinse  →  удаление trash (с учётом DAG)
     d. pack   →  project archive во staging
     e. commit →  атомарный rename в den/secrets, den/packs, den/manifests
     f. при любой ошибке → rollback по WAL, очистка staging
6. Отчёт: что убрано, куда легли архивы, success, rolled_back?
```

Пакетный режим CLI: `racc raid --root ~/DEV/PROJS --den ~/.raccpack/den --yes`.

---

## 5. Потоки данных

### 5.1. Внутрь системы

| Вход | Кто читает | Куда |
|------|------------|------|
| Файловая система projects | core scan/secrets/clean/pack | Report DTO, archives |
| `config.toml` / env `RACCPACK_*` | core config | RaccConfig |
| Passphrase / age recipients | только archive/stash (memory zeroize) | age encrypt |
| Git (optional) | GitClient | GitState / GitFileStatus |

### 5.2. Наружу

| Выход | Формат | Кто показывает |
|-------|--------|----------------|
| ScanReport, dig results | struct / JSON | CLI, TUI, Desktop |
| Progress events | phase, %, message | TUI, Desktop, CLI spinner |
| Secret archives | `.age` | den |
| Project archives | `.tar.zst` | den |
| Logs | tracing, **без raw secrets** | все frontends |
| Ephemeral reveal | одноразовый in-memory | CLI / TUI / Desktop (opt-in) |

### 5.3. Секреты

```
disk file  →  secrets engine (match)  →  Risk + masked preview
                 │
                 ├─ report (masked / hash only)
                 ├─ stash path: read bytes → age encrypt → write archive
                 │                → optional delete/redact source
                 └─ reveal (opt-in): EphemeralSecret → UI → Drop/zeroize
```

Frontend получает `masked`, `value_hash`, `path`, `risk`. Raw — только внутри core на время encrypt/reveal и только при explicit opt-in.

---

## 6. Оркестрация raid и атомарность

### 6.1. Проблема fail-fast без отката

При ошибке одной фазы (например, pack после успешного stash) классический fail-fast оставляет orphan-артефакты в `staging/` / `secrets/`. Это перекладывает уборку на пользователя и нарушает обещание надёжности.

### 6.2. Целевое поведение (Atomic)

1. Весь `raid` работает в одном `staging/{raid_id}/`.
2. Каждый побочный эффект записывается в **Write-Ahead Log** (WAL) **до** выполнения.
3. Финальные пути в den появляются **только** через атомарный `rename` при 100 % успехе всех фаз.
4. При любом `Err`:
   - WAL читается в обратном порядке;
   - действия откатываются;
   - `staging/{raid_id}/` удаляется.
5. После rollback система гарантированно в исходном состоянии.

### 6.3. Режимы

| Режим | Поведение | Когда |
|-------|-----------|--------|
| **Atomic** (default) | WAL + rollback | production, `--yes` |
| **FailFast** | прерывание без отката (legacy/debug) | `--fail-fast` |

Dry-run не создаёт WAL и не трогает FS.

### 6.4. Resume

Полноценный resume (продолжение с места остановки без повторного шифрования) **вне scope** первой реализации атомарности. WAL достаточно богат, чтобы добавить resume позже как опциональный режим.

---

## 7. Детекция стека: от PriorityTable к CompositeDag

### 7.1. Текущее ограничение

«Один язык ≈ один файл + статическая таблица приоритетов» хорошо работает для простых проектов, но слепнет на монорепозиториях и гибридах (Rust backend + React frontend). Контекст вложенности теряется; rinse может оставить гигабайты нерелевантного мусора или удалить не то.

### 7.2. Целевая модель

- Детекторы остаются модульными (один язык / экосистема ≈ один модуль).
- Появляется **WorkspaceDetector / CompositeDetector**:
  - опрашивает все модули реестра;
  - строит **DAG** технологий внутри дерева;
  - не выбирает «одного победителя», а сливает экспертные мнения в богатое дерево проекта.
- Фаза разрешения конфликтов: вложенность и confidence учитываются явно.

### 7.3. Режимы

| Режим | Описание | Default |
|-------|----------|---------|
| `PriorityTable` | текущее поведение (обратная совместимость) | да, до стабилизации DAG |
| `CompositeDag` | дерево/DAG стека | включается конфигом / флагом |

`rinse` и `pack` выигрывают от DAG: чистят и упаковывают только релевантные поддеревья.

---

## 8. Эфемерная верификация секретов (Safe Reveal)

### 8.1. Проблема masked-by-default

Абсолютное маскирование без безопасного просмотра создаёт слепую зону: пользователь не может отличить false-positive (тестовый токен) от реальной утечки. Итог — либо игнор шума, либо случайное удаление нужного кода.

### 8.2. Целевое поведение

- По умолчанию всё masked.
- Opt-in **ephemeral reveal**:
  - CLI: интерактивный флаг / подкоманда, защищённый терминальный ввод, стирание из history.
  - Desktop: IPC `reveal_secret_ephemeral` → изолированный React-компонент (минуя Zustand) → zeroize при закрытии модалки.
  - TUI: аналогичный безопасный modal.
- Сырое значение **никогда** не попадает в глобальный store, логи, JSON-отчёты, clipboard (без отдельного явного подтверждения).
- Опциональный audit-log факта запроса (без значения).

### 8.3. Границы доверия (уточнение)

| Зона | Доверие | Правило |
|------|---------|---------|
| **core** | highest | Единственное место, где raw secret в памяти допустим; zeroize после use. |
| **CLI/TUI** | high (local user) | Могут запросить reveal; по умолчанию masked. |
| **Desktop renderer (React)** | lower | Не получает raw в store; только через изолированный ephemeral path. |
| **den на диске** | user-managed | age-файлы; права FS на пользователе. |
| **CI mode** | machine | JSON report + fail on CRITICAL; обычно без reveal. |

---

## 9. Workspace (рекомендуемая структура репо)

```
raccpack/
  Cargo.toml                 # workspace
  crates/
    raccpack-core/           # domain + use-cases (no UI)
    raccpack-cli/            # binary: clap → core
    raccpack-tui/            # binary: ratatui → core
    raccpack-tauri/          # Tauri app (Rust side = BFF)
  apps/
    desktop-ui/              # React + Zustand (Vite)
  docs/
    architecture-vision.md   # этот документ
    roadmap-v1.md            # дорожная карта
    ...
  wiki/                      # пользовательская документация (VitePress)
```

Зависимости:

- `cli` / `tui` / `tauri` → `raccpack-core`
- `desktop-ui` → только через Tauri IPC
- `core` **не** зависит от clap / ratatui / tauri / react

---

## 10. Контракты между UI и core

### 10.1. Стабильные DTO (serde)

- `ScanReport { root, projects: [Project], total_size_bytes, schema_version }`
- `Project { path, name, stack, stack_tree?, size_bytes, is_git_repo, git_state?, … }`
- `SensitiveFile { path, risk, git_status, content_match?, value_hash, masked }`
- `RaidProgress { phase, progress, message, phase_complete }`
- `RaidResult { stages, success, rolled_back, paths_to_artefacts }`
- `EphemeralSecret` — не serde в отчёты; только in-memory, `Drop + zeroize`

### 10.2. Progress

UI подписывается → core шлёт события по фазам → UI не блокирует event loop.

### 10.3. Ошибки

Один тип `raccpack_core::Error` (+ `ConfigError`) с `suggestion()` для UX.  
UI мапит в строки; не парсит текст ошибок regex’ом.

---

## 11. Конфигурация и пути

```
RACCPACK_CONFIG  →  override config path
~/.config/raccpack/config.toml  →  default (XDG)

scan_root   — где лежат проекты (вход)
den_dir     — куда складывать архивы (выход)

[detect]
mode = "priority_table" | "composite_dag"

[orchestration]
mode = "atomic" | "fail_fast"
```

UI (все три) умеют override через flags / settings; core резолвит relative paths от HOME.

---

## 12. Расширяемость

| Что | Как |
|-----|-----|
| Новые языки | markers + detect rules / модули в core |
| Новые secret patterns | groups + tables в secrets; toggle в config |
| Другой encrypt backend | trait `SecretVault` / `EncryptionBackend` |
| Другой UI | только новый frontend на том же facade |
| CI | CLI `--json` + exit policy |
| Атомарность | WAL + rollback в orchestration (уже в core) |

Плагины сторонних pattern-pack’ов до 1.0.0 **не** нужны.

---

## 13. Нефункциональные требования (архитектурные)

- **Безопасность:** masked by default; age; zeroize; no secrets in traces; ephemeral reveal only.
- **Атомарность:** raid либо полностью успешен, либо оставляет систему чистой.
- **Производительность:** один проход walk где возможно; parallel sniff с лимитом jobs; cache sniff.
- **Предсказуемость:** dry-run по умолчанию для destructive ops; явный `--yes` / confirm в UI.
- **Тестируемость:** core без UI; `GitClient` mock; tempfile fixtures; orphan/rollback regression suite.
- **Портативность:** Linux primary; macOS/Windows — через те же crate.

---

## 14. Что сознательно вне scope v1 / до 1.0.0

- Облачный sync den
- Автоматический PR «удали секреты»
- Полноценный secret manager (Vault/KMS) как primary store
- HTTP multi-user server
- Редактирование файлов проекта «умным» redact
- Resume после сбоя raid (можно добавить позже поверх WAL)
- Плагины сторонних детекторов

---

## 15. Карта решений (коротко)

| Вопрос | Решение |
|--------|---------|
| Где бизнес-логика? | Только `raccpack-core` |
| Кто оркестрирует raid? | Facade + orchestration в core; UI только триггерит |
| Как обеспечить отсутствие orphan? | Staging + WAL + atomic rename + rollback |
| Как определять стек в monorepo? | CompositeDag (опционально), PriorityTable для совместимости |
| Как не светить секреты? | DTO masked; reveal opt-in ephemeral; zeroize в core |
| Как Desktop говорит с core? | Tauri commands = BFF, React ↔ IPC |
| Где state UI? | TUI local; Desktop Zustand (masked only); CLI stateless |
| Куда артефакты? | `den_dir`, структура подпроектов/даты — policy core |

---

## 16. Следующий шаг

1. Зафиксировать обновлённый roadmap (см. `raccpack-roadmap-v1.md`).
2. Для каждого нового/изменённого этапа написать отдельную спеку в `docs/`.
3. Начать с атомарности внутри A3 (raid) — это закрывает самый острый риск надёжности.

Документ можно уточнять, но **правило границы core / UI** и **инварианты безопасности + атомарности** менять не стоит.
