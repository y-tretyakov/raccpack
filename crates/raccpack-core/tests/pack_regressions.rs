//! Regression tests for the M4.3 P1 hardening follow-up.
//!
//! Two production fixes under test:
//!
//! 1. `archive/pack.rs` walker rewrite: the `WalkDir` + `filter_entry` (with a
//!    counter mutated inside the closure) becomes an explicit DFS
//!    (`read_dir` + own stack) in which pruned directories are counted in the
//!    main loop. Behavior must be equivalent (prune the same dirs, count the
//!    same numbers, respect `max_depth`, never follow symlinks) and archive
//!    entries must additionally become deterministic: every directory's
//!    entries are processed in ascending lossy-name order, so the archive's
//!    entry names come out in strictly ascending lexical order.
//! 2. `den/place.rs` split: `place_pack` becomes a thin wrapper
//!    (`ensure_den` → `place_pack_ensured`). The public `place_pack` contract
//!    (bootstrap a fresh den, honor `output_name`) must be unchanged.
//!
//! Cases: deterministic order, pruned dirs counted + not archived, a pruned dir
//! in the middle never stops sibling processing, `max_depth` clamping,
//! symlinked dirs never followed/archived (Unix), and the `place_pack` public
//! wrapper still bootstraps a fresh den including a named artifact.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no sleeps, no
//! real git/HOME. The symlink case is `#[cfg(unix)]`; the rest is portable.
//! Assertions on archive ordering read the decoded tar stream in *entry order*
//! and never re-sort, so a sort inside this test could never mask a regression.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use raccpack_core::{pack_tree, place_pack, PackTreeOptions, PlacePackRequest, DEN_VERSION};
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel` with `contents`.
fn write(root: &Path, rel: &str, contents: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
}

/// Decode a `.tar.zst` archive and return `(relative_name, contents)` per entry
/// in the archive's actual entry order (never re-sorted).
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
        Read::read_to_end(&mut entry, &mut content).expect("read entry contents");
        out.push((name, content));
    }
    out
}

/// Decode a `.tar.zst` archive and return the relative entry names in order.
fn unpack_names(path: &Path) -> Vec<String> {
    unpack_entries(path)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

/// A source archive sitting in its own tempdir, plus a fresh (nonexistent) den
/// path — the two ingredients every `place_pack` regression needs.
fn den_and_source(contents: &[u8]) -> (TempDir, PathBuf, PathBuf) {
    let temp = TempDir::new().unwrap();
    let den = temp.path().join("den");
    let staging = temp.path().join("staging");
    fs::create_dir_all(&staging).unwrap();
    let source = staging.join("pack.tar.zst");
    fs::write(&source, contents).unwrap();
    (temp, den, source)
}

// --- Case 1: deterministic archive order (P1 fix) ---------------------------
//
// Files are created in a deliberately mixed order (z, a, m; and inside the
// subdir b, a) so the OS readdir order on tmpfs/ext4 cannot accidentally
// coincide with lexical order. The rewrite's sorted DFS must still emit the
// entries in strictly ascending lexical order.

#[test]
fn archive_entry_order_is_ascending_lexical() {
    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    write(&proj, "z.txt", b"z\n");
    write(&proj, "a.txt", b"a\n");
    write(&proj, "m.txt", b"m\n");
    write(&proj, "subdir/b.txt", b"b2\n");
    write(&proj, "subdir/a.txt", b"a2\n");

    let out = temp.path().join("out.tar.zst");
    let result =
        pack_tree(&proj, &out, &PackTreeOptions::default()).expect("pack_tree must succeed");

    let names = unpack_names(&out);
    assert_eq!(
        names,
        vec!["a.txt", "m.txt", "subdir/a.txt", "subdir/b.txt", "z.txt"],
        "entries must appear in strictly ascending lexical order: {names:?}"
    );
    assert_eq!(result.file_count, 5, "all five files must be packed");
}

// --- Case 2: pruned dirs counted and not archived ---------------------------

#[test]
fn pruned_dirs_counted_and_not_archived() {
    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    write(&proj, "src/main.rs", b"fn main() {}\n");
    write(&proj, "node_modules/pkg/index.js", b"module.exports = 1;\n");
    write(&proj, "root.txt", b"root\n");

    let out = temp.path().join("out.tar.zst");
    let result =
        pack_tree(&proj, &out, &PackTreeOptions::default()).expect("pack_tree must succeed");

    assert!(
        result.skipped_dir_names >= 1,
        "node_modules must be counted as a pruned dir, got {}",
        result.skipped_dir_names
    );
    assert_eq!(
        result.skipped_secret_files, 0,
        "no secret-named files exist in this fixture"
    );

    let names = unpack_names(&out);
    for name in &names {
        assert!(
            !name.starts_with("node_modules"),
            "archive must not contain anything under a pruned dir: {name}"
        );
    }
    assert!(names.contains(&"root.txt".to_string()), "{names:?}");
    assert!(names.contains(&"src/main.rs".to_string()), "{names:?}");
}

// --- Case 3: pruned dir in the middle never breaks siblings ------------------

#[test]
fn pruned_dir_in_middle_does_not_stop_sibling_processing() {
    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    // `dist/` is in the default policy and sorts lexically between `a.txt` and
    // `z.txt`, so it lands mid-DSF: skipping it must not abort the rest.
    write(&proj, "a.txt", b"a\n");
    write(&proj, "dist/bundle.js", b"// built output\n");
    write(&proj, "z.txt", b"z\n");

    let out = temp.path().join("out.tar.zst");
    let result =
        pack_tree(&proj, &out, &PackTreeOptions::default()).expect("pack_tree must succeed");

    assert!(
        result.skipped_dir_names >= 1,
        "dist must be counted as a pruned dir, got {}",
        result.skipped_dir_names
    );

    let names = unpack_names(&out);
    assert!(names.contains(&"a.txt".to_string()), "{names:?}");
    assert!(
        names.contains(&"z.txt".to_string()),
        "siblings after the pruned dir must still be archived: {names:?}"
    );
    assert!(
        names.iter().all(|n| !n.starts_with("dist")),
        "nothing under the pruned dist/ may be archived: {names:?}"
    );
    assert_eq!(result.file_count, 2);
}

// --- Case 4: max_depth is still respected ------------------------------------
//
// WalkDir semantics that the rewrite must reproduce exactly: `max_depth: 0`
// yields only the root directory (depth 0) which is never archived, so no
// files — even ones sitting directly in the root — are packed.

#[test]
fn max_depth_zero_yields_no_files() {
    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    write(&proj, "root.txt", b"root-level\n");
    write(&proj, "a/b/deep.txt", b"depth 3\n");

    let out = temp.path().join("out.tar.zst");
    let result = pack_tree(
        &proj,
        &out,
        &PackTreeOptions {
            max_depth: 0,
            ..PackTreeOptions::default()
        },
    )
    .expect("pack_tree must succeed");

    assert_eq!(
        result.file_count, 0,
        "at max_depth 0 the walk visits only the root, which is never archived"
    );
    let names = unpack_names(&out);
    assert!(
        names.is_empty(),
        "even root-level files must not be archived at max_depth 0: {names:?}"
    );
}

#[test]
fn max_depth_two_includes_depth_two_excludes_depth_three() {
    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    write(&proj, "a/b.txt", b"depth 2\n");
    write(&proj, "a/b/c.txt", b"depth 3\n");

    let out = temp.path().join("out.tar.zst");
    let result = pack_tree(
        &proj,
        &out,
        &PackTreeOptions {
            max_depth: 2,
            ..PackTreeOptions::default()
        },
    )
    .expect("pack_tree must succeed");

    let names = unpack_names(&out);
    assert!(
        names.contains(&"a/b.txt".to_string()),
        "the depth-2 file must be archived: {names:?}"
    );
    assert!(
        !names.contains(&"a/b/c.txt".to_string()),
        "the depth-3 file must be excluded: {names:?}"
    );
    assert_eq!(result.file_count, 1);
}

// --- Case 5: symlinked dir is never followed or archived ----------------------

#[cfg(unix)]
#[test]
fn symlink_dir_not_followed_and_not_archived() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let proj = temp.path().join("proj");
    write(&proj, "README.txt", b"hi\n");

    // Real directory *outside* the project root; `linked` must not reach it.
    let outside = temp.path().join("outside-real");
    write(&outside, "secret.txt", b"external-content-xyz\n");
    symlink(&outside, proj.join("linked")).expect("create directory symlink");

    let out = temp.path().join("out.tar.zst");
    let result =
        pack_tree(&proj, &out, &PackTreeOptions::default()).expect("pack_tree must succeed");

    let names = unpack_names(&out);
    assert!(names.contains(&"README.txt".to_string()), "{names:?}");
    assert!(
        names
            .iter()
            .all(|n| n != "linked" && !n.starts_with("linked")),
        "the symlink and anything beneath it must not be archived: {names:?}"
    );

    // Nothing inside the archive may carry the outside file's content.
    let needle: &[u8] = b"external-content-xyz";
    for (name, content) in unpack_entries(&out) {
        assert!(
            !content.windows(needle.len()).any(|window| window == needle),
            "entry {name} leaks contents from outside the project root"
        );
    }
    assert_eq!(result.file_count, 1, "only README.txt is a regular file");
}

// --- Case 6: place_pack public wrapper still bootstraps a fresh den -----------

#[test]
fn place_pack_bootstraps_fresh_den() {
    let (_temp, den, source) = den_and_source(b"zstd-archive-bytes");

    let result = place_pack(&PlacePackRequest {
        den_root: den.clone(),
        project_name: "proj".to_string(),
        source_archive: source,
        timestamp: Some("20260804T155230Z".to_string()),
        output_name: None,
    })
    .expect("place_pack must bootstrap a freshly non-existent den");

    // Skeleton written exactly as ensure_den would have done.
    let version = fs::read_to_string(den.join(".den-version")).expect(".den-version exists");
    assert_eq!(
        version.trim(),
        DEN_VERSION,
        "version marker must match the current den version"
    );
    assert!(
        den.join("README.txt").is_file(),
        "README.txt must exist after the wrapper's ensure_den"
    );
    for dir in ["packs", "staging", "manifests", "secrets"] {
        assert!(den.join(dir).is_dir(), "den skeleton dir missing: {dir}");
    }

    let rel = PathBuf::from("packs/2026/08/proj__20260804T155230Z.tar.zst");
    assert_eq!(result.relative_path, rel);
    assert_eq!(result.absolute_path, den.join(&rel));
    assert!(result.absolute_path.is_file(), "artifact must be placed");
    assert_eq!(result.size_bytes, b"zstd-archive-bytes".len() as u64);
}

// --- Case 7: place_pack accepts output_name and a named artifact ---------------

#[test]
fn place_pack_honors_named_output() {
    let (_temp, den, source) = den_and_source(b"data");

    let result = place_pack(&PlacePackRequest {
        den_root: den.clone(),
        project_name: "proj".to_string(),
        source_archive: source,
        timestamp: Some("20260804T155230Z".to_string()),
        output_name: Some("custom".to_string()),
    })
    .expect("place_pack with a custom output_name must succeed");

    // Still under the dated packs/ tree, but the filename is the custom name.
    let rel = PathBuf::from("packs/2026/08/custom.tar.zst");
    assert_eq!(result.relative_path, rel);
    assert_eq!(result.absolute_path, den.join(&rel));
    assert!(
        result.absolute_path.is_file(),
        "named artifact must be placed"
    );
    assert_eq!(result.size_bytes, b"data".len() as u64);

    assert!(
        den.join("packs").is_dir() && den.join(".den-version").is_file(),
        "fresh-den bootstrap must still run for a named artifact"
    );
}
