---
title: Facade API (public contract)
description: The public contract of the raccpack core — base types, sniff/dig/stash/rinse/pack/raid operations, and invariants.
---

# Facade API (public contract)

The facade is the single public contract of the core on which all interfaces run (CLI, TUI, Desktop). If you are writing an integration or automation — these are the data structures and operations guaranteed to be stable.

::: info
This section targets users automating raccpack (CI scripts, tools). Signatures are shown in simplified form; exact names and fields live in the `raccpack-core` crate.
:::

## Base types

### Paths and mode

```rust
pub struct WorkspacePaths {
    pub scan_root: PathBuf,   // input: where projects live
    pub den_dir: PathBuf,     // output: the den storage
}

pub enum RunMode {
    DryRun,   // report only; writes and deletes nothing
    Commit,   // real changes
}
```

- `DryRun` — the default mode for destructive operations: nothing is created in `secrets/` or `packs/`, sources are not removed.
- `Commit` — real execution: archives, trash removal, moving secrets out.

### Secret exit policy

```rust
pub enum SecretExitPolicy {
    Ignore,              // always 0 when there are no errors
    FailOnCritical,      // code 2 on Critical
    FailOnHighOrAbove,   // code 2 on High and above
}
```

Applied by the CLI to the exit code; inside the core operations always succeed as long as scanning completed without errors.

### Progress

Long operations accept a `ProgressSink` — a callback receiving events:

```rust
pub struct ProgressEvent {
    pub operation: OperationKind,   // Sniff | Dig | Stash | Rinse | Pack | Raid
    pub phase: String,              // "stash" | "rinse" | "pack" | "move" | "scan"
    pub phase_index: u32,
    pub phase_count: u32,
    pub percent: u8,                // 0..=100
    pub overall_percent: u8,
    pub message: String,            // human-readable, no raw secrets
    pub phase_complete: bool,
}
```

The CLI uses this for the spinner/progress, the TUI for repainting, Desktop for Tauri events.

### Session context

```rust
pub struct AppContext {
    pub config: RaccConfig,
    pub paths: WorkspacePaths,
    pub mode: RunMode,
    pub secret_groups_override: Option<EnabledGroups>,
    pub exit_policy: SecretExitPolicy,
}
```

An interface builds `AppContext` once per session and passes it into all calls.

## Operations

### `sniff` - discover projects

```rust
pub struct SniffOptions {
    pub force_refresh: bool,   // ignore cache
    pub max_depth: Option<usize>,
}

pub struct SniffResult {
    pub report: ScanReport,   // { root, projects, total_size_bytes, schema_version }
    pub from_cache: bool,     // true when the result came from cache
    pub duration_ms: u64,
}

pub fn sniff(ctx: &AppContext, opts: &SniffOptions,
             progress: &mut dyn ProgressSink) -> Result<SniffResult>;
```

**Status: implemented.** CLI: `racc sniff`.

### `dig` - find secrets

```rust
pub struct DigOptions {
    pub project: Option<PathBuf>,  // limit to one project
    pub find_repeated: bool,       // look for repeated values
    pub scan_content: bool,        // read contents (default true)
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

// Helper for the exit code
pub fn exit_code_for_secrets(files: &[SensitiveFile], policy: SecretExitPolicy) -> i32;
```

`SensitiveFile` and `RepeatedSecret` carry only **masked** data: path, risk, labels, masked value, hash. No raw values.

**Status: implemented.** CLI: `racc dig`.

### `stash` - move secrets into an age archive

```rust
pub enum AgeIdentity {
    Passphrase(String),      // passphrase (zeroized after use)
    Recipients(Vec<String>), // public age recipient keys
}

pub struct StashOptions {
    pub target: PathBuf,
    pub only_files: Option<Vec<PathBuf>>,
    pub min_risk: SensitiveRisk,   // default High
    pub remove_sources: bool,      // remove originals (Commit only)
    pub batch_id: Option<String>,
}

pub fn stash(ctx: &AppContext, opts: &StashOptions, identity: &AgeIdentity,
             progress: &mut dyn ProgressSink) -> Result<StashResult>;
```

Behavior:

- `DryRun` — computes the list and the future archive path, **writing** and **deleting** nothing.
- `Commit` — writes the `.age` archive to `den/secrets/…`, removing sources when `remove_sources: true`.
- The passphrase is never returned and never appears in error messages.

**Status: implemented.** CLI: `racc stash`.

### `rinse` - clean build trash

```rust
pub struct RinseOptions {
    pub target: PathBuf,             // project
    pub strategies: Option<Vec<String>>,
    pub include_custom_patterns: bool,
}

pub fn rinse(ctx: &AppContext, opts: &RinseOptions,
             progress: &mut dyn ProgressSink) -> Result<RinseResult>;
```

`DryRun` only lists what would be removed; `Commit` removes directories. Rinse never touches secret files — that's `stash`'s job.

**Status: implemented.** CLI: `racc rinse`.

### `pack` - pack a project

```rust
pub struct PackOptions {
    pub project: PathBuf,
    pub output_name: Option<String>,  // default {slug}__{ts}.tar.zst
    pub deny_content_secrets: bool,   // check contents while packing
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

The packing core (`pack_tree`) and the facade `pack` (DryRun/Commit) are implemented, as is the `racc pack` CLI command.

**Status: core and CLI implemented (MVP 0.1).**

### `raid` - full cycle

```rust
pub enum OrchestrationMode {
    Atomic,     // default: staging + deferred removals, WAL rollback
    FailFast,   // legacy A3.1: stop at the first failed phase
}

pub struct RaidOptions {
    pub project: PathBuf,
    pub mode: OrchestrationMode,   // default Atomic
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
    pub den_artifacts: Vec<PathBuf>,   // final paths in the den
    pub success: bool,
    pub dry_run: bool,
    pub rolled_back: bool,             // failed commit rolled back to pre-raid
    pub rollback_warnings: Vec<String>,// non-fatal issues during rollback
}

pub fn raid(ctx: &AppContext, opts: &RaidOptions, identity: Option<&AgeIdentity>,
            progress: &mut dyn ProgressSink) -> Result<RaidResult>;
```

Fixed phase order: **stash → rinse → pack → move**. In the default mode (`OrchestrationMode::Atomic`) artifacts are written to a temporary `den/staging/{id}/` and moved into the den only at commit; a failed commit rolls back via WAL — the report gets `rolled_back: true`. In `FailFast` mode (the `--fail-fast` flag), after the first failed phase the following phases do not run, while already-written artifacts remain in the den.

**Status: implemented.** CLI: `racc raid`.

### `raid_batch` - batch raid across projects

Discovers all projects under a root directory and runs `raid()` on each one. Projects are found via the same candidate discovery as `sniff`. Per-project errors are captured in the result and do not abort the batch (unless `stop_on_project_failure` is set).

```rust
pub struct RaidBatchOptions {
    pub root: PathBuf,                        // root directory to scan for projects
    pub raid: RaidOptions,                    // shared per-project raid config; `project` is overwritten per candidate
    pub only: Vec<String>,                    // substring filter on project name or path
    pub limit: Option<usize>,                 // cap on the number of projects to raid
    pub stop_on_project_failure: bool,        // stop the batch after the first project failure
}

pub struct RaidBatchResult {
    pub root: PathBuf,
    pub dry_run: bool,
    pub projects_total: usize,                // total candidates discovered
    pub projects_run: usize,                  // after filtering and limiting
    pub results: Vec<RaidBatchItem>,
    pub success: bool,                        // false if any project failed or errored
}

pub struct RaidBatchItem {
    pub project_path: PathBuf,
    pub project_name: String,
    pub outcome: RaidBatchOutcome,
}

pub enum RaidBatchOutcome {
    Raided(Box<RaidResult>),                  // raid completed (check RaidResult::success)
    Skipped { reason: String },               // filtered out or limit reached
    Error { message: String },                // raid returned an Err
}

pub fn raid_batch(
    ctx: &AppContext,
    opts: &RaidBatchOptions,
    identity: Option<&AgeIdentity>,
    progress: &mut dyn ProgressSink,
) -> Result<RaidBatchResult>;
```

Behavior:

- Projects are discovered via `find_candidates` under `opts.root`.
- Each candidate is filtered by `only` (substring match on name or path) and capped by `limit`.
- The shared `opts.raid` config is cloned per project; the `project` field is overwritten to the candidate's path.
- Per-project raid errors are captured as `RaidBatchOutcome::Error` and do not abort the batch unless `stop_on_project_failure: true`.
- `DryRun` / `Commit` behavior follows the embedded `RaidOptions`.

**Status: implemented.** Facade only (no CLI command yet).

## Reports and data

### Stable DTOs (serde-friendly)

- `ScanReport { root, projects, total_size_bytes, schema_version }`
- `Project { path, name, stack, size_bytes, is_git_repo }`
- `Stack { language, frameworks, markers }`
- `SensitiveFile { path, risk, labels, content_match?, git_status? }`
- `SensitiveRisk` — `Low | Medium | High | Critical`
- `MaskedValue { masked, value_hash, original_len }`

Reports serialize to JSON (`--json`) and include `schema_version` for compatibility checks in CI.

### Raid manifest (JSON)

After each raid a manifest is written to `den/manifests/{yyyy}/{mm}/`. Example (no raw secrets):

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
  "tool": { "name": "raccpack", "core_version": "0.3.9" }
}
```

Artifact paths are **relative to the den root**, so the den can be moved as a whole.

## Contract invariants

1. The facade never returns raw secret material in results.
2. `DryRun` creates no files in `secrets/` or `packs/` and removes no sources.
3. Names in the den are unique thanks to timestamp + short_id.
4. Paths in manifests are relative to the den root.
5. In `Atomic` mode (default) a failed commit rolls back via WAL (`rolled_back`); in `FailFast` mode already-written artifacts remain in the den.
6. All paths in the API are `PathBuf`; interfaces normalize them before calls.

## See also

- [Architecture](/architecture) — layers and trust boundaries.
- [Roadmap](/roadmap) — the status of each operation.
