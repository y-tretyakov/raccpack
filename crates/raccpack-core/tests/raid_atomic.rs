//! Integration tests for A3.3 PR2 — atomic raid: shared staging +
//! deferred destructive ops.
//!
//! Spec: `docs/alpha/a3_new/a3.3-atomic-upgrade.md` §7 (ORPHAN-1/3/4) and the
//! A3.3 PR2 Test brief.
//!
//! Covers:
//! 1. Atomic full-success Commit: artifacts placed; sources / rinse deletes
//!    deferred to the commit; staging cleaned; no rollback event.
//! 2. Atomic ≡ FailFast on full-success Commit (field-level equality
//!    invariant; paths differ per run so the result is compared per-field).
//! 3. ORPHAN-1: pack fails after stash staged secrets → no den artifact,
//!    staging cleaned, sources untouched.
//! 4. ORPHAN-3: DryRun → zero FS (no den, no staging).
//! 5. ORPHAN-4: FailFast + pack fail may leave an orphan `.age` (documented).
//! 6. StashEmpty is a no-op; the run continues and packs.
//! 7. pack-only atomic places a single artifact.
//! 8. remove_sources=false: stashed files stay in the project but are
//!    excluded from the pack archive (`exclude_files`).
//! 9. A failed atomic stash does not create the den.
//! 10. RaidResult JSON never leaks raw secret values.
//! 11. ORPHAN-2 (PR3): a rename fails mid-commit → reverse-WAL rollback:
//!     the placed `.age` is removed, staging cleaned, sources untouched,
//!     `rolled_back` true, a `"rollback"` completion event is emitted.
//! 12. PR3: irreversible source/trash deletes surface `rollback_warnings`.
//!
//! Fault injection: a chmod-000 regular file (`src/chunk.bin`) makes `pack`
//! fail on read (Unix only) while stash skips it (content scan is best-effort
//! and the name matches no filename marker). Non-root environment assumed, as
//! in the rest of the suite.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{
    raid, AgeIdentity, AppContext, NullProgress, OrchestrationMode, PackPhaseOpts, RaccConfig,
    RaidOptions, RaidResult, RinsePhaseOpts, RunMode, StashPhaseOpts,
};
use tempfile::TempDir;
use zeroize::Zeroizing;

/// A long, distinctive secret value used to prove no result leaks it.
const PASSWORD_VALUE: &str = "SUPERSECRETVALUE_raid_atomic_8080";

/// Test passphrase for age encryption (must be non-empty).
const PASSPHRASE: &str = "raccpack a3.3 atomic raid test passphrase";

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`, returning the path.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
    path
}

/// Create a hermetic workspace root with `proj/` and `den/` sibling paths.
fn workspace() -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new().expect("create temp dir");
    let proj = temp.path().join("proj");
    let den = temp.path().join("den");
    fs::create_dir_all(&proj).expect("create project dir");
    (temp, proj, den)
}

/// Build an `AppContext` for a `raid` run: scan root = the project itself, and
/// the den is always an explicit TempDir path so the real `~/.raccpack/den` is
/// never touched.
fn ctx_for(project_root: &Path, den_dir: &Path, mode: RunMode) -> AppContext {
    let config = RaccConfig::default()
        .with_scan_root(project_root)
        .with_den_dir(den_dir);
    AppContext::from_config(config, mode).expect("AppContext::from_config")
}

/// Default raid options for a project (all phases enabled, Atomic mode).
fn raid_options(project: &Path) -> RaidOptions {
    RaidOptions {
        project: project.to_path_buf(),
        ..RaidOptions::default()
    }
}

/// A passphrase identity backed by the shared test passphrase.
fn identity() -> AgeIdentity {
    AgeIdentity::Passphrase(Zeroizing::new(PASSPHRASE.to_string()))
}

/// An empty passphrase identity (age encryption must reject it).
fn empty_identity() -> AgeIdentity {
    AgeIdentity::Passphrase(Zeroizing::new(String::new()))
}

/// Run `raid` with a null sink, passing the passphrase identity.
fn raid_once(ctx: &AppContext, opts: &RaidOptions) -> RaidResult {
    let mut progress = NullProgress;
    raid(ctx, opts, Some(&identity()), &mut progress).expect("raid should succeed")
}

/// Sink that collects emitted events for assertions.
#[derive(Default)]
struct RecordingSink {
    events: Vec<raccpack_core::ProgressEvent>,
}

impl raccpack_core::ProgressSink for RecordingSink {
    fn emit(&mut self, event: raccpack_core::ProgressEvent) {
        self.events.push(event);
    }
}

/// Run `raid` recording every progress event; returns the result and the
/// events.
fn raid_recorded(
    ctx: &AppContext,
    opts: &RaidOptions,
    identity: Option<&AgeIdentity>,
) -> (RaidResult, Vec<raccpack_core::ProgressEvent>) {
    let mut sink = RecordingSink::default();
    let result = raid(ctx, opts, identity, &mut sink).expect("raid should return a result");
    (result, sink.events)
}

/// The raid-level completion events (`OperationKind::Raid` and
/// `phase_complete`), like the nested facade events are filtered out.
fn raid_completions(events: &[raccpack_core::ProgressEvent]) -> Vec<&raccpack_core::ProgressEvent> {
    events
        .iter()
        .filter(|e| e.operation == raccpack_core::OperationKind::Raid && e.phase_complete)
        .collect()
}

/// Current UTC `yyyy/mm` (from the same clock the den naming uses), so a test
/// can predict where an artifact would land.
fn current_den_year_month() -> (String, String) {
    let ts = raccpack_core::utc_timestamp_now();
    (ts[0..4].to_string(), ts[4..6].to_string())
}

/// A project fixture with a sensitive file, a trash dir and a normal file.
fn full_fixture(proj: &Path) -> Vec<PathBuf> {
    vec![
        write(proj, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n")),
        write(proj, "README.md", "# demo project\n"),
        write(proj, "node_modules/pkg/index.js", "module.exports = 1;\n"),
    ]
}

/// Add a chmod-000 regular file that breaks `pack` but is skipped by `stash`.
///
/// Returns the path so the test can restore permissions before the TempDir is
/// dropped. Unix only: `Permissions::from_mode(0o000)`.
#[cfg(unix)]
fn add_unreadable_file(proj: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    write(proj, "src/chunk.bin", "binary payload\n");
    let path = proj.join("src/chunk.bin");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("chmod 000");
    path
}

/// Recursively collect the absolute paths of files under `root` with `ext`
/// (missing root → empty).
fn collect_files_with_ext(root: &Path, ext: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !root.exists() {
        return files;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e.to_string_lossy()) == Some(ext.into()) {
                files.push(path);
            }
        }
    }
    files
}

/// Assert that `den/staging` contains no files and no nested directories.
fn staging_is_clean(den: &Path) {
    let staging = den.join("staging");
    if !staging.exists() {
        return;
    }
    let mut stack = vec![staging.clone()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("read staging dir").flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                panic!("staging must be empty, found file: {}", path.display());
            }
        }
    }
}

/// Entry names of a `tar.zst` archive, in archive order.
fn tar_entry_names(tar_zst: &Path) -> Vec<String> {
    let file = fs::File::open(tar_zst).expect("open pack archive");
    let decoder = zstd::stream::read::Decoder::new(file).expect("zstd decode");
    let mut archive = tar::Archive::new(decoder);
    let mut names = Vec::new();
    for entry in archive.entries().expect("list tar entries") {
        let entry = entry.expect("read tar entry");
        names.push(
            entry
                .path()
                .expect("entry path")
                .to_string_lossy()
                .into_owned(),
        );
    }
    names
}

/// Field-level equality of a successful atomic run vs a successful fail-fast
/// run. Exact paths differ (separate den dirs and timestamps), so the variable
/// fields are compared structurally and the paths are asserted to be real
/// files under the respective den.
fn assert_semantically_equal(atomic: &RaidResult, fail_fast: &RaidResult) {
    assert_eq!(atomic.stages, fail_fast.stages);
    assert_eq!(atomic.success, fail_fast.success);
    assert_eq!(atomic.dry_run, fail_fast.dry_run);
    assert_eq!(atomic.rolled_back, fail_fast.rolled_back);
    assert_eq!(atomic.rollback_warnings, fail_fast.rollback_warnings);

    let a_stash = atomic.stash.as_ref().expect("atomic stash result");
    let f_stash = fail_fast.stash.as_ref().expect("fail-fast stash result");
    assert_eq!(a_stash.files_archived, f_stash.files_archived);
    assert_eq!(a_stash.bytes_archived, f_stash.bytes_archived);
    assert_eq!(a_stash.removed_sources, f_stash.removed_sources);
    assert_eq!(a_stash.dry_run, f_stash.dry_run);
    assert!(
        a_stash.archive_path.is_file(),
        "atomic stash archive missing"
    );
    assert!(
        f_stash.archive_path.is_file(),
        "fail-fast stash archive missing"
    );

    let a_rinse = atomic.rinse.as_ref().expect("atomic rinse result");
    let f_rinse = fail_fast.rinse.as_ref().expect("fail-fast rinse result");
    assert_eq!(a_rinse.removed.len(), f_rinse.removed.len());
    assert_eq!(a_rinse.bytes_freed, f_rinse.bytes_freed);
    assert_eq!(a_rinse.dry_run, f_rinse.dry_run);

    let a_pack = atomic.pack.as_ref().expect("atomic pack result");
    let f_pack = fail_fast.pack.as_ref().expect("fail-fast pack result");
    assert_eq!(a_pack.file_count, f_pack.file_count);
    assert_eq!(a_pack.skipped_secret_files, f_pack.skipped_secret_files);
    assert_eq!(a_pack.dry_run, f_pack.dry_run);
    assert!(a_pack.output.is_file(), "atomic pack missing");
    assert!(f_pack.output.is_file(), "fail-fast pack missing");

    assert_eq!(
        atomic.den_artifacts.len(),
        fail_fast.den_artifacts.len(),
        "den_artifacts count must match"
    );
    assert_eq!(atomic.den_artifacts.len(), 2, "age + pack");
    assert!(
        atomic.den_artifacts.iter().all(|p| p.is_file()),
        "every atomic artifact must be placed"
    );
}

// --- Case 1: atomic full-success Commit --------------------------------------

#[test]
fn atomic_commit_places_artifacts_and_applies_deferred_ops() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = raid_once(&ctx, &raid_options(&proj));

    assert!(
        result.success,
        "full atomic commit must succeed: {result:?}"
    );
    assert!(!result.dry_run);
    assert!(!result.rolled_back, "rollback is a PR3 feature");
    assert!(result.rollback_warnings.is_empty());

    let stash = result.stash.as_ref().expect("stash result");
    assert!(
        stash.archive_path.is_file(),
        "age archive missing: {}",
        stash.archive_path.display()
    );
    assert!(
        stash.archive_path.starts_with(&den),
        "stash archive must live under the den"
    );
    assert_eq!(stash.removed_sources, 1, ".env must be removed at commit");

    let pack = result.pack.as_ref().expect("pack result");
    assert!(
        pack.output.is_file(),
        "pack missing: {}",
        pack.output.display()
    );
    assert!(
        pack.output.starts_with(&den),
        "pack must live under the den"
    );

    assert!(
        !proj.join(".env").exists(),
        "stash.remove_sources=true must delete .env (in the commit)"
    );
    assert!(
        !proj.join("node_modules").exists(),
        "rinse must remove node_modules (in the commit)"
    );
    assert!(
        proj.join("README.md").is_file(),
        "normal project files must survive"
    );

    assert_eq!(result.den_artifacts.len(), 2, "age + pack");
    assert_eq!(
        collect_files_with_ext(&den.join("secrets"), "age").len(),
        1,
        "exactly one .age under den/secrets"
    );
    assert_eq!(
        collect_files_with_ext(&den.join("packs"), "zst").len(),
        1,
        "exactly one .tar.zst under den/packs"
    );

    staging_is_clean(&den);

    let names: Vec<&str> = result.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["stash", "rinse", "pack", "move"]);
}

// --- Case 2: atomic ≡ fail-fast on full success ------------------------------

#[test]
fn atomic_equals_fail_fast_on_full_success_commit() {
    let (temp_a, proj_a, den_a) = workspace();
    let (temp_b, proj_b, den_b) = workspace();
    let _ = (temp_a, temp_b);
    full_fixture(&proj_a);
    full_fixture(&proj_b);

    let atomic_ctx = ctx_for(&proj_a, &den_a, RunMode::Commit);
    let fail_fast_ctx = ctx_for(&proj_b, &den_b, RunMode::Commit);

    let atomic = raid_once(&atomic_ctx, &raid_options(&proj_a));
    let fail_fast_opts = RaidOptions {
        project: proj_b.clone(),
        mode: OrchestrationMode::FailFast,
        ..RaidOptions::default()
    };
    let fail_fast = raid_once(&fail_fast_ctx, &fail_fast_opts);

    assert!(atomic.success);
    assert!(fail_fast.success);
    assert_semantically_equal(&atomic, &fail_fast);
    staging_is_clean(&den_a);
}

// --- Case 3: ORPHAN-1 — pack fails after stash staged secrets ----------------

#[cfg(unix)]
#[test]
fn atomic_pack_failure_leaves_no_den_artifact_and_cleans_staging() {
    use std::os::unix::fs::PermissionsExt;

    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let unreadable = add_unreadable_file(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = raid_once(&ctx, &raid_options(&proj));

    // Restore permissions so TempDir cleanup can always remove the file.
    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644))
        .expect("restore file permissions");

    assert!(
        !result.success,
        "a failing pack must fail the atomic raid: {result:?}"
    );
    assert!(
        result.den_artifacts.is_empty(),
        "no artifact may be reported"
    );
    assert!(
        result.stash.is_none(),
        "staged secrets were never committed"
    );
    assert!(result.pack.is_none());
    assert!(!result.rolled_back, "rollback is a PR3 feature");

    assert!(
        collect_files_with_ext(&den.join("secrets"), "age").is_empty(),
        "ORPHAN-1: no .age may exist in den/secrets"
    );
    assert!(
        collect_files_with_ext(&den, "age").is_empty(),
        "no .age anywhere under the den"
    );
    staging_is_clean(&den);

    assert!(
        proj.join(".env").is_file(),
        "source removal must be deferred — .env must survive a failed pack"
    );
}

// --- Case 4: ORPHAN-3 — DryRun writes nothing ---------------------------------

#[test]
fn atomic_dry_run_writes_nothing() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let result = raid_once(&ctx, &raid_options(&proj));

    assert!(result.success, "dry run must succeed: {result:?}");
    assert!(result.dry_run);
    assert!(result.den_artifacts.is_empty());
    assert!(!den.exists(), "ORPHAN-3: dry run must not create the den");
    assert!(!den.join(".den-version").exists());
    assert!(!den.join("staging").exists());
    for fixture in [".env", "README.md", "node_modules/pkg/index.js"] {
        assert!(
            proj.join(fixture).is_file(),
            "dry run must never remove sources: {fixture}"
        );
    }
}

// --- Case 5: ORPHAN-4 — fail-fast may leave an orphan artifact ----------------

#[cfg(unix)]
#[test]
fn fail_fast_pack_failure_may_leave_orphan_artifact() {
    use std::os::unix::fs::PermissionsExt;

    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let unreadable = add_unreadable_file(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    let opts = RaidOptions {
        project: proj.clone(),
        mode: OrchestrationMode::FailFast,
        ..RaidOptions::default()
    };

    let result = raid_once(&ctx, &opts);

    fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644))
        .expect("restore file permissions");

    assert!(
        !result.success,
        "pack failure must fail the raid: {result:?}"
    );
    assert!(
        result.stash.is_some(),
        "fail-fast stash placed its archive before pack failed"
    );
    assert!(
        collect_files_with_ext(&den.join("secrets"), "age").len() == 1,
        "ORPHAN-4: fail-fast leaves the placed .age in the den (documented)"
    );
    assert_eq!(result.den_artifacts.len(), 1);
    assert!(
        result.den_artifacts[0].is_file(),
        "the orphaned .age must still be on disk"
    );

    assert!(
        !proj.join(".env").exists(),
        "fail-fast removes sources immediately after placement"
    );
    assert!(
        !proj.join("node_modules").exists(),
        "fail-fast rinse deletes mid-pipeline"
    );
}

// --- Case 6: StashEmpty is a no-op, the run continues -------------------------

#[test]
fn atomic_stash_empty_continues_and_packs() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    write(&proj, "README.md", "# clean project, no secrets\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = raid_once(&ctx, &raid_options(&proj));

    assert!(
        result.success,
        "empty stash must not fail the raid: {result:?}"
    );
    assert!(result.stash.is_none(), "no archive was created");
    let stash_stage = result.stages.iter().find(|s| s.name == "stash").unwrap();
    assert!(stash_stage.success);
    assert_eq!(stash_stage.message, "nothing to stash");
    assert!(result.pack.is_some(), "pack must still run");
    assert_eq!(
        collect_files_with_ext(&den.join("packs"), "zst").len(),
        1,
        "one .tar.zst must be placed"
    );
    staging_is_clean(&den);
}

// --- Case 7: pack-only atomic -------------------------------------------------

#[test]
fn atomic_pack_only_places_single_artifact() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    write(&proj, "README.md", "# demo\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let opts = RaidOptions {
        project: proj.clone(),
        mode: OrchestrationMode::Atomic,
        stash: StashPhaseOpts {
            enabled: false,
            min_risk: raccpack_core::SensitiveRisk::High,
            remove_sources: true,
        },
        rinse: RinsePhaseOpts { enabled: false },
        pack: PackPhaseOpts {
            enabled: true,
            deny_content_secrets: true,
        },
    };

    let mut progress = NullProgress;
    let result = raid(&ctx, &opts, None, &mut progress).expect("pack-only raid");

    assert!(result.success);
    assert_eq!(result.den_artifacts.len(), 1, "pack artifact only");
    assert!(result.den_artifacts[0].is_file());
    assert!(result.pack.is_some());
    staging_is_clean(&den);
}

// --- Case 8: remove_sources=false → sources kept, excluded from pack ----------

#[test]
fn atomic_remove_sources_false_keeps_sources_and_excludes_them_from_pack() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    write(&proj, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n"));
    // A High-risk content secret under a name pack would NOT deny (content deny
    // is Critical-only): without exclude_files this would leak into the pack.
    write(&proj, "notes.txt", "slack: xoxb-1234567890123456\n");
    write(&proj, "README.md", "# demo\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let opts = RaidOptions {
        project: proj.clone(),
        mode: OrchestrationMode::Atomic,
        stash: StashPhaseOpts {
            enabled: true,
            min_risk: raccpack_core::SensitiveRisk::High,
            remove_sources: false,
        },
        rinse: RinsePhaseOpts { enabled: false },
        pack: PackPhaseOpts {
            enabled: true,
            deny_content_secrets: true,
        },
    };
    let mut progress = NullProgress;
    let result = raid(&ctx, &opts, Some(&identity()), &mut progress)
        .expect("remove_sources=false atomic raid");

    assert!(result.success, "raid must succeed: {result:?}");

    assert!(
        proj.join(".env").is_file(),
        "remove_sources=false must keep .env in the project"
    );
    assert!(
        proj.join("notes.txt").is_file(),
        "remove_sources=false must keep notes.txt in the project"
    );

    let pack = result.pack.as_ref().expect("pack result");
    let names = tar_entry_names(&pack.output);
    assert_eq!(
        names,
        vec!["README.md"],
        "stashed files must be excluded from the pack: {names:?}"
    );
    assert!(
        !names.contains(&".env".to_string()),
        ".env must never appear in the pack"
    );
    assert!(
        !names.contains(&"notes.txt".to_string()),
        "a stashed High-risk secret must never leak into the pack"
    );
    staging_is_clean(&den);
}

// --- Case 9: failed atomic stash does not create the den ----------------------

#[test]
fn atomic_stash_failure_does_not_create_den() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let opts = raid_options(&proj);
    let mut progress = NullProgress;
    let result = raid(&ctx, &opts, Some(&empty_identity()), &mut progress)
        .expect("a phase failure returns Ok(RaidResult)");

    assert!(
        !result.success,
        "empty passphrase must fail the stash phase"
    );
    assert!(
        !den.exists(),
        "a failed stash must not create the den: {}",
        den.display()
    );
    assert!(
        proj.join(".env").is_file(),
        "failed stash must not remove sources"
    );
}

// --- Case 10: no raw secret in RaidResult JSON --------------------------------

#[test]
fn atomic_result_json_never_leaks_raw_secret() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = raid_once(&ctx, &raid_options(&proj));
    let json = serde_json::to_string(&result).expect("serialize RaidResult");

    assert!(
        !json.contains(PASSWORD_VALUE),
        "RaidResult JSON must never leak raw values: {json}"
    );
    assert!(
        !json.to_lowercase().contains(&PASSWORD_VALUE.to_lowercase()),
        "RaidResult JSON must not leak raw values (case-insensitive)"
    );
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"name\":\"move\""));
}

// --- Case 11: ORPHAN-2 — a mid-commit rename failure is rolled back ----------

#[test]
fn atomic_commit_rename_failure_rolls_back_placed_artifact() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    // Blocker: `den/packs/{yyyy}/{mm}` is a regular FILE, so the pack's
    // `create_dir_all` in the commit fails AFTER the stash `.age` was placed.
    // (The stash rename has no such conflict — `den/secrets/…` is free.)
    let (year, month) = current_den_year_month();
    let blocker = den.join("packs").join(&year).join(&month);
    write(&den, &format!("packs/{year}/{month}"), "blocker file\n");

    let opts = raid_options(&proj);
    let (result, events) = raid_recorded(&ctx, &opts, Some(&identity()));

    assert!(
        !result.success,
        "a mid-commit placement failure must fail the raid: {result:?}"
    );
    assert!(
        result.rolled_back,
        "ORPHAN-2: the commit must be rolled back via the WAL: {result:?}"
    );
    assert!(
        result.den_artifacts.is_empty(),
        "no artifact may be reported after a rollback: {:?}",
        result.den_artifacts
    );
    assert!(
        collect_files_with_ext(&den.join("secrets"), "age").is_empty(),
        "the placed stash .age must be removed by the rollback"
    );
    assert!(
        collect_files_with_ext(&den.join("packs"), "zst").is_empty(),
        "no pack may survive a failed commit"
    );
    assert!(
        blocker.is_file(),
        "the pre-existing blocker file must be untouched"
    );
    staging_is_clean(&den);

    assert!(
        proj.join(".env").is_file(),
        "remove_sources is deferred to the commit and never reached — .env must survive"
    );
    assert!(
        proj.join("node_modules").exists(),
        "rinse deletes are deferred — node_modules must survive"
    );

    let completions = raid_completions(&events);
    assert!(
        completions.iter().any(|e| e.phase == "rollback"),
        "a commit rollback must emit a \"rollback\" completion event: {completions:?}"
    );
    assert!(
        result
            .rollback_warnings
            .iter()
            .any(|w| w.contains("could not remove directory")),
        "the non-empty packs dir must be reported as a rollback warning: {:?}",
        result.rollback_warnings
    );
}

#[test]
fn atomic_commit_success_emits_no_rollback_event() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    let opts = raid_options(&proj);

    let (result, events) = raid_recorded(&ctx, &opts, Some(&identity()));

    assert!(
        result.success,
        "full atomic commit must succeed: {result:?}"
    );
    assert!(!result.rolled_back);
    assert!(result.rollback_warnings.is_empty());
    let completions = raid_completions(&events);
    assert!(
        completions.iter().all(|e| e.phase != "rollback"),
        "a successful commit must not emit a rollback event: {completions:?}"
    );
}

// --- Case 12: PR3 — irreversible source deletes surface warnings --------------

#[cfg(unix)]
#[test]
fn atomic_commit_irreversible_source_deletes_report_warnings() {
    use std::os::unix::fs::PermissionsExt;

    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    // Make `node_modules/pkg` unremovable but still readable (non-root):
    // `remove_trash_dirs` in the commit fails AFTER stash already removed
    // `.env`, while the rinse scan can still read `pkg` to size the trash dir.
    // Perms 0555 = read+execute, no write: scanning works, removal does not.
    let pkg = proj.join("node_modules/pkg");
    fs::set_permissions(&pkg, fs::Permissions::from_mode(0o555))
        .expect("chmod 555 node_modules/pkg");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let opts = raid_options(&proj);
    let (result, events) = raid_recorded(&ctx, &opts, Some(&identity()));

    fs::set_permissions(&pkg, fs::Permissions::from_mode(0o755))
        .expect("restore node_modules/pkg permissions");

    assert!(
        !result.success,
        "the rinse failure must fail the raid: {result:?}"
    );
    assert!(
        result.rolled_back,
        "the placed artifacts must be rolled back even when some deletes are irreversible"
    );
    assert!(
        collect_files_with_ext(&den.join("secrets"), "age").is_empty(),
        "the placed stash .age must be removed by the rollback"
    );
    assert!(
        collect_files_with_ext(&den.join("packs"), "zst").is_empty(),
        "the placed pack must be removed by the rollback"
    );
    staging_is_clean(&den);

    assert!(
        !proj.join(".env").exists(),
        ".env was removed before the failure and cannot be restored (documented)"
    );
    assert!(
        proj.join("node_modules/pkg/index.js").is_file(),
        "the failed remove_dir_all must leave node_modules untouched"
    );

    assert!(
        result
            .rollback_warnings
            .iter()
            .any(|w| w.contains("cannot restore deleted file")),
        "the irreversible source delete must be reported: {:?}",
        result.rollback_warnings
    );
    let completions = raid_completions(&events);
    assert!(
        completions.iter().any(|e| e.phase == "rollback"),
        "a rolled-back commit must emit the rollback event: {completions:?}"
    );
}
