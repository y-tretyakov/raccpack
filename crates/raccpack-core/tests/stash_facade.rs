//! Integration tests for A1.3 — facade `stash` + den `secrets/` artifacts.
//!
//! Covers the 9 required cases from
//! `docs/alpha/a1/a1.3-facade-stash-den.md` §6:
//! 1. DryRun reports the expected path, writes nothing (no den skeleton, no
//!    `.age`), keeps `removed_sources = 0` and a non-empty manifest for a
//!    fixture `.env`;
//! 2. Commit places the `.age` under `den/secrets/{yyyy}/{mm}` and (with the
//!    `age-decrypt` feature) the decrypt + untar roundtrip restores the file;
//! 3. `remove_sources: true` deletes the originals only after a successful
//!    placement;
//! 4. `remove_sources: false` (default) leaves the sources in place;
//! 5. `min_risk` filters the selection (Critical drops a High-only `.env`);
//! 6. `AgeIdentity::Recipients` fails with `Error::Unsupported`;
//! 7. an empty passphrase fails with `Error::Encrypt` and writes nothing;
//! 8. `StashResult` serde JSON never contains raw secret content;
//! 9. progress events follow the spec table (0/30/70/[90]/100, final
//!    `phase_complete`).
//!
//! Extras beyond the mandatory list:
//! - staging is clean after a successful commit;
//! - a den nested inside the project tree is rejected (F-PATH-3);
//! - `batch_id` overrides the name token while `yyyy/mm` still derive from now;
//! - DryRun never bootstraps the den.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no sleeps. Run
//! the full suite (including the decrypt roundtrip) with:
//!
//! ```text
//! cargo test -p raccpack-core stash_facade --features age-decrypt
//! ```

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(feature = "age-decrypt")]
use std::io::Read;

use raccpack_core::{
    stash, AgeIdentity, AppContext, Error, NullProgress, OperationKind, ProgressEvent,
    ProgressSink, RaccConfig, RunMode, StashOptions, StashResult,
};
use tempfile::TempDir;
use zeroize::Zeroizing;

/// A long, distinctive password value used to prove no JSON output leaks it.
const PASSWORD_VALUE: &str = "SUPERSECRETVALUE_xyz987_abcdef";

/// Test passphrase for age encryption (must be non-empty).
const PASSPHRASE: &str = "raccpack a1.3 facade stash test passphrase";

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`, returning its path.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
    path
}

/// Create a hermetic workspace root containing an empty `proj/` directory.
fn project_dir() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create temp dir");
    let proj = temp.path().join("proj");
    fs::create_dir_all(&proj).expect("create project dir");
    (temp, proj)
}

/// Build an `AppContext` for a `stash` run: scan root = the project itself,
/// and the den is always an explicit TempDir path so the real
/// `~/.raccpack/den` is never touched.
fn ctx_for(project_root: &Path, den_dir: &Path, mode: RunMode) -> AppContext {
    let config = RaccConfig::default()
        .with_scan_root(project_root)
        .with_den_dir(den_dir);
    AppContext::from_config(config, mode).expect("AppContext::from_config")
}

/// Default stash options for a project (whole tree, High threshold, no remove).
fn stash_options(project: &Path) -> StashOptions {
    StashOptions {
        target: project.to_path_buf(),
        only_files: None,
        min_risk: raccpack_core::SensitiveRisk::High,
        remove_sources: false,
        batch_id: None,
    }
}

/// A passphrase identity backed by the shared test passphrase.
fn identity() -> AgeIdentity {
    AgeIdentity::Passphrase(Zeroizing::new(PASSPHRASE.to_string()))
}

/// Run `stash` with a null sink; panics with context on error.
fn stash_once(ctx: &AppContext, opts: &StashOptions) -> StashResult {
    let mut progress = NullProgress;
    stash(ctx, opts, &identity(), &mut progress).expect("stash should succeed")
}

/// Run `stash` recording every progress event; returns the events.
fn stash_recorded(ctx: &AppContext, opts: &StashOptions) -> Vec<ProgressEvent> {
    let mut sink = RecordingSink::default();
    stash(ctx, opts, &identity(), &mut sink).expect("stash should succeed");
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

/// Recursively collect the regular files under `root` (missing root → empty).
fn collect_files(root: &Path) -> Vec<PathBuf> {
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
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files
}

// --- Case 1: DryRun writes nothing -------------------------------------------

#[test]
fn dry_run_reports_expected_path_and_writes_nothing() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    let env = write(&proj, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n"));
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let result = stash_once(&ctx, &stash_options(&proj));

    assert!(result.dry_run, "dry run must be reported");
    assert_eq!(result.removed_sources, 0);
    assert_eq!(result.files_archived, 1);
    assert!(result.bytes_archived > 0);
    assert_eq!(result.manifest.len(), 1);
    assert_eq!(result.manifest[0].original_path, env);
    assert_eq!(result.manifest[0].risk, raccpack_core::SensitiveRisk::High);

    let path_str = result.archive_path.to_string_lossy();
    assert!(
        path_str.contains("secrets/"),
        "expected path must live under secrets/: {path_str}"
    );
    assert!(
        path_str.ends_with("__secrets.age"),
        "expected path must end with __secrets.age: {path_str}"
    );
    let file_name = result.archive_path.file_name().unwrap().to_string_lossy();
    assert!(
        file_name.starts_with("proj__"),
        "expected path must carry the project slug: {file_name}"
    );

    assert!(
        !den.exists(),
        "dry run must not create the den: {}",
        den.display()
    );
    assert!(env.is_file(), "dry run must never remove sources");
}

#[test]
fn dry_run_does_not_bootstrap_den() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    stash_once(&ctx, &stash_options(&proj));

    assert!(
        !den.join(".den-version").exists(),
        "dry run must not create the den skeleton"
    );
    assert!(
        !den.join("secrets").exists(),
        "dry run must not create secrets/"
    );
}

// --- Case 2: Commit places the archive under secrets/{yyyy}/{mm} --------------

#[test]
fn commit_places_age_under_secrets_year_month() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    let env = write(&proj, ".env", "PASSWORD=local-value\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = stash_once(&ctx, &stash_options(&proj));

    assert!(!result.dry_run, "commit must report dry_run == false");
    assert!(result.removed_sources == 0);
    assert_eq!(result.files_archived, 1);
    assert!(env.is_file(), "sources must remain by default");

    assert!(
        result.archive_path.is_file(),
        "archive missing: {}",
        result.archive_path.display()
    );
    let rel = result
        .archive_path
        .strip_prefix(&den)
        .expect("archive must live under the den");
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(parts.len(), 4, "secrets/{{yyyy}}/{{mm}}/name, got {rel:?}");
    assert_eq!(parts[0], "secrets");
    assert!(
        parts[1].len() == 4 && parts[1].chars().all(|c| c.is_ascii_digit()),
        "year segment must be yyyy: {:?}",
        parts[1]
    );
    assert!(
        parts[2].len() == 2 && parts[2].chars().all(|c| c.is_ascii_digit()),
        "month segment must be mm: {:?}",
        parts[2]
    );
    assert!(
        parts[3].starts_with("proj__") && parts[3].ends_with("__secrets.age"),
        "artifact name must match the convention: {}",
        parts[3]
    );

    let head = fs::read(&result.archive_path).unwrap();
    assert!(
        head.starts_with(b"age-encryption.org/v1"),
        "age binary magic header expected"
    );
}

#[cfg(feature = "age-decrypt")]
#[test]
fn commit_decrypt_untar_restores_original_content() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=alpha\n");
    write(&proj, ".env.local", "PASSWORD=beta\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = stash_once(&ctx, &stash_options(&proj));
    assert_eq!(result.files_archived, 2);

    let plaintext = raccpack_core::archive::age_vault::decrypt_file_from_age(
        &result.archive_path,
        &Zeroizing::new(PASSPHRASE.to_string()),
    )
    .unwrap();

    let mut archive = tar::Archive::new(&plaintext[..]);
    let mut contents = Vec::new();
    for item in archive.entries().unwrap() {
        let mut entry = item.unwrap();
        let name = entry.path().unwrap().to_string_lossy().into_owned();
        assert!(!name.contains(".."), "tar entry must stay relative: {name}");
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).unwrap();
        contents.push((name, buf));
    }

    assert_eq!(contents.len(), 2);
    let mut names: Vec<String> = contents.iter().map(|(n, _)| n.clone()).collect();
    names.sort();
    assert_eq!(names, vec![".env", ".env.local"]);
    for (name, buf) in contents {
        let expected = fs::read(proj.join(&name)).unwrap();
        assert_eq!(buf, expected, "roundtrip must restore {name}");
    }
}

// --- Case 3: remove_sources true deletes originals -----------------------------

#[test]
fn commit_remove_sources_deletes_originals_after_success() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    let env = write(&proj, ".env", "PASSWORD=remove-me\n");
    let env_local = write(&proj, ".env.local", "PASSWORD=remove-me-too\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let mut opts = stash_options(&proj);
    opts.remove_sources = true;
    let result = stash_once(&ctx, &opts);

    assert!(!result.dry_run);
    assert_eq!(result.removed_sources, 2);
    assert!(
        result.archive_path.is_file(),
        "archive must exist after source removal"
    );
    assert!(!env.exists(), ".env must be deleted");
    assert!(!env_local.exists(), ".env.local must be deleted");
}

// --- Case 4: remove_sources false leaves sources -------------------------------

#[test]
fn commit_remove_sources_false_keeps_originals() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    let env = write(&proj, ".env", "PASSWORD=keep-me\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = stash_once(&ctx, &stash_options(&proj));
    assert_eq!(result.removed_sources, 0);
    assert!(env.is_file(), "default must never remove sources");
}

// --- Case 5: min_risk filters -------------------------------------------------

#[test]
fn min_risk_critical_filters_high_only_env() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "APP=local\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let mut opts = stash_options(&proj);
    opts.min_risk = raccpack_core::SensitiveRisk::Critical;
    let err = stash(&ctx, &opts, &identity(), &mut NullProgress).unwrap_err();

    assert!(
        matches!(err, Error::StashEmpty { .. }),
        "High-only .env at Critical must surface nothing-to-stash, got: {err}"
    );
}

// --- Case 6: Recipients identity is unsupported --------------------------------

#[test]
fn recipients_identity_is_unsupported() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let recipients = AgeIdentity::Recipients(vec![
        "age1fakefakefakefakefakefakefakefakefakefakefakefake".to_string(),
    ]);
    let err = stash(&ctx, &stash_options(&proj), &recipients, &mut NullProgress).unwrap_err();
    assert!(
        matches!(err, Error::Unsupported { .. }),
        "recipients must be rejected, got: {err}"
    );
    assert!(!den.exists(), "a rejected identity must not create the den");
}

// --- Case 7: empty passphrase fails --------------------------------------------

#[test]
fn empty_passphrase_is_encrypt_error_and_writes_nothing() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let empty = AgeIdentity::Passphrase(Zeroizing::new(String::new()));
    let err = stash(&ctx, &stash_options(&proj), &empty, &mut NullProgress).unwrap_err();

    assert!(matches!(err, Error::Encrypt { .. }), "got: {err}");
    assert!(
        !den.exists(),
        "no den may be created for an empty passphrase"
    );
}

// --- Case 8: StashResult serde never leaks raw content -------------------------

#[test]
fn stash_result_json_has_no_raw_secret_content() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n"));
    write(&proj, ".env.local", "PASSWORD=another-local-secret\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = stash_once(&ctx, &stash_options(&proj));
    let json = serde_json::to_string(&result).expect("serialize StashResult");
    assert!(
        !json.contains(PASSWORD_VALUE),
        "StashResult JSON must never leak raw values: {json}"
    );
    assert!(
        !json.contains("another-local-secret"),
        "StashResult JSON must never leak raw values: {json}"
    );
    assert!(json.contains("\"dry_run\":false"));
    assert!(json.contains("__secrets.age"));

    let decoded: StashResult = serde_json::from_str(&json).expect("deserialize StashResult");
    assert_eq!(decoded, result, "serde roundtrip must preserve StashResult");
}

#[test]
fn dry_run_json_also_has_no_raw_content() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n"));
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let result = stash_once(&ctx, &stash_options(&proj));
    let json = serde_json::to_string(&result).expect("serialize StashResult");
    assert!(
        !json.contains(PASSWORD_VALUE),
        "dry-run JSON must never leak raw values: {json}"
    );
    assert!(json.contains("\"dry_run\":true"));
}

// --- Case 9: progress events follow the spec table -----------------------------

#[test]
fn progress_commit_emits_spec_table() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let events = stash_recorded(&ctx, &stash_options(&proj));
    let percents: Vec<u8> = events.iter().map(|e| e.percent).collect();
    assert_eq!(
        percents,
        vec![0, 30, 70, 100],
        "commit without removal must emit the spec steps"
    );

    for e in &events {
        assert_eq!(e.operation, OperationKind::Stash, "operation must be Stash");
        assert_eq!(e.phase, "stash", "phase must be \"stash\"");
        assert_eq!(
            e.percent, e.overall_percent,
            "single-phase stash must equate percent and overall_percent"
        );
        assert_eq!(e.phase_index, 0);
        assert_eq!(e.phase_count, 1);
    }
    let completes: Vec<bool> = events.iter().map(|e| e.phase_complete).collect();
    assert_eq!(
        completes,
        vec![false, false, false, true],
        "only the final event marks the phase complete"
    );
    assert_eq!(events.last().unwrap().message, "Done");
}

#[test]
fn progress_commit_with_remove_emits_removal_step() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let mut opts = stash_options(&proj);
    opts.remove_sources = true;
    let events = stash_recorded(&ctx, &opts);
    let percents: Vec<u8> = events.iter().map(|e| e.percent).collect();
    assert_eq!(
        percents,
        vec![0, 30, 70, 90, 100],
        "removal must add the 90 step"
    );
}

#[test]
fn progress_dry_run_emits_two_events() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let events = stash_recorded(&ctx, &stash_options(&proj));
    let percents: Vec<u8> = events.iter().map(|e| e.percent).collect();
    assert_eq!(percents, vec![0, 100], "dry run must emit select + done");
    let completes: Vec<bool> = events.iter().map(|e| e.phase_complete).collect();
    assert_eq!(completes, vec![false, true]);
}

// --- Extras ------------------------------------------------------------------

#[test]
fn staging_is_clean_after_commit() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    stash_once(&ctx, &stash_options(&proj));

    let leftover = collect_files(&den.join("staging"));
    assert!(
        leftover.is_empty(),
        "staging must be clean after a successful commit: {leftover:?}"
    );
}

#[test]
fn den_inside_project_is_rejected() {
    let (temp, proj) = project_dir();
    let _ = temp; // TempDir stays alive until end of scope, then cleans up.
    let den = proj.join(".den");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    write(&proj, ".env", "PASSWORD=value\n");

    let err = stash(&ctx, &stash_options(&proj), &identity(), &mut NullProgress)
        .expect_err("a den nested inside the project tree must be rejected");
    assert!(
        matches!(&err, Error::Other { .. }),
        "expected Error::Other, got {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("staging") || msg.contains("inside"),
        "guard message should mention the hazard: {msg}"
    );
    assert!(
        !msg.contains(PASSWORD_VALUE),
        "error must not leak raw secret material: {msg}"
    );
}

#[test]
fn batch_id_controls_name_token_but_not_year_month() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let mut opts = stash_options(&proj);
    opts.batch_id = Some("nightly-run".to_string());
    let result = stash_once(&ctx, &opts);

    let rel = result
        .archive_path
        .strip_prefix(&den)
        .expect("archive under den");
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(parts.len(), 4);
    assert!(
        parts[1].len() == 4 && parts[1].chars().all(|c| c.is_ascii_digit()),
        "year must still derive from now: {:?}",
        parts[1]
    );
    assert_eq!(parts[3], "proj__nightly-run__secrets.age");
    assert!(result.archive_path.is_file());
}

#[test]
fn invalid_batch_id_is_rejected() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, ".env", "PASSWORD=value\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let mut opts = stash_options(&proj);
    opts.batch_id = Some("a/b".to_string());
    let err = stash(&ctx, &opts, &identity(), &mut NullProgress).unwrap_err();
    assert!(
        err.to_string().contains("invalid stash batch id"),
        "got: {err}"
    );
}
