//! Integration tests for M2.2 — language/framework detect → `Stack`.
//!
//! Covers the behavioral contract from the stage spec §4–§7: language from
//! marker `language_hint`s by the priority table (Cargo.toml > go.mod >
//! … > Makefile), framework hints by shallow file names (next/nuxt/vite/
//! angular/deno/django/rails/sbt), the pure `stack_from_candidate`,
//! path-backed `detect_stack`, fail-fast batch `detect_stacks`,
//! `candidate_to_project`, `project_size_bytes` with `SkipPolicy`, error
//! mapping (`PathNotFound` / `NotADirectory`) and determinism.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//!
//! Every test name contains `detect` or `stack` so the narrow runs
//! `cargo test -p raccpack-core -- detect` and `-- stack` catch them all.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::detect::{
    candidate_to_project, detect_stack, detect_stacks, stack_from_candidate,
};
use raccpack_core::scan::size::project_size_bytes;
use raccpack_core::{
    find_candidates, CandidateOptions, Error, MarkerHit, MarkerKind, ProjectCandidate, SkipPolicy,
    Stack,
};
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel` (1-byte content).
fn write(root: &Path, rel: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, "x").expect("write fixture file");
}

/// Create a directory (and parents) at `root/rel`, leaving it empty.
fn write_dir(root: &Path, rel: &str) {
    fs::create_dir_all(root.join(rel)).expect("create fixture dir");
}

/// Build a [`MarkerHit`] with the given fields.
fn hit(name: &str, kind: MarkerKind, language_hint: Option<&str>) -> MarkerHit {
    MarkerHit {
        name: name.to_string(),
        kind,
        language_hint: language_hint.map(str::to_string),
    }
}

/// Build a [`ProjectCandidate`] with the given markers.
fn candidate(
    path: PathBuf,
    name: &str,
    markers: Vec<MarkerHit>,
    is_git_repo: bool,
) -> ProjectCandidate {
    ProjectCandidate {
        path,
        name: name.to_string(),
        markers,
        is_git_repo,
    }
}

/// Sorted copy of `Stack.markers` for order-independent comparison.
fn sorted_markers(stack: &Stack) -> Vec<String> {
    let mut markers = stack.markers.clone();
    markers.sort();
    markers
}

/// Run `find_candidates` over a fresh fixture and return the single candidate.
fn single_candidate(root: &Path, name: &str) -> ProjectCandidate {
    let cands = find_candidates(root, &CandidateOptions::default())
        .expect("find_candidates must succeed on temp fixture");
    let cand = cands
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("candidate {name} missing: {cands:?}"));
    cand.clone()
}

// --- Case 1: Cargo.toml only → Rust -----------------------------------------

#[test]
fn detect_language_cargo_only_is_rust() {
    let cand = candidate(
        PathBuf::from("/tmp/app-rust"),
        "app-rust",
        vec![hit("Cargo.toml", MarkerKind::FileName, Some("Rust"))],
        false,
    );
    let stack = stack_from_candidate(&cand);
    assert_eq!(stack.language.as_deref(), Some("Rust"));
    assert_eq!(stack.markers, vec!["Cargo.toml".to_string()]);
    assert!(stack.frameworks.is_empty());
}

// --- Case 2: package.json + next.config.js → JS/TS + Next.js ----------------

#[test]
fn detect_stack_nextjs_language_and_framework() {
    let root = TempDir::new().unwrap();
    write(root.path(), "web/package.json");
    write(root.path(), "web/next.config.js");

    let markers = vec![hit(
        "package.json",
        MarkerKind::FileName,
        Some("JavaScript"),
    )];
    let stack = detect_stack(&root.path().join("web"), &markers).expect("detect must succeed");
    assert!(
        stack.language.is_some(),
        "a JS manifest must yield a language"
    );
    assert!(
        stack.frameworks.contains(&"Next.js".to_string()),
        "next.config.js must map to Next.js, got {:?}",
        stack.frameworks
    );
    assert!(stack.markers.contains(&"package.json".to_string()));
}

// --- Case 3: go.mod → Go ----------------------------------------------------

#[test]
fn detect_language_go_only_is_go() {
    let cand = candidate(
        PathBuf::from("/tmp/app-go"),
        "app-go",
        vec![hit("go.mod", MarkerKind::FileName, Some("Go"))],
        false,
    );
    let stack = stack_from_candidate(&cand);
    assert_eq!(stack.language.as_deref(), Some("Go"));
    assert_eq!(stack.markers, vec!["go.mod".to_string()]);
}

// --- Case 4: Cargo.toml + package.json conflict → Rust + both markers -------

#[test]
fn detect_language_conflict_cargo_wins_over_package_json() {
    let markers = vec![
        hit("Cargo.toml", MarkerKind::FileName, Some("Rust")),
        hit("package.json", MarkerKind::FileName, Some("JavaScript")),
    ];
    let cand = candidate(PathBuf::from("/tmp/polyglot"), "polyglot", markers, false);
    let stack = stack_from_candidate(&cand);

    assert_eq!(
        stack.language.as_deref(),
        Some("Rust"),
        "Cargo.toml must outrank package.json in the priority table"
    );
    assert_eq!(
        sorted_markers(&stack),
        vec!["Cargo.toml".to_string(), "package.json".to_string()],
        "both markers must be reported (sorted)"
    );
}

// --- Priority table: additional unambiguous orderings ------------------------

#[test]
fn detect_language_priority_go_beats_package_json() {
    let cand = candidate(
        PathBuf::from("/tmp/mixed"),
        "mixed",
        vec![
            hit("go.mod", MarkerKind::FileName, Some("Go")),
            hit("package.json", MarkerKind::FileName, Some("JavaScript")),
        ],
        false,
    );
    let stack = stack_from_candidate(&cand);
    assert_eq!(stack.language.as_deref(), Some("Go"));
}

#[test]
fn detect_language_priority_python_beats_gemfile() {
    let cand = candidate(
        PathBuf::from("/tmp/mixed2"),
        "mixed2",
        vec![
            hit("Gemfile", MarkerKind::FileName, Some("Ruby")),
            hit("requirements.txt", MarkerKind::FileName, Some("Python")),
        ],
        false,
    );
    let stack = stack_from_candidate(&cand);
    assert_eq!(
        stack.language.as_deref(),
        Some("Python"),
        "requirements.txt must outrank Gemfile"
    );
}

// --- Case 5: only .git → language None, marker kept --------------------------

#[test]
fn detect_stack_git_only_has_no_language() {
    let root = TempDir::new().unwrap();
    write_dir(root.path(), "repo/.git");

    let cands = find_candidates(root.path(), &CandidateOptions::default()).unwrap();
    let cand = cands
        .iter()
        .find(|c| c.name == "repo")
        .unwrap_or_else(|| panic!("git-only candidate missing: {cands:?}"));
    assert!(cand.is_git_repo, "`.git` marker must set is_git_repo");

    let stack = stack_from_candidate(cand);
    assert_eq!(stack.language, None, "`.git` must not imply a language");
    assert!(stack.markers.contains(&".git".to_string()));
    assert!(stack.frameworks.is_empty());
}

// --- Case 6: stack_from_candidate is pure (no FS access) ----------------------

#[test]
fn detect_stack_from_candidate_is_pure_no_fs() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("does-not-exist");
    let cand = candidate(
        missing.clone(),
        "ghost",
        vec![hit("Cargo.toml", MarkerKind::FileName, Some("Rust"))],
        false,
    );
    assert!(
        !missing.exists(),
        "fixture path must not exist so purity is actually exercised"
    );

    let stack = stack_from_candidate(&cand);
    assert_eq!(stack.language.as_deref(), Some("Rust"));
    assert!(stack.markers.contains(&"Cargo.toml".to_string()));
    assert!(
        stack.frameworks.is_empty(),
        "pure detect cannot enrich frameworks"
    );
}

// --- Case 7: detect_stack / enrich sees a framework file ----------------------

#[test]
fn detect_stack_enriches_framework_from_next_config() {
    let root = TempDir::new().unwrap();
    write(root.path(), "web/package.json");
    write(root.path(), "web/next.config.mjs");

    let stack = detect_stack(
        &root.path().join("web"),
        &[hit(
            "package.json",
            MarkerKind::FileName,
            Some("JavaScript"),
        )],
    )
    .expect("detect must succeed");
    assert!(
        stack.frameworks.contains(&"Next.js".to_string()),
        "next.config.mjs must map to Next.js, got {:?}",
        stack.frameworks
    );
}

// --- Case 8: project_size_bytes ----------------------------------------------

#[test]
fn stack_size_sums_files_and_skips_node_modules() {
    let root = TempDir::new().unwrap();
    let src = "fn main() { println!(\"hi\"); }\n";
    let readme = "# demo\n";
    let vendored = "var x = 1;\n".repeat(100);

    let proj = root.path().join("proj");
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::create_dir_all(proj.join("node_modules/lodash")).unwrap();
    fs::create_dir_all(proj.join("target/debug")).unwrap();
    fs::write(proj.join("src/main.rs"), src).unwrap();
    fs::write(proj.join("README.md"), readme).unwrap();
    fs::write(proj.join("node_modules/lodash/index.js"), &vendored).unwrap();
    fs::write(proj.join("target/debug/app"), &vendored).unwrap();

    let expected = (src.len() + readme.len()) as u64;
    let actual =
        project_size_bytes(&proj, &SkipPolicy::default_scan(), 6).expect("size must succeed");
    assert_eq!(
        actual, expected,
        "only real project files counted; node_modules/target skipped"
    );
}

#[test]
fn stack_size_empty_dir_is_zero() {
    let root = TempDir::new().unwrap();
    write_dir(root.path(), "empty");
    let size = project_size_bytes(&root.path().join("empty"), &SkipPolicy::default_scan(), 6)
        .expect("size must succeed");
    assert_eq!(size, 0);
}

#[test]
fn stack_size_missing_path_is_path_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("nope");
    let err = project_size_bytes(&missing, &SkipPolicy::default_scan(), 6).unwrap_err();
    assert!(
        matches!(err, Error::PathNotFound { .. }),
        "missing path must map to PathNotFound: {err}"
    );
}

#[test]
fn stack_size_file_path_is_not_a_directory() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a.txt");
    fs::write(&file, "not a dir").unwrap();
    let err = project_size_bytes(&file, &SkipPolicy::default_scan(), 6).unwrap_err();
    assert!(
        matches!(err, Error::NotADirectory { .. }),
        "a file path must map to NotADirectory: {err}"
    );
}

// --- Case 9: determinism ------------------------------------------------------

#[test]
fn stack_from_candidate_is_deterministic() {
    let markers = vec![
        hit("Cargo.toml", MarkerKind::FileName, Some("Rust")),
        hit("package.json", MarkerKind::FileName, Some("JavaScript")),
    ];
    let cand = candidate(PathBuf::from("/tmp/poly"), "poly", markers, false);

    let a = stack_from_candidate(&cand);
    let b = stack_from_candidate(&cand);
    assert_eq!(a, b, "same input must produce the same Stack");
    assert_eq!(a.markers, b.markers, "marker ordering must be stable");
}

#[test]
fn detect_stack_is_deterministic() {
    let root = TempDir::new().unwrap();
    write(root.path(), "web/package.json");
    write(root.path(), "web/next.config.js");

    let markers = vec![hit(
        "package.json",
        MarkerKind::FileName,
        Some("JavaScript"),
    )];
    let path = root.path().join("web");
    let a = detect_stack(&path, &markers).expect("detect must succeed");
    let b = detect_stack(&path, &markers).expect("detect must succeed");
    assert_eq!(a, b, "same fixture must produce the same Stack twice");
    assert_eq!(
        a.frameworks, b.frameworks,
        "framework ordering must be stable"
    );
}

// --- Case 10: framework hints for other ecosystems ----------------------------

#[test]
fn detect_framework_vite_by_file_name() {
    let root = TempDir::new().unwrap();
    write(root.path(), "app/vite.config.ts");

    let stack = detect_stack(&root.path().join("app"), &[]).expect("detect must succeed");
    assert!(
        stack.frameworks.contains(&"Vite".to_string()),
        "vite.config.ts must map to Vite, got {:?}",
        stack.frameworks
    );
}

#[test]
fn detect_framework_nuxt_by_file_name() {
    let root = TempDir::new().unwrap();
    write(root.path(), "app/nuxt.config.ts");

    let stack = detect_stack(&root.path().join("app"), &[]).expect("detect must succeed");
    assert!(
        stack.frameworks.contains(&"Nuxt".to_string()),
        "nuxt.config.ts must map to Nuxt, got {:?}",
        stack.frameworks
    );
}

#[test]
fn detect_framework_angular_by_file_name() {
    let root = TempDir::new().unwrap();
    write(root.path(), "app/angular.json");

    let stack = detect_stack(&root.path().join("app"), &[]).expect("detect must succeed");
    assert!(
        stack.frameworks.contains(&"Angular".to_string()),
        "angular.json must map to Angular, got {:?}",
        stack.frameworks
    );
}

#[test]
fn detect_framework_deno_by_file_name() {
    let root = TempDir::new().unwrap();
    write(root.path(), "app/deno.json");

    let stack = detect_stack(&root.path().join("app"), &[]).expect("detect must succeed");
    assert!(
        stack.frameworks.contains(&"Deno".to_string()),
        "deno.json must map to Deno, got {:?}",
        stack.frameworks
    );
}

#[test]
fn detect_framework_django_by_manage_py() {
    let root = TempDir::new().unwrap();
    write(root.path(), "django/manage.py");

    let stack = detect_stack(&root.path().join("django"), &[]).expect("detect must succeed");
    assert!(
        stack.frameworks.contains(&"Django".to_string()),
        "manage.py must map to Django, got {:?}",
        stack.frameworks
    );
}

#[test]
fn detect_framework_rails_by_gemfile_and_application_rb() {
    let root = TempDir::new().unwrap();
    write(root.path(), "rails/Gemfile");
    write(root.path(), "rails/config/application.rb");

    let stack = detect_stack(
        &root.path().join("rails"),
        &[hit("Gemfile", MarkerKind::FileName, Some("Ruby"))],
    )
    .expect("detect must succeed");
    assert!(
        stack.frameworks.contains(&"Rails".to_string()),
        "Gemfile + config/application.rb must map to Rails, got {:?}",
        stack.frameworks
    );
    assert_eq!(stack.language.as_deref(), Some("Ruby"));
}

#[test]
fn detect_framework_sbt_by_build_sbt() {
    let root = TempDir::new().unwrap();
    write(root.path(), "scala/build.sbt");

    let stack = detect_stack(&root.path().join("scala"), &[]).expect("detect must succeed");
    assert!(
        stack
            .frameworks
            .iter()
            .any(|f| f.to_lowercase().contains("sbt")),
        "build.sbt must map to a Scala/sbt hint, got {:?}",
        stack.frameworks
    );
}

// --- Case 11: detect_stacks batch + fail-fast ---------------------------------

#[test]
fn detect_stacks_batch_preserves_order_and_fills_stacks() {
    let root = TempDir::new().unwrap();
    write(root.path(), "b/Cargo.toml");
    write(root.path(), "a/go.mod");

    // Deliberately reversed vs path order to prove input order is preserved.
    let candidates = vec![
        single_candidate(root.path(), "a"),
        single_candidate(root.path(), "b"),
    ];

    let pairs = detect_stacks(&candidates).expect("batch detect must succeed");
    assert_eq!(pairs.len(), 2, "one pair per candidate");
    assert_eq!(
        pairs[0].0, candidates[0],
        "first pair must correspond to the first input candidate"
    );
    assert_eq!(
        pairs[1].0, candidates[1],
        "second pair must correspond to the second input candidate"
    );
    assert_eq!(pairs[0].1.language.as_deref(), Some("Go"));
    assert_eq!(pairs[1].1.language.as_deref(), Some("Rust"));
}

#[test]
fn detect_stacks_fails_fast_on_missing_path() {
    let temp = TempDir::new().unwrap();
    let good = candidate(
        temp.path().join("ok"),
        "ok",
        vec![hit("Cargo.toml", MarkerKind::FileName, Some("Rust"))],
        false,
    );
    let bad = candidate(
        temp.path().join("missing"),
        "missing",
        vec![hit("go.mod", MarkerKind::FileName, Some("Go"))],
        false,
    );

    let err = detect_stacks(&[good, bad]).expect_err("missing path must fail the batch");
    assert!(
        matches!(err, Error::PathNotFound { .. }),
        "fail-fast must surface the missing path: {err}"
    );
}

#[test]
fn detect_stacks_fails_fast_on_file_path() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("f.txt");
    fs::write(&file, "x").unwrap();

    let good = candidate(
        temp.path().join("ok"),
        "ok",
        vec![hit("Cargo.toml", MarkerKind::FileName, Some("Rust"))],
        false,
    );
    let bad = candidate(file, "f", vec![], false);

    fs::create_dir_all(temp.path().join("ok")).expect("create ok dir");

    let err = detect_stacks(&[good, bad]).expect_err("a file path must fail the batch");
    assert!(
        matches!(err, Error::NotADirectory { .. }),
        "fail-fast must surface the non-directory: {err}"
    );
}

// --- Case 12: candidate_to_project -------------------------------------------

#[test]
fn detect_candidate_to_project_copies_fields() {
    let cand = candidate(
        PathBuf::from("/tmp/proj"),
        "proj",
        vec![hit("Cargo.toml", MarkerKind::FileName, Some("Rust"))],
        true,
    );
    let stack = stack_from_candidate(&cand);

    let project = candidate_to_project(cand.clone(), stack.clone(), 42);

    assert_eq!(project.path, cand.path);
    assert_eq!(project.name, cand.name);
    assert_eq!(project.stack, stack);
    assert_eq!(project.size_bytes, 42);
    assert_eq!(
        project.is_git_repo, cand.is_git_repo,
        "is_git_repo must carry over"
    );
}

// --- Case 13: Makefile (no hint) → no language, marker kept -------------------

#[test]
fn detect_makefile_only_has_no_language_but_keeps_marker() {
    let cand = candidate(
        PathBuf::from("/tmp/c"),
        "c",
        vec![hit("Makefile", MarkerKind::FileName, None)],
        false,
    );
    let stack = stack_from_candidate(&cand);
    assert_eq!(stack.language, None, "Makefile must not imply a language");
    assert!(stack.markers.contains(&"Makefile".to_string()));
    assert!(stack.frameworks.is_empty());
}

#[test]
fn detect_makefile_does_not_override_higher_priority_marker() {
    let cand = candidate(
        PathBuf::from("/tmp/mixed"),
        "mixed",
        vec![
            hit("Cargo.toml", MarkerKind::FileName, Some("Rust")),
            hit("Makefile", MarkerKind::FileName, None),
        ],
        false,
    );
    let stack = stack_from_candidate(&cand);
    assert_eq!(stack.language.as_deref(), Some("Rust"));
    assert!(stack.markers.contains(&"Makefile".to_string()));
}

// --- detect_stack error mapping -----------------------------------------------

#[test]
fn detect_stack_missing_path_is_path_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("nope");
    let err = detect_stack(&missing, &[]).unwrap_err();
    assert!(
        matches!(err, Error::PathNotFound { .. }),
        "missing path must map to PathNotFound: {err}"
    );
}

#[test]
fn detect_stack_file_path_is_not_a_directory() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a.txt");
    fs::write(&file, "x").unwrap();
    let err = detect_stack(&file, &[]).unwrap_err();
    assert!(
        matches!(err, Error::NotADirectory { .. }),
        "a file path must map to NotADirectory: {err}"
    );
}

// --- End-to-end: find_candidates → stack_from_candidate ------------------------

#[test]
fn detect_stack_end_to_end_from_discovered_candidate() {
    let root = TempDir::new().unwrap();
    write(root.path(), "app-rust/Cargo.toml");
    write(root.path(), "app-node/package.json");
    write(root.path(), "nested/deep/go.mod");

    let cands = find_candidates(root.path(), &CandidateOptions::default()).unwrap();
    let mut stacks: Vec<(String, Option<String>)> = cands
        .iter()
        .map(|c| {
            let stack = stack_from_candidate(c);
            (c.name.clone(), stack.language)
        })
        .collect();
    stacks.sort();

    assert_eq!(
        stacks,
        vec![
            ("app-node".to_string(), Some("JavaScript".to_string())),
            ("app-rust".to_string(), Some("Rust".to_string())),
            ("deep".to_string(), Some("Go".to_string())),
        ]
    );
}
