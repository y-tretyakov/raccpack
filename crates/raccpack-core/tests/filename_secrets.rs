//! Integration tests for M3.1 — filename patterns + risk model (severity API).
//!
//! Covers the behavioral contract from
//! docs/mvp/m3/m3.1-filename-patterns-risk.md: the data-driven
//! `DEFAULT_FILENAME_PATTERNS` table, `match_filename` / `match_filename_all`
//! (Exact/Suffix/Prefix/Contains checks on `file_name()`, case-sensitive, only
//! files, max-risk on multiple hits with first-in-table tie-break),
//! `scan_filenames` (root validation, `SkipPolicy`, `max_depth`, files only,
//! symlinks never followed, `min_risk` filtering, deterministic path ordering)
//! and the severity helpers `upgrade_risk` / `SensitiveRisk::at_least`.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//! Symlink tests are Linux/Unix-only, so they are `#[cfg(unix)]`.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{
    match_filename, match_filename_all, scan_filenames, upgrade_risk, Error, FilenameScanOptions,
    FindingSource, NameMatchKind, SensitiveFinding, SensitiveRisk, DEFAULT_FILENAME_PATTERNS,
};
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`.
fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, "x").expect("write fixture file");
}

/// Create a directory (and parents) at `root/rel`, leaving it empty.
fn write_dir(root: &Path, rel: &str) {
    fs::create_dir_all(root.join(rel)).expect("create fixture dir");
}

/// Run `scan_filenames` and unwrap; temp fixtures must never fail.
fn scan(root: &Path, opts: &FilenameScanOptions) -> Vec<SensitiveFinding> {
    scan_filenames(root, opts).expect("scan_filenames must succeed on temp fixture")
}

/// Finding paths relative to `root`, preserving the returned order.
fn rel_paths(findings: &[SensitiveFinding], root: &Path) -> Vec<PathBuf> {
    findings
        .iter()
        .map(|f| {
            f.path
                .strip_prefix(root)
                .expect("finding must live under the scan root")
                .to_path_buf()
        })
        .collect()
}

// --- Mandatory case 1: .env --------------------------------------------------

#[test]
fn env_file_is_high_risk() {
    let m = match_filename(Path::new("/proj/.env")).expect(".env must match");
    assert_eq!(m.pattern_id, "env_file");
    assert_eq!(m.risk, SensitiveRisk::High);
}

// --- Mandatory case 2: id_rsa -------------------------------------------------

#[test]
fn id_rsa_is_critical_risk() {
    let m = match_filename(Path::new("/proj/id_rsa")).expect("id_rsa must match");
    assert_eq!(m.pattern_id, "id_rsa");
    assert_eq!(m.risk, SensitiveRisk::Critical);
}

// --- Mandatory case 3: notes.txt has no match ----------------------------------

#[test]
fn notes_txt_has_no_filename_match() {
    assert!(match_filename(Path::new("/proj/notes.txt")).is_none());
    assert!(match_filename_all(Path::new("/proj/notes.txt")).is_empty());
}

// --- Mandatory case 4: foo.pem suffix ------------------------------------------

#[test]
fn pem_suffix_is_high_risk() {
    let m = match_filename(Path::new("/proj/foo.pem")).expect("pem must match");
    assert_eq!(m.pattern_id, "private_key_pem");
    assert_eq!(m.risk, SensitiveRisk::High);
}

// --- Mandatory case 5: dual pattern -> max risk ---------------------------------

#[test]
fn env_production_dual_pattern_max_risk() {
    let m = match_filename(Path::new("/proj/.env.production")).expect(".env.production must match");
    assert_eq!(m.pattern_id, "env_prod");
    assert_eq!(m.risk, SensitiveRisk::Critical);

    let all = match_filename_all(Path::new("/proj/.env.production"));
    assert_eq!(all.len(), 2);
    let ids: Vec<&str> = all.iter().map(|m| m.pattern_id.as_str()).collect();
    assert_eq!(ids, vec!["env_prod", "env_prefix"]);
    let risks: Vec<SensitiveRisk> = all.iter().map(|m| m.risk).collect();
    assert_eq!(risks, vec![SensitiveRisk::Critical, SensitiveRisk::High]);
}

/// `.env.local` matches two High rows; the tie must break to the first row in
/// the table (`env_local`), while `match_filename_all` keeps table order.
#[test]
fn env_local_tie_breaks_to_first_in_table() {
    let m = match_filename(Path::new("/proj/.env.local")).expect(".env.local must match");
    assert_eq!(m.pattern_id, "env_local");
    assert_eq!(m.risk, SensitiveRisk::High);

    let all = match_filename_all(Path::new("/proj/.env.local"));
    let ids: Vec<&str> = all.iter().map(|m| m.pattern_id.as_str()).collect();
    assert_eq!(ids, vec!["env_local", "env_prefix"]);
}

// --- Mandatory case 6: scan skips node_modules ---------------------------------

#[test]
fn scan_filenames_skips_node_modules() {
    let root = TempDir::new().unwrap();
    write(root.path(), ".env");
    write(root.path(), "node_modules/.env");

    let findings = scan(root.path(), &FilenameScanOptions::default());
    let rels = rel_paths(&findings, root.path());
    assert_eq!(rels, vec![PathBuf::from(".env")]);
    assert!(
        !rels.iter().any(|p| p.starts_with("node_modules")),
        "node_modules must not be descended: {rels:?}"
    );
    let f = &findings[0];
    assert_eq!(f.risk, SensitiveRisk::High);
    assert!(
        matches!(&f.source, FindingSource::Filename { pattern_id } if pattern_id == "env_file")
    );
    assert!(!f.label.is_empty(), "findings carry a human label");
}

// --- Mandatory case 7: min_risk filters High -----------------------------------

#[test]
fn scan_filenames_min_risk_filters_high() {
    let root = TempDir::new().unwrap();
    write(root.path(), ".env");
    write(root.path(), "id_rsa");

    let opts = FilenameScanOptions {
        min_risk: SensitiveRisk::Critical,
        ..FilenameScanOptions::default()
    };
    let rels = rel_paths(&scan(root.path(), &opts), root.path());
    assert_eq!(rels, vec![PathBuf::from("id_rsa")]);
}

// --- Mandatory case 8: severity ordering ----------------------------------------

#[test]
fn risk_ordering_low_medium_high_critical() {
    let mut levels = vec![
        SensitiveRisk::Critical,
        SensitiveRisk::Low,
        SensitiveRisk::High,
        SensitiveRisk::Medium,
    ];
    levels.sort();
    assert_eq!(
        levels,
        vec![
            SensitiveRisk::Low,
            SensitiveRisk::Medium,
            SensitiveRisk::High,
            SensitiveRisk::Critical,
        ]
    );
    assert!(SensitiveRisk::Low < SensitiveRisk::Medium);
    assert!(SensitiveRisk::Medium < SensitiveRisk::High);
    assert!(SensitiveRisk::High < SensitiveRisk::Critical);
}

// --- Mandatory case 9: upgrade_risk never downgrades ----------------------------

#[test]
fn upgrade_risk_never_downgrades() {
    assert_eq!(
        upgrade_risk(SensitiveRisk::High, SensitiveRisk::Critical),
        SensitiveRisk::Critical
    );
    assert_eq!(
        upgrade_risk(SensitiveRisk::Low, SensitiveRisk::Medium),
        SensitiveRisk::Medium
    );
    assert_eq!(
        upgrade_risk(SensitiveRisk::Critical, SensitiveRisk::Low),
        SensitiveRisk::Critical
    );
    assert_eq!(
        upgrade_risk(SensitiveRisk::High, SensitiveRisk::High),
        SensitiveRisk::High
    );
}

// --- Mandatory case 10: deterministic finding order ------------------------------

#[test]
fn scan_filenames_results_are_deterministic() {
    let root = TempDir::new().unwrap();
    // Scrambled creation order; results must be path-ascending regardless.
    write(root.path(), "z/foo.pem");
    write(root.path(), "b/wallet.dat");
    write(root.path(), "a/.env");
    write(root.path(), "m/id_rsa");

    let findings = scan(root.path(), &FilenameScanOptions::default());
    let rels = rel_paths(&findings, root.path());
    assert_eq!(
        rels,
        vec![
            PathBuf::from("a/.env"),
            PathBuf::from("b/wallet.dat"),
            PathBuf::from("m/id_rsa"),
            PathBuf::from("z/foo.pem"),
        ]
    );
    assert_eq!(findings[0].risk, SensitiveRisk::High);
    assert_eq!(findings[1].risk, SensitiveRisk::Critical);
}

// --- Extra: match_filename_all table order ---------------------------------------

#[test]
fn match_filename_all_preserves_table_order() {
    let all = match_filename_all(Path::new("/proj/.env.production"));
    let ids: Vec<&str> = all.iter().map(|m| m.pattern_id.as_str()).collect();
    assert_eq!(ids, vec!["env_prod", "env_prefix"]);
}

// --- Extra: credentials basename ------------------------------------------------

#[test]
fn credentials_basename_is_high_risk() {
    let m = match_filename(Path::new("/home/user/.aws/credentials")).expect("must match");
    assert_eq!(m.risk, SensitiveRisk::High);
    assert_eq!(m.pattern_id, "aws_credentials");

    let all = match_filename_all(Path::new("/proj/credentials"));
    let ids: Vec<&str> = all.iter().map(|m| m.pattern_id.as_str()).collect();
    assert_eq!(ids, vec!["aws_credentials", "aws_credentials_path"]);
}

// --- Extra: wallet.dat (Contains) -----------------------------------------------

#[test]
fn wallet_dat_and_containing_names_critical() {
    for name in ["wallet.dat", "backup-wallet.dat.old", "foo/wallet.dat"] {
        let m = match_filename(Path::new(name))
            .unwrap_or_else(|| panic!("`{name}` must match the wallet pattern"));
        assert_eq!(m.pattern_id, "wallet");
        assert_eq!(m.risk, SensitiveRisk::Critical);
    }
}

// --- Extra: directories are not filename secrets --------------------------------

#[test]
fn scan_filenames_ignores_dot_env_directory() {
    let root = TempDir::new().unwrap();
    write_dir(root.path(), ".env");
    write(root.path(), ".env/placeholder.txt");
    write(root.path(), "keep/.env");

    let rels = rel_paths(
        &scan(root.path(), &FilenameScanOptions::default()),
        root.path(),
    );
    assert_eq!(
        rels,
        vec![PathBuf::from("keep/.env")],
        "a directory named `.env` must not produce a finding"
    );
}

// --- Extra: max_depth ------------------------------------------------------------

#[test]
fn scan_filenames_respects_max_depth() {
    let root = TempDir::new().unwrap();
    write(root.path(), ".env");
    write(root.path(), "a/b/.env");
    write(root.path(), "a/b/id_rsa");

    let opts = FilenameScanOptions {
        max_depth: 1,
        ..FilenameScanOptions::default()
    };
    let rels = rel_paths(&scan(root.path(), &opts), root.path());
    assert_eq!(rels, vec![PathBuf::from(".env")]);
}

// --- Extra: ensure_scan_root error cases -----------------------------------------

#[test]
fn scan_filenames_nonexistent_root_path_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("does-not-exist");
    let err = scan_filenames(&missing, &FilenameScanOptions::default()).expect_err("must fail");
    assert!(matches!(err, Error::PathNotFound { .. }));
}

#[test]
fn scan_filenames_file_root_not_a_directory() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a-file.txt");
    fs::write(&file, "not a dir").unwrap();
    let err = scan_filenames(&file, &FilenameScanOptions::default()).expect_err("must fail");
    assert!(matches!(err, Error::NotADirectory { .. }));
}

// --- Extra: case sensitivity ------------------------------------------------------

#[test]
fn filename_match_is_case_sensitive() {
    assert!(
        match_filename(Path::new("/proj/.ENV")).is_none(),
        "`.ENV` must not match the `.env` pattern on Linux"
    );
    assert!(match_filename(Path::new("/proj/ID_RSA")).is_none());
    assert!(match_filename(Path::new("/proj/.env")).is_some());
}

#[test]
fn scan_filenames_is_case_sensitive() {
    let root = TempDir::new().unwrap();
    write(root.path(), ".ENV");
    write(root.path(), ".env");

    let rels = rel_paths(
        &scan(root.path(), &FilenameScanOptions::default()),
        root.path(),
    );
    assert_eq!(rels, vec![PathBuf::from(".env")]);
}

// --- Extra: at_least threshold ----------------------------------------------------

#[test]
fn risk_at_least_threshold_behavior() {
    assert!(SensitiveRisk::High.at_least(SensitiveRisk::High));
    assert!(SensitiveRisk::Critical.at_least(SensitiveRisk::High));
    assert!(SensitiveRisk::Low.at_least(SensitiveRisk::Low));
    assert!(!SensitiveRisk::High.at_least(SensitiveRisk::Critical));
    assert!(!SensitiveRisk::Low.at_least(SensitiveRisk::Medium));
}

// --- Extra: table integrity ---------------------------------------------------------

#[test]
fn default_filename_patterns_table_integrity() {
    const EXPECTED_ROWS: usize = 28;
    assert_eq!(
        DEFAULT_FILENAME_PATTERNS.len(),
        EXPECTED_ROWS,
        "table must hold the MVP rows from the spec"
    );

    let mut seen = std::collections::HashSet::new();
    for p in DEFAULT_FILENAME_PATTERNS {
        assert!(!p.id.is_empty(), "every pattern needs an id");
        assert!(seen.insert(p.id), "duplicate pattern id: {}", p.id);
        assert!(
            !p.pattern.is_empty(),
            "pattern {} has an empty pattern",
            p.id
        );
        assert!(!p.label.is_empty(), "pattern {} has an empty label", p.id);
    }

    let by_id = |id: &str| {
        DEFAULT_FILENAME_PATTERNS
            .iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("pattern id `{id}` must exist"))
    };
    assert_eq!(by_id("env_file").risk, SensitiveRisk::High);
    assert_eq!(by_id("env_file").kind, NameMatchKind::Exact);
    assert_eq!(by_id("env_prod").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("env_prefix").kind, NameMatchKind::Prefix);
    assert_eq!(by_id("id_rsa").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("id_rsa").kind, NameMatchKind::Exact);
    assert_eq!(by_id("private_key_pem").kind, NameMatchKind::Suffix);
    assert_eq!(by_id("private_key_pem").risk, SensitiveRisk::High);
    assert_eq!(by_id("wallet").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("wallet").kind, NameMatchKind::Contains);
    assert_eq!(by_id("docker_config").risk, SensitiveRisk::Medium);
    assert_eq!(by_id("git_credentials").risk, SensitiveRisk::Critical);

    // The `credentials` basename is intentionally represented twice with
    // distinct ids (context-dependent rows); both must be High.
    let aws: Vec<_> = DEFAULT_FILENAME_PATTERNS
        .iter()
        .filter(|p| p.pattern == "credentials")
        .collect();
    assert_eq!(aws.len(), 2);
    assert!(aws.iter().all(|p| p.risk == SensitiveRisk::High));
}

// --- Extra: symlinks are never followed -------------------------------------------

#[cfg(unix)]
#[test]
fn scan_filenames_does_not_follow_symlink_dir() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    write(outside.path(), ".env");
    symlink(outside.path(), root.path().join("link")).unwrap();

    let findings = scan(root.path(), &FilenameScanOptions::default());
    assert!(
        !rel_paths(&findings, root.path())
            .iter()
            .any(|p| p.ends_with(".env")),
        "contents of a symlinked external dir must never be reported"
    );
}
