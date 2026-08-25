use std::path::PathBuf;

use super::*;
use crate::domain::Stack;
use crate::scan::MarkerKind;

/// Shared marker-hit fixture (also used by [`super::workspace_tests`]).
pub(super) fn hit(name: &str, hint: Option<&str>) -> MarkerHit {
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

    let stack = detect_stack(&proj, &[hit("package.json", Some("JavaScript"))]).expect("detect ok");
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
