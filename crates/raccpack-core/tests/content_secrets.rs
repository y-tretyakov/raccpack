//! Integration tests for M3.2 — content markers + size limits + mask/fingerprint.
//!
//! Covers the behavioral contract from
//! docs/mvp/m3/m3.2-content-markers.md §4–§8: the `DEFAULT_CONTENT_MARKERS`
//! table (order, uniqueness, regex validity, risks/kinds), `mask_secret` /
//! `fingerprint_secret` (short vs long values, char-based masking with byte
//! `original_len`, raw values never in `Debug`), `scan_file_content` (size /
//! read / binary / empty skips, line-oriented 1-based line numbers, prefix token
//! extraction, one hit per occurrence per line) and `scan_secrets` (merge of
//! filename + content per path with risk upgrade, `min_risk` filtering,
//! `scan_content` toggle, deterministic path-ascending order, node_modules
//! policy, best-effort read-error handling).
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//! Permission tests are Unix-only (`#[cfg(unix)]`).

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{
    fingerprint_secret, mask_secret, scan_file_content, scan_secrets, ContentHit, ContentMatchKind,
    ContentScanLimits, Error, FindingSource, SecretScanOptions, SensitiveRisk,
    DEFAULT_CONTENT_MARKERS,
};
use tempfile::TempDir;

/// A deterministic AWS-style access key id (matches the `aws_access_key` prefix).
const AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`, returning its path.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
    path
}

/// Findings' paths relative to `root`, preserving the returned order.
fn rel_paths(findings: &[raccpack_core::SensitiveFinding], root: &Path) -> Vec<PathBuf> {
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

// --- Mandatory case 1: AKIA content hit, masked, raw never in Debug -----------

#[test]
fn content_aws_key_hit_is_critical_and_masked() {
    let root = TempDir::new().unwrap();
    let path = write(root.path(), "notes.txt", AWS_KEY);
    let hits = scan_file_content(&path, &ContentScanLimits::default())
        .expect("content scan must succeed on a text fixture");
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert_eq!(hit.marker_id, "aws_access_key");
    assert_eq!(hit.risk, SensitiveRisk::Critical);
    assert_eq!(hit.line, Some(1));
    assert!(
        hit.masked.masked.starts_with("AKIA") || hit.masked.masked == "****",
        "masked must keep a safe prefix or be fully masked"
    );
    assert!(
        !format!("{:?}", hit.masked).contains(AWS_KEY),
        "MaskedValue Debug must not leak the raw value"
    );
}

#[test]
fn content_finding_debug_never_leaks_raw() {
    let root = TempDir::new().unwrap();
    let path = write(root.path(), "notes.txt", AWS_KEY);
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    let f = findings
        .iter()
        .find(|f| f.path == path)
        .expect("the AKIA file must produce a finding");
    let dbg = format!("{:?}", f);
    assert!(
        !dbg.contains(AWS_KEY),
        "SensitiveFinding Debug must never contain the raw value: {dbg}"
    );
}

#[test]
fn masked_value_debug_never_contains_raw() {
    let mv = mask_secret(AWS_KEY);
    assert!(
        !format!("{:?}", mv).contains(AWS_KEY),
        "Debug of MaskedValue must never contain the raw value"
    );
}

// --- Mandatory case 2: PEM private key header --------------------------------

#[test]
fn content_pem_private_key_header_is_critical() {
    let root = TempDir::new().unwrap();
    let path = write(
        root.path(),
        "pemdata.txt",
        "-----BEGIN RSA PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0B\n",
    );
    let hits =
        scan_file_content(&path, &ContentScanLimits::default()).expect("content scan must succeed");
    assert!(
        hits.iter()
            .any(|h| { h.marker_id == "private_key_header" && h.risk == SensitiveRisk::Critical }),
        "the PEM header must fire private_key_header: {hits:?}"
    );

    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    let f = findings
        .iter()
        .find(|f| f.path == path)
        .expect("finding exists");
    assert_eq!(f.risk, SensitiveRisk::Critical);
    assert!(
        matches!(
            &f.source,
            FindingSource::Content { marker_id, .. } if marker_id == "private_key_header"
        ),
        "the finding must be content-driven: {:?}",
        f.source
    );
}

// --- Mandatory case 3: oversize file -> no content, filename survives ---------

#[test]
fn content_oversize_file_is_skipped_but_filename_still_found() {
    let root = TempDir::new().unwrap();
    let notes = write(root.path(), "notes.txt", AWS_KEY); // 20 bytes
    let limits = ContentScanLimits {
        max_file_bytes: 16,
        max_read_bytes: 16,
        skip_binary: true,
    };
    assert!(
        scan_file_content(&notes, &limits).unwrap().is_empty(),
        "a file larger than max_file_bytes must yield no content hits"
    );

    let env = write(root.path(), ".env", AWS_KEY);
    let opts = SecretScanOptions {
        limits,
        ..SecretScanOptions::default()
    };
    let findings = scan_secrets(root.path(), &opts).expect("scan_secrets must succeed");
    assert_eq!(
        findings.len(),
        1,
        "only the .env filename finding may survive"
    );
    let f = &findings[0];
    assert_eq!(f.path, env);
    assert_eq!(f.risk, SensitiveRisk::High);
    assert!(
        matches!(&f.source, FindingSource::Filename { pattern_id } if pattern_id == "env_file")
    );
    assert_eq!(f.sources.len(), 1);
    assert!(
        f.content_match.is_none(),
        "oversize content must never be read"
    );
}

// --- Mandatory case 4: binary file is skipped ---------------------------------

#[test]
fn content_binary_file_with_null_is_skipped() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("blob.bin");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"hello\x00AKIAIOSFODNN7EXAMPLE").unwrap();

    let hits = scan_file_content(&path, &ContentScanLimits::default())
        .expect("binary sniff must not error");
    assert!(hits.is_empty(), "binary content must be skipped: {hits:?}");

    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    assert!(
        rel_paths(&findings, root.path()).is_empty(),
        "a binary blob must produce no findings: {findings:?}"
    );
}

// --- Mandatory case 5: .env + password assignment -> merged finding -----------

#[test]
fn content_env_file_with_password_merges_sources() {
    let root = TempDir::new().unwrap();
    let path = write(root.path(), ".env", "password=supersecretvalue123\n");
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.path, path);
    // max(High filename, High content) == High
    assert_eq!(f.risk, SensitiveRisk::High);
    assert_eq!(f.sources.len(), 2, "one Filename + one Content source");

    assert!(
        matches!(&f.source, FindingSource::Filename { pattern_id } if pattern_id == "env_file"),
        "primary source must be the filename match: {:?}",
        f.source
    );
    assert!(matches!(
        &f.sources[1],
        FindingSource::Content { marker_id, .. } if marker_id == "generic_secret_assign"
    ));
    assert_eq!(f.sources.len(), f.labels.len());
    assert_eq!(f.label, f.labels[0]);
    assert!(f.content_match.is_some(), "the content hit must be carried");
}

// --- Mandatory case 6: mask_secret never contains the full raw ----------------

#[test]
fn mask_does_not_contain_full_raw() {
    let raw = "supersecretvalue";
    let mv = mask_secret(raw);
    assert!(
        !mv.masked.contains(raw),
        "masked must never contain the full raw string: {}",
        mv.masked
    );
}

// --- Mandatory case 7: same raw in two files -> identical hash -----------------

#[test]
fn same_raw_in_two_files_has_identical_hash_and_mask() {
    let root = TempDir::new().unwrap();
    let a = write(root.path(), "a.txt", AWS_KEY);
    let b = write(root.path(), "b.txt", AWS_KEY);
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    assert_eq!(findings.len(), 2);
    let fa = findings.iter().find(|f| f.path == a).expect("a finding");
    let fb = findings.iter().find(|f| f.path == b).expect("b finding");
    let ca = fa.content_match.as_ref().expect("a has a content match");
    let cb = fb.content_match.as_ref().expect("b has a content match");
    assert_eq!(ca.value_hash, cb.value_hash);
    assert_eq!(ca.masked, cb.masked);
    assert_eq!(ca.original_len, cb.original_len);
}

// --- Mandatory case 8: node_modules is never scanned --------------------------

#[test]
fn content_inside_node_modules_is_never_scanned() {
    let root = TempDir::new().unwrap();
    write(root.path(), "node_modules/pkg/secret_file", AWS_KEY);
    let keep = write(root.path(), ".env", "APP=x\n");
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].path, keep);
    assert!(
        rel_paths(&findings, root.path())
            .iter()
            .all(|p| !p.starts_with("node_modules")),
        "node_modules must not be descended: {findings:?}"
    );
}

// --- Mandatory case 9: table integrity + regex validity -----------------------

#[test]
fn content_marker_table_has_12_unique_rows_in_order() {
    const EXPECTED_IDS: [&str; 12] = [
        "aws_access_key",
        "aws_secret_assign",
        "generic_api_key_assign",
        "generic_secret_assign",
        "private_key_header",
        "github_pat",
        "github_oauth",
        "slack_token",
        "stripe_live",
        "stripe_test",
        "connection_string",
        "jwt_like",
    ];
    assert_eq!(
        DEFAULT_CONTENT_MARKERS.len(),
        12,
        "the table must hold the MVP rows from the spec"
    );
    let ids: Vec<&str> = DEFAULT_CONTENT_MARKERS.iter().map(|m| m.id).collect();
    assert_eq!(ids, EXPECTED_IDS, "id order is part of the table contract");

    let mut seen = std::collections::HashSet::new();
    for m in DEFAULT_CONTENT_MARKERS {
        assert!(seen.insert(m.id), "duplicate marker id: {}", m.id);
        assert!(
            !m.pattern.is_empty(),
            "marker {} has an empty pattern",
            m.id
        );
        assert!(!m.label.is_empty(), "marker {} has an empty label", m.id);
    }
}

#[test]
#[allow(clippy::invalid_regex)]
fn content_marker_regexes_all_compile() {
    for m in DEFAULT_CONTENT_MARKERS {
        if m.kind == ContentMatchKind::Regex {
            assert!(
                regex::Regex::new(m.pattern).is_ok(),
                "marker `{}` has an invalid regex: {}",
                m.id,
                m.pattern
            );
        }
    }
    // The compile gate must reject a genuinely broken regex.
    assert!(regex::Regex::new("(").is_err());
}

#[test]
fn content_marker_risks_and_kinds_match_spec() {
    let by_id = |id: &str| {
        DEFAULT_CONTENT_MARKERS
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("marker `{id}` must exist"))
    };
    assert_eq!(by_id("aws_access_key").kind, ContentMatchKind::Prefix);
    assert_eq!(by_id("aws_access_key").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("aws_secret_assign").kind, ContentMatchKind::Regex);
    assert_eq!(by_id("aws_secret_assign").risk, SensitiveRisk::Critical);
    assert_eq!(
        by_id("generic_api_key_assign").kind,
        ContentMatchKind::Regex
    );
    assert_eq!(by_id("generic_api_key_assign").risk, SensitiveRisk::High);
    assert_eq!(by_id("generic_secret_assign").kind, ContentMatchKind::Regex);
    assert_eq!(by_id("generic_secret_assign").risk, SensitiveRisk::High);
    assert_eq!(by_id("private_key_header").kind, ContentMatchKind::Regex);
    assert_eq!(by_id("private_key_header").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("github_pat").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("github_oauth").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("slack_token").risk, SensitiveRisk::High);
    assert_eq!(by_id("stripe_live").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("stripe_test").risk, SensitiveRisk::Medium);
    assert_eq!(by_id("connection_string").risk, SensitiveRisk::Critical);
    assert_eq!(by_id("jwt_like").kind, ContentMatchKind::Regex);
    assert_eq!(by_id("jwt_like").risk, SensitiveRisk::Medium);
}

// --- Mandatory case 10: empty file -> no panic, no findings -------------------

#[test]
fn content_empty_file_produces_no_hits_or_findings() {
    let root = TempDir::new().unwrap();
    let path = write(root.path(), "empty.txt", "");
    let hits =
        scan_file_content(&path, &ContentScanLimits::default()).expect("empty file must not error");
    assert!(hits.is_empty());

    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must not panic on an empty tree");
    assert!(
        findings.is_empty(),
        "an empty innocuous file must not match"
    );
}

// --- Extras: mask rules table ---------------------------------------------------

#[test]
fn mask_short_values_are_fully_asterisked() {
    for raw in ["", "a", "abcd", "12345678"] {
        let mv = mask_secret(raw);
        assert_eq!(mv.masked, "****", "len {} must mask to ****", raw.len());
        assert_eq!(mv.original_len, raw.len());
        if !raw.is_empty() {
            assert!(!mv.masked.contains(raw));
        }
    }
}

#[test]
fn mask_long_values_keep_first4_and_last2() {
    assert_eq!(mask_secret("123456789").masked, "1234…89");
    assert_eq!(mask_secret("supersecretvalue").masked, "supe…ue");
}

#[test]
fn mask_is_char_based_but_len_is_bytes() {
    let raw = "абвгдежзик"; // 10 chars, 20 bytes
    let mv = mask_secret(raw);
    assert_eq!(mv.masked, "абвг…ик");
    assert_eq!(raw.len(), 20);
    assert_eq!(mv.original_len, 20, "original_len counts bytes, not chars");
}

#[test]
fn mask_value_hash_is_stable_hex_and_len_tracked() {
    let mv = mask_secret(AWS_KEY);
    assert_eq!(mv.original_len, AWS_KEY.len());
    assert_eq!(mv.value_hash.len(), 64, "blake3 hex is 64 chars");
    assert!(
        mv.value_hash.chars().all(|c| c.is_ascii_hexdigit()),
        "value_hash must be hex: {}",
        mv.value_hash
    );
}

#[test]
fn mask_fingerprint_is_deterministic_and_distinguishes_inputs() {
    let a1 = fingerprint_secret("supersecretvalue");
    let a2 = fingerprint_secret("supersecretvalue");
    let b = fingerprint_secret("supersecretvalue2");
    assert_eq!(a1, a2);
    assert_ne!(a1, b);
    assert_eq!(fingerprint_secret(""), fingerprint_secret(""));
}

// --- Extras: content-only finding ------------------------------------------------

#[test]
fn content_only_finding_without_filename_match() {
    let root = TempDir::new().unwrap();
    let path = write(root.path(), "notes.txt", "sk_live_xyz123456789\n");
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.path, path);
    assert_eq!(f.risk, SensitiveRisk::Critical);
    assert!(
        f.content_match.is_some(),
        "content_match must carry the preview"
    );
    assert_eq!(f.sources.len(), 1);
    assert!(
        matches!(&f.source, FindingSource::Content { marker_id, .. } if marker_id == "stripe_live"),
        "a content-only finding is primary-sourced from the marker: {:?}",
        f.source
    );
}

// --- Extras: line numbers ---------------------------------------------------------

#[test]
fn content_line_numbers_are_one_based() {
    let root = TempDir::new().unwrap();
    let path = write(root.path(), "lines.txt", "first line\nAKIAIOSFODNN7EXAMPLE");
    let hits =
        scan_file_content(&path, &ContentScanLimits::default()).expect("content scan must succeed");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, Some(2));
}

#[test]
fn content_multiple_matching_lines_produce_one_hit_per_line() {
    let root = TempDir::new().unwrap();
    let path = write(root.path(), "lines.txt", "AKIA111111\nAKIA222222\n");
    let hits =
        scan_file_content(&path, &ContentScanLimits::default()).expect("content scan must succeed");
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|h| h.marker_id == "aws_access_key"));
    assert!(hits.iter().any(|h| h.line == Some(1)));
    assert!(hits.iter().any(|h| h.line == Some(2)));
}

// --- Extras: multiple occurrences on one line ---------------------------------------

#[test]
fn content_multiple_occurrences_on_one_line() {
    let root = TempDir::new().unwrap();
    let path = write(
        root.path(),
        "many.txt",
        "AKIA111111111111 AKIA222222222222\n",
    );
    let hits =
        scan_file_content(&path, &ContentScanLimits::default()).expect("content scan must succeed");
    let aws: Vec<&ContentHit> = hits
        .iter()
        .filter(|h| h.marker_id == "aws_access_key")
        .collect();
    assert_eq!(
        aws.len(),
        2,
        "each prefix occurrence must yield a hit: {hits:?}"
    );
    assert!(aws.iter().all(|h| h.line == Some(1)));
    assert_ne!(
        aws[0].masked.masked, aws[1].masked.masked,
        "distinct tokens must mask to distinct previews"
    );
}

// --- Extras: scan_content toggle -----------------------------------------------------

#[test]
fn content_scan_disabled_yields_no_content_findings() {
    let root = TempDir::new().unwrap();
    write(root.path(), "notes.txt", AWS_KEY);
    let opts = SecretScanOptions {
        scan_content: false,
        ..SecretScanOptions::default()
    };
    let findings = scan_secrets(root.path(), &opts).expect("scan_secrets must succeed");
    assert!(
        findings.is_empty(),
        "with scan_content disabled an innocuous name must not match: {findings:?}"
    );
}

#[test]
fn content_scan_options_default_scan_content_true() {
    assert!(SecretScanOptions::default().scan_content);
}

// --- Extras: min_risk filter ----------------------------------------------------------

#[test]
fn content_min_risk_critical_filters_high_hits() {
    let root = TempDir::new().unwrap();
    let keep = write(root.path(), "one.txt", AWS_KEY); // Critical
    write(root.path(), "two.txt", "xoxb-1234567890\n"); // High (slack_token)
    let opts = SecretScanOptions {
        min_risk: SensitiveRisk::Critical,
        ..SecretScanOptions::default()
    };
    let findings = scan_secrets(root.path(), &opts).expect("scan_secrets must succeed");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].path, keep);
    assert_eq!(findings[0].risk, SensitiveRisk::Critical);
}

// --- Extras: deterministic ordering ------------------------------------------------------

#[test]
fn content_scan_results_are_sorted_by_path() {
    let root = TempDir::new().unwrap();
    write(root.path(), "b/note.txt", AWS_KEY);
    write(root.path(), "a/note.txt", "xoxb-1234567890\n");
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    let rels = rel_paths(&findings, root.path());
    assert_eq!(
        rels,
        vec![PathBuf::from("a/note.txt"), PathBuf::from("b/note.txt")],
        "results must be path-ascending regardless of creation order"
    );
    assert_eq!(findings[0].risk, SensitiveRisk::High);
    assert_eq!(findings[1].risk, SensitiveRisk::Critical);
}

// --- Extras: read-error skip (Unix-only) ------------------------------------------------

#[cfg(unix)]
#[test]
fn content_unreadable_file_is_skipped_best_effort() {
    use std::os::unix::fs::PermissionsExt;
    let root = TempDir::new().unwrap();
    let path = write(root.path(), "credentials", AWS_KEY);
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&path, perms).unwrap();

    // Best-effort: either a clean skip (Ok/empty) or an error; never a panic.
    let hits = scan_file_content(&path, &ContentScanLimits::default()).unwrap_or_default();
    // Only when the file is genuinely unreadable (non-root) must it yield nothing.
    if fs::read(&path).is_err() {
        assert!(
            hits.is_empty(),
            "an unreadable file must not yield content hits"
        );
    }

    // Filename detection is unaffected by readability.
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must not error on an unreadable file");
    let f = findings
        .iter()
        .find(|f| f.path == path)
        .expect("finding exists");
    assert!(
        matches!(&f.source, FindingSource::Filename { pattern_id } if pattern_id == "aws_credentials"),
        "the credentials basename must still be found: {:?}",
        f.source
    );

    // Restore permissions so TempDir can clean up.
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&path, perms).unwrap();
}

// --- Extras: max_read_bytes caps reading -------------------------------------------------

#[test]
fn content_max_read_bytes_caps_reading() {
    let root = TempDir::new().unwrap();
    let path = write(
        root.path(),
        "many.txt",
        "AKIA111111111111 AKIA222222222222\n",
    );
    let capped = ContentScanLimits {
        max_file_bytes: 1_048_576,
        max_read_bytes: 10,
        skip_binary: true,
    };
    // Only the first 10 bytes ("AKIA111111") are read: the second token is cut.
    let hits = scan_file_content(&path, &capped).expect("truncated read must not error");
    assert!(
        !hits.is_empty(),
        "the first occurrence is within the read window"
    );
    assert!(
        hits.len() <= 1,
        "a truncated read cannot find two occurrences: {hits:?}"
    );

    let full =
        scan_file_content(&path, &ContentScanLimits::default()).expect("full read must not error");
    assert!(
        full.len() >= 2,
        "a full read finds both occurrences: {full:?}"
    );
}

#[test]
fn content_scan_limits_default_values() {
    let limits = ContentScanLimits::default();
    assert_eq!(limits.max_file_bytes, 1_048_576);
    assert_eq!(limits.max_read_bytes, 1_048_576);
    assert!(limits.skip_binary);
}

// --- Extras: risk upgrade filename High + content Critical ------------------------------

#[test]
fn content_upgrade_filename_high_to_content_critical() {
    let root = TempDir::new().unwrap();
    let path = write(root.path(), ".env", AWS_KEY);
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.path, path);
    // filename `.env` is High; content AKIA is Critical -> merged Critical
    assert_eq!(f.risk, SensitiveRisk::Critical);
    assert_eq!(f.sources.len(), 2, "filename + content sources");
    assert!(
        matches!(&f.source, FindingSource::Filename { pattern_id } if pattern_id == "env_file"),
        "primary source stays the filename match"
    );
    let cm = f.content_match.as_ref().expect("content match present");
    assert!(cm.masked.starts_with("AKIA"));
    assert_eq!(cm.original_len, AWS_KEY.len());
}

#[test]
fn content_finding_source_and_label_are_primary_sources() {
    let root = TempDir::new().unwrap();
    write(root.path(), ".env", "password=supersecretvalue123\n");
    let findings = scan_secrets(root.path(), &SecretScanOptions::default())
        .expect("scan_secrets must succeed");
    let f = &findings[0];
    assert_eq!(f.source, f.sources[0]);
    assert_eq!(f.label, f.labels[0]);
    assert_eq!(f.sources.len(), f.labels.len());
}

// --- Extras: root validation ---------------------------------------------------------------

#[test]
fn content_scan_missing_root_is_path_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("does-not-exist");
    let err = scan_secrets(&missing, &SecretScanOptions::default())
        .expect_err("a missing root must fail");
    assert!(matches!(err, Error::PathNotFound { .. }));
}
