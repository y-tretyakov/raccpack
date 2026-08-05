//! Integration tests for the M1.2 domain DTO types.
//!
//! Covers `SensitiveRisk`, `Stack`, `Project`, `ScanReport` and `Error`
//! as specified in docs/mvp/m1/m1.2-domain-dto.md.

use std::cmp::Ordering;
use std::error::Error as StdError;
use std::path::PathBuf;

use raccpack_core::{Error, Project, Result, ScanReport, SensitiveRisk, Stack};

fn all_risks() -> Vec<SensitiveRisk> {
    vec![
        SensitiveRisk::Low,
        SensitiveRisk::Medium,
        SensitiveRisk::High,
        SensitiveRisk::Critical,
    ]
}

// --- Case 1: SensitiveRisk ordering -------------------------------------

#[test]
fn risk_ordering_partial_ord_chain() {
    assert!(SensitiveRisk::Low < SensitiveRisk::Medium);
    assert!(SensitiveRisk::Medium < SensitiveRisk::High);
    assert!(SensitiveRisk::High < SensitiveRisk::Critical);
    assert!(SensitiveRisk::Critical > SensitiveRisk::Low);
}

#[test]
fn risk_ordering_via_cmp_and_max() {
    assert_eq!(
        SensitiveRisk::Critical.cmp(&SensitiveRisk::High),
        Ordering::Greater
    );
    assert_eq!(
        SensitiveRisk::Low.cmp(&SensitiveRisk::Medium),
        Ordering::Less
    );
    assert_eq!(
        SensitiveRisk::Medium.max(SensitiveRisk::High),
        SensitiveRisk::High
    );
    assert_eq!(
        SensitiveRisk::Critical.min(SensitiveRisk::Low),
        SensitiveRisk::Low
    );
}

#[test]
fn risk_ordering_sort_is_ascending() {
    let mut levels = vec![
        SensitiveRisk::Critical,
        SensitiveRisk::Low,
        SensitiveRisk::High,
        SensitiveRisk::Medium,
    ];
    levels.sort();
    assert_eq!(
        levels,
        vec![
            SensitiveRisk::Low,
            SensitiveRisk::Medium,
            SensitiveRisk::High,
            SensitiveRisk::Critical,
        ]
    );
}

// --- Case 2: SensitiveRisk serde roundtrip --------------------------------

#[test]
fn risk_serde_roundtrip_every_variant() {
    for risk in all_risks() {
        let json = serde_json::to_string(&risk).unwrap();
        let back: SensitiveRisk = serde_json::from_str(&json).unwrap();
        assert_eq!(back, risk, "roundtrip failed for {risk:?}");
    }
}

#[test]
fn risk_serde_uses_pascal_case_names() {
    assert_eq!(
        serde_json::to_string(&SensitiveRisk::Low).unwrap(),
        "\"Low\""
    );
    assert_eq!(
        serde_json::to_string(&SensitiveRisk::Medium).unwrap(),
        "\"Medium\""
    );
    assert_eq!(
        serde_json::to_string(&SensitiveRisk::High).unwrap(),
        "\"High\""
    );
    assert_eq!(
        serde_json::to_string(&SensitiveRisk::Critical).unwrap(),
        "\"Critical\""
    );
}

#[test]
fn risk_serde_deserialize_pascal_case() {
    assert_eq!(
        serde_json::from_str::<SensitiveRisk>("\"Low\"").unwrap(),
        SensitiveRisk::Low
    );
    assert_eq!(
        serde_json::from_str::<SensitiveRisk>("\"Medium\"").unwrap(),
        SensitiveRisk::Medium
    );
    assert_eq!(
        serde_json::from_str::<SensitiveRisk>("\"High\"").unwrap(),
        SensitiveRisk::High
    );
    assert_eq!(
        serde_json::from_str::<SensitiveRisk>("\"Critical\"").unwrap(),
        SensitiveRisk::Critical
    );
}

// --- Case 3: as_str / from_str_ignore_case ---------------------------------

#[test]
fn risk_as_str_roundtrips() {
    for risk in all_risks() {
        assert_eq!(
            SensitiveRisk::from_str_ignore_case(risk.as_str()),
            Some(risk)
        );
    }
}

#[test]
fn risk_from_str_ignore_case_is_case_insensitive() {
    for input in ["critical", "CRITICAL", "Critical"] {
        assert_eq!(
            SensitiveRisk::from_str_ignore_case(input),
            Some(SensitiveRisk::Critical),
            "input {input:?} should map to Critical"
        );
    }
    assert_eq!(
        SensitiveRisk::from_str_ignore_case("HIGH"),
        Some(SensitiveRisk::High)
    );
    assert_eq!(
        SensitiveRisk::from_str_ignore_case("medium"),
        Some(SensitiveRisk::Medium)
    );
}

#[test]
fn risk_from_str_ignore_case_unknown_is_none() {
    assert_eq!(SensitiveRisk::from_str_ignore_case("unknown"), None);
    assert_eq!(SensitiveRisk::from_str_ignore_case(""), None);
    assert_eq!(SensitiveRisk::from_str_ignore_case("critical "), None);
}

// --- Case 4: Stack::default() ----------------------------------------------

#[test]
fn stack_default_is_empty() {
    let stack = Stack::default();
    assert_eq!(stack.language, None);
    assert!(stack.frameworks.is_empty());
    assert!(stack.markers.is_empty());
}

// --- Case 5: Project + ScanReport serde roundtrip --------------------------

fn sample_project() -> Project {
    Project {
        path: PathBuf::from("/tmp/projects/alpha"),
        name: "alpha".to_string(),
        stack: Stack {
            language: Some("Rust".to_string()),
            frameworks: vec!["Axum".to_string()],
            markers: vec!["Cargo.toml".to_string()],
        },
        size_bytes: 4096,
        is_git_repo: true,
    }
}

fn sample_report() -> ScanReport {
    ScanReport {
        root: PathBuf::from("/tmp/projects"),
        projects: vec![sample_project()],
        total_size_bytes: 4096,
        schema_version: 1,
    }
}

#[test]
fn project_serde_roundtrip() {
    let project = sample_project();
    let json = serde_json::to_string(&project).unwrap();
    let back: Project = serde_json::from_str(&json).unwrap();
    assert_eq!(project, back);
}

#[test]
fn project_serde_path_is_a_string_in_json() {
    let json = serde_json::to_string(&sample_project()).unwrap();
    assert!(json.contains("\"/tmp/projects/alpha\""));
}

#[test]
fn scan_report_serde_roundtrip() {
    let report = sample_report();
    let json = serde_json::to_string(&report).unwrap();
    let back: ScanReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report, back);
}

#[test]
fn scan_report_schema_version_is_one() {
    assert_eq!(sample_report().schema_version, 1);
    let json = serde_json::to_string(&sample_report()).unwrap();
    assert!(json.contains("\"schema_version\":1"));
}

#[test]
fn scan_report_with_multiple_projects_roundtrips() {
    let mut report = sample_report();
    report.projects.push(Project {
        path: PathBuf::from("/tmp/projects/beta"),
        name: "beta".to_string(),
        stack: Stack {
            language: None,
            frameworks: vec![],
            markers: vec![],
        },
        size_bytes: 128,
        is_git_repo: false,
    });
    report.total_size_bytes += 128;

    let json = serde_json::to_string(&report).unwrap();
    let back: ScanReport = serde_json::from_str(&json).unwrap();
    assert_eq!(report, back);
}

// --- Case 6: Error::suggestion() -------------------------------------------

#[test]
fn error_suggestion_some_for_path_problems() {
    let not_found = Error::PathNotFound {
        path: PathBuf::from("/missing"),
    };
    assert!(not_found.suggestion().is_some());

    let not_dir = Error::NotADirectory {
        path: PathBuf::from("/file"),
    };
    assert!(not_dir.suggestion().is_some());
}

#[test]
fn error_suggestion_none_for_other_variants() {
    assert_eq!(
        Error::Config {
            message: "bad config".to_string()
        }
        .suggestion(),
        None
    );
    assert_eq!(
        Error::Other {
            message: "boom".to_string()
        }
        .suggestion(),
        None
    );
    assert_eq!(
        Error::Io {
            path: PathBuf::from("/x"),
            source: std::io::Error::other("io"),
        }
        .suggestion(),
        None
    );
}

// --- Case 7: Error implements std::error::Error + Display ------------------

fn assert_error<E: std::error::Error>() {}

#[test]
fn error_implements_std_error() {
    assert_error::<Error>();
}

#[test]
fn error_displays_something_useful() {
    let err = Error::PathNotFound {
        path: PathBuf::from("/tmp/not-there"),
    };
    let text = err.to_string();
    assert!(!text.is_empty());
    assert!(
        text.contains("tmp"),
        "display should contain the path, got {text:?}"
    );
}

#[test]
fn error_io_has_source_chain() {
    let io_source = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let err = Error::Io {
        path: PathBuf::from("/x"),
        source: io_source,
    };
    let dyn_err: &dyn std::error::Error = &err;
    let source = dyn_err.source().expect("Io variant must expose a source");
    assert_eq!(
        source.downcast_ref::<std::io::Error>().unwrap().kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

#[test]
fn error_non_io_variants_have_no_source() {
    let err = Error::PathNotFound {
        path: PathBuf::from("/missing"),
    };
    assert!(StdError::source(&err).is_none());
    let err = Error::Config {
        message: "x".to_string(),
    };
    assert!(StdError::source(&err).is_none());
}

// --- Result alias ------------------------------------------------------------

#[test]
fn result_alias_resolves_to_error() {
    let ok: Result<u32> = Ok(42);
    assert!(matches!(ok, Ok(42)));
    let err: Result<u32> = Err(Error::PathNotFound {
        path: PathBuf::from("/nope"),
    });
    assert!(err.is_err());
}
