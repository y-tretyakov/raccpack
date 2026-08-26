# raccpack TUI — Переосмысленная спецификация

**Версия:** 2.0  
**Дата:** 2026-08-25  
**Статус:** Architecture / Implementation Specification  
**Целевая ветка:** `dev`  
**Репозиторий:** `y-tretyakov/raccpack`

---

## 0. Ключевая идея

TUI для `raccpack` не должен быть «GUI, нарисованный символами».

Это должен быть **операционный терминальный интерфейс для управления pipeline raccpack**:

> **Discover → Inspect → Protect → Clean → Pack → Verify**

TUI должен отвечать на три вопроса в любой момент:

1. **Что происходит?**
2. **Что будет изменено?**
3. **Безопасно ли это выполнять?**

Главный принцип:

> **TUI — thin client над `raccpack-core`, а не второе приложение с собственной бизнес-логикой.**

Существующий core уже предоставляет use-cases `sniff`, `dig`, `stash`, `rinse`, `pack`, `raid`, единый `AppContext`, `WorkspacePaths` и `ProgressSink`/`ProgressEvent`. TUI должен использовать эти API напрямую, а не вызывать CLI через subprocess.

---

# 1. Что меняется относительно первоначальной идеи

Исходная спецификация была построена вокруг набора независимых экранов:

`Overview / Projects / Secrets / Den / Raid / Settings`.

Это рабочий вариант, но он слишком близок к desktop GUI и плохо отражает реальную модель raccpack.

## 1.1. Новая модель

Вместо «шести страниц» вводится **операционный workspace**:

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│ RACC / WORKSPACE                         dev · ~/DEV/PROJS · READY            │
├──────────────┬───────────────────────────────────────────────────────────────┤
│ WORKSPACE    │                                                               │
│              │                    ACTIVE WORKSPACE                           │
│  Overview    │                                                               │
│  Projects    │   selected project / findings / pipeline / preview            │
│  Findings    │                                                               │
│  Den         │                                                               │
│  Operations  │                                                               │
│  Config      │                                                               │
│              │                                                               │
├──────────────┴───────────────────────────────────────────────────────────────┤
│ OPERATION ─ raid · my-app · phase 2/4 · rinse · 43%                          │
├──────────────────────────────────────────────────────────────────────────────┤
│ LOG 12:42:01 scan... 12 projects │ WARN 3 secrets │ ? help │ : command       │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Основные сущности интерфейса

- **Workspace** — текущее окружение raccpack.
- **Project** — обнаруженный проект.
- **Finding** — результат обнаружения чувствительных данных.
- **Artifact** — объект в den.
- **Operation** — выполняемый use-case.
- **Preview** — безопасное описание будущих изменений.
- **Event stream** — прогресс и диагностические события.

Это лучше соответствует доменной модели core.

---

# 2. Архитектурные ограничения

## 2.1. Запрещено

TUI не должен:

- реализовывать собственный scanner;
- самостоятельно искать `.env`;
- самостоятельно определять stack;
- самостоятельно удалять trash;
- самостоятельно создавать `tar.zst`;
- самостоятельно шифровать secrets;
- запускать `racc sniff`, `racc dig`, `racc raid` через shell;
- дублировать правила безопасности из core;
- хранить собственную альтернативную конфигурацию проекта.

Любое действие должно проходить через `raccpack-core`.

## 2.2. Разрешено TUI

TUI отвечает только за:

- navigation;
- state;
- rendering;
- keyboard/mouse input;
- filtering/sorting отображаемых данных;
- command palette;
- confirmation;
- presentation of progress;
- presentation of errors;
- persistent UI preferences;
- terminal lifecycle.

---

# 3. Технологический стек

## 3.1. Основной стек

```text
Rust
├── ratatui
├── crossterm
├── tokio
├── serde
├── serde_json
├── tracing
└── raccpack-core
```

### Почему Rust + ratatui

Это естественный выбор для проекта:

- core уже Rust;
- TUI может использовать domain API напрямую;
- нет IPC;
- нет subprocess overhead;
- типобезопасное состояние;
- хорошая производительность;
- минимальный runtime footprint.

`termion` не нужен как основной backend. Для современной реализации предпочтителен `crossterm`.

---

# 4. Структура workspace

Рекомендуется добавить отдельный crate:

```text
crates/
├── raccpack-core/
├── raccpack-cli/
└── raccpack-tui/
    ├── src/
    │   ├── main.rs
    │   ├── app.rs
    │   ├── state.rs
    │   ├── event.rs
    │   ├── command.rs
    │   ├── keymap.rs
    │   ├── actions.rs
    │   ├── runtime.rs
    │   ├── theme.rs
    │   ├── layout.rs
    │   ├── i18n.rs
    │   ├── persistence.rs
    │   ├── safety.rs
    │   ├── views/
    │   │   ├── mod.rs
    │   │   ├── overview.rs
    │   │   ├── projects.rs
    │   │   ├── findings.rs
    │   │   ├── den.rs
    │   │   ├── operations.rs
    │   │   └── config.rs
    │   └── widgets/
    │       ├── mod.rs
    │       ├── header.rs
    │       ├── sidebar.rs
    │       ├── footer.rs
    │       ├── log_view.rs
    │       ├── progress.rs
    │       ├── table.rs
    │       ├── modal.rs
    │       ├── command_palette.rs
    │       ├── confirm.rs
    │       ├── project_card.rs
    │       └── empty_state.rs
    └── Cargo.toml
```

CLI и TUI должны оставаться отдельными presentation layers.

---

# 5. Entry point

Предпочтительный UX:

```bash
racc tui
```

Дополнительно:

```bash
racc --tui
```

Если архитектура CLI позволяет без усложнения — `tui` как subcommand является предпочтительным вариантом.

TUI не должен менять семантику существующих команд.

---

# 6. Модель приложения

## 6.1. App

```rust
pub struct App {
    pub mode: AppMode,
    pub view: ViewId,
    pub workspace: WorkspaceState,
    pub operation: OperationState,
    pub terminal: TerminalState,
    pub ui: UiState,
    pub notifications: NotificationState,
}
```

## 6.2. ViewId

```rust
pub enum ViewId {
    Overview,
    Projects,
    Findings,
    Den,
    Operations,
    Config,
}
```

`Raid` больше не является отдельной «страницей».

Raid — это **operation**.

Это важное изменение.

---

# 7. Почему Raid не должен быть экраном

В исходной концепции `Raid` — отдельный view.

Лучше представить его как workflow:

```text
Project selected
       │
       ▼
Operation Preview
       │
       ├── stash
       ├── rinse
       ├── pack
       └── move / commit
       │
       ▼
Operation Result
```

Тогда одна и та же инфраструктура может отображать:

- sniff;
- dig;
- stash;
- rinse;
- pack;
- raid.

Это устраняет дублирование UI.

---

# 8. Operation Engine

## 8.1. Общая модель

```rust
pub enum Operation {
    Sniff(SniffRequest),
    Dig(DigRequest),
    Stash(StashRequest),
    Rinse(RinseRequest),
    Pack(PackRequest),
    Raid(RaidRequest),
}
```

Каждая операция имеет:

```text
Idle
  ↓
Preparing
  ↓
Preview
  ↓
AwaitingConfirmation
  ↓
Running
  ↓
Completed
  ├── Success
  ├── Failed
  └── Cancelled
```

## 8.2. Dry-run first

Для destructive operations UX должен быть:

```text
SELECT
  ↓
PREVIEW
  ↓
CONFIRM
  ↓
EXECUTE
  ↓
VERIFY
```

Никогда:

```text
SELECT → ENTER → DELETE
```

---

# 9. Safety Model

Безопасность — часть UX, а не только core.

## 9.1. Уровни действий

### READ

Безопасно:

- sniff;
- просмотр проектов;
- просмотр manifest;
- просмотр статистики;
- просмотр masked findings.

### ANALYZE

Потенциально дорогие:

- dig;
- repeated-secret analysis;
- refresh scan.

### WRITE

Изменяющие den:

- pack;
- stash.

### DESTRUCTIVE

Удаляющие данные:

- rinse;
- stash с `remove_sources`;
- raid;
- операции с `--yes`.

---

# 10. Confirmation policy

## Обычные действия

```text
Enter
```

## Write operations

Показывается preview:

```text
┌─ PLAN ───────────────────────────────────────────────┐
│ my-app                                               │
│                                                      │
│ STASH       3 sensitive files → encrypted archive   │
│ RINSE       428 MB build artifacts                  │
│ PACK        74 MB → my-app-20260825.tar.zst         │
│                                                      │
│ No source deletion                                  │
│                                                      │
│ [Enter] Continue   [Esc] Cancel                     │
└──────────────────────────────────────────────────────┘
```

## Destructive operations

Требуется явное подтверждение:

```text
Type: YES
```

или безопасный phrase confirmation:

```text
Type: raid my-app
```

Не использовать простое `y` для реально destructive операций.

---

# 11. Preview — центральный UX-компонент

Preview должен показывать не только команды, а **семантический план изменений**.

Например:

```text
RAID PLAN

Project
  ~/DEV/PROJS/my-app

STASH
  findings:       3
  encrypted:      3
  source removal: no

RINSE
  strategies:     target, target/debug, node_modules
  candidates:     128
  estimated:      1.4 GB

PACK
  output:         my-app-20260825-1242.tar.zst
  estimated size: 82 MB

Risk
  CRITICAL       0
  HIGH           3
  MEDIUM         1

Result
  source tree will remain intact

[Enter] execute
[Esc] cancel
```

---

# 12. Главный экран Overview

Overview не должен быть dashboard ради dashboard.

Он должен отвечать:

> «В каком состоянии сейчас мой workspace?»

```text
┌─ WORKSPACE ────────────────────────────────────────────────────────────────┐
│ ~/DEV/PROJS                                                                 │
│                                                                             │
│  PROJECTS       FINDINGS        DEN          LAST OPERATION                  │
│     27             14           8             raid · success                │
│                                                                             │
│  ───────────────── PIPELINE ─────────────────                              │
│                                                                             │
│  sniff ✓    dig ✓    protect !    rinse —    pack —                         │
│                                                                             │
│  ATTENTION                                                                 │
│  ! 3 HIGH findings require review                                           │
│  ! sniff cache is 2h old                                                    │
│                                                                             │
│  RECENT                                                                       │
│  12:41 sniff     27 projects                                                 │
│  12:44 dig       14 findings                                                 │
│  12:49 raid       my-app                                                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

# 13. Projects view

Основной компонент — высокопроизводительная таблица.

```text
┌─ PROJECTS ─────────────────────────────────────────────────────────────────┐
│ FILTER: rust  │ SORT: risk ↓                                                │
├────┬────────────────┬────────────┬────────┬─────────┬────────┬──────────────┤
│    │ PROJECT        │ STACK      │ SIZE   │ FINDINGS│ RISK   │ GIT          │
├────┼────────────────┼────────────┼────────┼─────────┼────────┼──────────────┤
│ >  │ my-app         │ rust/node  │ 128 MB │ 3       │ HIGH   │ main *       │
│    │ api-gateway    │ go         │ 45 MB  │ 0       │ —      │ develop      │
│    │ frontend       │ node       │ 89 MB  │ 1       │ MEDIUM │ main         │
└────┴────────────────┴────────────┴────────┴─────────┴────────┴──────────────┘

↑↓ navigate   Enter inspect   d dig   r raid   / filter
```

## Project detail

Не отдельная страница.

Использовать split/overlay:

```text
Projects
───────────────┬──────────────────────────────
my-app         │ my-app
api             │ rust + node
frontend        │ 128 MB
                │
                │ findings: 3
                │ git: main *
                │
                │ [d] dig
                │ [r] raid
                │ [Enter] preview
```

---

# 14. Findings view

`Secrets` переименовать в `Findings`.

Причина: core работает не только с «секретами», а с `SensitiveFinding`, risk, filename/content matches и повторяющимися находками.

Таблица:

```text
RISK     PROJECT       LOCATION                  TYPE       MATCH
CRITICAL my-app        deploy.yml:14             content    ********
HIGH     my-app        .env                      filename   .env
HIGH     api            config/prod.json:22      content    ********
MEDIUM   frontend      scripts/build.sh:18      content    ********
```

Никогда не показывать raw secret.

---

# 15. Finding detail

```text
┌─ FINDING ─────────────────────────────────────────────────┐
│ Risk       HIGH                                           │
│ Project    my-app                                         │
│ Location   src/config.rs:42                               │
│ Source     content                                        │
│ Pattern    AWS_ACCESS_KEY                                 │
│                                                           │
│ Value      **************                                 │
│ Fingerprint 9f2c…                                         │
│                                                           │
│ Recommended action                                        │
│   stash into encrypted den archive                       │
│                                                           │
│ [s] stash   [Esc] close                                   │
└───────────────────────────────────────────────────────────┘
```

---

# 16. Den view

Den — это не просто файловый browser.

Нужны три semantic tabs:

```text
[Packs] [Secrets] [Manifests]
```

## Packs

```text
DATE         PROJECT       SIZE       STATUS
2026-08-25   my-app        82 MB      valid
2026-08-24   api           31 MB      valid
```

## Secrets

```text
DATE         PROJECT       FILES      ENCRYPTION
2026-08-25   my-app        3          age
```

## Manifests

Показывать metadata:

- project;
- timestamp;
- stages;
- artifacts;
- schema version;
- hashes;
- counts.

Секретное содержимое не расшифровывать автоматически.

---

# 17. Operations view

Это новый центральный экран.

Он показывает историю и активные операции.

```text
┌─ OPERATIONS ───────────────────────────────────────────────────────────────┐
│ RUNNING                                                                     │
│                                                                             │
│ ● raid · my-app                     rinse        █████████░░ 82%             │
│                                                                             │
│ HISTORY                                                                     │
│ ✓ 12:49 raid    my-app        4/4 stages                                  │
│ ✓ 12:44 dig     workspace     14 findings                                  │
│ ✓ 12:41 sniff   workspace     27 projects                                 │
│ ✗ 11:20 pack    old-app       permission denied                            │
└─────────────────────────────────────────────────────────────────────────────┘
```

При активной операции:

```text
Enter → operation detail
```

---

# 18. Operation Detail

```text
┌─ RAID · my-app ────────────────────────────────────────────────────────────┐
│                                                                             │
│  ✓ 1/4 STASH       100%                                                    │
│  ● 2/4 RINSE        67%    █████████████░░░░░░                            │
│  ○ 3/4 PACK          —                                                     │
│  ○ 4/4 MOVE          —                                                     │
│                                                                             │
│  Current: removing target/debug                                            │
│  Processed: 812 MB                                                          │
│                                                                             │
│  LOG                                                                        │
│  12:51:01 INFO rinse started                                                │
│  12:51:02 INFO target/debug matched                                         │
│  12:51:04 INFO removing files                                               │
│                                                                             │
│ [Esc] cancel   [l] logs   [q] background                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

# 19. Progress — использовать существующий core API

Core уже предоставляет:

```rust
ProgressEvent {
    operation,
    phase,
    phase_index,
    phase_count,
    percent,
    overall_percent,
    message,
    phase_complete,
}
```

TUI должен отображать его напрямую.

Не создавать параллельную модель процентов.

## Adapter

```rust
struct TuiProgressSink {
    tx: Sender<AppEvent>,
}
```

```rust
impl ProgressSink for TuiProgressSink {
    fn emit(&mut self, event: ProgressEvent) {
        let _ = self.tx.send(AppEvent::Progress(event));
    }
}
```

---

# 20. Event-driven architecture

Главный цикл:

```text
Terminal Input ───────┐
                      │
Core Progress ────────┼──> Event Channel ──> App State ──> Render
                      │
Worker Result ────────┤
                      │
Timer / Tick ─────────┘
```

## AppEvent

```rust
pub enum AppEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Tick,
    Resize(u16, u16),

    Progress(ProgressEvent),

    OperationStarted(OperationId),
    OperationFinished(OperationResult),

    Error(AppError),

    Notification(Notification),
}
```

UI thread никогда не должен блокироваться.

---

# 21. Async model

Не выполнять core operation внутри render loop.

```text
UI thread
   │
   ├── input
   ├── state transition
   └── render
          │
          ▼
     tokio task
          │
          ▼
     raccpack-core
          │
          ▼
     ProgressSink
          │
          ▼
     AppEvent channel
```

## Главное правило

**Render loop должен быть deterministic и non-blocking.**

---

# 22. Cancellation

Если операция поддерживает cancellation — использовать cancellation token.

```rust
CancellationToken
```

UX:

```text
[Esc] Cancel
```

Но отмена не должна делать вид, что уже выполненные destructive changes были откатаны.

После cancel показывать:

```text
CANCELLED

Completed:
  stash ✓
  rinse 67%

Not executed:
  pack
  move

Workspace may be partially processed.
```

---

# 23. Atomicity и честный UI

TUI не должен обещать rollback, если core его не предоставляет.

Особенно для Raid.

Если `raid` работает в atomic mode — отображать:

```text
MODE: ATOMIC
```

Если `fail_fast`:

```text
MODE: FAIL-FAST
```

Если произошла частичная ошибка:

```text
PARTIAL FAILURE

stash      ✓
rinse      ✓
pack       ✗
move       —

No automatic rollback was performed.
```

---

# 24. Command Palette

`:` открывает command palette.

Примеры:

```text
:sniff
:sniff --force
:dig
:dig project=my-app
:raid
:raid project=my-app
:pack
:stash
:rinse
:den
:refresh
:theme nocturnal
:lang en
:config
:quit
```

Palette должна быть не просто текстовым меню.

Поддержать:

- fuzzy search;
- aliases;
- command history;
- context-aware commands;
- argument hints.

---

# 25. Context actions

Большинство действий должно быть контекстным.

Для Project:

```text
d  Dig
r  Raid
p  Pack
s  Stash
i  Inspect
g  Git
```

Для Finding:

```text
s  Stash
i  Inspect
f  Filter project
```

Для Den artifact:

```text
i  Inspect
m  Manifest
o  Open path
```

Это быстрее глобального меню.

---

# 26. Keymap

Основной принцип:

**vim-like navigation + conventional terminal shortcuts.**

```text
↑↓ / j k       move
h l             horizontal navigation
Enter           primary action
Esc             back / cancel
Tab             next focus
Shift+Tab       previous focus
Space           select
/               filter
:               command palette
?               help
r               refresh
q               quit
Ctrl+C          quit / cancel
```

Цифры не должны быть единственным способом навигации.

---

# 27. Focus model

TUI должен иметь явный focus manager.

```rust
pub enum Focus {
    Navigation,
    Main,
    SidePanel,
    Log,
    Modal,
}
```

`Tab` циклически меняет focus.

Это особенно важно для:

- accessibility;
- forms;
- narrow terminals;
- mouse mode.

---

# 28. Layout strategy

Не привязывать UI к конкретному разрешению.

Использовать layout breakpoints:

```text
>= 140 columns
    sidebar + main + optional detail

120–139
    sidebar + main

100–119
    top navigation + main

80–99
    compact navigation + main

< 80
    minimal mode
```

Высота:

```text
>= 40
    main + operation + log

30–39
    main + compact operation

24–29
    main + status

< 24
    warning / minimal mode
```

---

# 29. Log system

Log panel должен быть **вторичным**, а не главным интерфейсом.

По умолчанию:

```text
3–5 lines
```

Expandable:

```text
[l] toggle
[ / ] resize
```

Уровни:

```text
TRACE
DEBUG
INFO
WARN
ERROR
```

Фильтрация:

```text
/log
```

или:

```text
:lvl warn
```

---

# 30. Notifications

Не использовать агрессивные toast-анимации.

Использовать transient status:

```text
✓ Raid completed · my-app · 12.4s
```

Ошибки:

```text
✗ Pack failed · permission denied
Press Enter for details
```

Notification хранится несколько секунд и затем исчезает.

История остаётся в Operations.

---

# 31. Empty states

Empty state должен быть функциональным.

Плохо:

```text
No projects.
```

Хорошо:

```text
        /\_/\\
       ( o.o )
        > ^ <

No projects discovered.

scan root:
~/DEV/PROJS

[Enter] Run sniff
```

Raccoon ASCII — часть branding, но не должен занимать половину терминала.

---

# 32. Skeleton loading

Skeleton использовать только там, где действительно есть asynchronous loading.

Не имитировать загрузку для синхронных операций.

Пример:

```text
░░░░░░░░░░  ░░░░░░  ░░░░░░
░░░░░░░░░░  ░░░░░░  ░░░░░░
░░░░░░░░░░  ░░░░░░  ░░░░░░
```

---

# 33. Virtualization

Для больших списков:

- не создавать widget на каждую строку;
- хранить dataset отдельно;
- вычислять visible range;
- рендерить только viewport;
- поддерживать stable selection.

Target:

```text
10 000 findings
< 16 ms render budget
```

---

# 34. Search / filtering

`/` открывает contextual search.

Примеры:

```text
rust
risk:high
project:my-app
stack:node
git:dirty
size:>100mb
```

В первой версии можно поддержать простой substring search.

Advanced query syntax — P1.

---

# 35. Sorting

Поддержать:

```text
name
size
risk
findings
modified
stack
git status
```

Состояние сортировки:

```text
sort = risk desc
```

---

# 36. Theme system

Default:

## Nocturnal

```text
background   #0b0c0e
foreground   #e8e6e1
surface      #141410
muted        #9a968c
accent       #c4c0b6
danger       #c45c4a
warning      #c49a6c
success      #6b8f71
```

Дополнительные:

- Moonlit;
- Ember;
- Fog;
- Moss.

Но theme должна быть **semantic**, а не набором цветов.

```rust
struct Theme {
    background: Color,
    surface: Color,
    text: Color,
    muted: Color,
    accent: Color,
    success: Color,
    warning: Color,
    danger: Color,
    selection: Color,
    border: Color,
}
```

---

# 37. Accessibility

TUI должен корректно работать без цветов.

Поддержать:

```text
NO_COLOR=1
```

Смысл нельзя кодировать только цветом.

Например:

```text
CRITICAL !
HIGH     ▲
MEDIUM   ~
LOW      ·
```

Цвет — enhancement, не единственный carrier of information.

---

# 38. Unicode policy

Основной UI должен работать в ASCII-safe режиме.

Использовать Unicode только если terminal capability позволяет.

Fallback:

```text
█ → #
░ → .
│ → |
┌ → +
└ → +
```

Raccoon art также должен иметь ASCII fallback.

---

# 39. Mouse

Mouse — optional.

Поддержать:

- click;
- scroll;
- selection;
- resize split;
- modal buttons.

Но:

> Любое действие должно быть доступно без мыши.

---

# 40. Persistent preferences

Хранить только UI state:

```toml
theme = "nocturnal"
language = "ru"
sidebar = true
log_height = 5
mouse = false
animations = false
last_view = "overview"
```

Не дублировать:

- scan_root;
- den_dir;
- rinse strategies;
- risk policy;
- detection mode.

Они принадлежат raccpack config.

---

# 41. Configuration view

Config view должен редактировать существующий `RaccConfig`, а не создавать второй config.

Секции:

```text
Paths
Scanner
Detection
Cleanup
Security
UI
```

Изменение destructive config требует confirmation.

---

# 42. Config boundary

Разделить:

```text
RaccConfig
    ↓
Core configuration

TuiPreferences
    ↓
Presentation-only configuration
```

Никогда не смешивать их.

---

# 43. Security rules

## Никогда не выводить

- plaintext secrets;
- passphrases;
- private keys;
- decrypted age content;
- raw sensitive values.

## Разрешено

- masked value;
- fingerprint;
- risk;
- pattern;
- location;
- hash;
- counts.

Passphrase input:

```text
Password: ********
```

Использовать `rpassword`/secure input mechanism.

После использования очищать sensitive buffers настолько, насколько это возможно.

---

# 44. Error UX

Ошибки должны иметь три уровня:

```text
Summary
Details
Recovery
```

Пример:

```text
✗ PACK FAILED

Summary
Permission denied writing den artifact.

Details
path: ~/.raccpack/den/packs/...

Recovery
Check permissions and den configuration.

[Enter] details
[r] retry
[Esc] close
```

Не показывать Rust backtrace обычному пользователю.

Backtrace — только debug mode.

---

# 45. Status bar

Status bar должен быть context-sensitive.

Normal:

```text
~/DEV/PROJS · 27 projects · q quit · ? help · : commands
```

Operation:

```text
RAID my-app · rinse 67% · Esc cancel · l logs
```

Modal:

```text
CONFIRMATION · Enter confirm · Esc cancel
```

---

# 46. Header

Минимальный:

```text
RACC 0.3.8
workspace: ~/DEV/PROJS
status: READY
```

Не тратить строку на декоративные элементы.

Версия может быть скрыта в compact mode.

---

# 47. Startup sequence

При запуске:

```text
1. Initialize terminal
2. Load TUI preferences
3. Load raccpack config
4. Build AppContext
5. Validate workspace
6. Render Overview
7. Do not automatically run destructive operations
```

`sniff` автоматически запускать только если это явно включено настройкой.

По умолчанию:

> **TUI не должен менять файловую систему просто от запуска.**

---

# 48. Shutdown

При выходе:

```text
stop accepting operations
wait / detach according to operation policy
restore terminal
flush logs
persist TUI preferences
```

Terminal state должен восстанавливаться даже при panic.

Использовать panic hook + RAII guard.

---

# 49. Concurrent operations

В первой версии:

**одна foreground operation одновременно.**

Разрешается:

- operation running;
- просмотр logs;
- просмотр статического state.

Не разрешается запускать два destructive operation одновременно.

P2:

```text
background operation queue
```

---

# 50. Data refresh strategy

Не делать полный rescan после каждого действия.

После operation:

```text
invalidate affected state
refresh relevant view
```

Например:

```text
stash my-app
    ↓
invalidate findings(my-app)
invalidate den
refresh project(my-app)
```

Полный sniff — только по запросу.

---

# 51. Cache awareness

Если sniff cache существует, Overview должен показывать:

```text
Projects: 27
Source: cache
Age: 12 min
```

Если cache stale:

```text
! cache is stale
[r] refresh
```

Не скрывать происхождение данных.

---

# 52. Domain mapping

TUI должен использовать существующие типы core:

```text
Project
ScanReport
SensitiveFinding
SensitiveRisk
Stack
SniffResult
DigResult
StashResult
RinseResult
PackResult
RaidResult
DenManifest
ProgressEvent
```

Не создавать копии domain entities без необходимости.

View models допустимы только для presentation-specific aggregation.

---

# 53. CLI parity

Каждая TUI action должна иметь эквивалентную CLI semantic operation.

| TUI | Core / CLI semantic |
|---|---|
| Sniff | `sniff` |
| Dig | `dig` |
| Stash | `stash` |
| Rinse | `rinse` |
| Pack | `pack` |
| Raid | `raid` |
| Init | `init` |

TUI не должен вводить скрытые операции, которых нет в core.

---

# 54. CLI JSON как reference

Существующий CLI имеет `--json`.

Это полезно как:

- debugging reference;
- integration-test oracle;
- documentation source.

Но TUI **не должен парсить собственный CLI JSON через subprocess**.

Использовать core API напрямую.

---

# 55. Testing strategy

## Unit

Тестировать:

- state transitions;
- key mapping;
- filters;
- sorting;
- confirmation rules;
- operation state;
- layout breakpoints;
- preferences serialization.

## Integration

Проверять:

```text
TUI action
   ↓
core operation
   ↓
ProgressEvent
   ↓
TUI state
```

## Snapshot

Использовать terminal snapshot testing для:

- Overview;
- Projects;
- Findings;
- Preview;
- Confirm;
- Operation Detail;
- Errors.

Например:

```text
insta
```

или аналогичный snapshot framework.

---

# 56. Safety tests

Обязательные тесты:

1. TUI никогда не выполняет destructive action без confirmation.
2. `Esc` не подтверждает operation.
3. `Ctrl+C` во время confirmation не подтверждает.
4. Secret values не попадают в rendered buffer.
5. Password input не попадает в logs.
6. Panic восстанавливает terminal.
7. Operation error корректно возвращает UI в stable state.
8. Partial failure не показывается как success.
9. Cancellation не показывается как rollback.
10. Existing config не изменяется при простом просмотре.

---

# 57. Performance targets

Целевые показатели:

| Метрика | Target |
|---|---:|
| Startup | < 150 ms без sniff |
| Input latency | < 50 ms |
| Render | < 16 ms typical |
| 2k findings | плавная навигация |
| 10k findings | без полного rerender |
| Log 10k lines | bounded memory |
| Resize | без panic |
| 80×24 | usable |

---

# 58. Log memory policy

Нельзя бесконечно хранить все строки в RAM.

Использовать ring buffer:

```text
capacity = 2_000–10_000 lines
```

Настройка:

```toml
log_buffer = 5000
```

Полный лог должен оставаться в tracing/file infrastructure, если включён.

---

# 59. Rendering architecture

Не смешивать:

```text
state mutation
business operation
render
```

Правильный поток:

```text
Event
  ↓
Reducer / Action
  ↓
State
  ↓
Renderer
```

Пример:

```rust
match event {
    AppEvent::Progress(progress) => {
        app.operation.apply(progress);
    }
    ...
}
```

Renderer только читает state.

---

# 60. View contract

Каждый view:

```rust
pub trait View {
    fn handle(&mut self, action: Action, ctx: &mut AppContext);
    fn render(&self, frame: &mut Frame, area: Rect, state: &AppState);
}
```

Но предпочтительнее не давать View прямой доступ к core mutation.

Лучше:

```text
View
 ↓
Action
 ↓
Application Controller
 ↓
Core
```

---

# 61. Application actions

```rust
pub enum Action {
    Navigate(ViewId),
    Refresh,
    OpenProject(ProjectId),
    OpenFinding(FindingId),

    StartSniff,
    StartDig,
    StartStash(StashRequest),
    StartRinse(RinseRequest),
    StartPack(PackRequest),
    StartRaid(RaidRequest),

    Confirm,
    Cancel,
    ShowHelp,
    OpenCommandPalette,
    Quit,
}
```

---

# 62. State machine

Главный AppState:

```text
BOOT
 ↓
READY
 ├── VIEWING
 ├── PREVIEW
 ├── CONFIRMING
 ├── RUNNING
 │     ├── SUCCESS
 │     ├── FAILURE
 │     └── CANCELLED
 └── ERROR
```

State machine должна предотвращать невозможные переходы.

Например:

```text
RUNNING → Confirm
```

недопустимо.

---

# 63. Den consistency

После записи artifact:

```text
write artifact
write manifest
verify artifact
update UI
```

UI не должен показывать artifact как `VALID`, пока core не вернул успешный результат.

---

# 64. Raid visualization

Raid — главная «звезда» TUI.

Предлагаемый вид:

```text
RAID · my-app

┌──────────── PIPELINE ────────────┐

   STASH        RINSE        PACK        MOVE
     ✓            ●            ○           ○
    100%          67%          —           —

────────────────────────────────────────────

Current phase
RINSE

target/debug
██████████████████░░░░░░ 67%

Processed     812 MB
Candidates    128
Removed       94

────────────────────────────────────────────

[Esc] cancel   [l] logs   [b] background
```

Это намного полезнее общего progress bar.

---

# 65. Risk visualization

Risk:

```text
CRITICAL  !!!
HIGH      !!
MEDIUM    !
LOW       .
```

Цвет + символ + текст.

Never rely on color alone.

---

# 66. Git integration

Core уже имеет Git abstraction.

TUI может показывать:

```text
main *
dirty
ahead 2
behind 1
```

Но Git status не должен блокировать основной workflow.

Если Git unavailable:

```text
GIT: unavailable
```

а не crash.

---

# 67. Stack visualization

Если найден stack tree:

```text
rust
 ├─ cargo
 └─ node
     ├─ npm
     └─ vite
```

Использовать это только как informational context.

---

# 68. Init

Init — отдельный bootstrap mode.

```text
racc tui
   ↓
config missing
   ↓
Bootstrap Wizard
```

Wizard:

```text
1 Workspace
2 Den
3 Detection / cleanup defaults
4 Review
5 Create
```

После создания:

```text
→ Overview
```

---

# 69. First-run UX

Если config существует:

```text
START → Overview
```

Если отсутствует:

```text
START → Init Wizard
```

Если config corrupted:

```text
ERROR
Configuration could not be loaded.

[r] retry
[o] open details
[q] quit
```

---

# 70. Settings

Настройки UI:

```text
Appearance
  Theme
  Language
  Mouse
  Animations

Layout
  Sidebar
  Log height
  Compact mode

Behavior
  Confirm destructive operations
  Auto refresh
  Cache awareness
```

Core settings редактировать отдельным разделом.

---

# 71. Internationalization

Первый релиз:

```text
ru
en
```

Все user-facing strings должны быть централизованы.

Не писать:

```rust
Span::raw("Запустить raid")
```

по всему проекту.

Использовать translation keys:

```text
operation.raid.run
operation.raid.preview
finding.risk.high
```

---

# 72. Documentation

Добавить:

```text
docs/tui.md
docs/tui-keymap.md
docs/tui-architecture.md
```

README:

```bash
racc tui
```

с коротким screenshot/ascii preview.

---

# 73. Implementation phases

## Phase 0 — Architecture

- [ ] создать `raccpack-tui`;
- [ ] подключить core;
- [ ] terminal lifecycle;
- [ ] event loop;
- [ ] state model;
- [ ] theme;
- [ ] keymap.

## Phase 1 — Shell

- [ ] Header;
- [ ] navigation;
- [ ] status bar;
- [ ] log panel;
- [ ] responsive layout;
- [ ] help overlay.

## Phase 2 — Read-only workspace

- [ ] Overview;
- [ ] Projects;
- [ ] Findings;
- [ ] Den;
- [ ] filtering;
- [ ] sorting;
- [ ] details.

## Phase 3 — Operations

- [ ] sniff;
- [ ] dig;
- [ ] progress;
- [ ] operation history;
- [ ] cancellation;
- [ ] errors.

## Phase 4 — Safe write workflows

- [ ] stash;
- [ ] rinse;
- [ ] pack;
- [ ] preview;
- [ ] confirmation;
- [ ] result verification.

## Phase 5 — Raid

- [ ] raid preview;
- [ ] phase pipeline;
- [ ] partial failure;
- [ ] atomic/fail-fast presentation;
- [ ] operation detail.

## Phase 6 — Polish

- [ ] command palette;
- [ ] themes;
- [ ] i18n;
- [ ] mouse;
- [ ] snapshot tests;
- [ ] performance;
- [ ] accessibility.

---

# 74. Priority matrix

| Feature | Priority |
|---|---|
| Terminal lifecycle | P0 |
| Core integration | P0 |
| Event loop | P0 |
| Overview | P0 |
| Projects | P0 |
| Findings | P0 |
| Operation model | P0 |
| ProgressEvent integration | P0 |
| Preview | P0 |
| Confirmation | P0 |
| Raid | P0 |
| Error handling | P0 |
| Den | P1 |
| Operation history | P1 |
| Command palette | P1 |
| Preferences | P1 |
| i18n | P1 |
| Themes | P1 |
| Mouse | P2 |
| Advanced filter language | P2 |
| Background jobs | P2 |
| Full Git detail | P2 |

---

# 75. Definition of Done

TUI считается production-ready только если:

- [ ] не дублирует business logic core;
- [ ] не вызывает CLI subprocess;
- [ ] запускается через `racc tui`;
- [ ] корректно работает без config и запускает init wizard;
- [ ] работает в 80×24;
- [ ] имеет полноценный keyboard-first UX;
- [ ] показывает реальные `ProgressEvent`;
- [ ] имеет Preview перед destructive operations;
- [ ] требует explicit confirmation;
- [ ] никогда не показывает plaintext secrets;
- [ ] корректно отображает partial failure;
- [ ] корректно обрабатывает cancellation;
- [ ] не блокирует render loop;
- [ ] восстанавливает terminal после panic/error;
- [ ] имеет snapshot tests;
- [ ] проходит `cargo test --workspace`;
- [ ] проходит `cargo clippy --workspace --all-targets --all-features`;
- [ ] проходит форматирование;
- [ ] имеет документацию.

---

# 76. Главные архитектурные решения

## Решение 1

**TUI не является альтернативным CLI.**

Это presentation layer над core.

## Решение 2

**Raid — operation, а не view.**

## Решение 3

**Preview — обязательная часть destructive UX.**

## Решение 4

**ProgressEvent из core — единственный источник progress.**

## Решение 5

**Config и TUI preferences разделены.**

## Решение 6

**UI state управляется событиями, renderer не выполняет работу.**

## Решение 7

**Безопасность выражается в UX, а не только в core.**

## Решение 8

**TUI должен быть полезен без мыши и цветов.**

---

# 77. Что сознательно НЕ включать в первую реализацию

Не тратить время на:

- сложные ASCII-анимации;
- декоративные transitions;
- полноценный mouse-first UX;
- drag-and-drop;
- dashboard с десятками KPI;
- background job scheduler;
- встроенный shell;
- встроенный editor;
- просмотр raw secret;
- автоматический запуск операций при старте.

Сначала сделать терминальный инструмент, которым приятно и безопасно пользоваться каждый день.

---

# 78. Итоговая концепция

Финальный raccpack TUI должен ощущаться не как «панель управления», а как **операционный cockpit**:

```text
                    RACC
                     │
             ┌───────┴───────┐
             │   WORKSPACE   │
             └───────┬───────┘
                     │
        ┌────────────┼────────────┐
        ▼            ▼            ▼
     PROJECTS     FINDINGS       DEN
        │            │            │
        └────────────┼────────────┘
                     ▼
                  PREVIEW
                     │
                     ▼
                OPERATION
                     │
          ┌──────────┼──────────┐
          ▼          ▼          ▼
        STASH       RINSE       PACK
          └──────────┼──────────┘
                     ▼
                    RAID
                     │
                     ▼
                  VERIFY
```

Главная UX-формула:

> **Inspect first. Preview second. Execute third. Verify last.**

Именно это отличает хороший TUI для `raccpack` от обычной оболочки над CLI.

---

## Appendix A — Existing repository constraints

На ветке `dev` проект уже разделён на `raccpack-core` и `raccpack-cli`. Workspace использует Rust 2021 и минимальную версию Rust 1.85.

`raccpack-core` не зависит от CLI/TUI и экспортирует application facade/use-cases, включая `sniff`, `dig`, `stash`, `rinse`, `pack`, `raid`, а также `ProgressEvent` и `ProgressSink`.

CLI уже предоставляет операции:

```text
sniff
dig
pack
stash
rinse
raid
init
```

и глобальные параметры вроде config/root/den/json/verbose.

Следовательно, архитектурно наиболее чистый путь — добавить третий presentation crate:

```text
raccpack-core
     ▲
     │
 ┌───┴────────┐
 │            │
CLI          TUI
```

а не строить:

```text
TUI → shell → CLI → core
```

---

## Appendix B — Relationship to the previous TUI specification

Эта версия сохраняет сильные идеи предыдущей спецификации:

- Rust + ratatui;
- keyboard-first;
- responsive terminal layout;
- virtualized lists;
- skeleton loading;
- themes;
- i18n;
- command palette;
- persistent UI preferences;
- log panel;
- ASCII raccoon branding;
- 80×24 compatibility.

Но переосмысливает структуру вокруг реальной модели raccpack:

```text
Projects
Findings
Den
Operations
Preview
Safety
Progress
```

а не вокруг имитации desktop navigation.

Это уменьшает связанность UI, лучше соответствует `raccpack-core` и создаёт основу для дальнейшего развития без превращения TUI в второй монолит.
