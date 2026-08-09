//! Integration tests for M4.1 — `pack_tree`: tar+zstd packing with a name
//! deny-list and `SkipPolicy` pruning, as specified in
//! docs/mvp/m4/m4.1-pack-tar-zstd.md §5–§7.
//!
//! Covers: archive root = contents of `source`, name deny (`.env`/`key.pem`),
//! `SkipPolicy` pruning (`target/`, `node_modules/`), symlinks skipped and
//! never followed, stats (`size_bytes`, `file_count`, `skipped_secret_files`,
//! `skipped_dir_names`), error mapping (`PathNotFound` / `NotADirectory`), the
//! `deny_name_secrets: false` escape hatch, and optional content-deny.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//! The symlink fixture and `/etc/passwd` probe are Linux/Unix-only, so the
//! whole file is `#[cfg(unix)]` (consistent with `skip_walk.rs`).
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use raccpack_core::archive::{
    self, pack_tree, ContentDenyOptions, PackTreeOptions, PackTreeResult,
};
use raccpack_core::{Error, SensitiveRisk};
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel` with `contents`.
fn write(root: &Path, rel: &str, contents: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
}

/// The spec §7 fixture: two includable files, one name-denied file, two
/// policy-pruned directories and one symlink pointing at `/etc/passwd`.
fn build_main_fixture() -> TempDir {
    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    write(&proj, "src/main.rs", b"fn main() {}\n");
    write(&proj, ".env", b"API_KEY=super-secret\n");
    write(&proj, "target/debug/x", b"compiled bytes\n");
    write(&proj, "node_modules/a.js", b"module.exports = 1;\n");
    write(&proj, "notes.txt", b"hello\n");
    symlink("/etc/passwd", proj.join("link")).unwrap();
    temp
}

/// Pack the main fixture with `opts` and return (tempdir, output, result).
fn pack_main_fixture(opts: &PackTreeOptions) -> (TempDir, PathBuf, PackTreeResult) {
    let temp = build_main_fixture();
    let src = temp.path().join("proj");
    let out = temp.path().join("out.tar.zst");
    let result = pack_tree(&src, &out, opts).expect("pack_tree must succeed");
    (temp, out, result)
}

/// Decode a `.tar.zst` archive and return `(relative_name, contents)` per entry.
fn unpack_entries(path: &Path) -> Vec<(String, Vec<u8>)> {
    let bytes = fs::read(path).expect("read archive bytes");
    let decoder =
        zstd::stream::read::Decoder::new(std::io::Cursor::new(bytes)).expect("zstd decode header");
    let mut archive = tar::Archive::new(decoder);
    let mut out = Vec::new();
    for entry in archive.entries().expect("read tar entries") {
        let mut entry = entry.expect("valid tar entry");
        let name = entry.path().unwrap().to_string_lossy().into_owned();
        let mut content = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut content).expect("read entry contents");
        out.push((name, content));
    }
    out
}

/// Decode a `.tar.zst` archive and return the relative entry names.
fn unpack_names(path: &Path) -> Vec<String> {
    unpack_entries(path)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

// --- Case 1: archive root = contents of source ------------------------------

#[test]
fn pack_archive_contains_expected_files() {
    let (_temp, out, result) = pack_main_fixture(&PackTreeOptions::default());

    let names = unpack_names(&out);
    assert!(names.iter().any(|n| n == "src/main.rs"), "{names:?}");
    assert!(names.iter().any(|n| n == "notes.txt"), "{names:?}");
    // Nothing else is archived: no dirs, no symlinks, no denied/skipped files.
    assert_eq!(
        names.len(),
        2,
        "only the two includable regular files: {names:?}"
    );
    assert_eq!(result.file_count, 2);
}

// --- Case 2: name deny (`.env`) ---------------------------------------------

#[test]
fn pack_archive_excludes_env_by_name() {
    let (_temp, out, result) = pack_main_fixture(&PackTreeOptions::default());

    let names = unpack_names(&out);
    assert!(!names.iter().any(|n| n == ".env"), "{names:?}");
    assert!(result.skipped_secret_files >= 1, "`.env` must be counted");
}

// --- Case 3: SkipPolicy prunes target/ and node_modules/ ----------------------

#[test]
fn pack_no_paths_under_skipped_dirs() {
    let (_temp, out, _result) = pack_main_fixture(&PackTreeOptions::default());

    let names = unpack_names(&out);
    for name in &names {
        assert!(
            !name.starts_with("target") && !name.starts_with("node_modules"),
            "archive must not contain anything under a skipped dir: {name}"
        );
    }
}

// --- Case 4: symlinks are skipped, never followed ----------------------------

#[test]
fn pack_symlink_not_followed() {
    let (_temp, out, result) = pack_main_fixture(&PackTreeOptions::default());

    let entries = unpack_entries(&out);
    assert!(
        !entries
            .iter()
            .any(|(name, _)| name == "link" || name.starts_with("link")),
        "the symlink itself must not be archived: {entries:?}"
    );

    // External filesystem must not leak through the link.
    let passwd = fs::read("/etc/passwd").expect("/etc/passwd is readable on unix");
    for (name, content) in &entries {
        assert_ne!(
            content, &passwd,
            "entry {name} carries /etc/passwd contents: external FS exposed"
        );
    }

    assert_eq!(result.file_count, 2, "only src/main.rs + notes.txt");
}

// --- Case 5: file_count matches included files -------------------------------

#[test]
fn pack_file_count_matches_included() {
    let (_temp, _out, result) = pack_main_fixture(&PackTreeOptions::default());
    assert_eq!(result.file_count, 2);
}

// --- Case 6: skipped_secret_files counts name-denied files -------------------

#[test]
fn pack_skipped_secret_files_counts_env() {
    let (_temp, _out, result) = pack_main_fixture(&PackTreeOptions::default());
    assert!(result.skipped_secret_files >= 1);
}

#[test]
fn pack_skipped_secret_files_counts_second_secret_name() {
    let temp = build_main_fixture();
    let proj = temp.path().join("proj");
    // A second name-denied file (`.pem` suffix is High by filename pattern).
    write(&proj, "keys/backup.pem", b"-----BEGIN PRIVATE KEY-----\n");
    let src = temp.path().join("proj");
    let out = temp.path().join("out.tar.zst");
    let result = pack_tree(&src, &out, &PackTreeOptions::default()).unwrap();

    assert!(result.skipped_secret_files >= 1);
    let names = unpack_names(&out);
    assert!(!names.iter().any(|n| n.contains("backup.pem")), "{names:?}");
}

// --- Case 7: roundtrip relative paths match ----------------------------------

#[test]
fn pack_roundtrip_relative_paths_match() {
    let (_temp, out, _result) = pack_main_fixture(&PackTreeOptions::default());

    let names: Vec<String> = unpack_names(&out);
    let mut expected: Vec<String> = vec!["notes.txt".into(), "src/main.rs".into()];
    expected.sort();
    let mut got = names.clone();
    got.sort();
    assert_eq!(
        got, expected,
        "roundtrip entry paths must match the included set"
    );
}

// --- Case 8: empty directory yields a valid archive ---------------------------

#[test]
fn pack_empty_dir_yields_valid_archive() {
    let temp = TempDir::new().unwrap();
    let empty = temp.path().join("empty");
    fs::create_dir_all(empty.join("nested")).unwrap();

    let out = temp.path().join("empty.tar.zst");
    let result = pack_tree(&empty, &out, &PackTreeOptions::default())
        .expect("packing an empty dir must not panic or error");

    assert_eq!(result.file_count, 0);
    assert!(out.exists());
    let names = unpack_names(&out);
    assert!(names.is_empty(), "no entries expected: {names:?}");
}

// --- Case 9: zstd level is accepted -------------------------------------------

#[test]
fn pack_zstd_level_accepted() {
    for level in [1i32, 9, 22] {
        let temp = build_main_fixture();
        let src = temp.path().join("proj");
        let out = temp.path().join("out.tar.zst");
        let opts = PackTreeOptions {
            zstd_level: level,
            ..PackTreeOptions::default()
        };
        let result = pack_tree(&src, &out, &opts)
            .unwrap_or_else(|e| panic!("zstd_level {level} must be accepted: {e:?}"));

        assert!(out.exists());
        assert_eq!(result.size_bytes, fs::metadata(&out).unwrap().len());
        let names = unpack_names(&out);
        assert_eq!(names.len(), 2, "archive still decodes at level {level}");
    }
}

// --- Extras: error mapping -----------------------------------------------------

#[test]
fn pack_source_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("does-not-exist");
    let out = temp.path().join("out.tar.zst");
    let err = pack_tree(&missing, &out, &PackTreeOptions::default())
        .expect_err("a missing source must fail");
    assert!(matches!(err, Error::PathNotFound { .. }), "{err:?}");
}

#[test]
fn pack_source_is_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a-file.txt");
    fs::write(&file, "not a directory").unwrap();
    let out = temp.path().join("out.tar.zst");
    let err =
        pack_tree(&file, &out, &PackTreeOptions::default()).expect_err("a file source must fail");
    assert!(matches!(err, Error::NotADirectory { .. }), "{err:?}");
}

// --- Extras: deny_name_secrets escape hatch ------------------------------------

#[test]
fn pack_deny_name_secrets_false_includes_env() {
    let (_temp, out, result) = pack_main_fixture(&PackTreeOptions {
        deny_name_secrets: false,
        ..PackTreeOptions::default()
    });

    let names = unpack_names(&out);
    assert!(names.iter().any(|n| n == ".env"), "{names:?}");
    assert_eq!(result.file_count, 3, "src/main.rs + notes.txt + .env");
    assert_eq!(result.skipped_secret_files, 0);
}

// --- Extras: content-deny --------------------------------------------------------

/// A fixture whose only interesting file carries a Critical content marker
/// (`aws_access_key` prefix). The file name `app.cfg` is not name-denied.
fn build_content_fixture() -> TempDir {
    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    write(
        &proj,
        "app.cfg",
        b"aws_secret_access_key = AKIAABCDEFGHIJKLMNOPQRST\n",
    );
    write(&proj, "readme.txt", b"benign\n");
    temp
}

#[test]
fn pack_content_deny_enabled_omits_secret_content() {
    let temp = build_content_fixture();
    let src = temp.path().join("proj");

    // Enabled at Critical: `app.cfg` is omitted and counted.
    let out = temp.path().join("deny.tar.zst");
    let result = pack_tree(
        &src,
        &out,
        &PackTreeOptions {
            content_deny: ContentDenyOptions {
                enabled: true,
                min_risk: SensitiveRisk::Critical,
            },
            ..PackTreeOptions::default()
        },
    )
    .unwrap();
    let names = unpack_names(&out);
    assert!(names.iter().any(|n| n == "readme.txt"), "{names:?}");
    assert!(!names.iter().any(|n| n == "app.cfg"), "{names:?}");
    assert_eq!(result.file_count, 1);
    assert!(result.skipped_secret_files >= 1);

    // Same fixture, content-deny off: `app.cfg` IS included.
    let out2 = temp.path().join("no-deny.tar.zst");
    let result2 = pack_tree(&src, &out2, &PackTreeOptions::default()).unwrap();
    let names2 = unpack_names(&out2);
    assert!(names2.iter().any(|n| n == "app.cfg"), "{names2:?}");
    assert_eq!(result2.file_count, 2);
    assert_eq!(result2.skipped_secret_files, 0);
}

#[test]
fn pack_content_deny_off_by_default() {
    assert!(!PackTreeOptions::default().content_deny.enabled);
    let defaults = ContentDenyOptions::default();
    assert!(!defaults.enabled);
    assert_eq!(defaults.min_risk, SensitiveRisk::Critical);

    // With default options a Critical-content file is still packed.
    let temp = build_content_fixture();
    let src = temp.path().join("proj");
    let out = temp.path().join("out.tar.zst");
    let result = pack_tree(&src, &out, &PackTreeOptions::default()).unwrap();
    let names = unpack_names(&out);
    assert!(names.iter().any(|n| n == "app.cfg"), "{names:?}");
    assert_eq!(result.file_count, 2);
}

// --- Extras: stats -----------------------------------------------------------------

#[test]
fn pack_size_bytes_matches_output_metadata() {
    let (_temp, out, result) = pack_main_fixture(&PackTreeOptions::default());
    assert_eq!(
        result.size_bytes,
        fs::metadata(&result.output).unwrap().len()
    );
    assert_eq!(result.output, out);
}

#[test]
fn pack_skipped_dir_names_counts_policy_dirs() {
    let (_temp, _out, result) = pack_main_fixture(&PackTreeOptions::default());
    // Default policy prunes both `target/` and `node_modules/`; assert >= 1 to
    // stay robust against policy-table additions.
    assert!(result.skipped_dir_names >= 1);
}

// --- Extra: name-deny helper -------------------------------------------------------

#[test]
fn pack_should_deny_file_in_pack_by_risk() {
    assert!(archive::should_deny_file_in_pack(Path::new(".env")));
    assert!(archive::should_deny_file_in_pack(Path::new(
        "keys/backup.pem"
    )));
    assert!(archive::should_deny_file_in_pack(Path::new("id_rsa")));
    assert!(!archive::should_deny_file_in_pack(Path::new("src/main.rs")));
    assert!(!archive::should_deny_file_in_pack(Path::new("notes.txt")));
}
