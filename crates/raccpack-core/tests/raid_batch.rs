//! Integration tests for D4.2 — raid_batch facade.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{
    AppContext, NullProgress, OrchestrationMode, PackPhaseOpts, RaccConfig, RaidBatchOptions,
    RaidBatchOutcome, RaidBatchResult, RaidOptions, RinsePhaseOpts, RunMode, SensitiveRisk,
    StashPhaseOpts,
};
use tempfile::TempDir;

fn workspace() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create work dir");
    let root = temp.path().join("projects");
    fs::create_dir_all(&root).expect("create projects dir");
    (temp, root)
}

fn create_project(root: &Path, name: &str) {
    let dir = root.join(name);
    fs::create_dir_all(dir.join("src")).expect("create src");
    fs::write(
        dir.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\n"),
    )
    .expect("write Cargo.toml");
    fs::write(dir.join("src/main.rs"), "fn main() {}\n").expect("write main.rs");
}

fn ctx_for(root: &Path, den: &Path, mode: RunMode) -> AppContext {
    let config = RaccConfig::default().with_scan_root(root).with_den_dir(den);
    AppContext::from_config(config, mode).expect("AppContext::from_config")
}

fn default_batch_opts(root: &Path) -> RaidBatchOptions {
    RaidBatchOptions {
        root: root.to_path_buf(),
        raid: RaidOptions {
            project: PathBuf::new(),
            mode: OrchestrationMode::Atomic,
            stash: StashPhaseOpts {
                enabled: false,
                min_risk: SensitiveRisk::High,
                remove_sources: false,
            },
            rinse: RinsePhaseOpts { enabled: false },
            pack: PackPhaseOpts {
                enabled: false,
                deny_content_secrets: true,
            },
        },
        only: Vec::new(),
        limit: None,
        stop_on_project_failure: false,
    }
}

fn raid_batch(ctx: &AppContext, opts: &RaidBatchOptions) -> RaidBatchResult {
    raccpack_core::raid_batch(ctx, opts, None, &mut NullProgress).expect("raid_batch")
}

// --- B1: root with 2 projects, DryRun → 2 items, zero den writes ---

#[test]
fn b1_dry_run_two_projects_zero_den_writes() {
    let (temp, root) = workspace();
    let den = temp.path().join("den");
    create_project(&root, "alpha");
    create_project(&root, "beta");

    let ctx = ctx_for(&root, &den, RunMode::DryRun);
    let opts = default_batch_opts(&root);
    let result = raid_batch(&ctx, &opts);

    assert_eq!(result.projects_total, 2);
    assert_eq!(result.projects_run, 2);
    assert!(result.success);
    assert!(result.dry_run);
    for item in &result.results {
        assert!(matches!(&item.outcome, RaidBatchOutcome::Raided(r) if r.dry_run));
    }
    assert!(!den.exists(), "zero den writes in dry run");
}

// --- B2: Commit → 2× distinct artifacts (check names) ---

#[test]
fn b2_commit_produces_distinct_artifacts() {
    let (temp, root) = workspace();
    let den = temp.path().join("den");
    create_project(&root, "alpha");
    create_project(&root, "beta");

    let ctx = ctx_for(&root, &den, RunMode::Commit);
    let opts = default_batch_opts(&root);
    let result = raid_batch(&ctx, &opts);

    assert!(result.success);
    assert_eq!(result.results.len(), 2);

    let names: Vec<_> = result
        .results
        .iter()
        .map(|i| i.project_name.clone())
        .collect();
    assert!(names.contains(&"alpha".to_string()));
    assert!(names.contains(&"beta".to_string()));
}

// --- B3: raid failure → Error variant, other projects still run ---

#[test]
fn b3_error_variant_captured_and_others_continue() {
    let (temp, root) = workspace();
    let den = temp.path().join("den");
    create_project(&root, "alpha");
    create_project(&root, "beta");

    let ctx = ctx_for(&root, &den, RunMode::DryRun);
    let mut opts = default_batch_opts(&root);
    // Enable stash with no identity → raid() returns Err for each project.
    opts.raid.stash.enabled = true;
    opts.raid.stash.min_risk = SensitiveRisk::Critical;

    let result =
        raccpack_core::raid_batch(&ctx, &opts, None, &mut NullProgress).expect("raid_batch call");
    // Both projects should error out (stash enabled + no identity).
    assert!(!result.success);
    assert_eq!(result.projects_run, 2);
    for item in &result.results {
        assert!(matches!(&item.outcome, RaidBatchOutcome::Error { .. }));
    }
}

// --- B4: stop_on_project_failure → second not run ---

#[test]
fn b4_stop_on_failure_skips_remaining() {
    let (temp, root) = workspace();
    let den = temp.path().join("den");
    create_project(&root, "alpha");
    create_project(&root, "beta");

    let ctx = ctx_for(&root, &den, RunMode::DryRun);
    let mut opts = default_batch_opts(&root);
    opts.stop_on_project_failure = true;
    // Enable stash + no identity → first project errors → stop.
    opts.raid.stash.enabled = true;
    opts.raid.stash.min_risk = SensitiveRisk::Critical;

    let result =
        raccpack_core::raid_batch(&ctx, &opts, None, &mut NullProgress).expect("raid_batch call");
    assert!(!result.success);
    assert_eq!(result.projects_run, 1);
    assert_eq!(result.results[0].project_name, "alpha");
}

// --- B5: --only filters to 1 ---

#[test]
fn b5_only_filter_limits_to_matching_project() {
    let (temp, root) = workspace();
    let den = temp.path().join("den");
    create_project(&root, "alpha");
    create_project(&root, "beta");

    let ctx = ctx_for(&root, &den, RunMode::DryRun);
    let mut opts = default_batch_opts(&root);
    opts.only = vec!["alpha".to_string()];
    let result = raid_batch(&ctx, &opts);

    assert_eq!(result.projects_total, 2);
    assert_eq!(result.projects_run, 1);
    assert_eq!(result.results[0].project_name, "alpha");
}

// --- B6: empty root → 0 projects, success true ---

#[test]
fn b6_empty_root_returns_zero_projects() {
    let (temp, root) = workspace();
    let den = temp.path().join("den");

    let ctx = ctx_for(&root, &den, RunMode::DryRun);
    let opts = default_batch_opts(&root);
    let result = raid_batch(&ctx, &opts);

    assert_eq!(result.projects_total, 0);
    assert_eq!(result.projects_run, 0);
    assert!(result.success);
    assert!(result.results.is_empty());
}

// --- B7: each project gets its own RaidResult with distinct project_path ---

#[test]
fn b7_each_project_gets_its_own_raid_result() {
    let (temp, root) = workspace();
    let den = temp.path().join("den");
    create_project(&root, "alpha");
    create_project(&root, "beta");

    let ctx = ctx_for(&root, &den, RunMode::DryRun);
    let opts = default_batch_opts(&root);
    let result = raid_batch(&ctx, &opts);

    assert_eq!(result.results.len(), 2);
    let alpha = result
        .results
        .iter()
        .find(|i| i.project_name == "alpha")
        .expect("alpha present");
    let beta = result
        .results
        .iter()
        .find(|i| i.project_name == "beta")
        .expect("beta present");

    assert!(alpha.project_path.ends_with("alpha"));
    assert!(beta.project_path.ends_with("beta"));

    assert!(matches!(&alpha.outcome, RaidBatchOutcome::Raided(_)));
    assert!(matches!(&beta.outcome, RaidBatchOutcome::Raided(_)));
}
