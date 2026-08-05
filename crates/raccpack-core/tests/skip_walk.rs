//! Integration tests for M1.4 — `SkipPolicy` and the safe walk helper.
//!
//! Covers `SkipPolicy::default_scan` / `empty` / `with_custom_dir_names` /
//! `with_skip_hidden_dirs`, `skip_reason_dir` ordering and determinism, the
//! `walk_tree` / `WalkOptions` traversal (respecting policy + `max_depth`,
//! never following symlinks) and `ensure_scan_root` validation, as specified
//! in docs/mvp/m1/m1.4-skip-policy-walk.md §4–§7.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//! Symlinks are Linux/Unix-only, so the whole file is `#[cfg(unix)]`.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use raccpack_core::{ensure_scan_root, walk_tree, Error, SkipPolicy, SkipReason, WalkOptions};
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`.
fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, "x").expect("write fixture file");
}

/// Run `walk_tree` over `root` with `opts`, unwrap every item, and return the
/// yielded paths relative to `root`.
///
/// `strip_prefix` panics if any entry ever leaves `root`, so this helper also
/// doubles as an escape check for every walk test.
fn walked_rel(root: &Path, opts: &WalkOptions) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for item in walk_tree(root, opts) {
        let entry = item.expect("walkdir must not error on temp fixtures");
        let rel = entry
            .path()
            .strip_prefix(root)
            .expect("every entry must stay under the scan root");
        out.push(rel.to_path_buf());
    }
    out
}

/// True if any component of the relative path equals `name`.
fn has_component(rel: &Path, name: &str) -> bool {
    rel.components()
        .any(|c| c.as_os_str().to_string_lossy() == name)
}

// --- Case 1: follow_links(false) / symlink isolation ------------------------

#[test]
fn walk_follow_links_false_never_leaves_scan_root() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    write(outside.path(), "confidential.txt");
    // A directory symlink pointing OUTSIDE the scan root.
    symlink(outside.path(), root.path().join("link_out")).unwrap();
    write(root.path(), "keep/keep.txt");

    let opts = WalkOptions::default();
    let mut saw_confidential = false;
    for item in walk_tree(root.path(), &opts) {
        let entry = item.expect("walkdir must not error");
        let path = entry.path();
        assert!(
            path.starts_with(root.path()),
            "entry escaped the scan root: {path:?}"
        );
        if path.ends_with("confidential.txt") {
            saw_confidential = true;
        }
    }
    assert!(
        !saw_confidential,
        "contents of a symlinked external dir must never appear"
    );
}

#[test]
fn walk_symlink_cycle_terminates_and_does_not_panic() {
    let root = TempDir::new().unwrap();
    write(root.path(), "sub/deep.txt");
    // Symlink pointing back at an ancestor: an infinite loop if followed.
    symlink(root.path(), root.path().join("sub").join("loop")).unwrap();

    let opts = WalkOptions::default();
    let paths = walked_rel(root.path(), &opts);

    assert!(paths.iter().any(|p| p.ends_with("deep.txt")));
    // The cycle link itself is reported, but never descended into.
    let loop_paths: Vec<_> = paths
        .iter()
        .filter(|p| p.starts_with(Path::new("sub/loop")))
        .collect();
    assert_eq!(
        loop_paths.len(),
        1,
        "only the symlink entry, never its target's contents: {paths:?}"
    );
    assert_eq!(loop_paths[0], Path::new("sub/loop"));
}

// --- Case 2 & 3: skip node_modules / target ---------------------------------

#[test]
fn walk_skips_node_modules_contents() {
    let root = TempDir::new().unwrap();
    write(root.path(), "a/node_modules/secret.txt");
    write(root.path(), "a/src/main.rs");

    let paths = walked_rel(root.path(), &WalkOptions::default());
    assert!(paths.iter().any(|p| p.ends_with("main.rs")));
    assert!(
        !paths.iter().any(|p| has_component(p, "node_modules")),
        "node_modules must not be descended: {paths:?}"
    );
}

#[test]
fn walk_skips_target_contents() {
    let root = TempDir::new().unwrap();
    write(root.path(), "a/target/debug/obj/lib.rlib");
    write(root.path(), "a/src/lib.rs");

    let paths = walked_rel(root.path(), &WalkOptions::default());
    assert!(paths.iter().any(|p| p.ends_with("lib.rs")));
    assert!(
        !paths.iter().any(|p| has_component(p, "target")),
        "target must not be descended: {paths:?}"
    );
}

// --- Case 4: max_depth --------------------------------------------------------

#[test]
fn walk_respects_max_depth() {
    let root = TempDir::new().unwrap();
    write(root.path(), "a/b/c/deep.txt");
    write(root.path(), "a/top.txt");

    // walkdir counts the root as depth 0: max_depth=2 yields root + 2 levels.
    let opts = WalkOptions {
        max_depth: 2,
        ..WalkOptions::default()
    };
    let paths = walked_rel(root.path(), &opts);

    // Root children at the boundary depth are still reported.
    assert!(paths.iter().any(|p| p.ends_with("top.txt")));
    assert!(paths.iter().any(|p| p.ends_with("a/b")));
    // Anything deeper than max_depth must not appear.
    assert!(!paths.iter().any(|p| p.ends_with("deep.txt")));
    assert!(!paths.iter().any(|p| has_component(p, "c")));
}

#[test]
fn walk_max_depth_one_yields_root_children() {
    let root = TempDir::new().unwrap();
    write(root.path(), "a/file.txt");
    write(root.path(), "a/deeper/too_deep.txt");

    let opts = WalkOptions {
        max_depth: 1,
        ..WalkOptions::default()
    };
    let paths = walked_rel(root.path(), &opts);
    assert!(paths.iter().any(|p| p.ends_with("a")));
    assert!(!paths.iter().any(|p| p.ends_with("file.txt")));
    assert!(!paths.iter().any(|p| p.ends_with("too_deep.txt")));
}

#[test]
fn walk_max_depth_zero_yields_only_root_when_included() {
    let root = TempDir::new().unwrap();
    write(root.path(), "a/file.txt");

    let opts = WalkOptions {
        max_depth: 0,
        include_root: true,
        ..WalkOptions::default()
    };
    let entries: Vec<PathBuf> = {
        let mut out = Vec::new();
        for item in walk_tree(root.path(), &opts) {
            out.push(item.expect("no error").path().to_path_buf());
        }
        out
    };
    assert_eq!(entries.len(), 1, "only the root itself: {entries:?}");
    assert_eq!(entries[0], root.path());
}

// --- Case 5: root validation ----------------------------------------------------

#[test]
fn walk_ensure_scan_root_missing_path_is_path_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("does-not-exist");
    let err = ensure_scan_root(&missing).expect_err("missing path must fail");
    assert!(matches!(err, Error::PathNotFound { .. }));
    assert_eq!(
        err.suggestion(),
        Some("Check that scan_root exists and is accessible.")
    );
}

#[test]
fn walk_ensure_scan_root_file_is_not_a_directory() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a-file.txt");
    fs::write(&file, "not a dir").unwrap();

    let err = ensure_scan_root(&file).expect_err("a file must fail");
    assert!(matches!(err, Error::NotADirectory { .. }));
    assert_eq!(
        err.suggestion(),
        Some("Provide a directory path, not a file.")
    );
}

#[test]
fn walk_ensure_scan_root_valid_dir_is_ok() {
    let temp = TempDir::new().unwrap();
    assert!(ensure_scan_root(temp.path()).is_ok());
}

// --- Case 6: default policy contents -------------------------------------------

#[test]
fn skip_policy_default_skips_node_modules_and_target() {
    let policy = SkipPolicy::default_scan();
    let defaults = [
        "node_modules",
        "target",
        ".git",
        ".svn",
        ".hg",
        "__pycache__",
        ".venv",
        "venv",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        ".cache",
        "dist",
        "build",
        ".idea",
        ".vscode",
        ".raccpack",
    ];
    for name in defaults {
        let path = Path::new("/proj").join(name);
        assert!(
            policy.should_skip_dir(&path),
            "default policy must skip `{name}`"
        );
        assert_eq!(
            policy.skip_reason_dir(&path),
            Some(SkipReason::DefaultDirName),
            "reason for `{name}`"
        );
    }
}

// --- Case 7: custom names -------------------------------------------------------

#[test]
fn skip_policy_with_custom_dir_names_skips_vendor() {
    let base = SkipPolicy::default_scan();
    assert!(
        !base.should_skip_dir(Path::new("/proj/vendor")),
        "vendor is not skipped before adding a custom name"
    );

    let policy = base.with_custom_dir_names(["vendor"]);
    assert!(policy.should_skip_dir(Path::new("/proj/vendor")));
    // Built-in defaults still apply after adding custom names.
    assert!(policy.should_skip_dir(Path::new("/proj/node_modules")));
}

#[test]
fn walk_skips_custom_vendor_dir() {
    let root = TempDir::new().unwrap();
    write(root.path(), "app/vendor/lib.js");
    write(root.path(), "app/src/index.js");

    let opts = WalkOptions {
        policy: SkipPolicy::default_scan().with_custom_dir_names(["vendor"]),
        ..WalkOptions::default()
    };
    let paths = walked_rel(root.path(), &opts);
    assert!(paths.iter().any(|p| p.ends_with("index.js")));
    assert!(
        !paths.iter().any(|p| has_component(p, "vendor")),
        "vendor must not be descended: {paths:?}"
    );
}

// --- Case 8: empty tree -----------------------------------------------------------

#[test]
fn walk_empty_tree_yields_zero_entries() {
    let root = TempDir::new().unwrap();
    let opts = WalkOptions {
        include_root: false,
        ..WalkOptions::default()
    };
    assert!(walked_rel(root.path(), &opts).is_empty());
}

#[test]
fn walk_empty_tree_yields_only_root_when_included() {
    let root = TempDir::new().unwrap();
    let opts = WalkOptions {
        include_root: true,
        ..WalkOptions::default()
    };
    let entries: Vec<PathBuf> = {
        let mut out = Vec::new();
        for item in walk_tree(root.path(), &opts) {
            out.push(item.expect("no error").path().to_path_buf());
        }
        out
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0], root.path());
}

// --- Case 9: symlink to a directory is yielded but never followed ----------------

#[test]
fn walk_symlink_to_dir_yielded_but_not_followed() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    write(outside.path(), "secret.txt");
    let link = root.path().join("link");
    symlink(outside.path(), &link).unwrap();

    let opts = WalkOptions::default();
    let mut saw_link = false;
    for item in walk_tree(root.path(), &opts) {
        let entry = item.expect("walkdir must not error");
        let path = entry.path();
        assert!(path.starts_with(root.path()));
        if path == link {
            saw_link = true;
            assert!(
                entry.file_type().is_symlink(),
                "the link must be reported as a symlink"
            );
            assert!(!entry.file_type().is_dir());
            assert!(!entry.file_type().is_file());
        }
        assert!(
            !path.ends_with("secret.txt"),
            "external contents must not be followed through the link"
        );
    }
    assert!(saw_link, "the symlink entry itself must be yielded");
}

// --- Case 10 & 11: determinism and reason classification --------------------------

#[test]
fn skip_reason_dir_is_deterministic() {
    let policy = SkipPolicy::default_scan().with_skip_hidden_dirs(true);
    for path in [
        "/proj/node_modules",
        "/proj/.secret_dir",
        "/proj/vendor",
        "/proj/keep",
        "/",
    ] {
        let p = Path::new(path);
        assert_eq!(
            policy.skip_reason_dir(p),
            policy.skip_reason_dir(p),
            "skip_reason_dir must be deterministic for {path}"
        );
    }
}

#[test]
fn skip_reason_default_vs_custom_vs_hidden() {
    let base = SkipPolicy::default_scan();
    assert_eq!(
        base.skip_reason_dir(Path::new("/proj/node_modules")),
        Some(SkipReason::DefaultDirName)
    );

    let custom = base.clone().with_custom_dir_names(["vendor"]);
    assert_eq!(
        custom.skip_reason_dir(Path::new("/proj/vendor")),
        Some(SkipReason::CustomPattern)
    );

    let hidden = base.with_skip_hidden_dirs(true);
    assert_eq!(
        hidden.skip_reason_dir(Path::new("/proj/.secret_dir")),
        Some(SkipReason::Hidden)
    );

    assert_eq!(
        SkipPolicy::empty().skip_reason_dir(Path::new("/proj/node_modules")),
        None
    );
}

#[test]
fn skip_reason_order_default_before_hidden() {
    // `.git` is in the built-in list, so even with the hidden flag on it must
    // report DefaultDirName, not Hidden (ordering DefaultDirName -> Hidden).
    let policy = SkipPolicy::default_scan().with_skip_hidden_dirs(true);
    assert_eq!(
        policy.skip_reason_dir(Path::new("/proj/.git")),
        Some(SkipReason::DefaultDirName)
    );
}

#[test]
fn skip_reason_dir_root_path_returns_none() {
    assert_eq!(
        SkipPolicy::default_scan().skip_reason_dir(Path::new("/")),
        None
    );
}

// --- Case 12: suffix pattern -------------------------------------------------------

#[test]
fn skip_policy_suffix_pattern_egg_info_matches() {
    let policy = SkipPolicy::default_scan();
    assert_eq!(
        policy.skip_reason_dir(Path::new("/proj/foo.egg-info")),
        Some(SkipReason::DefaultDirName)
    );
    assert!(policy.should_skip_dir(Path::new("/proj/pkg/foo.egg-info")));
    // Suffix match, not prefix: a bare `egg-info` name does not match.
    assert_eq!(policy.skip_reason_dir(Path::new("/proj/egg-info")), None);
}

#[test]
fn walk_skips_egg_info_suffix_dir() {
    let root = TempDir::new().unwrap();
    write(root.path(), "pkg/foo.egg-info/PKG-INFO");
    write(root.path(), "pkg/setup.py");

    let paths = walked_rel(root.path(), &WalkOptions::default());
    assert!(paths.iter().any(|p| p.ends_with("setup.py")));
    assert!(
        !paths.iter().any(|p| has_component(p, "foo.egg-info")),
        "egg-info dir must not be descended: {paths:?}"
    );
}

// --- Case 13: hidden-dirs flag off by default --------------------------------------

#[test]
fn skip_hidden_dirs_off_by_default_skips_nothing_extra() {
    let policy = SkipPolicy::default_scan();
    // A dot-dir not present in the name list is NOT skipped by default.
    assert_eq!(policy.skip_reason_dir(Path::new("/proj/.secret_dir")), None);
    assert!(!policy.should_skip_dir(Path::new("/proj/.secret_dir")));
}

#[test]
fn skip_hidden_dirs_on_skips_other_dot_dirs() {
    let policy = SkipPolicy::default_scan().with_skip_hidden_dirs(true);
    assert_eq!(
        policy.skip_reason_dir(Path::new("/proj/.secret_dir")),
        Some(SkipReason::Hidden)
    );
    assert!(policy.should_skip_dir(Path::new("/proj/.secret_dir")));
}

#[test]
fn walk_skips_hidden_dirs_when_enabled() {
    // NOTE: `TempDir` paths are hidden dot-dirs (e.g. `/.tmpXXXXXX`), and the
    // hidden rule applies to the walk root itself, so walk a non-hidden
    // subdirectory as the scan root.
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("scanroot");
    fs::create_dir(&root).expect("create scan root");
    write(&root, ".secret_dir/data.txt");
    write(&root, "visible.txt");

    let opts = WalkOptions {
        policy: SkipPolicy::default_scan().with_skip_hidden_dirs(true),
        ..WalkOptions::default()
    };
    let paths = walked_rel(&root, &opts);
    assert!(paths.iter().any(|p| p.ends_with("visible.txt")));
    assert!(
        !paths.iter().any(|p| has_component(p, ".secret_dir")),
        "hidden dir must be skipped: {paths:?}"
    );
}

#[test]
fn walk_descends_dot_dir_by_default() {
    let root = TempDir::new().unwrap();
    write(root.path(), ".secret_dir/data.txt");

    let paths = walked_rel(root.path(), &WalkOptions::default());
    assert!(
        paths.iter().any(|p| p.ends_with("data.txt")),
        "dot dirs are descended by default: {paths:?}"
    );
}

// --- Extras ------------------------------------------------------------------------

#[test]
fn walk_empty_policy_descends_node_modules() {
    let root = TempDir::new().unwrap();
    write(root.path(), "node_modules/pkg/x.txt");

    let opts = WalkOptions {
        policy: SkipPolicy::empty(),
        ..WalkOptions::default()
    };
    let paths = walked_rel(root.path(), &opts);
    assert!(
        paths.iter().any(|p| p.ends_with("x.txt")),
        "empty policy must not skip anything: {paths:?}"
    );
}
