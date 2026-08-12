//! Integration tests for M4.3 — facade `pack` + DryRun/Commit.
//!
//! Covers the mandatory cases from docs/mvp/m4/m4.3-facade-pack.md §5 and the
//! facade contract in raccpack-facade-and-den.md §7: DryRun writes nothing,
//! Commit writes a readable `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst`, name-deny
//! (`.env`) is always on, content-deny (AKIA) is on by default and via the
//! explicit flag, progress events follow the spec table, path errors map to
//! `Error::PathNotFound` / `Error::NotADirectory`, the den bootstraps
//! (`.den-version`, `README.txt`, `packs/`), `PackResult` serde roundtrips, a
//! custom `output_name` lands at `packs/{yyyy}/{mm}/my-artifact.tar.zst`, a
//! repeat pack never clobbers the previous artifact even in the same second,
//! staging is left clean, and a den nested inside the project is rejected.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git, no
//! sleeps, and the den always points into the TempDir. Assertions are robust to
//! a 1-second boundary crossing (see `pack_repeat_run_does_not_clobber`).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use raccpack_core::{
    pack, AppContext, Error, NullProgress, OperationKind, PackOptions, PackResult, ProgressEvent,
    ProgressSink, RaccConfig, RunMode,
};
use tempfile::TempDir;

/// A deterministic AWS-style access key id (matches the `aws_access_key` prefix).
const AWS_ACCESS_KEY: &str = "AKIAABCDEFGHIJKLMNOPQRST";

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

/// Build an `AppContext` for a `pack` run: scan root = the project itself, and
/// the den is always an explicit TempDir path so the real `~/.raccpack/den` is
/// never touched.
fn ctx_for(project_root: &Path, den_dir: &Path, mode: RunMode) -> AppContext {
    let config = RaccConfig::default()
        .with_scan_root(project_root)
        .with_den_dir(den_dir);
    AppContext::from_config(config, mode).expect("AppContext::from_config")
}

/// Default pack options for a project with content-deny as specified.
fn pack_options(project: &Path, deny_content_secrets: bool) -> PackOptions {
    PackOptions {
        project: project.to_path_buf(),
        output_name: None,
        deny_content_secrets,
        zstd_level: None,
    }
}

/// Run `pack` with a null sink; panics with context on error.
fn pack_once(ctx: &AppContext, opts: &PackOptions) -> PackResult {
    let mut progress = NullProgress;
    pack(ctx, opts, &mut progress).expect("pack should succeed")
}

/// Run `pack` recording every progress event; returns the events.
fn pack_recorded(ctx: &AppContext, opts: &PackOptions) -> Vec<ProgressEvent> {
    let mut sink = RecordingSink::default();
    pack(ctx, opts, &mut sink).expect("pack should succeed");
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

/// Decode a `.tar.zst` archive and return its entry names.
fn unpack_names(path: &Path) -> Vec<String> {
    let bytes = fs::read(path).expect("read archive bytes");
    let decoder =
        zstd::stream::read::Decoder::new(std::io::Cursor::new(bytes)).expect("zstd decode header");
    let mut archive = tar::Archive::new(decoder);
    let mut out = Vec::new();
    for entry in archive.entries().expect("read tar entries") {
        let mut entry = entry.expect("valid tar entry");
        let name = entry.path().unwrap().to_string_lossy().into_owned();
        let mut content = Vec::new();
        Read::read_to_end(&mut entry, &mut content).expect("read entry contents");
        out.push(name);
    }
    out
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

// --- Case 1: DryRun writes nothing ------------------------------------------

#[test]
fn dry_run_creates_no_den_files() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let result = pack_once(&ctx, &pack_options(&proj, true));
    assert!(result.dry_run, "dry run must be reported");
    assert_eq!(result.source, proj);

    assert!(
        !den.join("packs").exists(),
        "dry run must not create packs/"
    );
    assert!(
        !den.join(".den-version").exists(),
        "dry run must not create the den skeleton"
    );

    let path_str = result.output.to_string_lossy();
    assert!(
        path_str.contains("packs/"),
        "output must live under packs/: {path_str}"
    );
    assert!(
        path_str.ends_with(".tar.zst"),
        "output must end with .tar.zst: {path_str}"
    );
    let file_name = result.output.file_name().unwrap().to_string_lossy();
    assert!(
        file_name.starts_with("proj__"),
        "output must carry the project slug: {file_name}"
    );
}

// --- Case 2: Commit writes a readable archive -------------------------------

#[test]
fn commit_writes_readable_archive() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    write(&proj, "notes.txt", "hello\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = pack_once(&ctx, &pack_options(&proj, true));
    assert!(!result.dry_run, "commit must report dry_run == false");
    assert_eq!(result.source, proj);
    assert!(
        result.output.is_file(),
        "artifact missing: {}",
        result.output.display()
    );
    assert!(
        result.size_bytes > 0,
        "non-empty project must yield a non-empty archive"
    );
    assert!(result.file_count >= 2, "two files must be packed");

    let names = unpack_names(&result.output);
    assert!(names.iter().any(|n| n == "src/main.rs"), "{names:?}");
    assert!(names.iter().any(|n| n == "notes.txt"), "{names:?}");
}

// --- Case 3: `.env` excluded by name-deny ------------------------------------

#[test]
fn commit_skips_env_file_by_name() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    write(&proj, ".env", "TOKEN=secret-value-1234567890\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let result = pack_once(&ctx, &pack_options(&proj, true));
    assert!(
        result.skipped_secret_files >= 1,
        "`.env` must be counted as skipped"
    );
    let names = unpack_names(&result.output);
    assert!(
        !names.iter().any(|n| n == ".env"),
        "`.env` must not be archived: {names:?}"
    );
    assert!(names.iter().any(|n| n == "src/main.rs"), "{names:?}");
}

// --- Case 4: content-deny on (explicit and default) --------------------------

#[test]
fn content_deny_skips_cred_file_explicit_and_default() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    write(&proj, "creds.txt", &format!("{AWS_ACCESS_KEY}\n"));
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    // Explicit flag.
    let explicit = pack_once(&ctx, &pack_options(&proj, true));
    assert!(
        explicit.skipped_secret_files >= 1,
        "the AKIA file must be counted as skipped"
    );
    let names = unpack_names(&explicit.output);
    assert!(!names.iter().any(|n| n == "creds.txt"), "{names:?}");
    assert!(names.iter().any(|n| n == "src/main.rs"), "{names:?}");

    // Default opts behave identically because `deny_content_secrets` defaults true.
    assert!(
        PackOptions::default().deny_content_secrets,
        "content deny must default on for the facade"
    );
    let defaults = pack_once(
        &ctx,
        &PackOptions {
            project: proj,
            ..Default::default()
        },
    );
    assert!(
        defaults.skipped_secret_files >= 1,
        "default opts must skip the AKIA file"
    );
    let names2 = unpack_names(&defaults.output);
    assert!(!names2.iter().any(|n| n == "creds.txt"), "{names2:?}");
}

// --- Case 5: progress events -------------------------------------------------

#[test]
fn pack_progress_commit_emits_spec_table() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let events = pack_recorded(&ctx, &pack_options(&proj, true));
    let percents: Vec<u8> = events.iter().map(|e| e.percent).collect();
    assert_eq!(
        percents,
        vec![0, 30, 80, 100],
        "commit must emit the spec progress steps"
    );
    assert_eq!(events.len(), 4);

    for e in &events {
        assert_eq!(e.operation, OperationKind::Pack, "operation must be Pack");
        assert_eq!(e.phase, "pack", "phase must be \"pack\"");
        assert_eq!(
            e.percent, e.overall_percent,
            "single-phase pack must equate percent and overall_percent"
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
    let last = events.last().unwrap();
    assert_eq!(last.percent, 100);
    assert_eq!(last.message, "Done");
}

#[test]
fn pack_progress_dry_run_emits_two_events() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    let ctx = ctx_for(&proj, &den, RunMode::DryRun);

    let events = pack_recorded(&ctx, &pack_options(&proj, true));
    let percents: Vec<u8> = events.iter().map(|e| e.percent).collect();
    assert_eq!(percents, vec![0, 100], "dry run must emit prepare + done");
    assert_eq!(events.len(), 2);

    for e in &events {
        assert_eq!(e.operation, OperationKind::Pack);
        assert_eq!(e.phase, "pack");
        assert_eq!(e.percent, e.overall_percent);
    }
    let completes: Vec<bool> = events.iter().map(|e| e.phase_complete).collect();
    assert_eq!(
        completes,
        vec![false, true],
        "dry run completes on the last event"
    );
}

// --- Case 6: invalid project path --------------------------------------------

#[test]
fn pack_missing_project_path_is_error() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let missing = temp.path().join("does-not-exist");
    let err = pack(&ctx, &pack_options(&missing, true), &mut NullProgress)
        .expect_err("a missing project path must fail");
    assert!(matches!(err, Error::PathNotFound { .. }), "{err:?}");
}

#[test]
fn pack_project_is_a_file_is_error() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "a-file.txt", "not a directory");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    let file = proj.join("a-file.txt");
    let err = pack(&ctx, &pack_options(&file, true), &mut NullProgress)
        .expect_err("a file project path must fail");
    assert!(matches!(err, Error::NotADirectory { .. }), "{err:?}");
}

// --- Case 7: den bootstrap -----------------------------------------------------

#[test]
fn pack_bootstraps_den_on_first_commit() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    pack_once(&ctx, &pack_options(&proj, true));

    let version = fs::read_to_string(den.join(".den-version")).expect(".den-version exists");
    assert!(
        version.contains('1'),
        "den version must be written: {version:?}"
    );
    assert!(den.join("README.txt").is_file(), "README.txt must exist");
    assert!(den.join("packs").is_dir(), "packs/ must exist");
}

// --- Case 8: PackResult serde roundtrip ---------------------------------------

#[test]
fn pack_result_serde_roundtrip() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    let result = pack_once(&ctx, &pack_options(&proj, true));

    assert!(!result.dry_run);
    let json = serde_json::to_string(&result).expect("serialize PackResult");
    let decoded: PackResult = serde_json::from_str(&json).expect("deserialize PackResult");
    assert_eq!(decoded, result, "serde roundtrip must preserve PackResult");
}

// --- Case 9: custom output_name ------------------------------------------------

#[test]
fn pack_custom_output_name_places_named_artifact() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    let opts = PackOptions {
        project: proj.clone(),
        output_name: Some("my-artifact".to_string()),
        ..Default::default()
    };

    let result = pack_once(&ctx, &opts);
    assert_eq!(
        result
            .output
            .file_name()
            .map(|n| n.to_string_lossy().into_owned()),
        Some("my-artifact.tar.zst".to_string())
    );

    // The artifact must sit at exactly `packs/{yyyy}/{mm}/my-artifact.tar.zst`.
    let pouch = den.join("packs");
    let found: Vec<PathBuf> = collect_files(&pouch)
        .into_iter()
        .filter(|p| {
            p.file_name()
                .map(|n| n == "my-artifact.tar.zst")
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(found.len(), 1, "exactly one named artifact: {found:?}");
    let rel = found[0]
        .strip_prefix(&pouch)
        .expect("artifact under packs/");
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    assert_eq!(parts.len(), 3, "packs/{{yyyy}}/{{mm}}/name, got {rel:?}");
    assert_eq!(parts[2], "my-artifact.tar.zst");
    assert!(
        parts[0].len() == 4 && parts[0].chars().all(|c| c.is_ascii_digit()),
        "year segment must be yyyy: {:?}",
        parts[0]
    );
    assert!(
        parts[1].len() == 2 && parts[1].chars().all(|c| c.is_ascii_digit()),
        "month segment must be mm: {:?}",
        parts[1]
    );
    assert_eq!(
        result.output, found[0],
        "result must point at the placed file"
    );
}

// --- Case 10: repeat pack never clobbers ----------------------------------------

#[test]
fn pack_repeat_run_does_not_clobber_previous_artifact() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    write(&proj, "notes.txt", "hello\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    let opts = pack_options(&proj, true);

    let first = pack_once(&ctx, &opts);
    let second = pack_once(&ctx, &opts);

    // Both artifacts must survive the back-to-back run. This holds whether the
    // two natural names collide (same second — the common case, so the second
    // run must disambiguate) or land in different seconds; no clobber ever.
    assert!(
        first.output.is_file(),
        "first artifact must exist: {}",
        first.output.display()
    );
    assert!(
        second.output.is_file(),
        "second artifact must exist: {}",
        second.output.display()
    );

    let existing = collect_files(&den.join("packs"));
    assert!(
        existing.len() >= 2,
        "a repeat pack must never destroy the previous artifact; found {} files: {existing:?}",
        existing.len()
    );
}

// --- Case 11: staging hygiene -----------------------------------------------------

#[test]
fn pack_leaves_no_staging_artifacts() {
    let (temp, proj) = project_dir();
    let den = temp.path().join("den");
    write(&proj, "src/main.rs", "fn main() {}\n");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);

    pack_once(&ctx, &pack_options(&proj, true));

    let staging = den.join("staging");
    let leftover = collect_files(&staging);
    assert!(
        leftover.is_empty(),
        "staging must be clean after a successful commit: {leftover:?}"
    );
    assert!(
        !leftover
            .iter()
            .any(|f| f.file_name().map(|n| n == "pack.tar.zst").unwrap_or(false)),
        "no pack.tar.zst may remain in staging"
    );
}

// --- Case 12: den inside project is rejected -------------------------------------

#[test]
fn pack_den_inside_project_is_rejected() {
    let (temp, proj) = project_dir();
    let _ = temp; // TempDir stays alive until end of scope, then cleans up.
    let den = proj.join(".den");
    let ctx = ctx_for(&proj, &den, RunMode::Commit);
    write(&proj, "src/main.rs", "fn main() {}\n");

    let err = pack(&ctx, &pack_options(&proj, true), &mut NullProgress)
        .expect_err("a den nested inside the packed tree must be rejected");
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
        !msg.contains(AWS_ACCESS_KEY) && !msg.contains("secret-value"),
        "error must not leak raw secret material: {msg}"
    );

    let polluted = collect_files(&den);
    assert!(
        !polluted
            .iter()
            .any(|f| f.to_string_lossy().ends_with("tar.zst")),
        "no partial archive may remain in the nested den: {polluted:?}"
    );
}
