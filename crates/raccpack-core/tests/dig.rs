//! Integration tests for M3.3 — facade `dig`.
//!
//! Covers the 9 required cases from docs/mvp/m3/m3.3-facade-dig.md §7: masked
//! findings with correct risk, no raw secrets in JSON output, repeated-secret
//! aggregation, `scan_content: false`, `project: Some(subdir)`, empty-tree
//! behavior, `exit_code_for_secrets` on CRITICAL, progress `phase_complete`,
//! and SkipPolicy. Extras cover exit policies, serde roundtrip,
//! `files_scanned` counting clean files, absolute project outside scan_root,
//! labels, `git_status == None`, and `DigResult.root` defaults.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//! `duration_ms` is never asserted (timing is flaky).

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{
    dig, exit_code_for_secrets, AppContext, DigOptions, DigResult, NullProgress, OperationKind,
    ProgressEvent, ProgressSink, RaccConfig, RunMode, SecretExitPolicy, SensitiveFile,
    SensitiveRisk,
};
use tempfile::TempDir;

/// A deterministic AWS-style access key id (matches the `aws_access_key` prefix).
const AWS_ACCESS_KEY: &str = "AKIAABCDEFGHIJKLMNOPQRST";

/// A long, distinctive password value used to prove raw values never serialize.
const PASSWORD_VALUE: &str = "supersecretvalue1234567890";

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`, returning its path.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
    path
}

/// Create a workspace: a `TempDir` with an existing `projects/` scan root.
fn workspace() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create work dir");
    let projects = temp.path().join("projects");
    fs::create_dir_all(&projects).expect("create projects dir");
    (temp, projects)
}

/// Build an `AppContext` from a config pointing at `root` (den is derived as a
/// sibling of the scan root so no real `~/.raccpack/den` is ever touched).
fn ctx_for(root: &Path) -> AppContext {
    let den = root.parent().expect("scan root has a parent").join("den");
    let config = RaccConfig::default()
        .with_scan_root(root)
        .with_den_dir(&den);
    AppContext::from_config(config, RunMode::DryRun).expect("AppContext::from_config")
}

/// Default dig options scanning the whole scan root with content enabled.
fn dig_options(project: Option<PathBuf>, find_repeated: bool, scan_content: bool) -> DigOptions {
    DigOptions {
        project,
        find_repeated,
        scan_content,
        use_heuristics: None,
    }
}

/// Run dig with a `NullProgress` sink and return the result.
fn dig_once(ctx: &AppContext, opts: &DigOptions) -> DigResult {
    let mut progress = NullProgress;
    dig(ctx, opts, &mut progress).expect("dig should succeed")
}

/// Run dig recording every progress event; returns the events.
fn dig_recorded(ctx: &AppContext, opts: &DigOptions) -> Vec<ProgressEvent> {
    let mut sink = RecordingSink::default();
    dig(ctx, opts, &mut sink).expect("dig should succeed");
    sink.events
}

/// Sink that collects emitted events for assertions.
#[derive(Default)]
struct RecordingSink {
    events: Vec<ProgressEvent>,
}

impl ProgressSink for RecordingSink {
    fn emit(&mut self, event: ProgressEvent) {
        self.events.push(event);
    }
}

/// Find the `SensitiveFile` for `path`, panicking with context if absent.
fn file_by_path<'a>(files: &'a [SensitiveFile], path: &Path) -> &'a SensitiveFile {
    files
        .iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("no finding for {}", path.display()))
}

/// Build a bare `SensitiveFile` with a given risk for exit-code unit tests.
fn sf(risk: SensitiveRisk) -> SensitiveFile {
    SensitiveFile {
        path: PathBuf::from("/tmp/dummy-secret"),
        risk,
        labels: Vec::new(),
        content_match: None,
        git_status: None,
    }
}

// --- Mandatory case 1: risk + masked content ---------------------------------

#[test]
fn dig_reports_env_and_aws_key_with_masked_content() {
    let (temp, root) = workspace();
    let _ = temp;
    let env = write(&root, ".env", "PASSWORD=supersecretvalue123\n");
    let aws = write(&root, "creds.txt", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, true));

    let env_f = file_by_path(&result.files, &env);
    assert_eq!(
        env_f.risk,
        SensitiveRisk::High,
        ".env filename match must be High"
    );
    assert!(env_f.content_match.is_some(), ".env carries a content hit");

    let aws_f = file_by_path(&result.files, &aws);
    assert_eq!(
        aws_f.risk,
        SensitiveRisk::Critical,
        "AKIA content is Critical"
    );
    let cm = aws_f
        .content_match
        .as_ref()
        .expect("the AWS file carries a content match");
    assert!(
        cm.masked.starts_with("AKIA"),
        "masked preview keeps the safe prefix: {}",
        cm.masked
    );
    assert!(
        !cm.masked.contains(AWS_ACCESS_KEY),
        "masked preview must not contain the raw value: {}",
        cm.masked
    );
}

// --- Mandatory case 2: no raw in JSON ---------------------------------------

#[test]
fn dig_json_contains_no_raw_secret() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n"));
    write(&root, "creds.txt", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, true));

    let json = serde_json::to_string_pretty(&result).expect("serialize DigResult");
    assert!(
        !json.contains(AWS_ACCESS_KEY),
        "JSON must never contain the raw AWS key"
    );
    assert!(
        !json.contains(PASSWORD_VALUE),
        "JSON must never contain the raw password"
    );
    assert_eq!(result.files.len(), 2);
}

// --- Mandatory case 3: repeated secrets --------------------------------------

#[test]
fn repeated_secrets_grouped_by_hash() {
    let (temp, root) = workspace();
    let _ = temp;
    let a = write(&root, "a.txt", &format!("{AWS_ACCESS_KEY}\n"));
    let b = write(&root, "b.txt", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, true, true));

    assert_eq!(result.files.len(), 2);
    assert_eq!(result.repeated.len(), 1, "one distinct repeated value");
    let repeated = &result.repeated[0];
    assert_eq!(repeated.count, 2);
    assert_eq!(repeated.paths.len(), 2);
    assert_eq!(
        repeated.risk,
        SensitiveRisk::Critical,
        "risk must be the max across occurrences"
    );
    assert!(
        !repeated.masked.contains(AWS_ACCESS_KEY),
        "masked preview must not contain the raw value: {}",
        repeated.masked
    );
    assert!(repeated.masked.starts_with("AKIA"));
    assert_eq!(repeated.value_hash.len(), 64, "blake3 hex is 64 chars");

    let mut sorted_paths = repeated.paths.clone();
    sorted_paths.sort();
    let mut expected = vec![a, b];
    expected.sort();
    assert_eq!(sorted_paths, expected, "both files must be listed");
}

#[test]
fn repeated_secrets_disabled_returns_empty() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, "a.txt", &format!("{AWS_ACCESS_KEY}\n"));
    write(&root, "b.txt", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, true));

    assert_eq!(result.files.len(), 2);
    assert!(
        result.repeated.is_empty(),
        "find_repeated=false must not aggregate"
    );
}

#[test]
fn repeated_password_secrets_grouped() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, "a/.env", "PASSWORD=supersecretvalue123\n");
    write(&root, "b/.env", "PASSWORD=supersecretvalue123\n");
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, true, true));

    assert_eq!(result.files.len(), 2);
    assert_eq!(result.repeated.len(), 1, "the shared password must group");
    let repeated = &result.repeated[0];
    assert_eq!(repeated.count, 2);
    assert_eq!(repeated.risk, SensitiveRisk::High);
    assert!(
        !repeated.masked.contains("supersecretvalue123"),
        "masked preview must not contain the raw password"
    );
}

// --- Mandatory case 4: scan_content: false -----------------------------------

#[test]
fn dig_scan_content_false_is_filename_only() {
    let (temp, root) = workspace();
    let _ = temp;
    let env = write(&root, ".env", "APP=x\n");
    write(&root, "notes.txt", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, false));

    assert_eq!(result.files.len(), 1, "only the filename hit may survive");
    assert_eq!(result.files[0].path, env);
    assert!(
        result.files[0].content_match.is_none(),
        "no content is read, so no content_match"
    );
}

// --- Mandatory case 5: project: Some(subdir) ---------------------------------

#[test]
fn dig_project_subdir_limits_findings() {
    let (temp, root) = workspace();
    let _ = temp;
    let project = root.join("sub");
    write(&project, ".env", "APP=x\n");
    write(&root, "sub2/.env", "APP=y\n");
    let ctx = ctx_for(&root);

    let result = dig_once(&ctx, &dig_options(Some(project.clone()), false, true));
    assert_eq!(
        result.root, project,
        "DigResult.root must be the project path"
    );
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, project.join(".env"));
}

// --- Mandatory case 6: empty tree --------------------------------------------

#[test]
fn dig_empty_tree_empty_files_and_zero_exit() {
    let (temp, root) = workspace();
    let _ = temp;
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, true));

    assert!(result.files.is_empty(), "empty tree => no findings");
    assert!(result.repeated.is_empty());
    for policy in [
        SecretExitPolicy::Ignore,
        SecretExitPolicy::FailOnCritical,
        SecretExitPolicy::FailOnHighOrAbove,
    ] {
        assert_eq!(
            exit_code_for_secrets(&[], policy),
            0,
            "empty findings => exit 0 under {policy:?}"
        );
    }
}

// --- Mandatory case 7: CRITICAL => exit 2 ------------------------------------

#[test]
fn exit_code_fail_on_critical_is_two_with_critical() {
    let critical = sf(SensitiveRisk::Critical);
    assert_eq!(
        exit_code_for_secrets(&[critical], SecretExitPolicy::FailOnCritical),
        2
    );

    let high = sf(SensitiveRisk::High);
    assert_eq!(
        exit_code_for_secrets(&[high], SecretExitPolicy::FailOnCritical),
        0,
        "High alone must not fail FailOnCritical"
    );
}

// --- Mandatory case 8: progress ----------------------------------------------

#[test]
fn dig_progress_emits_phase_complete() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, ".env", "PASSWORD=supersecretvalue123\n");
    let ctx = ctx_for(&root);
    let opts = dig_options(None, false, true);

    let events = dig_recorded(&ctx, &opts);
    assert!(
        !events.is_empty(),
        "dig must emit at least one progress event"
    );
    assert!(
        events.iter().all(|e| e.operation == OperationKind::Dig),
        "all events belong to Dig"
    );
    assert!(
        events.iter().all(|e| e.phase == "dig"),
        "the dig phase name must be \"dig\""
    );

    let first = &events[0];
    assert!(
        !first.phase_complete,
        "the first event marks the phase as not complete"
    );
    assert_eq!(first.overall_percent, 0);

    let last = events.last().expect("events is non-empty");
    assert!(
        last.phase_complete,
        "the final event marks the phase complete"
    );
    assert_eq!(last.percent, 100);
    assert_eq!(last.overall_percent, 100);

    assert!(
        events.iter().any(|e| {
            e.operation == OperationKind::Dig
                && e.phase == "dig"
                && e.phase_complete
                && e.overall_percent == 100
        }),
        "at least one event must be the completed dig phase at 100%"
    );
    assert!(
        events
            .windows(2)
            .all(|w| w[0].overall_percent <= w[1].overall_percent),
        "overall progress must never decrease"
    );
}

// --- Mandatory case 9: SkipPolicy --------------------------------------------

#[test]
fn dig_skips_node_modules_content() {
    let (temp, root) = workspace();
    let _ = temp;
    let env = write(&root, ".env", &format!("{AWS_ACCESS_KEY}\n"));
    write(
        &root,
        "node_modules/pkg/.env",
        &format!("{AWS_ACCESS_KEY}\n"),
    );
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, true));

    assert_eq!(
        result.files.len(),
        1,
        "node_modules must never be descended: {:#?}",
        result.files
    );
    assert_eq!(result.files[0].path, env, "only the top-level .env");
}

// --- Extras: exit-code policies ----------------------------------------------

#[test]
fn exit_code_ignore_is_always_zero() {
    let files = vec![sf(SensitiveRisk::Critical), sf(SensitiveRisk::High)];
    assert_eq!(exit_code_for_secrets(&files, SecretExitPolicy::Ignore), 0);
}

#[test]
fn exit_code_fail_on_high_or_above() {
    let high = sf(SensitiveRisk::High);
    assert_eq!(
        exit_code_for_secrets(&[high], SecretExitPolicy::FailOnHighOrAbove),
        2
    );

    for risk in [SensitiveRisk::Medium, SensitiveRisk::Low] {
        assert_eq!(
            exit_code_for_secrets(&[sf(risk)], SecretExitPolicy::FailOnHighOrAbove),
            0,
            "risk {risk:?} must not fail FailOnHighOrAbove"
        );
    }
}

#[test]
fn exit_code_mixed_files_require_any_high_or_critical() {
    let files = vec![sf(SensitiveRisk::Medium), sf(SensitiveRisk::Low)];
    assert_eq!(
        exit_code_for_secrets(&files, SecretExitPolicy::FailOnHighOrAbove),
        0
    );
    let files = vec![sf(SensitiveRisk::Critical), sf(SensitiveRisk::Low)];
    assert_eq!(
        exit_code_for_secrets(&files, SecretExitPolicy::FailOnCritical),
        2
    );
}

// --- Extras: DTO / result invariants ------------------------------------------

#[test]
fn dig_result_serde_roundtrip() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, ".env", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, true, true));

    let json = serde_json::to_string(&result).expect("serialize DigResult");
    let decoded: DigResult = serde_json::from_str(&json).expect("deserialize DigResult");
    assert_eq!(decoded, result, "serde roundtrip must preserve DigResult");
}

#[test]
fn dig_root_equals_scan_root_without_project() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, ".env", "APP=x\n");
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, true));
    assert_eq!(result.root, root);
}

#[test]
fn dig_git_status_none_when_not_a_repo() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, ".env", &format!("{AWS_ACCESS_KEY}\n"));
    write(&root, "creds.txt", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, true, true));

    assert!(!result.files.is_empty());
    assert!(
        result.files.iter().all(|f| f.git_status.is_none()),
        "a plain directory without .git must leave every git_status None"
    );

    let json = serde_json::to_string(&result).expect("serialize DigResult");
    let decoded: DigResult = serde_json::from_str(&json).expect("deserialize DigResult");
    assert_eq!(
        decoded, result,
        "JSON roundtrip must survive git enrichment (all None here)"
    );
}

#[test]
fn dig_files_scanned_counts_clean_files() {
    let (temp, root) = workspace();
    let _ = temp;
    write(&root, "a.txt", "hello\n");
    write(&root, "b.txt", "world\n");
    write(&root, "c.txt", "xyz\n");
    write(&root, ".env", "APP=x\n");
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, true));

    assert_eq!(result.files.len(), 1);
    assert_eq!(
        result.files_scanned, 4,
        "files_scanned must count every regular file, clean or not"
    );
}

// --- Extras: project outside scan_root ----------------------------------------

#[test]
fn dig_project_absolute_outside_scan_root_allowed() {
    let (temp, root) = workspace();
    let _ = temp;
    let outside = TempDir::new().expect("create unrelated project dir");
    let env = write(outside.path(), ".env", "PASSWORD=supersecretvalue123\n");
    let ctx = ctx_for(&root);

    let result = dig_once(
        &ctx,
        &dig_options(Some(outside.path().to_path_buf()), false, true),
    );
    assert_eq!(
        result.root,
        outside.path(),
        "an absolute project path is scanned as-is"
    );
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, env);
}

// --- Extras: labels ------------------------------------------------------------

#[test]
fn dig_labels_include_filename_and_content() {
    let (temp, root) = workspace();
    let _ = temp;
    let env = write(&root, ".env", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&root);
    let result = dig_once(&ctx, &dig_options(None, false, true));

    let f = file_by_path(&result.files, &env);
    assert!(!f.labels.is_empty(), "findings carry human labels");
    assert!(
        f.labels.contains(&"Environment file".to_string()),
        "env_file label present: {:?}",
        f.labels
    );
    assert!(
        f.labels.contains(&"AWS access key".to_string()),
        "aws_access_key label present: {:?}",
        f.labels
    );
    assert_eq!(f.labels.len(), 2, "one filename + one content label");
}
