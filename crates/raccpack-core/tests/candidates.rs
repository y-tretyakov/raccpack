//! Integration tests for M2.1 — candidate discovery via `find_candidates`.
//!
//! Covers the behavioral contract from the stage spec: marker detection via
//! exact, case-sensitive `file_name()` matches; `.git` as a `DirName` marker
//! with `accept_git_only`; `SkipPolicy` honoring (no candidates inside
//! `node_modules` / `target`); `max_depth` in walkdir semantics (root =
//! depth 0); symlink isolation; root validation errors; stable path ordering;
//! determinism; `extra_markers`; marker-hit fields; and the `name` field.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//! Symlinks are Linux/Unix-only, so the symlink test is `#[cfg(unix)]`.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{
    find_candidates, CandidateOptions, Error, MarkerDef, MarkerKind, ProjectCandidate,
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

/// Run `find_candidates` and unwrap the result; fixtures must never error.
fn find(root: &Path, opts: &CandidateOptions) -> Vec<ProjectCandidate> {
    find_candidates(root, opts).expect("find_candidates must succeed on temp fixture")
}

/// Candidate paths relative to `root`, preserving the returned order.
fn rel_paths(cands: &[ProjectCandidate], root: &Path) -> Vec<PathBuf> {
    cands
        .iter()
        .map(|c| {
            c.path
                .strip_prefix(root)
                .expect("candidate must live under the scan root")
                .to_path_buf()
        })
        .collect()
}

fn names_of(cands: &[ProjectCandidate]) -> Vec<&str> {
    cands.iter().map(|c| c.name.as_str()).collect()
}

/// The standard fixture from the stage spec.
fn standard_fixture() -> TempDir {
    let root = TempDir::new().unwrap();
    write(root.path(), "app-rust/Cargo.toml");
    write(root.path(), "app-node/package.json");
    write(root.path(), "app-node/node_modules/left-pad/package.json");
    write(root.path(), "nested/deep/go.mod");
    write_dir(root.path(), "only-git/.git");
    write_dir(root.path(), "empty-dir");
    write(root.path(), "target/debug/foo");
    root
}

// --- Case 1: default discovery ----------------------------------------------

#[test]
fn candidates_finds_rust_node_gomod_and_git_only() {
    let root = standard_fixture();

    let cands = find(root.path(), &CandidateOptions::default());
    let rels = rel_paths(&cands, root.path());

    // app-rust (Cargo.toml), app-node (package.json), nested/deep (go.mod) and
    // only-git (.git; accept_git_only defaults to true). Nothing inside
    // `node_modules` or `target` may surface, and `empty-dir` has no markers.
    let expected: Vec<PathBuf> = ["app-node", "app-rust", "nested/deep", "only-git"]
        .iter()
        .map(PathBuf::from)
        .collect();
    assert_eq!(rels, expected, "candidates must be sorted stably by path");

    let mut names = names_of(&cands);
    names.sort();
    assert_eq!(
        names,
        vec!["app-node", "app-rust", "deep", "only-git"],
        "candidate `name` fields"
    );
}

// --- Case 2: is_git_repo ----------------------------------------------------

#[test]
fn only_git_candidate_is_git_repo_and_others_are_not() {
    let root = standard_fixture();
    let cands = find(root.path(), &CandidateOptions::default());

    for c in &cands {
        assert!(
            c.path.starts_with(root.path()),
            "every candidate must live under the scan root: {:?}",
            c.path
        );
    }

    let git = cands.iter().find(|c| c.name == "only-git").unwrap();
    assert!(git.is_git_repo, "`only-git` carries the `.git` marker");
    for c in cands.iter().filter(|c| c.name != "only-git") {
        assert!(
            !c.is_git_repo,
            "{} must not be flagged as a git repo",
            c.name
        );
    }
}

// --- Case 3: markerless directory -------------------------------------------

#[test]
fn empty_dir_is_not_a_candidate() {
    let root = standard_fixture();
    let cands = find(root.path(), &CandidateOptions::default());
    assert!(
        !cands.iter().any(|c| c.name == "empty-dir"),
        "a directory without markers must never be a candidate"
    );
}

// --- Case 4: multiple markers in one directory ------------------------------

#[test]
fn dual_marker_dir_yields_single_candidate_with_multiple_hits() {
    let root = TempDir::new().unwrap();
    write(root.path(), "polyglot/Cargo.toml");
    write(root.path(), "polyglot/package.json");

    let cands = find(root.path(), &CandidateOptions::default());
    assert_eq!(cands.len(), 1, "one dir with two markers = one candidate");
    let c = &cands[0];
    assert_eq!(c.name, "polyglot");
    assert!(c.markers.len() >= 2, "both markers must be reported");
    let hits: Vec<&str> = c.markers.iter().map(|m| m.name.as_str()).collect();
    assert!(hits.contains(&"Cargo.toml"));
    assert!(hits.contains(&"package.json"));
}

// --- Case 5: max_depth --------------------------------------------------------

#[test]
fn max_depth_one_hides_deep_candidates_but_keeps_shallow_ones() {
    let root = standard_fixture();
    // walkdir counts the root as depth 0: `app-rust`, `app-node` and `only-git`
    // sit one level down and are visited (their markers are discovered while
    // visiting them), whereas `nested/deep` (and its `go.mod`) is two levels
    // down and out of reach.
    let opts = CandidateOptions {
        max_depth: 1,
        ..CandidateOptions::default()
    };
    let cands = find(root.path(), &opts);
    let mut names = names_of(&cands);
    names.sort();

    for shallow in ["app-node", "app-rust", "only-git"] {
        assert!(
            names.contains(&shallow),
            "depth-1 candidate {shallow} must still be found"
        );
    }
    assert!(
        !names.contains(&"deep"),
        "`nested/deep` must be out of max_depth: {names:?}"
    );
    assert!(!names.contains(&"empty-dir"));
}

// --- Case 6: root validation errors -------------------------------------------

#[test]
fn candidates_missing_root_is_path_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("does-not-exist");
    let err = match find_candidates(&missing, &CandidateOptions::default()) {
        Ok(_) => panic!("missing scan root must fail"),
        Err(e) => e,
    };
    assert!(matches!(err, Error::PathNotFound { .. }));
}

#[test]
fn candidates_file_as_root_is_not_a_directory() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a-file.txt");
    fs::write(&file, "not a dir").unwrap();

    let err = match find_candidates(&file, &CandidateOptions::default()) {
        Ok(_) => panic!("a file as scan root must fail"),
        Err(e) => e,
    };
    assert!(matches!(err, Error::NotADirectory { .. }));
}

// --- Case 7: determinism --------------------------------------------------------

#[test]
fn candidates_are_deterministic_across_runs() {
    let root = standard_fixture();
    let a = rel_paths(
        &find(root.path(), &CandidateOptions::default()),
        root.path(),
    );
    let b = rel_paths(
        &find(root.path(), &CandidateOptions::default()),
        root.path(),
    );
    assert_eq!(a, b, "two runs over the same fixture must agree exactly");
}

// --- Case 8: symlinked dirs are never entered ------------------------------------

#[cfg(unix)]
#[test]
fn candidates_never_enter_symlinked_directory() {
    let root = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    write(outside.path(), "external/package.json");
    symlink(outside.path(), root.path().join("link_out")).unwrap();
    write(root.path(), "keep/Cargo.toml");

    let cands = find(root.path(), &CandidateOptions::default());

    assert_eq!(names_of(&cands), vec!["keep"]);
    for c in &cands {
        assert!(
            c.path.starts_with(root.path()),
            "candidate escaped the scan root: {:?}",
            c.path
        );
    }
    assert!(
        !cands
            .iter()
            .any(|c| c.name == "external" || c.name == "link_out"),
        "symlinked external contents must never surface as candidates"
    );
}

// --- Case 9: accept_git_only --------------------------------------------------------

#[test]
fn accept_git_only_false_excludes_git_only_candidates() {
    let root = standard_fixture();
    let opts = CandidateOptions {
        accept_git_only: false,
        ..CandidateOptions::default()
    };
    let cands = find(root.path(), &opts);

    let mut names = names_of(&cands);
    names.sort();
    assert_eq!(names, vec!["app-node", "app-rust", "deep"]);
}

// --- Case 10: the scan root itself ------------------------------------------------

#[test]
fn marker_in_scan_root_makes_root_a_candidate() {
    let root = TempDir::new().unwrap();
    write(root.path(), "Cargo.toml");

    let cands = find(root.path(), &CandidateOptions::default());
    assert_eq!(cands.len(), 1);
    let c = &cands[0];
    assert_eq!(
        c.path,
        root.path(),
        "candidate path must equal the scan root"
    );
    assert_eq!(
        c.name,
        root.path().file_name().unwrap().to_string_lossy(),
        "name must be the scan root's directory name"
    );
    assert!(c.markers.iter().any(|m| m.name == "Cargo.toml"));
}

// --- Case 11: extra_markers ----------------------------------------------------------

#[test]
fn extra_markers_detect_custom_marker_with_language_hint() {
    let root = TempDir::new().unwrap();
    write(root.path(), "ruby-app/Rakefile");

    let opts = CandidateOptions {
        extra_markers: vec![MarkerDef {
            kind: MarkerKind::FileName,
            name: "Rakefile",
            language_hint: Some("Ruby"),
        }],
        ..CandidateOptions::default()
    };
    let cands = find(root.path(), &opts);
    assert_eq!(cands.len(), 1);
    let c = &cands[0];
    assert_eq!(c.name, "ruby-app");
    assert_eq!(c.markers.len(), 1);
    let hit = &c.markers[0];
    assert_eq!(hit.name, "Rakefile");
    assert_eq!(hit.kind, MarkerKind::FileName);
    assert_eq!(hit.language_hint.as_deref(), Some("Ruby"));
}

// --- Case 12: marker hit fields -------------------------------------------------------

#[test]
fn marker_hits_report_name_kind_and_language_hint() {
    let root = standard_fixture();
    let cands = find(root.path(), &CandidateOptions::default());

    let find_hit = |cand: &str, marker: &str| {
        cands
            .iter()
            .find(|c| c.name == cand)
            .unwrap_or_else(|| panic!("candidate {cand} missing"))
            .markers
            .iter()
            .find(|m| m.name == marker)
            .unwrap_or_else(|| panic!("marker {marker} missing in {cand}"))
    };

    let rust = find_hit("app-rust", "Cargo.toml");
    assert_eq!(rust.kind, MarkerKind::FileName);
    assert_eq!(rust.language_hint.as_deref(), Some("Rust"));

    let js = find_hit("app-node", "package.json");
    assert_eq!(js.kind, MarkerKind::FileName);
    assert_eq!(js.language_hint.as_deref(), Some("JavaScript"));

    let go = find_hit("deep", "go.mod");
    assert_eq!(go.kind, MarkerKind::FileName);
    assert_eq!(go.language_hint.as_deref(), Some("Go"));

    let git = find_hit("only-git", ".git");
    assert_eq!(git.kind, MarkerKind::DirName);
    assert_eq!(git.language_hint, None);
}

// --- Case 13: empty scan root ----------------------------------------------------------

#[test]
fn empty_scan_root_yields_no_candidates() {
    let root = TempDir::new().unwrap();
    let cands = find(root.path(), &CandidateOptions::default());
    assert!(cands.is_empty());
}

// --- Case 14: candidate name is the directory name ---------------------------------------

#[test]
fn candidate_name_is_directory_name() {
    let root = standard_fixture();
    for c in find(root.path(), &CandidateOptions::default()) {
        let expected = c
            .path
            .file_name()
            .unwrap_or_else(|| panic!("{:?} has no file_name", c.path))
            .to_string_lossy();
        assert_eq!(c.name, expected, "name for {:?}", c.path);
    }
}
