//! Detect languages and frameworks from marker hits and shallow file names.
//!
//! Entry points: [`stack_from_candidate`] (pure, marker-based),
//! [`detect_stack`] (path + markers, enriches frameworks from the top level of
//! the project directory), [`detect_stacks`] (fail-fast batch),
//! [`candidate_to_project`] (assembly helper for M2.3) and
//! [`crate::scan::project_size_bytes`].
//!
//! # Merge policy
//!
//! Detectors are organized per ecosystem — one `*.rs` per language — and this
//! module is the only place that enumerates them ([`detector_registry()`]) and
//! merges their contributions into a single [`Stack`]:
//!
//! - **language** — resolved centrally from marker `language_hint`s by the §4.1
//!   priority table (`types::resolve_language`); detector-provided languages
//!   are ignored, so detectors leave `language` unset.
//! - **frameworks** — union of the `frameworks` returned by the applicable
//!   detectors, in registry order, deduplicated (first occurrence wins).
//! - **markers** — the names of every hit, sorted lexically and deduplicated.
//!
//! A detector applies when it matches the hits (one of its ecosystem markers is
//! present). When `markers` is empty — the path-only [`detect_stack`] case the
//! spec describes as "markers ещё не собраны" — every detector probes the
//! directory so framework files are still found.
//!
//! # Errors
//!
//! [`detect_stack`] / [`detect_stacks`] validate the path first
//! ([`Error::PathNotFound`], [`Error::NotADirectory`]); shallow `read_dir`
//! failures surface as [`Error::Io`]. [`stack_from_candidate`] never touches
//! the filesystem.
//!
//! # Determinism
//!
//! Registry order is fixed, detector framework order is fixed, directory names
//! are sorted before matching and `markers` are sorted before deduplication, so
//! equal inputs always produce equal [`Stack`]s regardless of `read_dir` order.

use std::path::Path;

use crate::domain::{Error, Project, Result, Stack};
use crate::scan::{MarkerHit, ProjectCandidate};

mod cpp;
mod git;
mod go;
mod jvm;
mod make;
mod node;
mod php;
mod python;
mod ruby;
mod rust;
mod traits;
mod types;

pub use traits::StackDetector;

use cpp::CppDetector;
use git::GitDetector;
use go::GoDetector;
use jvm::JvmDetector;
use make::MakeDetector;
use node::NodeDetector;
use php::PhpDetector;
use python::PythonDetector;
use ruby::RubyDetector;
use rust::RustDetector;
use types::resolve_language;

/// Stable order registry (ecosystem modules).
///
/// Order is fixed (rust, node, go, python, jvm, ruby, php, cpp, make, git) and
/// mirrors the marker registry order, so the framework union order is
/// deterministic.
pub fn detector_registry() -> &'static [&'static dyn StackDetector] {
    &[
        &RustDetector,
        &NodeDetector,
        &GoDetector,
        &PythonDetector,
        &JvmDetector,
        &RubyDetector,
        &PhpDetector,
        &CppDetector,
        &MakeDetector,
        &GitDetector,
    ]
}

/// Build a [`Stack`] from an already-discovered candidate.
///
/// PURE: never touches the filesystem. Language is resolved from the marker
/// `language_hint`s by the §4.1 priority table, `markers` are the sorted unique
/// hit names, and `frameworks` stay empty — enrichment requires reading the
/// project directory, which only [`detect_stack`] does.
pub fn stack_from_candidate(candidate: &ProjectCandidate) -> Stack {
    let language = resolve_language(&candidate.markers);
    let markers = sorted_unique_marker_names(&candidate.markers);
    Stack {
        language,
        frameworks: Vec::new(),
        markers,
    }
}

/// Detect a [`Stack`] for a project directory given its marker hits.
///
/// `path` must be an existing directory; a missing path maps to
/// [`Error::PathNotFound`] and a non-directory to [`Error::NotADirectory`].
/// Framework hints are collected from the top-level file names (see the module
/// merge policy); with an empty `markers` slice every detector probes the
/// directory. Shallow-read IO failures surface as [`Error::Io`].
pub fn detect_stack(path: &Path, markers: &[MarkerHit]) -> Result<Stack> {
    if !path.exists() {
        return Err(Error::PathNotFound {
            path: path.to_path_buf(),
        });
    }
    if !path.is_dir() {
        return Err(Error::NotADirectory {
            path: path.to_path_buf(),
        });
    }

    let probe_all = markers.is_empty();
    let mut frameworks: Vec<String> = Vec::new();
    for detector in detector_registry() {
        if !probe_all && !detector.matches(markers) {
            continue;
        }
        let contribution = detector.detect(markers, path)?;
        for framework in contribution.frameworks {
            if !frameworks.contains(&framework) {
                frameworks.push(framework);
            }
        }
    }

    let language = resolve_language(markers);
    let markers = sorted_unique_marker_names(markers);
    Ok(Stack {
        language,
        frameworks,
        markers,
    })
}

/// Detect stacks for every candidate.
///
/// Fail-fast: the first failing candidate path aborts the whole batch. Input
/// order is preserved in the returned pairs.
pub fn detect_stacks(candidates: &[ProjectCandidate]) -> Result<Vec<(ProjectCandidate, Stack)>> {
    candidates
        .iter()
        .map(|candidate| {
            let stack = detect_stack(&candidate.path, &candidate.markers)?;
            Ok((candidate.clone(), stack))
        })
        .collect()
}

/// Assemble a [`Project`] from a candidate, its [`Stack`] and its size.
pub fn candidate_to_project(candidate: ProjectCandidate, stack: Stack, size_bytes: u64) -> Project {
    Project {
        path: candidate.path,
        name: candidate.name,
        stack,
        size_bytes,
        is_git_repo: candidate.is_git_repo,
    }
}

/// Names of every hit, sorted lexically and deduplicated.
fn sorted_unique_marker_names(markers: &[MarkerHit]) -> Vec<String> {
    let mut names: Vec<String> = markers.iter().map(|hit| hit.name.clone()).collect();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::domain::Stack;
    use crate::scan::MarkerKind;

    fn hit(name: &str, hint: Option<&str>) -> MarkerHit {
        MarkerHit {
            name: name.to_string(),
            kind: MarkerKind::FileName,
            language_hint: hint.map(str::to_string),
        }
    }

    fn candidate_with(hits: Vec<MarkerHit>) -> ProjectCandidate {
        ProjectCandidate {
            path: PathBuf::from("/tmp/fixture"),
            name: "fixture".to_string(),
            markers: hits,
            is_git_repo: false,
        }
    }

    #[test]
    fn cargo_only_is_rust() {
        let stack = stack_from_candidate(&candidate_with(vec![hit("Cargo.toml", Some("Rust"))]));
        assert_eq!(stack.language.as_deref(), Some("Rust"));
        assert_eq!(stack.markers, vec!["Cargo.toml".to_string()]);
        assert!(stack.frameworks.is_empty());
    }

    #[test]
    fn go_only_is_go() {
        let stack = stack_from_candidate(&candidate_with(vec![hit("go.mod", Some("Go"))]));
        assert_eq!(stack.language.as_deref(), Some("Go"));
    }

    #[test]
    fn conflict_cargo_wins_over_package_json_and_keeps_both_markers() {
        let stack = stack_from_candidate(&candidate_with(vec![
            hit("Cargo.toml", Some("Rust")),
            hit("package.json", Some("JavaScript")),
        ]));
        assert_eq!(stack.language.as_deref(), Some("Rust"));
        assert_eq!(
            stack.markers,
            vec!["Cargo.toml".to_string(), "package.json".to_string()]
        );
    }

    #[test]
    fn priority_table_orders_are_respected() {
        assert_eq!(
            stack_from_candidate(&candidate_with(vec![
                hit("Gemfile", Some("Ruby")),
                hit("requirements.txt", Some("Python")),
            ]))
            .language
            .as_deref(),
            Some("Python")
        );
        assert_eq!(
            stack_from_candidate(&candidate_with(vec![
                hit("package.json", Some("JavaScript")),
                hit("go.mod", Some("Go")),
            ]))
            .language
            .as_deref(),
            Some("Go")
        );
    }

    #[test]
    fn git_only_has_no_language_but_keeps_marker() {
        let stack = stack_from_candidate(&candidate_with(vec![MarkerHit {
            name: ".git".to_string(),
            kind: MarkerKind::DirName,
            language_hint: None,
        }]));
        assert_eq!(stack.language, None);
        assert_eq!(stack.markers, vec![".git".to_string()]);
        assert!(stack.frameworks.is_empty());
    }

    #[test]
    fn makefile_only_has_no_language_but_keeps_marker() {
        let stack = stack_from_candidate(&candidate_with(vec![hit("Makefile", None)]));
        assert_eq!(stack.language, None);
        assert_eq!(stack.markers, vec!["Makefile".to_string()]);
    }

    #[test]
    fn makefile_does_not_override_higher_priority_marker() {
        let stack = stack_from_candidate(&candidate_with(vec![
            hit("Cargo.toml", Some("Rust")),
            hit("Makefile", None),
        ]));
        assert_eq!(stack.language.as_deref(), Some("Rust"));
        assert_eq!(
            stack.markers,
            vec!["Cargo.toml".to_string(), "Makefile".to_string()]
        );
    }

    #[test]
    fn extra_marker_hint_is_used_when_nothing_is_in_priority_table() {
        let stack = stack_from_candidate(&candidate_with(vec![hit("Project.toml", Some("Julia"))]));
        assert_eq!(stack.language.as_deref(), Some("Julia"));
    }

    #[test]
    fn stack_from_candidate_is_pure() {
        let missing = PathBuf::from("/definitely/not/a/real/path");
        let stack = stack_from_candidate(&candidate_with(vec![hit("Cargo.toml", Some("Rust"))]));
        assert_eq!(stack.language.as_deref(), Some("Rust"));
        assert!(!missing.exists());
        assert!(stack.frameworks.is_empty());
    }

    #[test]
    fn stack_from_candidate_is_deterministic() {
        let cand = candidate_with(vec![
            hit("Cargo.toml", Some("Rust")),
            hit("package.json", Some("JavaScript")),
        ]);
        assert_eq!(stack_from_candidate(&cand), stack_from_candidate(&cand));
    }

    #[test]
    fn empty_stack_is_language_none() {
        let stack = stack_from_candidate(&candidate_with(Vec::new()));
        assert_eq!(stack, Stack::default());
    }

    #[test]
    fn detect_stack_enriches_nextjs_framework() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("web");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("package.json"), "{}").unwrap();
        std::fs::write(proj.join("next.config.mjs"), "").unwrap();

        let stack =
            detect_stack(&proj, &[hit("package.json", Some("JavaScript"))]).expect("detect ok");
        assert_eq!(stack.language.as_deref(), Some("JavaScript"));
        assert!(stack.frameworks.contains(&"Next.js".to_string()));
        assert_eq!(stack.markers, vec!["package.json".to_string()]);
    }

    #[test]
    fn detect_stack_probes_frameworks_without_markers() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("vite.config.ts"), "").unwrap();
        std::fs::write(app.join("deno.json"), "{}").unwrap();

        let stack = detect_stack(&app, &[]).expect("detect ok");
        assert!(stack.frameworks.contains(&"Vite".to_string()));
        assert!(stack.frameworks.contains(&"Deno".to_string()));
        assert_eq!(stack.language, None);
    }

    #[test]
    fn detect_stack_rails_requires_gemfile_and_application_rb() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("rails");
        std::fs::create_dir_all(app.join("config")).unwrap();
        std::fs::write(app.join("Gemfile"), "").unwrap();
        std::fs::write(app.join("config/application.rb"), "").unwrap();

        let stack = detect_stack(&app, &[hit("Gemfile", Some("Ruby"))]).expect("detect ok");
        assert_eq!(stack.language.as_deref(), Some("Ruby"));
        assert!(stack.frameworks.contains(&"Rails".to_string()));
    }

    #[test]
    fn detect_stack_path_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        assert!(matches!(
            detect_stack(&missing, &[]),
            Err(Error::PathNotFound { .. })
        ));

        let file = dir.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        assert!(matches!(
            detect_stack(&file, &[]),
            Err(Error::NotADirectory { .. })
        ));
    }

    #[test]
    fn detect_stack_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let proj = dir.path().join("web");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("package.json"), "{}").unwrap();
        std::fs::write(proj.join("next.config.js"), "").unwrap();

        let markers = vec![hit("package.json", Some("JavaScript"))];
        let a = detect_stack(&proj, &markers).expect("detect ok");
        let b = detect_stack(&proj, &markers).expect("detect ok");
        assert_eq!(a, b);
        assert_eq!(a.frameworks, b.frameworks);
    }

    #[test]
    fn detect_stacks_preserves_input_order() {
        let dir = tempfile::tempdir().unwrap();
        let rust = dir.path().join("b");
        let go = dir.path().join("a");
        std::fs::create_dir_all(&rust).unwrap();
        std::fs::create_dir_all(&go).unwrap();
        std::fs::write(rust.join("Cargo.toml"), "").unwrap();
        std::fs::write(go.join("go.mod"), "").unwrap();

        let a = ProjectCandidate {
            path: go,
            name: "a".to_string(),
            markers: vec![hit("go.mod", Some("Go"))],
            is_git_repo: false,
        };
        let b = ProjectCandidate {
            path: rust,
            name: "b".to_string(),
            markers: vec![hit("Cargo.toml", Some("Rust"))],
            is_git_repo: false,
        };

        let pairs = detect_stacks(&[a.clone(), b.clone()]).expect("batch ok");
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, a);
        assert_eq!(pairs[1].0, b);
        assert_eq!(pairs[0].1.language.as_deref(), Some("Go"));
        assert_eq!(pairs[1].1.language.as_deref(), Some("Rust"));
    }

    #[test]
    fn candidate_to_project_copies_fields() {
        let cand = candidate_with(vec![hit("Cargo.toml", Some("Rust"))]);
        let stack = stack_from_candidate(&cand);
        let project = candidate_to_project(cand.clone(), stack.clone(), 42);
        assert_eq!(project.path, cand.path);
        assert_eq!(project.name, cand.name);
        assert_eq!(project.stack, stack);
        assert_eq!(project.size_bytes, 42);
        assert_eq!(project.is_git_repo, cand.is_git_repo);
    }
}
