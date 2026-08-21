//! Facade use-case `dig`: locate and classify sensitive files.
//!
//! Read-only: never writes to the den, never touches the cache, and never calls
//! age/stash. Results carry only masked values — no raw secrets leave the scan.
//! [`dig`] maps one [`crate::secrets::scan_secrets`] walk into public DTOs and
//! optionally aggregates repeated values by their masked hash; exit-policy is
//! applied separately via [`exit_code_for_secrets`].

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::domain::{Result, SensitiveRisk};
use crate::git::{find_repo_root, GitClient, GitFileStatus, ProcessGitClient};
use crate::scan::{ensure_scan_root, SkipPolicy};
use crate::secrets::{
    scan::scan_secrets_with_count, ContentScanLimits, MaskedValue, SecretScanOptions,
    SensitiveFinding,
};

use super::context::{AppContext, SecretExitPolicy};
use super::progress::{OperationKind, ProgressEvent, ProgressSink};

/// Options controlling [`dig`].
#[derive(Debug, Clone)]
pub struct DigOptions {
    /// Scan just this directory when set (may be outside `scan_root`); None →
    /// the whole `ctx.paths.scan_root`.
    pub project: Option<PathBuf>,
    /// Detect values repeating across two or more files.
    pub find_repeated: bool,
    /// Scan file contents in addition to filenames. Default true.
    pub scan_content: bool,
    /// Reserved: entropy heuristics are not implemented on MVP and are ignored
    /// without error.
    pub use_heuristics: Option<bool>,
}

impl Default for DigOptions {
    fn default() -> Self {
        Self {
            project: None,
            find_repeated: false,
            scan_content: true,
            use_heuristics: None,
        }
    }
}

/// A sensitive file discovered by [`dig`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensitiveFile {
    /// Path of the sensitive file.
    pub path: PathBuf,
    /// Computed severity (max over all sources).
    pub risk: SensitiveRisk,
    /// Filename and/or content labels.
    pub labels: Vec<String>,
    /// Masked preview of the highest-risk content hit, if any.
    pub content_match: Option<MaskedValue>,
    /// Git status of the file as a stable snake_case string (`"tracked"`,
    /// `"untracked"`, `"ignored"`, …) when the scanned root sits inside a git
    /// working tree; `None` outside a repo or whenever git is unavailable
    /// (missing binary, timeout, error) — git problems never fail a dig run.
    pub git_status: Option<String>,
}

/// A value repeating across two or more sensitive files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepeatedSecret {
    /// Stable hash of the raw value; never the value itself.
    pub value_hash: String,
    /// Masked preview of the value.
    pub masked: String,
    /// Highest risk across all occurrences.
    pub risk: SensitiveRisk,
    /// All files containing the value.
    pub paths: Vec<PathBuf>,
    /// Number of files containing the value.
    pub count: usize,
}

/// Outcome of a [`dig`] run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigResult {
    /// The directory that was scanned.
    pub root: PathBuf,
    /// Sensitive files found, sorted by path then risk.
    pub files: Vec<SensitiveFile>,
    /// Repeated values across files (only when requested).
    pub repeated: Vec<RepeatedSecret>,
    /// Wall-clock duration of the run in milliseconds.
    pub duration_ms: u64,
    /// Number of regular files walked (with or without a finding).
    pub files_scanned: u64,
}

/// Find and classify sensitive files under `opts.project` or the scan root.
///
/// Validates the root with [`ensure_scan_root`], runs the combined secret scan
/// ([`crate::secrets::scan_secrets`]) with `max_depth` from
/// `ctx.config.scanner.max_depth`, `SkipPolicy::default_scan()`, and the
/// `scan_content` / `find_repeated` flags from `opts`, then enriches findings
/// with git status via a default [`ProcessGitClient`]. Progress is emitted as
/// a single `"dig"` phase at 0 / 50 / 100 percent. The exit policy is never
/// applied here — callers use [`exit_code_for_secrets`] on the returned files.
pub fn dig(
    ctx: &AppContext,
    opts: &DigOptions,
    progress: &mut dyn ProgressSink,
) -> Result<DigResult> {
    let git = ProcessGitClient::new();
    dig_with_git(ctx, opts, progress, &git)
}

/// [`dig`] with an injected [`GitClient`] (mock in tests, custom backends).
///
/// Git enrichment is best-effort: any client error degrades every
/// `git_status` to `None` while the run itself stays `Ok`.
pub fn dig_with_git(
    ctx: &AppContext,
    opts: &DigOptions,
    progress: &mut dyn ProgressSink,
    git: &dyn GitClient,
) -> Result<DigResult> {
    let t0 = Instant::now();

    let root = opts
        .project
        .clone()
        .unwrap_or_else(|| ctx.paths.scan_root.clone());
    ensure_scan_root(&root)?;

    progress.emit(dig_event(0, "Digging for secrets…", false));

    let secret_opts = SecretScanOptions {
        max_depth: ctx.config.scanner.max_depth,
        policy: SkipPolicy::default_scan(),
        min_risk: SensitiveRisk::Low,
        scan_content: opts.scan_content,
        limits: ContentScanLimits::default(),
        find_repeated: opts.find_repeated,
    };

    let (findings, files_scanned) = scan_secrets_with_count(&root, &secret_opts)?;

    progress.emit(dig_event(
        50,
        format!("Scanned {files_scanned} files…"),
        false,
    ));

    let files: Vec<SensitiveFile> = findings.iter().map(finding_to_file).collect();
    let files = enrich_with_git_status(files, &root, git);
    let repeated = if opts.find_repeated {
        aggregate_by_hash(&findings)
    } else {
        Vec::new()
    };

    progress.emit(dig_event(
        100,
        format!("Found {} sensitive paths", files.len()),
        true,
    ));

    Ok(DigResult {
        root,
        files,
        repeated,
        duration_ms: elapsed_ms(t0),
        files_scanned,
    })
}

/// Exit code for the CLI after a secret phase: 0 or 2, never other values.
///
/// `Ignore` never fails; `FailOnCritical` fails only on CRITICAL findings;
/// `FailOnHighOrAbove` fails on HIGH-or-above. IO errors stay exit 1 at the CLI
/// layer and never reach this helper.
pub fn exit_code_for_secrets(files: &[SensitiveFile], policy: SecretExitPolicy) -> i32 {
    match policy {
        SecretExitPolicy::Ignore => 0,
        SecretExitPolicy::FailOnCritical => {
            if files.iter().any(|f| f.risk == SensitiveRisk::Critical) {
                2
            } else {
                0
            }
        }
        SecretExitPolicy::FailOnHighOrAbove => {
            if files.iter().any(|f| f.risk >= SensitiveRisk::High) {
                2
            } else {
                0
            }
        }
    }
}

fn finding_to_file(finding: &SensitiveFinding) -> SensitiveFile {
    SensitiveFile {
        path: finding.path.clone(),
        risk: finding.risk,
        labels: finding.labels.clone(),
        content_match: finding.content_match.clone(),
        git_status: None,
    }
}

/// Best-effort git enrichment: fill `git_status` on each finding.
///
/// Skips entirely when there are no findings (git is never invoked). Any
/// failure — not a repo, client error, missing path in the answer — leaves the
/// affected `git_status` as `None`; this function never returns an error.
fn enrich_with_git_status(
    mut files: Vec<SensitiveFile>,
    root: &Path,
    git: &dyn GitClient,
) -> Vec<SensitiveFile> {
    if files.is_empty() {
        return files;
    }
    if let Some(statuses) = collect_git_statuses(root, &files, git) {
        for file in &mut files {
            if let Some(status) = statuses.get(&file.path) {
                file.git_status = Some(status.as_str().to_string());
            }
        }
    }
    files
}

/// Resolve statuses for all finding paths, or `None` when git cannot help.
///
/// The repo root is the nearest `.git` ancestor; when none exists the scanned
/// root itself is offered to the client, which stays the single authority on
/// "is this a repo".
fn collect_git_statuses(
    root: &Path,
    files: &[SensitiveFile],
    git: &dyn GitClient,
) -> Option<HashMap<PathBuf, GitFileStatus>> {
    let repo = find_repo_root(root).unwrap_or_else(|| root.to_path_buf());
    if !git.is_repo(&repo).ok()? {
        return None;
    }
    let paths: Vec<PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    git.files_status(&repo, &paths).ok()
}

/// Aggregate masked content hits by `value_hash`, keeping only hashes that
/// occur in two or more findings. Ordering is deterministic: risk descending,
/// then `value_hash` ascending.
fn aggregate_by_hash(findings: &[SensitiveFinding]) -> Vec<RepeatedSecret> {
    let mut groups: BTreeMap<String, HashGroup> = BTreeMap::new();
    for finding in findings {
        let Some(masked) = &finding.content_match else {
            continue;
        };
        let group = groups
            .entry(masked.value_hash.clone())
            .or_insert_with(|| HashGroup {
                masked: masked.masked.clone(),
                risk: finding.risk,
                paths: Vec::new(),
                count: 0,
            });
        group.risk = group.risk.max(finding.risk);
        group.paths.push(finding.path.clone());
        group.count += 1;
    }

    let mut repeated: Vec<RepeatedSecret> = groups
        .into_iter()
        .filter(|(_, group)| group.count >= 2)
        .map(|(value_hash, group)| RepeatedSecret {
            value_hash,
            masked: group.masked,
            risk: group.risk,
            paths: group.paths,
            count: group.count,
        })
        .collect();

    repeated.sort_by(|a, b| b.risk.cmp(&a.risk).then(a.value_hash.cmp(&b.value_hash)));
    repeated
}

/// Mutable aggregation state for one repeated value hash.
struct HashGroup {
    masked: String,
    risk: SensitiveRisk,
    paths: Vec<PathBuf>,
    count: usize,
}

/// Build a progress event for the single `"dig"` phase.
fn dig_event(percent: u8, message: impl Into<String>, phase_complete: bool) -> ProgressEvent {
    ProgressEvent {
        operation: OperationKind::Dig,
        phase: "dig".to_string(),
        phase_index: 0,
        phase_count: 1,
        percent,
        overall_percent: percent,
        message: message.into(),
        phase_complete,
    }
}

/// Wall-clock milliseconds since `t0`, saturating at `u64::MAX`.
fn elapsed_ms(t0: Instant) -> u64 {
    u64::try_from(t0.elapsed().as_millis()).unwrap_or(u64::MAX)
}
