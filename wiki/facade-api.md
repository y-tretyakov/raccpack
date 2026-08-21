---
title: Facade API (публичный контракт)
description: Публичный контракт ядра raccpack — базовые типы, операции sniff/dig/stash/rinse/pack/raid и инварианты.
---

# Facade API (публичный контракт)

Facade — единый публичный контракт ядра, на котором работают все интерфейсы (CLI, TUI, Desktop). Если вы пишете интеграцию или автоматизацию — это те структуры данных и операции, которые гарантированно стабильны.

::: info
Раздел ориентирован на пользователей, автоматизирующих raccpack (CI-скрипты, инструменты). Сигнатуры приведены в упрощённом виде; точные имена и поля живут в crate `raccpack-core`.
:::

## Базовые типы

### Пути и режим

```rust
pub struct WorkspacePaths {
    pub scan_root: PathBuf,   // вход: где проекты
    pub den_dir: PathBuf,     // выход: хранилище den
}

pub enum RunMode {
    DryRun,   // только отчёт, ничего не пишет и не удаляет
    Commit,   // реальные изменения
}
```

- `DryRun` — режим по умолчанию для разрушающих операций: ничего не создаётся в `secrets/` и `packs/`, источники не удаляются.
- `Commit` — реальное выполнение: архивы, удаление мусора, вынос секретов.

### Политика выхода при секретах

```rust
pub enum SecretExitPolicy {
    Ignore,              // всегда 0, если нет ошибок
    FailOnCritical,      // код 2 при Critical
    FailOnHighOrAbove,   // код 2 при High и выше
}
```

Применяется в CLI к коду выхода; в самом ядре операции всегда успешны, если сканирование прошло без ошибок.

### Прогресс

Длинные операции принимают `ProgressSink` — колбэк, получающий события:

```rust
pub struct ProgressEvent {
    pub operation: OperationKind,   // Sniff | Dig | Stash | Rinse | Pack | Raid
    pub phase: String,              // "stash" | "rinse" | "pack" | "move" | "scan"
    pub phase_index: u32,
    pub phase_count: u32,
    pub percent: u8,                // 0..=100
    pub overall_percent: u8,
    pub message: String,            // человекочитаемо, без сырых секретов
    pub phase_complete: bool,
}
```

CLI использует это для спиннера/прогресса, TUI — для перерисовки, Desktop — для событий Tauri.

### Контекст сессии

```rust
pub struct AppContext {
    pub config: RaccConfig,
    pub paths: WorkspacePaths,
    pub mode: RunMode,
    pub secret_groups_override: Option<EnabledGroups>,
    pub exit_policy: SecretExitPolicy,
}
```

Интерфейс собирает `AppContext` один раз на сессию и передаёт во все вызовы.

## Операции

### `sniff` - найти проекты

```rust
pub struct SniffOptions {
    pub force_refresh: bool,   // игнорировать кэш
    pub max_depth: Option<usize>,
}

pub struct SniffResult {
    pub report: ScanReport,   // { root, projects, total_size_bytes, schema_version }
    pub from_cache: bool,     // true, если результат из кэша
    pub duration_ms: u64,
}

pub fn sniff(ctx: &AppContext, opts: &SniffOptions,
             progress: &mut dyn ProgressSink) -> Result<SniffResult>;
```

**Статус: реализовано.** CLI: `racc sniff`.

### `dig` - найти секреты

```rust
pub struct DigOptions {
    pub project: Option<PathBuf>,  // ограничить одним проектом
    pub find_repeated: bool,       // искать повторяющиеся значения
    pub scan_content: bool,        // читать содержимое (default true)
    pub use_heuristics: Option<bool>,
}

pub struct DigResult {
    pub root: PathBuf,
    pub files: Vec<SensitiveFile>,
    pub repeated: Vec<RepeatedSecret>,
    pub duration_ms: u64,
    pub files_scanned: u64,
}

pub fn dig(ctx: &AppContext, opts: &DigOptions,
           progress: &mut dyn ProgressSink) -> Result<DigResult>;

// Хелпер для кода выхода
pub fn exit_code_for_secrets(files: &[SensitiveFile], policy: SecretExitPolicy) -> i32;
```

`SensitiveFile` и `RepeatedSecret` содержат только **masked** данные: путь, риск, метки, маскированное значение, хеш. Сырых значений нет.

**Статус: реализовано.** CLI: `racc dig`.

### `stash` - вынести секреты в age-архив

```rust
pub enum AgeIdentity {
    Passphrase(String),      // парольная фраза (zeroize после использования)
    Recipients(Vec<String>), // публичные recipient-ключи age
}

pub struct StashOptions {
    pub target: PathBuf,
    pub only_files: Option<Vec<PathBuf>>,
    pub min_risk: SensitiveRisk,   // по умолчанию High
    pub remove_sources: bool,      // удалить исходники (только Commit)
    pub batch_id: Option<String>,
}

pub fn stash(ctx: &AppContext, opts: &StashOptions, identity: &AgeIdentity,
             progress: &mut dyn ProgressSink) -> Result<StashResult>;
```

Поведение:

- `DryRun` — считает список и будущий путь архива, **не** пишет и **не** удаляет.
- `Commit` — пишет `.age`-архив в `den/secrets/…`, при `remove_sources: true` удаляет исходники.
- Passphrase не возвращается и не попадает в тексты ошибок.

**Статус: реализовано.** CLI: `racc stash`.

### `rinse` - очистить мусор сборки

```rust
pub struct RinseOptions {
    pub target: PathBuf,             // проект
    pub strategies: Option<Vec<String>>,
    pub include_custom_patterns: bool,
}

pub fn rinse(ctx: &AppContext, opts: &RinseOptions,
             progress: &mut dyn ProgressSink) -> Result<RinseResult>;
```

`DryRun` только перечисляет удаляемое; `Commit` удаляет каталоги. Файлы секретов `rinse` не трогает — это забота `stash`.

**Статус: реализовано.** CLI: `racc rinse`.

### `pack` - упаковать проект

```rust
pub struct PackOptions {
    pub project: PathBuf,
    pub output_name: Option<String>,  // по умолчанию {slug}__{ts}.tar.zst
    pub deny_content_secrets: bool,   // проверять содержимое при упаковке
    pub zstd_level: Option<u32>,
}

pub struct PackResult {
    pub source: PathBuf,
    pub output: PathBuf,
    pub size_bytes: u64,
    pub file_count: usize,
    pub skipped_secret_files: usize,
    pub dry_run: bool,
}

pub fn pack(ctx: &AppContext, opts: &PackOptions,
            progress: &mut dyn ProgressSink) -> Result<PackResult>;
```

Ядро упаковки (`pack_tree`) и facade `pack` (DryRun/Commit) реализованы, как и CLI-команда `racc pack`.

**Статус: ядро и CLI реализованы (MVP 0.1).**

### `raid` - полный цикл

```rust
pub enum OrchestrationMode {
    Atomic,     // по умолчанию: staging + отложенные удаления, откат через WAL
    FailFast,   // legacy A3.1: остановиться на первой упавшей фазе
}

pub struct RaidOptions {
    pub project: PathBuf,
    pub mode: OrchestrationMode,   // по умолчанию Atomic
    pub stash: StashPhaseOpts,     // { enabled, min_risk, remove_sources }
    pub rinse: RinsePhaseOpts,     // { enabled }
    pub pack: PackPhaseOpts,       // { enabled, deny_content_secrets }
}

pub struct RaidResult {
    pub project_path: PathBuf,
    pub stages: Vec<RaidStageResult>,  // stash | rinse | pack | move
    pub stash: Option<StashResult>,
    pub rinse: Option<RinseResult>,
    pub pack: Option<PackResult>,
    pub den_artifacts: Vec<PathBuf>,   // итоговые пути в den
    pub success: bool,
    pub dry_run: bool,
    pub rolled_back: bool,             // неудачный commit откачен к pre-raid
    pub rollback_warnings: Vec<String>,// нефатальные проблемы при откате
}

pub fn raid(ctx: &AppContext, opts: &RaidOptions, identity: Option<&AgeIdentity>,
            progress: &mut dyn ProgressSink) -> Result<RaidResult>;
```

Фиксированный порядок фаз: **stash → rinse → pack → move**. По умолчанию (`OrchestrationMode::Atomic`) артефакты пишутся во временный `den/staging/{id}/`, а в den переносятся только в commit; неудачный commit откатывается через WAL — отчёт получает `rolled_back: true`. В режиме `FailFast` (флаг `--fail-fast`) после первой упавшей фазы следующие не выполняются, а уже записанные артефакты остаются в den.

**Статус: реализовано.** CLI: `racc raid`.

## Отчёты и данные

### Стабильные DTO (serde-friendly)

- `ScanReport { root, projects, total_size_bytes, schema_version }`
- `Project { path, name, stack, size_bytes, is_git_repo }`
- `Stack { language, frameworks, markers }`
- `SensitiveFile { path, risk, labels, content_match?, git_status? }`
- `SensitiveRisk` — `Low | Medium | High | Critical`
- `MaskedValue { masked, value_hash, original_len }`

Отчёты сериализуются в JSON (`--json`) и содержат `schema_version` для проверки совместимости в CI.

### Manifest raid (JSON)

После каждого raid в `den/manifests/{yyyy}/{mm}/` пишется манифест. Пример (без сырых секретов):

```json
{
  "schema_version": 1,
  "created_at": "2026-08-04T15:52:30Z",
  "project_path": "/home/user/DEV/PROJS/my-api",
  "project_slug": "my-api",
  "dry_run": false,
  "success": true,
  "stages": [
    { "name": "stash", "success": true, "message": "archived 3 files", "skipped": false },
    { "name": "rinse", "success": true, "message": "removed 2 dirs, 140MB", "skipped": false },
    { "name": "pack", "success": true, "message": "wrote pack 12MB", "skipped": false },
    { "name": "move", "success": true, "message": "finalized", "skipped": false }
  ],
  "artifacts": {
    "secrets_archive": "secrets/2026/08/my-api__20260804T155230Z__secrets.age",
    "project_pack": "packs/2026/08/my-api__20260804T155230Z.tar.zst"
  },
  "stash_manifest": [
    { "original_path": "/home/user/DEV/PROJS/my-api/.env", "risk": "High", "size_bytes": 412 }
  ],
  "tool": { "name": "raccpack", "core_version": "0.2.14" }
}
```

Пути артефактов — **относительно корня den**, поэтому den можно переносить целиком.

## Инварианты контракта

1. Facade не возвращает сырой секретный материал в результатах.
2. `DryRun` не создаёт файлов в `secrets/` и `packs/` и не удаляет источники.
3. Имена в den уникальны за счёт timestamp + short_id.
4. Пути в манифестах — относительно корня den.
5. В режиме `Atomic` (по умолчанию) неудачный commit откатывается через WAL (`rolled_back`); в режиме `FailFast` уже записанные артефакты остаются в den.
6. Все пути в API — `PathBuf`; интерфейсы нормализуют их до вызова.

## См. также

- [Архитектура](/architecture) — слои и границы доверия.
- [Дорожная карта](/roadmap) — какой статус у каждой операции.
