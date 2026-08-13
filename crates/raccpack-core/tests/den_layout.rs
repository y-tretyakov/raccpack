//! Integration tests for M4.2 — den layout: `ensure_den`, naming helpers
//! (`project_slug`, `utc_timestamp_now`, `short_id`, `pack_relative_path`,
//! `staging_pack_path`) and `place_pack`, as specified in
//! docs/mvp/m4/m4.2-den-layout.md §4–§5 and raccpack-facade-and-den.md §9.
//!
//! Covers: den skeleton creation + idempotent second call, incompatible
//! `.den-version` rejection, slug sanitization (incl. paths and the 80-char
//! cap), timestamp / short-id / relative-path formats, place_pack move + size
//! and relative layout, missing source, and two concurrent place_pack calls
//! with distinct timestamps (no clobber).
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no git. The
//! tests only use the public `raccpack_core::den` API, so they keep working
//! regardless of internal layout choices. The incompatible-version error
//! variant is intentionally not pinned (Dev may add `Error::DenVersion` or
//! reuse `Error::Other`) — only the fact of rejection is specified.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::den::{
    ensure_den, pack_relative_path, place_pack, project_slug, short_id, staging_pack_path,
    utc_timestamp_now, DenPaths, PlacePackRequest, DEN_VERSION,
};
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

fn write(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, contents).expect("write fixture file");
}

fn read_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// A ready-to-place source archive in its own tempdir.
fn write_source(contents: &[u8]) -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("pack.tar.zst");
    write(&path, contents);
    (temp, path)
}

fn assert_den_layout(paths: &DenPaths, root: &Path) {
    assert_eq!(paths.root, root);
    assert_eq!(paths.packs, root.join("packs"));
    assert_eq!(paths.staging, root.join("staging"));
    assert_eq!(paths.manifests, root.join("manifests"));
    assert_eq!(paths.secrets, root.join("secrets"));
    for dir in [
        &paths.packs,
        &paths.staging,
        &paths.manifests,
        &paths.secrets,
    ] {
        assert!(dir.is_dir(), "den dir missing: {}", dir.display());
    }
}

/// Check `YYYYMMDDThhmmssZ` by length and character positions (no regex crate
/// needed in the test).
fn assert_utc_timestamp(s: &str) {
    assert_eq!(s.len(), 16, "timestamp must be YYYYMMDDThhmmssZ, got {s:?}");
    for c in s[..8].chars() {
        assert!(c.is_ascii_digit(), "date part must be digits: {s:?}");
    }
    assert_eq!(s.as_bytes()[8], b'T', "separator must be 'T': {s:?}");
    for c in s[9..15].chars() {
        assert!(c.is_ascii_digit(), "time part must be digits: {s:?}");
    }
    assert_eq!(s.as_bytes()[15], b'Z', "must end in 'Z': {s:?}");
}

// --- §5.1 ensure_den: skeleton + idempotency --------------------------------

#[test]
fn den_ensure_creates_skeleton_and_is_idempotent() {
    assert_eq!(DEN_VERSION, "1");
    let den = TempDir::new().unwrap();
    let root = den.path();

    let paths = ensure_den(root).expect("first ensure_den must succeed");
    assert_den_layout(&paths, root);
    assert_eq!(read_string(&root.join(".den-version")), "1\n");

    let readme = read_string(&root.join("README.txt"));
    for needle in [
        "raccpack den",
        "secrets/",
        "packs/",
        "manifests/",
        "Do not commit",
        "Keep passphrase offline",
    ] {
        assert!(readme.contains(needle), "README.txt missing {needle:?}");
    }

    let version_before = read_string(&root.join(".den-version"));
    let readme_before = read_string(&root.join("README.txt"));

    // Second call must succeed and leave every file untouched.
    let paths2 = ensure_den(root).expect("second ensure_den must be idempotent");
    assert_den_layout(&paths2, root);
    assert_eq!(read_string(&root.join(".den-version")), version_before);
    assert_eq!(read_string(&root.join("README.txt")), readme_before);
}

// --- §5.2 incompatible .den-version -----------------------------------------

#[test]
fn den_ensure_rejects_incompatible_version() {
    let den = TempDir::new().unwrap();
    write(&den.path().join(".den-version"), b"99\n");

    let result = ensure_den(den.path());
    assert!(
        result.is_err(),
        "incompatible .den-version major must be rejected"
    );
}

#[test]
fn den_ensure_accepts_current_version() {
    let den = TempDir::new().unwrap();
    write(&den.path().join(".den-version"), b"1\n");
    assert!(ensure_den(den.path()).is_ok());
}

// --- §5.3 project_slug ------------------------------------------------------

#[test]
fn den_project_slug_sanitizes_name_and_path() {
    assert_eq!(project_slug("My App!"), "My-App");
    assert_eq!(project_slug("/home/u/My App"), "My-App");
    assert_eq!(project_slug("already-clean_1.0"), "already-clean_1.0");
    // Deterministic: same input always produces the same slug.
    assert_eq!(project_slug("My App!"), project_slug("My App!"));
}

#[test]
fn den_project_slug_caps_length_at_80() {
    let long = "a-".repeat(100);
    let slug = project_slug(&long);
    assert!(
        slug.len() <= 80,
        "slug must be capped at 80 chars: {slug:?}"
    );
}

#[test]
fn den_project_slug_only_allows_safe_chars() {
    let slug = project_slug("До?? #$%^ & App!!");
    assert!(!slug.is_empty());
    for c in slug.chars() {
        assert!(
            c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'),
            "slug must only contain [a-zA-Z0-9._-], got {c:?} in {slug:?}"
        );
    }
}

// --- Naming format helpers --------------------------------------------------

#[test]
fn den_utc_timestamp_now_matches_format() {
    for _ in 0..3 {
        assert_utc_timestamp(&utc_timestamp_now());
    }
}

#[test]
fn den_short_id_is_hex_8() {
    for _ in 0..3 {
        let id = short_id();
        assert_eq!(id.len(), 8, "short_id must be 8 hex chars: {id:?}");
        assert!(
            id.chars()
                .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f')),
            "short_id must be lowercase hex [0-9a-f]: {id:?}"
        );
    }
    // Collision odds for two random 8-hex ids are 2^-32 — effectively never.
    assert_ne!(short_id(), short_id(), "two short_ids should differ");
}

#[test]
fn den_pack_relative_path_layout() {
    let rel = pack_relative_path("my-app", "20260804T155230Z");
    let expected = Path::new("packs")
        .join("2026")
        .join("08")
        .join("my-app__20260804T155230Z.tar.zst");
    assert_eq!(rel, expected);
    // §5.5: the path starts with packs/.
    assert!(
        rel.starts_with("packs"),
        "expected packs/ prefix: {}",
        rel.display()
    );
}

#[test]
fn den_staging_pack_path_layout() {
    let den = TempDir::new().unwrap();
    let path = staging_pack_path(den.path(), "abcd1234");
    assert_eq!(
        path,
        den.path()
            .join("staging")
            .join("abcd1234")
            .join("pack.tar.zst")
    );
}

// --- §5.4–§5.6 place_pack ---------------------------------------------------

#[test]
fn den_place_pack_moves_tempfile_into_layout() {
    let den = TempDir::new().unwrap();
    ensure_den(den.path()).expect("bootstrap den");
    let contents = b"zstd-archive-bytes-1234";
    let (_src_dir, source) = write_source(contents);

    let req = PlacePackRequest {
        den_root: den.path().to_path_buf(),
        project_name: "My App!".to_string(),
        source_archive: source.clone(),
        timestamp: Some("20260804T155230Z".to_string()),
        output_name: None,
    };
    let result = place_pack(&req).expect("place_pack must succeed");

    // §5.4: file landed at packs/yyyy/mm/slug__ts.tar.zst.
    let expected_rel = Path::new("packs")
        .join("2026")
        .join("08")
        .join("My-App__20260804T155230Z.tar.zst");
    assert_eq!(result.relative_path, expected_rel);

    // §5.5: relative path starts with packs/.
    assert!(
        result.relative_path.starts_with("packs"),
        "expected packs/ prefix: {}",
        result.relative_path.display()
    );

    let expected_abs = den.path().join(&expected_rel);
    assert_eq!(result.absolute_path, expected_abs);
    assert!(
        result.absolute_path.is_file(),
        "artifact missing: {}",
        result.absolute_path.display()
    );

    // The source tempfile was moved, not copied.
    assert!(!source.exists(), "source must be moved (no longer present)");

    // §5.6: size matches the source bytes.
    assert_eq!(result.size_bytes, contents.len() as u64);
    assert_eq!(
        fs::metadata(&result.absolute_path).unwrap().len(),
        contents.len() as u64
    );
}

#[test]
fn den_place_pack_missing_source_fails() {
    let den = TempDir::new().unwrap();
    ensure_den(den.path()).unwrap();
    let missing = den.path().join("staging").join("nope").join("pack.tar.zst");

    let req = PlacePackRequest {
        den_root: den.path().to_path_buf(),
        project_name: "proj".to_string(),
        source_archive: missing,
        timestamp: Some("20260804T155230Z".to_string()),
        output_name: None,
    };
    let result = place_pack(&req);
    assert!(result.is_err(), "missing source archive must fail");
}

// --- §5.7 concurrent place_pack, no clobber ---------------------------------

#[test]
fn den_place_pack_concurrent_no_clobber() {
    let den = TempDir::new().unwrap();
    ensure_den(den.path()).expect("bootstrap den before racing");

    let (_src_a, src_a) = write_source(b"AAAA");
    let (_src_b, src_b) = write_source(b"BBBB");

    let root = den.path().to_path_buf();
    let results = std::thread::scope(|scope| {
        let h1 = scope.spawn({
            let root = root.clone();
            let src = src_a.clone();
            move || {
                let req = PlacePackRequest {
                    den_root: root,
                    project_name: "proj".to_string(),
                    source_archive: src,
                    timestamp: Some("20260804T155000Z".to_string()),
                    output_name: None,
                };
                let res = place_pack(&req).expect("thread 1 place_pack");
                (res.absolute_path, res.relative_path)
            }
        });
        let h2 = scope.spawn({
            let root = root.clone();
            let src = src_b.clone();
            move || {
                let req = PlacePackRequest {
                    den_root: root,
                    project_name: "proj".to_string(),
                    source_archive: src,
                    timestamp: Some("20260804T155001Z".to_string()),
                    output_name: None,
                };
                let res = place_pack(&req).expect("thread 2 place_pack");
                (res.absolute_path, res.relative_path)
            }
        });
        [
            h1.join().expect("thread 1 panicked"),
            h2.join().expect("thread 2 panicked"),
        ]
    });

    let (abs_a, rel_a) = &results[0];
    let (abs_b, rel_b) = &results[1];
    assert_ne!(
        rel_a, rel_b,
        "distinct timestamps must produce distinct relative paths"
    );
    assert!(abs_a.is_file(), "missing artifact: {}", abs_a.display());
    assert!(abs_b.is_file(), "missing artifact: {}", abs_b.display());
}
