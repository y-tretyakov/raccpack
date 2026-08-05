# Facade API и структура den

Дополнение к [видению архитектуры](raccpack-architecture-vision.md).  
Здесь — **конкретные сигнатуры** application-слоя и **раскладка файлов** в den.

Имена crate/модулей ориентировочные (`raccpack_core::app` или `raccpack_core::facade`). Типы отчётов — из domain (`report`).

---

## 1. Общие типы входа/выхода

```rust
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Куда писать артефакты и откуда сканировать.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePaths {
    /// Корень с проектами (вход).
    pub scan_root: PathBuf,
    /// Каталог den (выход).
    pub den_dir: PathBuf,
}

/// Режим разрушающих операций.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunMode {
    /// Только отчёт, без записи/удаления.
    DryRun,
    /// Реально писать архивы / удалять trash / выносить секреты.
    Commit,
}

impl RunMode {
    pub fn is_dry_run(self) -> bool {
        matches!(self, RunMode::DryRun)
    }
}

/// Политика при CRITICAL/HIGH секретах в CI-режиме.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretExitPolicy {
    /// Всегда exit 0, если нет IO-ошибок.
    Ignore,
    /// Exit 2, если есть Critical.
    FailOnCritical,
    /// Exit 2, если есть Critical или High.
    FailOnHighOrAbove,
}

/// Событие прогресса длинной операции (raid / глубокий dig / pack).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub operation: OperationKind,
    pub phase: String,       // стабильный id: "stash" | "rinse" | "pack" | "move" | "scan"
    pub phase_index: u32,    // 0-based
    pub phase_count: u32,
    pub percent: u8,         // 0..=100 в рамках phase или overall — см. overall_percent
    pub overall_percent: u8, // 0..=100 по всей операции
    pub message: String,     // human, без raw secrets
    pub phase_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    Sniff,
    Dig,
    Stash,
    Rinse,
    Pack,
    Raid,
}

/// Куда слать прогресс. UI передаёт свой sink.
pub trait ProgressSink: Send {
    fn emit(&mut self, event: ProgressEvent);
}

/// Sink, который ничего не делает (CLI без spinner, тесты).
pub struct NullProgress;
impl ProgressSink for NullProgress {
    fn emit(&mut self, _event: ProgressEvent) {}
}
```

Ошибки — `raccpack_core::Error` / `ConfigError` (см. core). Facade не вводит отдельный mega-enum, кроме узких result-типов операций.

---

## 2. Контекст выполнения

Все use-case’ы принимают общий контекст, чтобы не таскать десяток аргументов.

```rust
/// Собранный runtime-контекст одной сессии UI/CLI.
pub struct AppContext {
    pub config: RaccConfig,
    pub paths: WorkspacePaths,
    pub mode: RunMode,
    /// Опционально: переопределить groups секретов на этот запуск.
    pub secret_groups_override: Option<EnabledGroups>,
    pub exit_policy: SecretExitPolicy,
}

impl AppContext {
    pub fn from_config(config: RaccConfig, mode: RunMode) -> Result<Self, ConfigError> {
        Ok(Self {
            paths: WorkspacePaths {
                scan_root: config.scan_root_dir()?,
                den_dir: config.den_dir()?,
            },
            secret_groups_override: None,
            exit_policy: SecretExitPolicy::FailOnCritical,
            mode,
            config,
        })
    }
}
```

CLI/TUI/Desktop собирают `AppContext` один раз на сессию (или на команду).

---

## 3. Facade: sniff

**Назначение:** найти проекты, стек, размеры, краткий git.

```rust
/// Опции только для sniff (не путать с global config).
#[derive(Debug, Clone, Default)]
pub struct SniffOptions {
    /// Игнорировать cache и пересканировать.
    pub force_refresh: bool,
    /// Макс. глубина; None → из config.scanner.max_depth.
    pub max_depth: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniffResult {
    pub report: ScanReport,
    pub from_cache: bool,
    pub duration_ms: u64,
}

/// Синхронный API (core). UI при необходимости оборачивает в spawn_blocking.
pub fn sniff(
    ctx: &AppContext,
    opts: &SniffOptions,
    progress: &mut dyn ProgressSink,
) -> Result<SniffResult>;
```

**Поведение:**

- Root = `ctx.paths.scan_root` (должен существовать).
- Cache: если `!force_refresh` и cache fresh → `from_cache: true`.
- Progress: phase `"scan"`, percent по числу найденных candidates / завершению.
- Не читает содержимое на секреты (это dig).

**Связанные DTO:** `ScanReport`, `Project`, `Stack`, `GitState` — из `report`.

---

## 4. Facade: dig

**Назначение:** найти секреты (filename + content + heuristics).

```rust
#[derive(Debug, Clone)]
pub struct DigOptions {
    /// Ограничить одним проектом; None → весь scan_root.
    pub project: Option<PathBuf>,
    /// Искать repeated values across files.
    pub find_repeated: bool,
    /// Включить content scan (default true).
    pub scan_content: bool,
    /// Включить entropy heuristics (default: как config group "heuristics").
    pub use_heuristics: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigResult {
    pub root: PathBuf,
    pub files: Vec<SensitiveFile>,
    pub repeated: Vec<RepeatedSecret>,
    pub duration_ms: u64,
    /// Сколько файлов просмотрено (для UX).
    pub files_scanned: u64,
}

pub fn dig(
    ctx: &AppContext,
    opts: &DigOptions,
    progress: &mut dyn ProgressSink,
) -> Result<DigResult>;
```

**Поведение:**

- Groups: `secret_groups_override` или `config.sensitive`.
- `SensitiveFile.content_match` и `RepeatedSecret` — **без raw**; только prefix/mask/hash.
- Progress: `"dig"`, overall по walk.
- Exit policy **не** применяется здесь (применяет CLI к коду выхода после dig); функция всегда `Ok` при успешном скане.

```rust
/// Хелпер для CLI exit code после dig/sniff+dig.
pub fn exit_code_for_secrets(files: &[SensitiveFile], policy: SecretExitPolicy) -> i32;
```

---

## 5. Facade: stash

**Назначение:** вынести секреты в age-архив(ы), опционально удалить источники.

```rust
#[derive(Debug, Clone)]
pub struct StashOptions {
    /// Проект или поддерево.
    pub target: PathBuf,
    /// Список конкретных файлов; None → все находки dig по target.
    pub only_files: Option<Vec<PathBuf>>,
    /// Минимальный risk для включения (default High).
    pub min_risk: SensitiveRisk,
    /// Удалить исходные файлы после успешного encrypt (только Commit).
    pub remove_sources: bool,
    /// Идентификатор batch (для имени архива); None → timestamp.
    pub batch_id: Option<String>,
}

/// Материал ключа. Не логировать. Zeroize после use внутри core.
pub enum AgeIdentity {
    Passphrase(String),           // на практике Zeroizing<String> / SecretString
    Recipients(Vec<String>),      // age recipient strings (public)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashResult {
    pub archive_path: PathBuf,    // относительно den или absolute
    pub files_archived: usize,
    pub bytes_archived: u64,
    pub removed_sources: usize,   // 0 в DryRun
    pub dry_run: bool,
    /// Манифест того, что ушло в архив (пути + risk, без raw).
    pub manifest: Vec<StashManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashManifestEntry {
    pub original_path: PathBuf,
    pub risk: SensitiveRisk,
    pub size_bytes: u64,
}

pub fn stash(
    ctx: &AppContext,
    opts: &StashOptions,
    identity: &AgeIdentity,
    progress: &mut dyn ProgressSink,
) -> Result<StashResult>;
```

**Поведение:**

- DryRun: считает список и путь будущего архива, **не** пишет age и **не** удаляет.
- Commit: пишет archive под den (см. §9), атомарно насколько возможно (temp + rename).
- Passphrase не возвращается и не попадает в `Error` Display.

---

## 6. Facade: rinse

**Назначение:** удалить build/trash-директории по стратегиям.

```rust
#[derive(Debug, Clone)]
pub struct RinseOptions {
    pub target: PathBuf,          // project root
    /// None → config.cleanup.enabled_strategies
    pub strategies: Option<Vec<String>>,
    pub include_custom_patterns: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RinseResult {
    pub removed: Vec<TrashDir>,
    pub bytes_freed: u64,
    pub dry_run: bool,
}

pub fn rinse(
    ctx: &AppContext,
    opts: &RinseOptions,
    progress: &mut dyn ProgressSink,
) -> Result<RinseResult>;
```

**Поведение:** DryRun только перечисляет; Commit удаляет dirs. Не трогает файлы секретов (это stash).

---

## 7. Facade: pack

**Назначение:** упаковать проект в архив **без** hard-denied secrets и skip-dirs.

```rust
#[derive(Debug, Clone)]
pub struct PackOptions {
    pub project: PathBuf,
    /// Имя артефакта; None → `{project_name}-{timestamp}.tar.zst`
    pub output_name: Option<String>,
    /// Content-scan перед pack: skip CRITICAL content matches.
    pub deny_content_secrets: bool,
    pub zstd_level: Option<u32>,  // None → config.advanced.zstd_level
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackResult {
    pub source: PathBuf,
    pub output: PathBuf,
    pub size_bytes: u64,
    pub file_count: usize,
    pub skipped_secret_files: usize,
    pub dry_run: bool,
}

pub fn pack(
    ctx: &AppContext,
    opts: &PackOptions,
    progress: &mut dyn ProgressSink,
) -> Result<PackResult>;
```

**Поведение:**

- Пишет во временный файл в den (или staging), затем rename.
- DryRun: `output` — ожидаемый путь, `size_bytes` может быть 0 или estimate.

---

## 8. Facade: raid (оркестрация)

**Назначение:** stash → rinse → pack → move/finalize в den одной операцией.

```rust
#[derive(Debug, Clone)]
pub struct RaidOptions {
    pub project: PathBuf,
    pub stash: StashPhaseOpts,
    pub rinse: RinsePhaseOpts,
    pub pack: PackPhaseOpts,
}

#[derive(Debug, Clone)]
pub struct StashPhaseOpts {
    pub enabled: bool,
    pub min_risk: SensitiveRisk,
    pub remove_sources: bool,
}

#[derive(Debug, Clone)]
pub struct RinsePhaseOpts {
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct PackPhaseOpts {
    pub enabled: bool,
    pub deny_content_secrets: bool,
}

impl Default for RaidOptions {
    fn default() -> Self {
        Self {
            project: PathBuf::new(), // caller must set
            stash: StashPhaseOpts {
                enabled: true,
                min_risk: SensitiveRisk::High,
                remove_sources: true,
            },
            rinse: RinsePhaseOpts { enabled: true },
            pack: PackPhaseOpts {
                enabled: true,
                deny_content_secrets: true,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidStageResult {
    pub name: String,          // "stash" | "rinse" | "pack" | "move"
    pub success: bool,
    pub message: String,       // без secrets
    pub skipped: bool,         // phase disabled
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaidResult {
    pub project_path: PathBuf,
    pub stages: Vec<RaidStageResult>,
    pub stash: Option<StashResult>,
    pub rinse: Option<RinseResult>,
    pub pack: Option<PackResult>,
    /// Итоговые пути в den (secrets archive, project archive, manifest).
    pub den_artifacts: Vec<PathBuf>,
    /// true только если каждая enabled-фаза success.
    pub success: bool,
    pub dry_run: bool,
}

/// identity нужен, если stash.enabled; иначе игнорируется.
pub fn raid(
    ctx: &AppContext,
    opts: &RaidOptions,
    identity: Option<&AgeIdentity>,
    progress: &mut dyn ProgressSink,
) -> Result<RaidResult>;
```

**Порядок фаз (фиксированный):**

1. **stash** — если enabled; fail → следующие фазы **не** выполняются (fail-fast), `success: false`.
2. **rinse** — если enabled.
3. **pack** — если enabled.
4. **move** — регистрация/перенос staging → финальные пути den, запись manifest JSON.

**Progress:** `phase_count = число enabled + 1 (move)`, `OperationKind::Raid`.

**Политика ошибок:**  
частичный успех возможен, только если явно введён флаг `continue_on_error` (v2). В v1 — fail-fast после первой failed enabled phase; уже созданные артефакты **не** откатываются автоматически (пути возвращаются в `den_artifacts` / stages message).

---

## 9. Структура den на диске

Den — **корень вывода**, не git-repo проектов. Пользователь указывает `den_dir` (default например `~/.raccpack/den`).

### 9.1. Дерево (v1)

```text
{den_dir}/
├── README.txt                 # кратко: что это за каталог (писать при первом raid)
├── .den-version               # например "1"
│
├── manifests/
│   └── {yyyy}/{mm}/
│       └── {project_slug}__{utc_timestamp}__{short_id}.json
│
├── secrets/
│   └── {yyyy}/{mm}/
│       └── {project_slug}__{utc_timestamp}__secrets.age
│
├── packs/
│   └── {yyyy}/{mm}/
│       └── {project_slug}__{utc_timestamp}.tar.zst
│
├── staging/                   # временные файлы; можно чистить
│   └── {short_id}/
│
└── logs/                      # опционально
    └── {yyyy}/{mm}/
        └── {short_id}.log
```

### 9.2. Соглашения об именах

| Токен | Правило |
|-------|---------|
| `project_slug` | Имя директории проекта, sanitized: только `[a-zA-Z0-9._-]`, пробелы → `-`, длина ≤ 80. |
| `utc_timestamp` | `YYYYMMDDThhmmssZ` (UTC). |
| `short_id` | 8 hex символов (случайные или от hash path+time) для уникальности. |

Пример:

```text
secrets/2026/08/my-api__20260804T155230Z__secrets.age
packs/2026/08/my-api__20260804T155230Z.tar.zst
manifests/2026/08/my-api__20260804T155230Z__a1b2c3d4.json
```

### 9.3. Manifest JSON

Пишется **после** успешного (или частично успешного) raid. Без raw secrets.

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
    {
      "original_path": "/home/user/DEV/PROJS/my-api/.env",
      "risk": "High",
      "size_bytes": 412
    }
  ],
  "tool": {
    "name": "raccpack",
    "core_version": "0.1.0"
  }
}
```

Пути артефактов в manifest — **относительно `den_dir`**, чтобы den был переносим.

### 9.4. Staging

```text
staging/{short_id}/
  pack partial...
  secrets partial...
```

- Commit: rename/move в `packs/` и `secrets/`, затем удалить staging dir.
- Crash mid-raid: staging может остаться; команда `racc den gc` (позже) чистит staging старше N дней.
- DryRun: staging не создаётся (или создаётся и сразу удаляется — лучше не создавать).

### 9.5. `.den-version`

Одна строка: `1`.  
При несовместимом major — core отказывается писать и предлагает migrate tool (v2+).

### 9.6. README.txt (шаблон)

```text
This directory is a raccpack den (output vault).
- secrets/  encrypted secret batches (age)
- packs/    project archives (no secrets)
- manifests/ JSON metadata for each raid

Do not commit this tree to git.
Keep passphrase offline.
```

### 9.7. Права (рекомендация)

- При создании den: `0700` на Unix.
- Файлы `.age`: `0600`.
- Core не обязан enforce на Windows; документировать для пользователя.

### 9.8. Несколько проектов

Один den обслуживает много проектов: различие только в `project_slug` и timestamp.  
Общий «latest» symlink **не** обязателен в v1 (можно добавить `latest/{slug}` позже).

---

## 10. Соответствие CLI / TUI / Desktop

| UI действие | Facade |
|-------------|--------|
| «Сканировать» | `sniff` |
| «Найти секреты» | `dig` |
| «Только вынести секреты» | `stash` |
| «Почистить мусор» | `rinse` |
| «Упаковать» | `pack` |
| «Полный рейд» | `raid` |
| Progress bar / events | `ProgressSink` → CLI spinner / TUI redraw / Tauri `emit` |
| Dry-run toggle | `AppContext.mode` |
| Passphrase dialog | UI → `AgeIdentity::Passphrase` → `stash`/`raid` (не в Zustand long-term) |

JSON CLI:

```bash
racc sniff --root PATH --json
racc dig --root PATH --json
racc raid --project PATH --den PATH --yes --json
```

`--json` печатает соответствующий `*Result` serde.

---

## 11. Минимальный порядок реализации API

1. DTO + `ProgressEvent` + `AppContext`  
2. `sniff` + cache  
3. `dig`  
4. `pack` (без content deny) → + deny  
5. `stash` (age)  
6. `rinse`  
7. `raid` склеивает 4–6 + manifest/den layout  
8. `den gc` / verify — после стабилизации layout  

---

## 12. Инварианты (кратко)

1. Facade **не** возвращает raw secret material в `*Result`.  
2. DryRun не создаёт файлов в `secrets/` и `packs/` и не удаляет источники.  
3. Имена в den уникальны за счёт timestamp + short_id.  
4. Manifest paths — relative to den root.  
5. Fail-fast raid не удаляет уже записанные age/pack автоматически.  
6. Все path’ы в API — `PathBuf`; UI нормализует до вызова (canonicalize по возможности).

Этого достаточно, чтобы CLI, TUI и Tauri BFF опирались на один контракт без дублирования политики den и фаз raid.
