//! Integration tests for B1.5 / B3.4 — ephemeral reveal API.
//!
//! Covers the opt-in `EphemeralSecret` + `FindingRef` + `reveal_finding`
//! surface:
//!   1. `EphemeralSecret` Debug redacts and `expose()` returns the raw value.
//!   2. `FindingRef` serde round-trips and never carries raw/masked values.
//!   3. `reveal_finding` returns the exact original value for a Prefix marker.
//!   4. `reveal_finding` returns the exact original value for a Regex marker.
//!   5. Disambiguation: one line with multiple candidates → hash-matched one.
//!   6. A stale reference (value changed since dig) ⇒ `Error::Other`.
//!   7. A path outside `dir_root` ⇒ `Error::PathOutsideTarget`.
//!   8. A directory (not a file) ⇒ `Error::NotAFile`.
//!   9. dig DTO: `SensitiveFile.content_ref` mirrors `content_match.value_hash`
//!      for content matches, and is `None` for filename-only findings.
//!   10. Revealed value round-trips: `mask_secret(revealed).value_hash` equals
//!       the reference hash.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no git needed.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

// Re-exported through the crate root via `lib.rs`.
use raccpack_core::{
    dig, fingerprint_secret, mask_secret, reveal_finding, AppContext, DigOptions, EphemeralSecret,
    Error, FindingRef, NullProgress, RaccConfig, RunMode, SensitiveFile, SensitiveRisk,
};

// --- Test helpers -----------------------------------------------------------

/// Create a `TempDir` root and write a file returning its absolute path.
fn write(root: &Path, name: &str, contents: &[u8]) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, contents).expect("write fixture file");
    path
}

/// A reference for the `aws_access_key` (Prefix "AKIA") marker on a line.
fn aws_ref(dir: &Path, name: &str, line: u32, value: &str) -> FindingRef {
    let path = dir.join(name);
    FindingRef {
        path: path.clone(),
        marker_id: "aws_access_key".to_string(),
        line,
        value_hash: fingerprint_secret(value),
    }
}

/// A reference for the `github_pat` (Prefix "ghp_") marker on a line.
fn gh_ref(dir: &Path, name: &str, line: u32, value: &str) -> FindingRef {
    let path = dir.join(name);
    FindingRef {
        path: path.clone(),
        marker_id: "github_pat".to_string(),
        line,
        value_hash: fingerprint_secret(value),
    }
}

/// Build an `AppContext` from a config pointing at `root` (den derived as a
/// sibling so no real `~/.raccpack/den` is ever touched).
fn ctx_for(root: &Path) -> AppContext {
    let den = root.parent().expect("scan root has a parent").join("den");
    let config = RaccConfig::default()
        .with_scan_root(root)
        .with_den_dir(&den);
    AppContext::from_config(config, RunMode::DryRun).expect("AppContext::from_config")
}

fn dig_project(root: &Path) -> raccpack_core::DigResult {
    let ctx = ctx_for(root);
    let opts = DigOptions {
        project: None,
        find_repeated: false,
        scan_content: true,
        use_heuristics: None,
    };
    let mut progress = NullProgress;
    dig(&ctx, &opts, &mut progress).expect("dig should succeed")
}

// Case 1: EphemeralSecret Debug redacts, expose returns raw.
#[test]
fn ephemeral_secret_debug_redacts_and_expose_returns_raw() {
    let secret = EphemeralSecret::new("AKIAABCDEFGHIJKLMNOPQRST".to_string());
    assert_eq!(secret.expose(), "AKIAABCDEFGHIJKLMNOPQRST");
    let debug = format!("{secret:?}");
    assert!(
        !debug.contains("AKIAABCDEFGHIJKLMNOPQRST"),
        "Debug must not leak the raw value: {debug}"
    );
    assert!(
        debug.contains("(**)") || debug.contains("ephemeral"),
        "redacted form expected: {debug}"
    );
}

// Case 2: FindingRef serde round-trip; no raw/masked value in JSON.
#[test]
fn finding_ref_serde_roundtrip_without_raw_or_masked() {
    let reference = FindingRef {
        path: PathBuf::from("/repo/.env"),
        marker_id: "aws_access_key".to_string(),
        line: 3,
        value_hash: fingerprint_secret("AKIAABCDEFGHIJKLMNOPQRST"),
    };
    let json = serde_json::to_string(&reference).expect("FindingRef serializes");
    assert!(
        !json.contains("AKIAABCDEFGHIJKLMNOPQRST"),
        "serialized FindingRef must not carry the raw value: {json}"
    );
    assert!(!json.contains("abcd…"), "no masked preview in JSON: {json}");
    let back: FindingRef = serde_json::from_str(&json).expect("FindingRef deserializes");
    assert_eq!(back, reference);
}

// Case 4: Regex marker — `aws_secret_assign`.
#[test]
fn reveal_content_ref_lens_original_aws_secret_assign_regex() {
    let dir = TempDir::new().unwrap();
    let value = "MYAWSsecretvalue123456789";
    let path = write(
        dir.path(),
        ".env",
        format!("AWS_SECRET_ACCESS_KEY = \"{value}\"\n").as_bytes(),
    );

    // Scan first to get a finding with content_ref, then reveal through it.
    let result = dig_project(dir.path());
    let file = result
        .files
        .iter()
        .find(|f| f.path == path)
        .expect(".env must be found");
    let reference = file
        .content_ref
        .as_ref()
        .expect("content hit => content_ref");
    assert_eq!(reference.marker_id, "aws_secret_assign");

    let secret = reveal_finding(&path, dir.path(), reference).expect("reveal should succeed");
    assert_eq!(
        secret.expose(),
        &format!("AWS_SECRET_ACCESS_KEY = \"{value}\"")
    );
}

// Case 3: Prefix marker (AKIA) returns exact original value.
#[test]
fn reveal_content_ref_lens_original_aws_prefix_value() {
    let dir = TempDir::new().unwrap();
    let value = "AKIAABCDEFGHIJKLMNOPQRST";
    let path = write(dir.path(), "creds.txt", format!("{value}\n").as_bytes());

    let reference = aws_ref(dir.path(), "creds.txt", 1, value);
    let secret = reveal_finding(&path, dir.path(), &reference).expect("reveal should succeed");
    assert_eq!(secret.expose(), value);

    // Round-trip: the revealed value re-masks to the same fingerprint.
    let masked = mask_secret(secret.expose());
    assert_eq!(masked.value_hash, reference.value_hash);
}

// Case 5: Disambiguate multiple candidate tokens on one line by hash.
#[test]
fn reveal_disambiguates_multiple_candidates_by_hash() {
    let dir = TempDir::new().unwrap();
    let want = "ghp_1111222233334444";
    let other = "ghp_AAAA";
    let path = write(
        dir.path(),
        "tokens.txt",
        format!("{other} {want}\n").as_bytes(),
    );

    let reference = gh_ref(dir.path(), "tokens.txt", 1, want);
    let secret = reveal_finding(&path, dir.path(), &reference).expect("matched candidate found");
    assert_eq!(secret.expose(), want);
}

// Case 6: Stale reference (hash no longer matches) => Error::Other.
#[test]
fn reveal_rejects_stale_hash_after_value_changed() {
    let dir = TempDir::new().unwrap();
    let path = write(dir.path(), "creds.txt", b"AKIASOMETHINGELSE12345\n");

    // Reference built against a value that is NOT in the file.
    let reference = aws_ref(dir.path(), "creds.txt", 1, "AKIANOTINFILE987654");
    let err = reveal_finding(&path, dir.path(), &reference).expect_err("stale reference must fail");
    match err {
        Error::Other { message } => {
            assert!(message.contains("no longer present"), "message: {message}");
        }
        other => panic!("expected Error::Other, got {other:?}"),
    }
}

// Case 7: Path outside dir_root => Error::PathOutsideTarget.
#[test]
fn reveal_rejects_path_outside_root() {
    let root = TempDir::new().unwrap();
    let elsewhere = TempDir::new().unwrap();
    let value = "AKIAOUTSIDEROCKET12345";
    let path = write(elsewhere.path(), ".env", format!("{value}\n").as_bytes());

    let reference = aws_ref(elsewhere.path(), ".env", 1, value);
    // dir_root does not contain `path` => must be rejected even though the hash
    // and line would otherwise match.
    let err = reveal_finding(&path, root.path(), &reference)
        .expect_err("outside-root path must be rejected");
    assert!(matches!(err, Error::PathOutsideTarget { .. }));
}

// Case 8: A directory (not a regular file) => Error::NotAFile.
#[test]
fn reveal_rejects_directory_as_not_a_file() {
    let dir = TempDir::new().unwrap();
    let subdir = dir.path().join("somedir");
    fs::create_dir_all(&subdir).unwrap();

    let reference = aws_ref(dir.path(), "somedir", 1, "AKIAWHATEVER1234567");
    let err =
        reveal_finding(&subdir, dir.path(), &reference).expect_err("a directory is not a file");
    assert!(matches!(err, Error::NotAFile { .. }));
}

// Case 9: dig DTO — content_ref mirrors content_match hash; None on filename-only.
#[test]
fn dig_dto_content_ref_matches_content_match_hash() {
    let dir = TempDir::new().unwrap();
    let value = "AKIAABCDEFGHIJKLMNOPQRST";
    // `dotenv`-style file with an AWS value -> content hit; also a filename hit.
    let path = write(dir.path(), ".env", format!("{value}\n").as_bytes());

    let result = dig_project(dir.path());
    let file = result
        .files
        .iter()
        .find(|f| f.path == path)
        .expect(".env with content hit must be in dig output");

    let content_match = file.content_match.as_ref().expect("content hit present");
    let content_ref = file
        .content_ref
        .as_ref()
        .expect("content hit => content_ref present");
    assert_eq!(
        content_ref.value_hash, content_match.value_hash,
        "content_ref.value_hash must equal content_match.value_hash"
    );
    assert_eq!(content_ref.path, path);
}

// Case 9b: filename-only finding (no content) => content_ref is None.
#[test]
fn dig_dto_content_ref_none_for_filename_only_finding() {
    let dir = TempDir::new().unwrap();
    // A bare file that matches a filename pattern but has no detectable secret
    // content (empty file). `.netrc` matches the filename table.
    let path = write(dir.path(), ".netrc", b"");

    let result = dig_project(dir.path());
    let file = result
        .files
        .iter()
        .find(|f| f.path == path)
        .expect(".netrc filename match must be reported");

    assert!(
        file.content_match.is_none(),
        "empty .netrc has no content hit"
    );
    assert!(
        file.content_ref.is_none(),
        "filename-only finding must have content_ref == None"
    );
}

// Case 10 (cross-cut): revealed value re-masks to the reference hash for the
// `generic_secret_assign` Regex marker too, guarding against marker drift.
#[test]
fn reveal_generic_secret_assign_roundtrips_to_hash() {
    let dir = TempDir::new().unwrap();
    let value = "superSecretTokenValue42";
    let path = write(
        dir.path(),
        "app.conf",
        format!("password={value}\n").as_bytes(),
    );

    let result = dig_project(dir.path());
    let file = result
        .files
        .iter()
        .find(|f| f.path == path)
        .expect("password assignment must be found");
    let reference = file
        .content_ref
        .as_ref()
        .expect("content hit => content_ref present");

    let secret = reveal_finding(&path, dir.path(), reference).expect("reveal succeeds");
    let masked = mask_secret(secret.expose());
    assert_eq!(masked.value_hash, reference.value_hash);
    assert!(secret.expose().contains(value));
}

// Case 10b: constructing a SensitiveFile directly (serde surface) — content_ref
// is a public field that serializes alongside, but still carries no raw value.
#[test]
fn sensitive_file_serde_keeps_content_ref_without_raw() {
    let file = SensitiveFile {
        path: PathBuf::from("/repo/.env"),
        risk: SensitiveRisk::Critical,
        labels: vec!["env".to_string()],
        content_match: Some(raccpack_core::secrets::mask_secret(
            "AKIAABCDEFGHIJKLMNOPQRST",
        )),
        content_ref: Some(FindingRef {
            path: PathBuf::from("/repo/.env"),
            marker_id: "aws_access_key".to_string(),
            line: 1,
            value_hash: fingerprint_secret("AKIAABCDEFGHIJKLMNOPQRST"),
        }),
        git_status: Some("tracked".to_string()),
    };
    let json = serde_json::to_string(&file).expect("SensitiveFile serializes");
    assert!(
        !json.contains("AKIAABCDEFGHIJKLMNOPQRST"),
        "SensitiveFile JSON must never carry the raw value: {json}"
    );
    let back: SensitiveFile = serde_json::from_str(&json).expect("SensitiveFile deserializes");
    assert_eq!(back.content_ref, file.content_ref);
}
