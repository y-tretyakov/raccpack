//! Discovery of trash directories under a project root (A2.1).
//!
//! [`find_trash_dirs`] walks a `target` project root without following
//! symlinks, records directories whose name matches a selected cleanup
//! strategy, prunes them so the walk never descends into a matched dir, and
//! (optionally) sums their byte size. Nothing is deleted at this stage.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::domain::{Error, Result};
use crate::scan::walk::{ensure_scan_root, map_walk_error};

use super::strategy::{StrategyId, TrashPattern, DEFAULT_STRATEGIES};

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
/// 3. Walk with `follow_links(false)` and `max_depth`, pruning any directory at
///    depth > 0 whose name matches a pattern (first match wins, strategy order
///    then pattern order) so it is never descended into and never recorded
///    twice. The root entry is never matched or pruned.
/// 4. Optionally sum sizes with a separate restricted walk.
/// 5. Sort by `path` and defensively check containment under `target`.
pub fn find_trash_dirs(opts: &DetectTrashOptions) -> Result<Vec<TrashDir>> {
    ensure_scan_root(&opts.target)?;
    let patterns = collect_patterns(&opts.strategy_ids)?;
    if patterns.is_empty() {
        return Ok(Vec::new());
    }

    let mut found: Vec<TrashDir> = Vec::new();
    for item in WalkDir::new(&opts.target)
        .follow_links(false)
        .max_depth(opts.max_depth)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 || !entry.file_type().is_dir() {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            match first_match(&name, &patterns) {
                Some(m) => {
                    found.push(TrashDir {
                        path: entry.path().to_path_buf(),
                        strategy: m.strategy.to_string(),
                        pattern_name: m.pattern.name.to_string(),
                        size_bytes: 0,
                    });
                    false
                }
                None => true,
            }
        })
    {
        item.map_err(|err| map_walk_error(err, &opts.target))?;
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
