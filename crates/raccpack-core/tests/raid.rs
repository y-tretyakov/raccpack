//! Integration tests for A3.1 — facade `raid` orchestration.
//!
//! Covers the 7 required cases from `docs/alpha/a3/a3.1-facade-raid.md` §6:
//! 1. All enabled DryRun: success true, no files in den.
//! 2. All enabled Commit fixture: age + pack exist; sources removed if
//!    stash.remove_sources; node_modules gone if rinse on.
//! 3. stash fails (empty passphrase) → rinse/pack not run; success false;
//!    stages coherent.
//! 4. stash.enabled=false → no identity required; rinse+pack run.
//! 5. pack only: stash/rinse skipped.
//! 6. den_artifacts contains expected paths on full success Commit.
//! 7. Default RaidOptions has stash/rinse/pack enabled.
//!
//! Extras beyond the mandatory list:
//! - `AgeIdentity::Recipients` with stash enabled → precondition `Err`;
//! - recipients identity is **ignored** when stash is disabled (spec §4);
//! - empty project path → precondition `Err`;
//! - missing identity with stash enabled → precondition `Err`;
//! - DryRun never bootstraps the den skeleton;
//! - stage messages never leak the raw secret value.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{
    raid, AgeIdentity, AppContext, Error, NullProgress, PackPhaseOpts, RaccConfig, RaidOptions,
    RaidResult, RinsePhaseOpts, RunMode, StashPhaseOpts,
};
use tempfile::TempDir;
use zeroize::Zeroizing;

/// A long, distinctive password value used to prove no result leaks it.
const PASSWORD_VALUE: &str = "SUPERSECRETVALUE_raid_xyz987";

/// Test passphrase for age encryption (must be non-empty).
const PASSPHRASE: &str = "raccpack a3.1 facade raid test passphrase";

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`.
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

/// Default raid options for a project (all phases enabled).
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

/// Run `raid` with a null sink; panics with context on error.
fn raid_once(ctx: &AppContext, opts: &RaidOptions) -> RaidResult {
    let mut progress = NullProgress;
    raid(ctx, opts, Some(&identity()), &mut progress).expect("raid should succeed")
}

/// A project fixture with a sensitive file, a trash dir and a normal file.
fn full_fixture(proj: &Path) -> Vec<PathBuf> {
    vec![
        write(proj, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n")),
        write(proj, "README.md", "# demo project\n"),
        write(proj, "node_modules/pkg/index.js", "module.exports = 1;\n"),
    ]
}

// --- Case 1: all enabled DryRun ---------------------------------------------

#[test]
fn dry_run_all_enabled_is_successful_and_writes_nothing() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    let fixtures = full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let result = raid_once(&ctx, &raid_options(&proj));

    assert!(result.success, "dry run must succeed: {result:?}");
    assert!(result.dry_run);
    assert!(
        result.den_artifacts.is_empty(),
        "dry run must not report artifacts: {:?}",
        result.den_artifacts
    );
    assert_eq!(result.project_path, proj);

    let names: Vec<&str> = result.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["stash", "rinse", "pack", "move"]);
    assert!(
        result.stages.iter().all(|s| s.success),
        "all stages must be successful in dry run: {:?}",
        result.stages
    );

    assert!(
        !den.exists(),
        "dry run must not create the den: {}",
        den.display()
    );
    for fixture in &fixtures {
        assert!(
            fixture.is_file(),
            "dry run must never remove sources: {}",
            fixture.display()
        );
    }
}

#[test]
fn dry_run_does_not_bootstrap_den_skeleton() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    raid_once(&ctx, &raid_options(&proj));

    assert!(!den.join(".den-version").exists());
    assert!(!den.join("secrets").exists());
    assert!(!den.join("packs").exists());
}

// --- Case 2: all enabled Commit ----------------------------------------------

#[test]
fn commit_all_enabled_places_artifacts_and_applies_phases() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = raid_once(&ctx, &raid_options(&proj));

    assert!(result.success, "full commit must succeed: {result:?}");
    assert!(!result.dry_run);

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
        "stash.remove_sources=true must delete .env"
    );
    assert!(
        !proj.join("node_modules").exists(),
        "rinse must remove node_modules"
    );
    assert!(
        proj.join("README.md").is_file(),
        "normal project files must survive"
    );

    let names: Vec<&str> = result.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["stash", "rinse", "pack", "move"]);
}

// --- Case 3: stash fails → fail-fast -----------------------------------------

#[test]
fn stash_failure_short_circuits_following_phases() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let empty = AgeIdentity::Passphrase(Zeroizing::new(String::new()));
    let opts = raid_options(&proj);
    let mut progress = NullProgress;
    let result = raid(&ctx, &opts, Some(&empty), &mut progress)
        .expect("a phase failure returns Ok(RaidResult)");

    assert!(!result.success, "stash failure must fail the raid");
    assert!(
        result.stash.is_none(),
        "failed stash must not report a sub-result"
    );
    assert!(result.rinse.is_none());
    assert!(result.pack.is_none());

    let names: Vec<&str> = result.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["stash", "rinse", "pack", "move"]);

    let stash_stage = &result.stages[0];
    assert!(!stash_stage.success);
    assert!(!stash_stage.skipped);

    for stage in &result.stages[1..] {
        assert!(!stage.success, "following stages must not succeed");
        assert!(stage.skipped, "following stages must be skipped");
    }

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

// --- Case 4: stash disabled → no identity required ---------------------------

#[test]
fn stash_disabled_runs_rinse_and_pack_without_identity() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let opts = RaidOptions {
        project: proj.clone(),
        stash: StashPhaseOpts {
            enabled: false,
            min_risk: raccpack_core::SensitiveRisk::High,
            remove_sources: true,
        },
        rinse: RinsePhaseOpts { enabled: true },
        pack: PackPhaseOpts {
            enabled: true,
            deny_content_secrets: true,
        },
    };

    let mut progress = NullProgress;
    let result = raid(&ctx, &opts, None, &mut progress).expect("no identity needed");

    assert!(
        result.success,
        "stash-disabled raid must succeed: {result:?}"
    );
    assert!(result.stash.is_none());

    let stash_stage = result.stages.iter().find(|s| s.name == "stash").unwrap();
    assert!(stash_stage.skipped, "stash must be skipped when disabled");
    assert!(result.rinse.is_some(), "rinse must run");
    assert!(result.pack.is_some(), "pack must run");

    assert!(
        !proj.join("node_modules").exists(),
        "rinse must still remove node_modules"
    );
    assert!(
        proj.join("README.md").is_file(),
        "normal files must survive"
    );
}

// --- Case 5: pack only --------------------------------------------------------

#[test]
fn pack_only_skips_stash_and_rinse() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    write(&proj, "README.md", "# demo\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let opts = RaidOptions {
        project: proj.clone(),
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
    for name in ["stash", "rinse"] {
        let stage = result.stages.iter().find(|s| s.name == name).unwrap();
        assert!(stage.skipped, "{name} must be skipped");
    }
    assert!(result.pack.is_some(), "pack must run");
    assert_eq!(result.den_artifacts.len(), 1, "pack artifact only");

    let stage_names: Vec<&str> = result.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(stage_names, vec!["stash", "rinse", "pack", "move"]);
}

// --- Case 6: den_artifacts on full success Commit -----------------------------

#[test]
fn den_artifacts_contain_expected_paths_on_full_commit() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    full_fixture(&proj);
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = raid_once(&ctx, &raid_options(&proj));

    assert!(result.success);
    assert_eq!(result.den_artifacts.len(), 2, "age + pack");

    let has_secrets_age = result.den_artifacts.iter().any(|p| {
        p.starts_with(&den)
            && p.to_string_lossy().contains("/secrets/")
            && p.to_string_lossy().ends_with("__secrets.age")
            && p.is_file()
    });
    let has_packs_zst = result.den_artifacts.iter().any(|p| {
        p.starts_with(&den)
            && p.to_string_lossy().contains("/packs/")
            && p.to_string_lossy().ends_with(".tar.zst")
            && p.is_file()
    });
    assert!(
        has_secrets_age,
        "den_artifacts must contain the .age: {:?}",
        result.den_artifacts
    );
    assert!(
        has_packs_zst,
        "den_artifacts must contain the pack: {:?}",
        result.den_artifacts
    );
}

// --- Case 7: default options ---------------------------------------------------

#[test]
fn default_options_have_all_phases_enabled() {
    let opts = RaidOptions::default();
    assert!(opts.stash.enabled);
    assert!(opts.rinse.enabled);
    assert!(opts.pack.enabled);
    assert_eq!(opts.stash.min_risk, raccpack_core::SensitiveRisk::High);
    assert!(opts.stash.remove_sources);
    assert!(opts.pack.deny_content_secrets);
}

// --- Extras --------------------------------------------------------------------

#[test]
fn recipients_identity_is_precondition_error_when_stash_enabled() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let recipients = AgeIdentity::Recipients(vec![
        "age1fakefakefakefakefakefakefakefakefakefakefakefake".to_string(),
    ]);
    let opts = raid_options(&proj);
    let mut progress = NullProgress;
    let err = raid(&ctx, &opts, Some(&recipients), &mut progress).unwrap_err();

    assert!(
        matches!(err, Error::Unsupported { .. }),
        "recipients with stash enabled must be Err(Unsupported), got: {err}"
    );
    assert!(
        !den.exists(),
        "a rejected precondition must not create the den"
    );
}

#[test]
fn recipients_identity_is_ignored_when_stash_disabled() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    write(&proj, "README.md", "# demo\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let recipients = AgeIdentity::Recipients(vec![
        "age1fakefakefakefakefakefakefakefakefakefakefakefake".to_string(),
    ]);
    let opts = RaidOptions {
        project: proj.clone(),
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
    let result = raid(&ctx, &opts, Some(&recipients), &mut progress)
        .expect("identity must be ignored when stash is disabled");

    assert!(result.success);
    assert!(result.pack.is_some(), "pack must still run");
}

#[test]
fn empty_project_path_is_precondition_error() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let opts = RaidOptions::default(); // project empty
    let mut progress = NullProgress;
    let err = raid(&ctx, &opts, Some(&identity()), &mut progress).unwrap_err();
    assert!(
        matches!(err, Error::Other { .. }),
        "empty project must be a precondition error, got: {err}"
    );
}

#[test]
fn missing_identity_with_stash_enabled_is_precondition_error() {
    let (temp, proj, den) = workspace();
    let _ = temp;
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let opts = raid_options(&proj);
    let mut progress = NullProgress;
    let err = raid(&ctx, &opts, None, &mut progress).unwrap_err();
    assert!(
        matches!(err, Error::Other { .. }),
        "missing identity with stash enabled must be Err, got: {err}"
    );
    assert!(
        !err.to_string().contains(PASSWORD_VALUE),
        "error must not leak secrets: {err}"
    );
}

#[test]
fn stage_messages_never_leak_raw_secrets() {
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
        !json.contains("supersecretvalue"),
        "RaidResult JSON must never leak raw values (case-insensitive check): {json}"
    );
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("\"name\":\"move\""));
}
