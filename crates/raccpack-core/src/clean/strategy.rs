//! Cleanup strategies (A2.1): named sets of trash-directory patterns.
//!
//! A strategy is a named set of exact directory names (plus optional
//! single-segment suffix globs like `*.egg-info`) that identify build/cache
//! garbage under a project root. The table is data-driven: adding a strategy
//! is a new entry in [`DEFAULT_STRATEGIES`], nothing else changes.
//!
//! **F-SKIP-1:** the pattern names in this table must stay consistent with
//! `scan::skip::DEFAULT_DIR_NAMES` and the future pack `default_pack()` list —
//! one source of truth going forward. Any addition or rename on either side
//! must be mirrored and covered by an invariant test.

use serde::{Deserialize, Serialize};

/// Identifier of a named cleanup strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrategyId {
    /// Rust (Cargo) build output.
    Rust,
    /// Node.js / npm / pnpm / yarn dependency and build directories.
    Node,
    /// Python interpreter caches and virtualenvs.
    Python,
    /// JVM / Gradle / Maven build outputs and caches.
    Jvm,
    /// Go vendored dependencies.
    Go,
    /// Generic cache and temp directories.
    Generic,
}

impl StrategyId {
    /// Canonical lowercase id, e.g. `"rust"`.
    pub fn as_str(self) -> &'static str {
        match self {
            StrategyId::Rust => "rust",
            StrategyId::Node => "node",
            StrategyId::Python => "python",
            StrategyId::Jvm => "jvm",
            StrategyId::Go => "go",
            StrategyId::Generic => "generic",
        }
    }

    /// Parse a strategy id ignoring case (`"Node"` → `Node`), or `None` for an
    /// unknown id.
    pub fn from_str_ignore_case(s: &str) -> Option<Self> {
        DEFAULT_STRATEGIES
            .iter()
            .find(|def| def.id.as_str().eq_ignore_ascii_case(s))
            .map(|def| def.id)
    }
}

/// How a [`TrashPattern`] matches a directory name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrashMatchKind {
    /// Exact directory name at any depth under the project (within `max_depth`).
    DirNameExact,
}

/// A single trash-directory pattern.
#[derive(Debug, Clone)]
pub struct TrashPattern {
    /// Match kind (currently always [`TrashMatchKind::DirNameExact`]).
    pub kind: TrashMatchKind,
    /// Exact name, or `*<suffix>` for a suffix match (`*.egg-info`).
    pub name: &'static str,
}

impl TrashPattern {
    /// `name == pattern`, or `pattern` is `*<suffix>` and `name` ends with it.
    ///
    /// Semantics intentionally mirror `scan::skip::SkipPolicy` (F-SKIP-1
    /// consistency); kept local because it is 2-3 lines and a cross-subsystem
    /// shared module would be heavier.
    pub fn matches(&self, name: &str) -> bool {
        match self.kind {
            TrashMatchKind::DirNameExact => match self.name.strip_prefix('*') {
                Some(suffix) => name.ends_with(suffix),
                None => name == self.name,
            },
        }
    }
}

/// A named cleanup strategy: stable id, human label, and matching patterns.
#[derive(Debug, Clone)]
pub struct StrategyDef {
    /// Stable strategy id.
    pub id: StrategyId,
    /// Human-readable label (report / UX).
    pub label: &'static str,
    /// Trash-directory patterns, checked in order.
    pub patterns: &'static [TrashPattern],
}

/// All cleanup strategies, in deterministic order.
///
/// Default configuration (see [`crate::config::default_enabled_strategies`])
/// enables only `rust`, `node`, and `python`. Careful names: `dist` (node) and
/// `build` (jvm) may be genuine source directories, and `vendor` (go) is
/// intentionally NOT in the default enabled set — `jvm`, `go`, and `generic`
/// are opt-in via `cleanup.enabled_strategies`.
pub static DEFAULT_STRATEGIES: &[StrategyDef] = &[
    // `target` is the Cargo build output and never source; always safe.
    StrategyDef {
        id: StrategyId::Rust,
        label: "Rust (Cargo)",
        patterns: &[TrashPattern {
            kind: TrashMatchKind::DirNameExact,
            name: "target",
        }],
    },
    StrategyDef {
        id: StrategyId::Node,
        label: "Node.js",
        patterns: &[
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "node_modules",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".next",
            },
            // `dist` is a *careful* name: some projects ship source there.
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "dist",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".nuxt",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "coverage",
            },
        ],
    },
    StrategyDef {
        id: StrategyId::Python,
        label: "Python",
        patterns: &[
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "__pycache__",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".venv",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "venv",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".tox",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".mypy_cache",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".pytest_cache",
            },
            // Suffix match: any name ending in `.egg-info` (e.g. `pkg.egg-info`).
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "*.egg-info",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".ruff_cache",
            },
        ],
    },
    StrategyDef {
        id: StrategyId::Jvm,
        label: "JVM",
        patterns: &[
            // `build` is a *careful* name: may be a genuine source directory.
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "build",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".gradle",
            },
            // `.m2` is rare inside a project but matches when it occurs.
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".m2",
            },
        ],
    },
    // `vendor` is *careful*: some Go projects keep it as an intentional copy
    // of dependencies, so this strategy is deliberately OFF by default.
    StrategyDef {
        id: StrategyId::Go,
        label: "Go",
        patterns: &[TrashPattern {
            kind: TrashMatchKind::DirNameExact,
            name: "vendor",
        }],
    },
    // `tmp`/`temp` may hold user data; generic is opt-in for this reason.
    StrategyDef {
        id: StrategyId::Generic,
        label: "Generic caches",
        patterns: &[
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: ".cache",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "tmp",
            },
            TrashPattern {
                kind: TrashMatchKind::DirNameExact,
                name: "temp",
            },
        ],
    },
];
