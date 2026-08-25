//! Discovery of trash directories under a project root (A2.1).
//!
//! [`find_trash_dirs`] walks a `target` project root with an explicit
//! depth-first traversal (`fs::read_dir`, symlinks never followed), records
//! directories whose name matches a selected cleanup strategy, prunes them so
//! the walk never descends into a matched dir, and (optionally) sums their
//! byte size. Nothing is deleted at this stage.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::domain::{Error, Result};
use crate::scan::walk::{ensure_scan_root, map_walk_error};

use super::strategy::{StrategyId, TrashPattern, DEFAULT_STRATEGIES};

/// A scope entry for scoped trash discovery (D3.1 DAG mode).
///
/// Tells [`find_trash_dirs_scoped`] which directory to scan and which
/// strategy patterns to search within it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeEntry {
    /// Absolute directory path to scan for trash dirs.
    pub path: PathBuf,
    /// Strategy ids whose patterns are searched under this scope.
    pub strategies: Vec<StrategyId>,
}

/// A discovered trash-directory candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrashDir {
    /// Matched directory path as walked from `target`.
    pub path: PathBuf,
    /// Strategy id ([`StrategyId::as_str`]), e.g. `"node"`.
    pub strategy: String,
    /// Matched pattern name, e.g. `"node_modules"`.
    pub pattern_name: String,
    /// Total byte size under `path`; `0` when `compute_size` was `false`.
    pub size_bytes: u64,
}

/// Options for [`find_trash_dirs`].
#[derive(Debug, Clone)]
pub struct DetectTrashOptions {
    /// Existing project root to scan.
    pub target: PathBuf,
    /// Strategies whose patterns are matched, in this order.
    pub strategy_ids: Vec<StrategyId>,
    /// Maximum directory depth to descend.
    pub max_depth: usize,
    /// Sum byte size under each hit via a separate restricted walk; `false`
    /// leaves [`TrashDir::size_bytes`] at `0`.
    pub compute_size: bool,
    /// Scoped search filter (D3.1 DAG mode).
    ///
    /// When `Some`, only the scopes listed are searched, each with its own
    /// strategy list. When `None`, the legacy flat walk is used: the entire
    /// `target` tree is searched with `strategy_ids`.
    pub scope_filter: Option<Vec<ScopeEntry>>,
}

/// A pattern paired with the strategy it came from (both `'static`).
#[derive(Debug, Clone, Copy)]
struct CollectedPattern {
    pattern: &'static TrashPattern,
    strategy: &'static str,
}

/// Find trash directories under `opts.target` matching the selected strategies.
///
/// Algorithm:
/// 1. Validate `target` via [`ensure_scan_root`].
/// 2. Collect patterns from the strategies in `strategy_ids` order. An empty
///    `strategy_ids` returns an empty vector (deterministic, no error).
/// 3. Explicit DFS with an own stack of `(dir, depth)`. Entries are classified
///    with `DirEntry::file_type()` (does not follow symlinks), so a symlink
///    (incl. symlink-to-dir) is never recorded and never descended into. A
///    directory at depth in 1..=max_depth whose name matches a pattern is
///    recorded (first match wins, strategy order then pattern order) and
///    pruned — never descended into — so a nested `target/node_modules` is not
///    rediscovered. The root entry is never matched. Entries at depth ==
///    max_depth are checked for a match but not descended into; nothing beyond
///    max_depth is visited. A directory that cannot be read fails fast with
///    [`Error::Io`] carrying the offending path.
/// 4. Optionally sum sizes with a separate restricted walk.
/// 5. Sort by `path` and defensively check containment under `target`.
pub fn find_trash_dirs(opts: &DetectTrashOptions) -> Result<Vec<TrashDir>> {
    ensure_scan_root(&opts.target)?;
    let patterns = collect_patterns(&opts.strategy_ids)?;
    if patterns.is_empty() {
        return Ok(Vec::new());
    }

    let mut found: Vec<TrashDir> = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = vec![(opts.target.clone(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        for item in fs::read_dir(&dir).map_err(|source| Error::Io {
            path: dir.clone(),
            source,
        })? {
            let entry = item.map_err(|source| Error::Io {
                path: dir.clone(),
                source,
            })?;
            let file_type = entry.file_type().map_err(|source| Error::Io {
                path: entry.path(),
                source,
            })?;
            if !file_type.is_dir() {
                continue;
            }
            let entry_depth = depth + 1;
            if entry_depth > opts.max_depth {
                continue;
            }
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();
            if let Some(m) = first_match(&name, &patterns) {
                found.push(TrashDir {
                    path: entry.path(),
                    strategy: m.strategy.to_string(),
                    pattern_name: m.pattern.name.to_string(),
                    size_bytes: 0,
                });
            } else if entry_depth < opts.max_depth {
                stack.push((entry.path(), entry_depth));
            }
        }
    }

    if opts.compute_size {
        for dir in &mut found {
            dir.size_bytes = dir_size_bytes(&dir.path)?;
        }
    }

    found.sort_by(|a, b| a.path.cmp(&b.path));
    for dir in &found {
        if dir.path.strip_prefix(&opts.target).is_err() {
            return Err(Error::Other {
                message: format!("trash dir escaped target root: {}", dir.path.display()),
            });
        }
    }
    Ok(found)
}

/// Find trash directories under `opts.target`, scoped by `scope_filter`.
///
/// When `opts.scope_filter` is `Some`, the flat walk is replaced by a
/// per-scope walk: each [`ScopeEntry`] in the list is walked independently
/// using only its own strategy patterns. This supports DAG mode rinse where
/// `target/` is only deleted under Rust scopes and `node_modules/` only
/// under Node scopes.
///
/// When `opts.scope_filter` is `None`, this delegates to [`find_trash_dirs`].
///
/// # Errors
///
/// Same as [`find_trash_dirs`] plus:
/// - A scope path not contained under `opts.target` → [`Error::Other`].
pub fn find_trash_dirs_scoped(opts: &DetectTrashOptions) -> Result<Vec<TrashDir>> {
    match &opts.scope_filter {
        None => find_trash_dirs(opts),
        Some(scopes) => {
            ensure_scan_root(&opts.target)?;
            let mut all: Vec<TrashDir> = Vec::new();

            for (i, scope) in scopes.iter().enumerate() {
                if scope.path.strip_prefix(&opts.target).is_err() {
                    return Err(Error::Other {
                        message: format!(
                            "scope path {} is not contained under target {}",
                            scope.path.display(),
                            opts.target.display()
                        ),
                    });
                }

                let scoped_opts = DetectTrashOptions {
                    target: scope.path.clone(),
                    strategy_ids: scope.strategies.clone(),
                    max_depth: opts.max_depth,
                    compute_size: opts.compute_size,
                    scope_filter: None,
                };

                let found = find_trash_dirs(&scoped_opts)?;

                // Filter: only keep dirs NOT under descendant scopes' roots.
                // A scope only prunes trash belonging to its own child scopes,
                // not ancestor scopes (child scope finds are legitimately under
                // the root scope's path).
                for dir in found {
                    let under_descendant_scope = scopes
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .filter(|(_, other)| {
                            other.path.starts_with(&scope.path) && other.path != scope.path
                        })
                        .any(|(_, other)| dir.path.starts_with(&other.path));

                    if !under_descendant_scope {
                        all.push(dir);
                    }
                }
            }

            all.sort_by(|a, b| a.path.cmp(&b.path));
            all.dedup_by(|a, b| a.path == b.path);
            Ok(all)
        }
    }
}

/// First [`CollectedPattern`] matching `name`, in strategy-then-pattern order.
fn first_match(name: &str, patterns: &[CollectedPattern]) -> Option<CollectedPattern> {
    patterns.iter().find(|p| p.pattern.matches(name)).copied()
}

/// Collect the patterns of the given strategies in `strategy_ids` order.
fn collect_patterns(strategy_ids: &[StrategyId]) -> Result<Vec<CollectedPattern>> {
    let mut patterns = Vec::new();
    for id in strategy_ids {
        let def = DEFAULT_STRATEGIES
            .iter()
            .find(|def| def.id == *id)
            .ok_or_else(|| Error::Other {
                message: format!("no cleanup strategy registered for `{}`", id.as_str()),
            })?;
        for pattern in def.patterns {
            patterns.push(CollectedPattern {
                pattern,
                strategy: id.as_str(),
            });
        }
    }
    Ok(patterns)
}

/// Sum the byte size of regular files under `path`, never following symlinks.
///
/// Intentionally NOT `scan::size::project_size_bytes`: that function honors a
/// `SkipPolicy` and would skip nested `node_modules`/`target`-like directories
/// inside the trash dir. A trash dir's full size must include everything under
/// it, so this helper walks unrestricted. Unreadable files are skipped and the
/// walk continues; symlinks are never followed or counted. `pub(crate)` so
/// `clean::remove::remove_trash_dir` reuses the same restricted walk (AGENTS
/// §8.3.1: shared helper, no copy).
pub(crate) fn dir_size_bytes(path: &Path) -> Result<u64> {
    let mut total: u64 = 0;
    for item in WalkDir::new(path).follow_links(false) {
        let entry = item.map_err(|err| map_walk_error(err, path))?;
        let file_type = entry.file_type();
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}
