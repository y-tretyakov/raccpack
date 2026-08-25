//! Rinse-scope mapping for composite DAG mode (D3.1).
//!
//! [`scopes_for_rinse`] walks a [`StackNode`] tree and produces a flat
//! [`Vec<ScopeEntry>`] that the rinse phase iterates over — one entry per
//! detected scope path with the strategies that apply to it.

use super::types::StackNode;
use crate::clean::detect::ScopeEntry;
use crate::clean::strategy::StrategyId;

/// Map a [`StackNode`] tree to a flat list of [`ScopeEntry`]s for rinse.
///
/// Every scope (root and children) gets strategies mapped to its
/// `detection.ecosystem` from the `enabled` list. The `"generic"` strategy
/// applies to all scopes when enabled. Unknown or empty ecosystems get no
/// strategies (conservative: no cleanup without a known ecosystem match).
///
/// If `enabled` is empty the result is a single entry covering the root
/// with an empty strategy list (a deterministic no-op, same as
/// `find_trash_dirs` with an empty list).
///
/// Deterministic: tree traversal is depth-first in `children` order; the
/// enabled list is sorted to guarantee output order independence.
pub fn scopes_for_rinse(root: &StackNode, enabled: &[StrategyId]) -> Vec<ScopeEntry> {
    let mut entries = Vec::new();
    let mut sorted_enabled = enabled.to_vec();
    sorted_enabled.sort_by_key(|id| id.as_str());

    let root_strategies = resolve_ecosystem_strategies(&root.detection.ecosystem, &sorted_enabled);
    entries.push(ScopeEntry {
        path: root.detection.scope.clone(),
        strategies: root_strategies,
    });

    for child in &root.children {
        collect_scopes(child, &sorted_enabled, &mut entries);
    }

    entries
}

/// Recursively collect scopes for a child node and its descendants.
fn collect_scopes(node: &StackNode, enabled: &[StrategyId], entries: &mut Vec<ScopeEntry>) {
    let scope_strategies = resolve_ecosystem_strategies(&node.detection.ecosystem, enabled);
    entries.push(ScopeEntry {
        path: node.detection.scope.clone(),
        strategies: scope_strategies,
    });

    for child in &node.children {
        collect_scopes(child, enabled, entries);
    }
}

/// Resolve the strategy ids for an ecosystem string from the `enabled` list.
///
/// - `"generic"` strategy applies to **all** scopes when in `enabled`.
/// - Each ecosystem maps to its own strategy (`"rust"` → `Rust`, etc.).
/// - `"generic"` / `""` / unknown ecosystems → **empty** (conservative: no
///   cleanup without a known ecosystem match).
/// - If `enabled` is empty the result is empty (no patterns to match).
fn resolve_ecosystem_strategies(ecosystem: &str, enabled: &[StrategyId]) -> Vec<StrategyId> {
    let mut strategies = Vec::new();

    // "generic" strategy applies to all scopes
    if enabled.contains(&StrategyId::Generic) {
        strategies.push(StrategyId::Generic);
    }

    match ecosystem {
        "rust" => {
            if enabled.contains(&StrategyId::Rust) {
                strategies.push(StrategyId::Rust);
            }
        }
        "node" => {
            if enabled.contains(&StrategyId::Node) {
                strategies.push(StrategyId::Node);
            }
        }
        "python" => {
            if enabled.contains(&StrategyId::Python) {
                strategies.push(StrategyId::Python);
            }
        }
        "jvm" | "java" | "kotlin" | "scala" => {
            if enabled.contains(&StrategyId::Jvm) {
                strategies.push(StrategyId::Jvm);
            }
        }
        "go" => {
            if enabled.contains(&StrategyId::Go) {
                strategies.push(StrategyId::Go);
            }
        }
        "generic" | "" => {}
        _ => {}
    }

    strategies
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::types::Detection;
    use std::path::PathBuf;

    fn make_detection(ecosystem: &str, scope: &str) -> Detection {
        Detection {
            ecosystem: ecosystem.to_string(),
            language: None,
            frameworks: Vec::new(),
            confidence: 0.9,
            scope: PathBuf::from(scope),
            markers: Vec::new(),
        }
    }

    fn leaf_node(ecosystem: &str, scope: &str) -> StackNode {
        StackNode {
            detection: make_detection(ecosystem, scope),
            children: Vec::new(),
        }
    }

    #[test]
    fn scopes_for_rinse_root_gets_ecosystem_strategy() {
        let root = leaf_node("rust", "/proj");
        let enabled = vec![StrategyId::Rust, StrategyId::Node, StrategyId::Python];
        let entries = scopes_for_rinse(&root, &enabled);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("/proj"));
        // Root ecosystem is "rust" → only Rust strategy
        assert_eq!(entries[0].strategies, vec![StrategyId::Rust]);
    }

    #[test]
    fn scopes_for_rinse_child_gets_ecosystem_strategy_only() {
        let root = StackNode {
            detection: make_detection("rust", "/proj"),
            children: vec![
                leaf_node("node", "/proj/web"),
                leaf_node("python", "/proj/ml"),
            ],
        };
        let enabled = vec![StrategyId::Rust, StrategyId::Node, StrategyId::Python];
        let entries = scopes_for_rinse(&root, &enabled);
        assert_eq!(entries.len(), 3);
        // Root: ecosystem "rust" → [Rust]
        assert_eq!(entries[0].strategies, vec![StrategyId::Rust]);
        // Node child
        assert_eq!(entries[1].path, PathBuf::from("/proj/web"));
        assert_eq!(entries[1].strategies, vec![StrategyId::Node]);
        // Python child
        assert_eq!(entries[2].path, PathBuf::from("/proj/ml"));
        assert_eq!(entries[2].strategies, vec![StrategyId::Python]);
    }

    #[test]
    fn scopes_for_rinse_unknown_ecosystem_gets_no_strategies() {
        let child = leaf_node("mystery-lang", "/proj/unknown");
        let root_with_child = StackNode {
            detection: make_detection("rust", "/proj"),
            children: vec![child],
        };
        let enabled = vec![StrategyId::Rust, StrategyId::Node];
        let entries = scopes_for_rinse(&root_with_child, &enabled);
        assert_eq!(entries.len(), 2);
        // Root: "rust" → [Rust]
        assert_eq!(entries[0].strategies, vec![StrategyId::Rust]);
        // Child: unknown → no strategies (conservative)
        assert!(entries[1].strategies.is_empty());
    }

    #[test]
    fn scopes_for_rinse_empty_enabled_gives_single_empty_strategies_entry() {
        let root = leaf_node("rust", "/proj");
        let entries = scopes_for_rinse(&root, &[]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].strategies, vec![]);
    }

    #[test]
    fn scopes_for_rinse_deep_tree() {
        let tree = StackNode {
            detection: make_detection("rust", "/proj"),
            children: vec![StackNode {
                detection: make_detection("node", "/proj/web"),
                children: vec![leaf_node("node", "/proj/web/app")],
            }],
        };
        let enabled = vec![StrategyId::Rust, StrategyId::Node];
        let entries = scopes_for_rinse(&tree, &enabled);
        assert_eq!(entries.len(), 3);
        // Root: "rust" → [Rust]
        assert_eq!(entries[0].strategies, vec![StrategyId::Rust]);
        // Child: "node" → [Node]
        assert_eq!(entries[1].strategies, vec![StrategyId::Node]);
        // Grandchild: "node" → [Node]
        assert_eq!(entries[2].path, PathBuf::from("/proj/web/app"));
        assert_eq!(entries[2].strategies, vec![StrategyId::Node]);
    }

    #[test]
    fn generic_strategy_applies_to_all_scopes() {
        let tree = StackNode {
            detection: make_detection("rust", "/proj"),
            children: vec![leaf_node("node", "/proj/web")],
        };
        let enabled = vec![StrategyId::Rust, StrategyId::Node, StrategyId::Generic];
        let entries = scopes_for_rinse(&tree, &enabled);
        assert_eq!(entries.len(), 2);
        // Root: "rust" → [Generic, Rust] (generic applies to all)
        assert_eq!(
            entries[0].strategies,
            vec![StrategyId::Generic, StrategyId::Rust]
        );
        // Child: "node" → [Generic, Node]
        assert_eq!(
            entries[1].strategies,
            vec![StrategyId::Generic, StrategyId::Node]
        );
    }
}
