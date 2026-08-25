use std::path::PathBuf;

use raccpack_core::detect::{clamp_confidence, Detection, StackNode};
use raccpack_core::domain::{Project, ScanReport, Stack};

fn detection_at(scope: &str) -> Detection {
    Detection {
        ecosystem: "node".to_string(),
        language: Some("TypeScript".to_string()),
        frameworks: vec!["Next.js".to_string()],
        confidence: 0.9,
        scope: PathBuf::from(scope),
        markers: vec!["package.json".to_string()],
    }
}

fn project_with(stack_tree: Option<StackNode>) -> Project {
    Project {
        path: PathBuf::from("/tmp/dto-fixture"),
        name: "dto-fixture".to_string(),
        stack: Stack {
            language: Some("Rust".to_string()),
            frameworks: vec!["Axum".to_string()],
            markers: vec!["Cargo.toml".to_string()],
        },
        size_bytes: 4096,
        is_git_repo: true,
        stack_tree,
    }
}

#[test]
fn detection_json_roundtrip_preserves_all_fields() {
    let detection = Detection {
        ecosystem: "node".to_string(),
        language: Some("TypeScript".to_string()),
        frameworks: vec!["Next.js".to_string(), "Vite".to_string()],
        confidence: 0.85,
        scope: PathBuf::from("/tmp/проекты/приложение"),
        markers: vec!["next.config.mjs".to_string(), "package.json".to_string()],
    };

    let json = serde_json::to_string(&detection).expect("serialize");
    let back: Detection = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(detection.ecosystem, back.ecosystem);
    assert_eq!(detection.language, back.language);
    assert_eq!(detection.frameworks, back.frameworks);
    assert_eq!(detection.confidence, back.confidence);
    assert_eq!(detection.scope, back.scope);
    assert_eq!(detection.markers, back.markers);
    assert_eq!(detection, back);
    assert!(back.scope.ends_with("приложение"));
}

#[test]
fn stack_node_roundtrip_with_nested_children() {
    let grandchild = StackNode {
        detection: detection_at("/tmp/app/web/admin"),
        children: Vec::new(),
    };
    let child = StackNode {
        detection: detection_at("/tmp/app/web"),
        children: vec![grandchild],
    };
    let root = StackNode {
        detection: detection_at("/tmp/app"),
        children: vec![
            child,
            StackNode {
                detection: detection_at("/tmp/app/services/api"),
                children: Vec::new(),
            },
        ],
    };

    let json = serde_json::to_string(&root).expect("serialize");
    let back: StackNode = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(root, back);
    assert_eq!(
        back.detection.scope,
        PathBuf::from("/tmp/app"),
        "root scope preserved"
    );
    assert_eq!(back.children.len(), 2, "root has two children in order");
    assert_eq!(
        back.children[0].detection.scope,
        PathBuf::from("/tmp/app/web"),
        "first child stays first"
    );
    assert_eq!(back.children[0].children.len(), 1, "third level reached");
    assert_eq!(
        back.children[0].children[0].detection.scope,
        PathBuf::from("/tmp/app/web/admin"),
        "grandchild survives nesting"
    );
    assert_eq!(
        back.children[1].detection.scope,
        PathBuf::from("/tmp/app/services/api"),
        "second child stays second"
    );
    assert!(back.children[1].children.is_empty());
}

#[test]
fn project_without_stack_tree_field_deserializes_to_none() {
    let legacy = r#"{
        "path": "/tmp/legacy",
        "name": "legacy",
        "stack": {"language": "Go", "frameworks": [], "markers": ["go.mod"]},
        "size_bytes": 128,
        "is_git_repo": false
    }"#;

    let project: Project = serde_json::from_str(legacy).expect("legacy json parses");

    assert_eq!(project.path, PathBuf::from("/tmp/legacy"));
    assert_eq!(project.name, "legacy");
    assert_eq!(project.stack.language.as_deref(), Some("Go"));
    assert!(
        project.stack_tree.is_none(),
        "missing stack_tree key must fall back to None"
    );
}

#[test]
fn project_serializes_stack_tree_null_and_keeps_flat_stack() {
    let project = project_with(None);

    let value = serde_json::to_value(&project).expect("serialize");

    assert_eq!(value["stack_tree"], serde_json::Value::Null);
    assert_eq!(value["stack"]["language"], serde_json::json!("Rust"));
    assert_eq!(value["stack"]["frameworks"][0], serde_json::json!("Axum"));
    assert_eq!(
        value["stack"]["markers"][0],
        serde_json::json!("Cargo.toml")
    );
    assert_eq!(value["name"], serde_json::json!("dto-fixture"));
}

/// D2.3 serde compat at the report level: a pre-`stack_tree` report (as old
/// scripts and cache files may contain) still deserializes, the schema version
/// stays 1 and re-serialization emits `null` for the additive field.
#[test]
fn scan_report_legacy_json_without_stack_tree_deserializes_with_schema_version() {
    let legacy = r#"{
        "root": "/tmp/legacy-root",
        "projects": [
            {
                "path": "/tmp/legacy-root/app",
                "name": "app",
                "stack": {"language": "Go", "frameworks": [], "markers": ["go.mod"]},
                "size_bytes": 512,
                "is_git_repo": true
            }
        ],
        "total_size_bytes": 512,
        "schema_version": 1
    }"#;

    let report: ScanReport = serde_json::from_str(legacy).expect("legacy report json parses");

    assert_eq!(
        report.schema_version, 1,
        "additive stack_tree must not bump schema_version"
    );
    assert_eq!(report.projects.len(), 1);
    let project = &report.projects[0];
    assert!(
        project.stack_tree.is_none(),
        "absent stack_tree key must default to None"
    );
    assert_eq!(project.stack.language.as_deref(), Some("Go"));
    assert_eq!(project.stack.markers, vec!["go.mod".to_string()]);

    let value = serde_json::to_value(&report).expect("reserialize");
    assert_eq!(
        value["projects"][0]["stack_tree"],
        serde_json::Value::Null,
        "the additive field must serialize as null, never be omitted"
    );
}

#[test]
fn project_with_some_stack_tree_serializes_and_roundtrips() {
    let tree = StackNode {
        detection: detection_at("/tmp/dto-fixture/tools"),
        children: vec![StackNode {
            detection: detection_at("/tmp/dto-fixture/tools/cli"),
            children: Vec::new(),
        }],
    };
    let project = project_with(Some(tree));

    let json = serde_json::to_string(&project).expect("serialize");
    let back: Project = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(project, back);
    assert!(back.stack_tree.is_some());
    assert_eq!(
        back.stack_tree.expect("some tree").detection.scope,
        PathBuf::from("/tmp/dto-fixture/tools")
    );
}

#[test]
fn clamp_confidence_bounds() {
    assert_eq!(clamp_confidence(1.5), 1.0);
    assert_eq!(clamp_confidence(-0.2), 0.0);
    assert_eq!(clamp_confidence(0.7), 0.7);
    assert_eq!(clamp_confidence(0.0), 0.0);
    assert_eq!(clamp_confidence(1.0), 1.0);

    assert_eq!(clamp_confidence(f32::NAN), 0.0);
    assert_eq!(clamp_confidence(f32::INFINITY), 0.0);
    assert_eq!(clamp_confidence(f32::NEG_INFINITY), 0.0);
}

#[test]
fn confidence_is_f32_in_json() {
    let mut detection = detection_at("/tmp/jvm-app");
    detection.confidence = 0.75;

    let json = serde_json::to_string(&detection).expect("serialize");
    assert!(
        json.contains("\"confidence\":0.75"),
        "confidence must be a bare JSON number, got: {json}"
    );

    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(value["confidence"].is_number());
    assert_eq!(value["confidence"], serde_json::json!(0.75));

    let back: Detection = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.confidence, 0.75);
}
