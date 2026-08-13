//! Invariant tests for M2.1 follow-up — the markers registry.
//!
//! The Dev subagent splits `scan/markers.rs` into per-language modules and
//! exposes a registry function `default_markers()` that must be an exact,
//! order-preserving copy of the pre-split `DEFAULT_MARKERS` table. These tests
//! lock the ground-truth table (14 entries, fixed order, stable kinds and
//! hints) so the mechanical refactor cannot silently drop, reorder, or rename
//! a marker.
//!
//! No fixtures, no network, no git: the tests only read the static registry.

use raccpack_core::scan::{default_markers, MarkerDef, MarkerKind};

/// Fold a `MarkerDef` into a comparable `(kind, name, language_hint)` triple.
fn triple(m: &MarkerDef) -> (MarkerKind, &str, Option<&str>) {
    (m.kind, m.name, m.language_hint)
}

/// The ground-truth marker table in its effective order (M2.1, pre-split).
fn expected_table() -> Vec<(MarkerKind, &'static str, Option<&'static str>)> {
    vec![
        (MarkerKind::FileName, "Cargo.toml", Some("Rust")),
        (MarkerKind::FileName, "package.json", Some("JavaScript")),
        (MarkerKind::FileName, "go.mod", Some("Go")),
        (MarkerKind::FileName, "pyproject.toml", Some("Python")),
        (MarkerKind::FileName, "setup.py", Some("Python")),
        (MarkerKind::FileName, "requirements.txt", Some("Python")),
        (MarkerKind::FileName, "pom.xml", Some("Java")),
        (MarkerKind::FileName, "build.gradle", Some("Java")),
        (MarkerKind::FileName, "build.gradle.kts", Some("Kotlin")),
        (MarkerKind::FileName, "Gemfile", Some("Ruby")),
        (MarkerKind::FileName, "composer.json", Some("PHP")),
        (MarkerKind::FileName, "CMakeLists.txt", Some("C++")),
        (MarkerKind::FileName, "Makefile", None),
        (MarkerKind::DirName, ".git", None),
    ]
}

#[test]
fn default_markers_is_complete_and_ordered() {
    let registry = default_markers();
    assert_eq!(
        registry.len(),
        14,
        "the registry must contain exactly the 14 pre-split markers"
    );

    let actual: Vec<_> = registry.iter().map(triple).collect();
    let expected = expected_table();
    assert_eq!(
        actual, expected,
        "the registry must equal the pre-split table, order included"
    );
}

#[test]
fn default_markers_kinds_and_hints_are_stable() {
    let registry = default_markers();

    for (i, marker) in registry.iter().enumerate() {
        if marker.name == ".git" {
            assert_eq!(
                marker.kind,
                MarkerKind::DirName,
                "`.git` at index {i} must keep its DirName kind"
            );
        } else {
            assert_eq!(
                marker.kind,
                MarkerKind::FileName,
                "marker `{}` at index {i} must keep its FileName kind",
                marker.name
            );
        }
    }

    let by_name = |name: &str| {
        registry
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("registry missing marker `{name}`"))
    };

    assert_eq!(by_name("Cargo.toml").language_hint, Some("Rust"));
    assert_eq!(by_name("package.json").language_hint, Some("JavaScript"));
    assert_eq!(by_name("go.mod").language_hint, Some("Go"));
    assert_eq!(by_name("pyproject.toml").language_hint, Some("Python"));
    assert_eq!(by_name("setup.py").language_hint, Some("Python"));
    assert_eq!(by_name("requirements.txt").language_hint, Some("Python"));
    assert_eq!(by_name("pom.xml").language_hint, Some("Java"));
    assert_eq!(by_name("build.gradle").language_hint, Some("Java"));
    assert_eq!(by_name("build.gradle.kts").language_hint, Some("Kotlin"));
    assert_eq!(by_name("Gemfile").language_hint, Some("Ruby"));
    assert_eq!(by_name("composer.json").language_hint, Some("PHP"));
    assert_eq!(by_name("CMakeLists.txt").language_hint, Some("C++"));
    assert_eq!(by_name("Makefile").language_hint, None);
    assert_eq!(by_name(".git").language_hint, None);
}

#[test]
fn default_markers_has_no_duplicates() {
    let registry = default_markers();

    let mut names: Vec<&str> = registry.iter().map(|m| m.name).collect();
    names.sort();
    let mut deduped = names.clone();
    deduped.dedup();
    assert_eq!(
        names, deduped,
        "marker names must be unique across the registry"
    );

    let mut pairs: Vec<(MarkerKind, &str)> = registry.iter().map(|m| (m.kind, m.name)).collect();
    pairs.sort_by_key(|(_, name)| *name);
    pairs.dedup();
    assert_eq!(
        registry.len(),
        pairs.len(),
        "(kind, name) pairs must be unique across the registry"
    );
}

#[test]
fn default_markers_is_deterministic_across_calls() {
    let a = default_markers();
    let b = default_markers();

    assert_eq!(a.len(), b.len(), "two calls must agree on length");
    assert!(
        a.iter().zip(b.iter()).all(|(x, y)| triple(x) == triple(y)),
        "two calls must agree on every (kind, name, hint) in order"
    );
}
