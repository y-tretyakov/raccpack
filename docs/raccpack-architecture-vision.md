# Raccpack — видение архитектуры

**Статус:** vision / target architecture  
**Контекст:** зелёный старт на основе идей `raccpack-core` (сканирование проектов, секреты, очистка, упаковка в den).  
**Не цель документа:** API каждой функции или UI-макеты. Цель — **что за системы, границы, потоки данных и кто чем владеет**.

---

## 1. Продукт в одном абзаце

Пользователь указывает **корневую папку с проектами** и **папку вывода (den)**. Система обходит дерево, определяет стек каждого проекта, находит секреты (ключи, токены, connection strings, credential-файлы), **убирает их из рабочих копий** в зашифрованные age-архивы, **чистит** мусор сборки, **упаковывает** каждый проект отдельно и складывает артефакты в den. Управление — через CLI, терминальный TUI или Desktop (Tauri).

**Инварианты безопасности:**

- Сырые секреты не пишутся в логи, отчёты по умолчанию и IPC без явного opt-in.
- Шифрование секретов — **age** (passphrase или recipients); 7z и прочее — опциональные backend’ы, не ядро доверия.
- «Очистка» и «упаковка» по умолчанию **dry-run / confirm**, пока пользователь не подтвердит destructive-режим.

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
│         snif / dig / stash / rinse / pack / raid / report         │
└────────────────────────┬────────────────────────────────────────┘
                         ▼
┌────────────────────────────────────────────────────────────────┐
│                     raccpack-core (library)                      │
│  domain │ scan │ detect │ secrets │ clean │ archive │ config     │
│  policy │ git  │ cache  │ report  │ error │ skip                 │
└────────┬───────────────┬─────────────────────┬──────────────────┘
         ▼               ▼                     ▼
   filesystem         git (opt)            age / tar+zstd
   (walk, IO)         subprocess           (encrypt, pack)
```

**Правило:** вся **бизнес-логика** живёт в `raccpack-core`. CLI / TUI / Desktop **не** дублируют эвристики секретов, правила skip и формат отчётов — только adapt UI ↔ core API.

---

## 3. Слои и ответственность

### 3.1. `raccpack-core` — ядро

Чистая library-crate (или workspace-crate без binary).

| Подсистема | Ответственность |
|------------|-----------------|
| **config** | Схема TOML, load/validate/migrate, пути den / scan root, feature flags (groups секретов, dry-run, parallel_jobs). |
| **scan** | Обход дерева, политики skip, кандидаты проектов (markers, `.git`, wrapper-логика). |
| **detect** | Языки и фреймворки → `Stack` / `Project`. |
| **secrets** | Filename patterns, content markers, heuristics, risk model, repeated secrets, git status файла. |
| **clean (rinse)** | Известный «мусор» (`node_modules`, `target`, caches…) по стратегиям и конфигу. |
| **archive** | Pack проекта (tar+zstd и т.п.), stash секретов (age), перемещение в den. |
| **git** | Опционально: dirty state, tracked/untracked/ignored для sensitive files. За интерфейсом `GitClient`. |
| **cache** | Кэш результатов sniff (версия схемы, инвалидация). |
| **report** | Стабильные DTO: `ScanReport`, `SensitiveFile`, `RaidResult`… serde-friendly. |
| **policy / skip** | Единые правила «что не обходить / что hard-deny в pack». |

**Ядро не знает** про Ratatui, Tauri, React, stdin prompts (кроме возврата данных для UI). Прогресс длинных операций — через callback / channel / `Iterator` событий, который UI подписывает.

### 3.2. Application facade (может быть частью core или тонкий `raccpack-app`)

Операции уровня use-case:

| Операция | Смысл |
|----------|--------|
| **sniff** | Найти проекты + стек + размеры (+ git summary). |
| **dig** | Найти секреты в root или в одном проекте. |
| **stash** | Вынести секреты в age-архив, опционально удалить/заменить в дереве. |
| **rinse** | Удалить trash dirs по стратегиям. |
| **pack** | Упаковать проект без секретов/мусора в архив. |
| **raid** | Оркестрация: stash → rinse → pack → move to den (с фазами и progress). |
| **report** | Экспорт JSON/текст для CI. |

Facade принимает `RaccConfig` + paths + `dry_run` + progress sink. Возвращает typed `Result`.

### 3.3. CLI

- Парсинг args (`clap`), глобальные флаги: `--config`, `--dry-run`, `--json`, `--den`, `--root`.
- Вызов facade, human или JSON output.
- Exit codes: 0 ok, 1 ошибка, 2 найдены CRITICAL секреты (политика настраивается).
- Без собственного «ума» про паттерны секретов.

### 3.4. TUI (Ratatui)

- Интерактивный обзор дерева проектов, фильтры risk, подтверждение raid.
- Подписка на progress events от `raid` / long scan.
- Те же facade-вызовы, что CLI; состояние экрана — локально в TUI.
- Не ходит в FS в обход core.

### 3.5. Desktop (Tauri + React + Zustand + BFF)

```
┌──────────── React (UI) ────────────┐
│  Zustand stores: scan, secrets,    │
│  raid progress, settings           │
└──────────────┬─────────────────────┘
               │ invoke / events
┌──────────────▼─────────────────────┐
│  Tauri commands = BFF layer        │
│  (Rust, рядом с core)              │
│  - validate paths                  │
│  - map DTO ↔ frontend types        │
│  - spawn long jobs, emit events    │
│  - never expose raw secrets by def │
└──────────────┬─────────────────────┘
               │
               ▼
         raccpack-core
```

**BFF (Backend-for-Frontend)** здесь — **Rust-команды Tauri**, не отдельный HTTP-сервер по умолчанию:

- Тонкая адаптация: пути, ошибки → UI-friendly messages, streaming progress через Tauri events.
- React **не** содержит эвристик секретов и не читает произвольные файлы с диска напрямую (только через commands).
- Zustand хранит **уже отфильтрованные** отчёты (masked secrets), не raw values.

Опционально позже: headless BFF (HTTP localhost) для remote UI — не требуется в v1.

---

## 4. Главный пользовательский поток (happy path)

```
1. Выбрать scan_root + den_dir
2. sniff(scan_root)  →  список Project + Stack + size
3. dig(scan_root | project)  →  SensitiveFile[] + RepeatedSecret[]
4. [UI] пользователь смотрит risk, git status, подтверждает
5. raid(project, den, dry_run=false):
     a. stash  →  age archive с секретами в den/secrets/…
     b. rinse  →  удаление trash
     c. pack   →  project archive без секретов
     d. move   →  финальное размещение в den
6. Отчёт: что убрано, куда легли архивы, success per stage
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
| Secret archives | `.age` (или chosen backend) | den |
| Project archives | `.tar.zst` | den |
| Logs | tracing, **без raw secrets** | все frontends |

### 5.3. Секреты

```
disk file  →  secrets engine (match)  →  Risk + masked preview
                 │
                 ├─ report (masked / hash only)
                 └─ stash path: read bytes → age encrypt → write archive
                                  → optional delete/redact source
```

Frontend получает `masked`, `value_hash`, `path`, `risk`. Raw — только внутри core на время encrypt и только при explicit reveal в trusted UI flow.

---

## 6. Границы доверия

| Зона | Доверие | Правило |
|------|---------|---------|
| **core** | highest | Единственное место, где raw secret в памяти допустим; zeroize после use. |
| **CLI/TUI** | high (local user) | Могут запросить reveal; по умолчанию masked. |
| **Desktop renderer (React)** | lower | Не получает raw; только DTO через BFF. XSS/plugins не должны читать passphrase из store. |
| **den на диске** | user-managed | age-файлы; права FS на пользователе. |
| **CI mode** | machine | JSON report + fail on CRITICAL; обычно без reveal. |

---

## 7. Workspace (рекомендуемая структура репо)

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
    ...
```

Зависимости:

- `cli` / `tui` / `tauri` → `raccpack-core`
- `desktop-ui` → только через Tauri IPC, не через npm-копию логики
- `core` **не** зависит от clap / ratatui / tauri / react

---

## 8. Контракты между UI и core

### 8.1. Стабильные DTO (serde)

Примеры (имена ориентировочные):

- `ScanReport { root, projects: [Project], total_size_bytes }`
- `Project { path, name, stack, size_bytes, is_git_repo, git_state?, … }`
- `SensitiveFile { path, risk, git_status, content_match? }`
- `RaidProgress { phase, progress, message, phase_complete }`
- `RaidResult { stages, success, paths to artefacts }`

Версионирование: поле `schema_version` в JSON-отчётах для CI.

### 8.2. Progress

```text
UI подписывается → core шлёт события по фазам raid/sniff
                 → UI не блокирует event loop (async или thread + channel)
```

CLI: progress bar / тихие логи.  
TUI: redraw по event.  
Desktop: `app.emit("raid-progress", payload)`.

### 8.3. Ошибки

Один тип `raccpack_core::Error` (+ `ConfigError`) с `suggestion()` для UX.  
UI мапит в строки; не парсит текст ошибок regex’ом.

---

## 9. Ключевые сценарии взаимодействия

### CLI only

```
user → clap → facade.sniff/dig/raid → stdout/JSON → exit code
```

### TUI

```
user keypress → tui state machine → facade (background thread)
             ← progress + report ← core
             → panels update
```

### Desktop

```
React action → Zustand → invoke("raid", { root, den, dryRun })
                      → Tauri command (BFF)
                      → core::raid + emit progress events
                      → Zustand listener updates UI
                      → result in store (masked)
```

Passphrase: отдельный secure prompt (Tauri dialog / system), в React store не класть длинноживущей строкой; передать в command и забыть.

---

## 10. Конфигурация и пути

```
RACCPACK_CONFIG  →  override config path
~/.config/raccpack/config.toml  →  default (XDG)

scan_root   — где лежат проекты (вход)
den_dir     — куда складывать архивы (выход)
```

UI (все три) умеют override через flags / settings screen; core резолвит relative paths от HOME и **ошибется**, если HOME/XDG недоступны и путь не абсолютный.

---

## 11. Расширяемость

| Что | Как |
|-----|-----|
| Новые языки | markers + detect rules в core |
| Новые secret patterns | groups + tables в secrets; toggle в config |
| Другой encrypt backend | trait `SecretVault` / `EncryptionBackend` в archive |
| Другой UI | только новый frontend на том же facade |
| CI | CLI `--json` + exit policy |

Плагины v1 **не** нужны: данные tables + config groups достаточно.

---

## 12. Нефункциональные требования (архитектурные)

- **Безопасность:** masked by default; age; zeroize; no secrets in traces.
- **Производительность:** один проход walk где возможно; parallel sniff с лимитом jobs; cache sniff.
- **Предсказуемость:** dry-run по умолчанию для destructive ops; явный `--yes` / confirm в UI.
- **Тестируемость:** core без UI; `GitClient` mock; tempfile fixtures.
- **Портативность:** Linux primary; macOS/Windows — через те же crate, FS paths через `Path`.

---

## 13. Что сознательно вне scope v1

- Облачный sync den
- Автоматический PR «удали секреты»
- Полноценный secret manager (Vault/KMS) как primary store
- HTTP multi-user server
- Редактирование файлов проекта «умным» redact (достаточно delete/stash)

---

## 14. Карта решений (коротко)

| Вопрос | Решение |
|--------|---------|
| Где бизнес-логика? | Только `raccpack-core` |
| Кто оркестрирует raid? | Facade в core (или app-crate), UI только триггерит |
| Как Desktop говорит с core? | Tauri commands = BFF, React ↔ IPC |
| Где state UI? | TUI local; Desktop Zustand; CLI stateless |
| Как не светить секреты? | DTO masked; reveal opt-in; zeroize в core |
| Откуда берётся стек? | detect по markers/файлам, не по shebang alone |
| Куда артефакты? | `den_dir`, структура подпроектов/даты — policy core |

---

## 15. Следующий шаг после этого документа

1. Зафиксировать **workspace + пустой core API surface** (DTO + facade signatures).  
2. Перенести/реализовать scan → detect → secrets → archive по приоритету.  
3. Тонкий CLI как первый consumer.  
4. TUI и Desktop — когда facade стабилен (не раньше).

Документ можно уточнять, но **правило границы core / UI** менять не стоит: иначе эвристики и политики разъедутся между CLI, TUI и React.
