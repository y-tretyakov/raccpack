//! Integration tests for A2.1 — cleanup strategies and `find_trash_dirs`.
//!
//! Covers `StrategyId` / `DEFAULT_STRATEGIES`, the `clean::strategy`
//! re-exports, `find_trash_dirs` behavior (strategy filtering, pruning,
//! compute_size, symlink safety, max_depth, sorting, root exclusion, error
//! variants) and the `[cleanup]` config toggles (defaults, strict unknown-id
//! validation, case-insensitivity) as specified in docs/alpha/a2/a2.1.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real user
//! dirs. The symlink test is Linux/Unix-only and guarded with `#[cfg(unix)]`.

use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::clean::strategy::{StrategyId, DEFAULT_STRATEGIES};
use raccpack_core::clean::{find_trash_dirs, DetectTrashOptions, TrashDir};
use raccpack_core::{CleanupConfig, ConfigError, Error, RaccConfig};
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write `content` at `root/rel`.
fn write_bytes(root: &Path, rel: &str, content: &[u8]) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, content).expect("write fixture file");
}

/// Write a 1-byte placeholder at `root/rel`.
fn write(root: &Path, rel: &str) {
    write_bytes(root, rel, b"x");
}

/// Build `DetectTrashOptions` for `target` with the given strategies. Size
/// computation is off and depth is generous unless a test overrides them.
fn opts(target: &Path, strategy_ids: Vec<StrategyId>) -> DetectTrashOptions {
    DetectTrashOptions {
        target: target.to_path_buf(),
        strategy_ids,
        max_depth: 16,
        compute_size: false,
        scope_filter: None,
    }
}

/// Run `find_trash_dirs` with `strategy_ids` and expect success.
fn find(target: &Path, strategy_ids: Vec<StrategyId>) -> Vec<TrashDir> {
    find_trash_dirs(&opts(target, strategy_ids)).expect("find_trash_dirs must not error")
}

/// Run `find_trash_dirs` with an explicit `compute_size` flag.
fn find_compute(target: &Path, strategy_ids: Vec<StrategyId>, compute_size: bool) -> Vec<TrashDir> {
    let options = DetectTrashOptions {
        compute_size,
        ..opts(target, strategy_ids)
    };
    find_trash_dirs(&options).expect("find_trash_dirs must not error")
}

// --- Case 1: Fixture discovery ----------------------------------------------

#[test]
fn find_trash_dirs_detects_target_and_node_modules() {
    let proj = TempDir::new().unwrap();
    write(proj.path(), "target/debug/x");
    write(proj.path(), "node_modules/a");
    write(proj.path(), "src/main.rs");

    let dirs = find(proj.path(), vec![StrategyId::Rust, StrategyId::Node]);

    assert_eq!(dirs.len(), 2, "exactly target + node_modules: {dirs:?}");

    let target = dirs
        .iter()
        .find(|d| d.pattern_name == "target")
        .expect("target must be found");
    assert_eq!(target.path, proj.path().join("target"));
    assert_eq!(target.strategy, "rust");
    assert_eq!(target.pattern_name, "target");

    let node_modules = dirs
        .iter()
        .find(|d| d.pattern_name == "node_modules")
        .expect("node_modules must be found");
    assert_eq!(node_modules.path, proj.path().join("node_modules"));
    assert_eq!(node_modules.strategy, "node");
    assert_eq!(node_modules.pattern_name, "node_modules");

    assert!(
        dirs.iter().all(
            |d| d.path != proj.path().join("src") && d.path != proj.path().join("target/debug")
        ),
        "neither src nor a nested build dir may be reported: {dirs:?}"
    );
}

// --- Case 2: Strategy filter -------------------------------------------------

#[test]
fn find_trash_dirs_strategy_filter_rust_only_skips_node_modules() {
    let proj = TempDir::new().unwrap();
    write(proj.path(), "target/debug/x");
    write(proj.path(), "node_modules/a");
    write(proj.path(), "src/main.rs");

    let dirs = find(proj.path(), vec![StrategyId::Rust]);

    assert_eq!(
        dirs.len(),
        1,
        "only target with rust-only strategies: {dirs:?}"
    );
    assert_eq!(dirs[0].path, proj.path().join("target"));
    assert_eq!(dirs[0].strategy, "rust");
    assert_eq!(dirs[0].pattern_name, "target");
}

// --- Case 3: compute_size ------------------------------------------------------

#[test]
fn find_trash_dirs_compute_size_true_counts_known_file() {
    let proj = TempDir::new().unwrap();
    let known: Vec<u8> = (0..4096).map(|i| (i % 251) as u8).collect();
    write_bytes(proj.path(), "target/debug/x", &known);
    write(proj.path(), "node_modules/a");

    let dirs = find_compute(proj.path(), vec![StrategyId::Rust], true);

    let target = dirs
        .iter()
        .find(|d| d.pattern_name == "target")
        .expect("target must be found");
    assert!(
        target.size_bytes >= known.len() as u64,
        "size_bytes {} must cover the known {}-byte file",
        target.size_bytes,
        known.len()
    );
}

#[test]
fn find_trash_dirs_compute_size_false_is_zero() {
    let proj = TempDir::new().unwrap();
    write_bytes(proj.path(), "target/debug/x", &vec![0u8; 4096]);

    let dirs = find_compute(proj.path(), vec![StrategyId::Rust], false);

    let target = dirs
        .iter()
        .find(|d| d.pattern_name == "target")
        .expect("target must be found");
    assert_eq!(target.size_bytes, 0);
}

// --- Case 4: Symlink safety ------------------------------------------------------

#[cfg(unix)]
#[test]
fn find_trash_dirs_symlink_not_reported_and_size_not_counted() {
    let proj = TempDir::new().unwrap();
    let external = TempDir::new().unwrap();
    let big = vec![0u8; 1024 * 1024];
    fs::write(external.path().join("big.bin"), &big).expect("write big external file");

    write(proj.path(), "node_modules/a");
    symlink(external.path(), proj.path().join(".cache")).expect("create .cache symlink");

    // `.cache` is a Generic pattern: the link would be reported if symlinks
    // were followed. It must not be, and its 1 MiB content must not be counted.
    let dirs = find_compute(
        proj.path(),
        vec![StrategyId::Node, StrategyId::Generic],
        true,
    );

    assert!(
        dirs.iter().all(|d| d.pattern_name != ".cache"),
        "a symlink to an external dir must never be reported as trash: {dirs:?}"
    );
    assert!(
        dirs.iter().all(|d| !d.path.ends_with(".cache")),
        "no trash dir may resolve through the symlink: {dirs:?}"
    );

    let total: u64 = dirs.iter().map(|d| d.size_bytes).sum();
    assert!(
        total < big.len() as u64,
        "external content must not be counted in any size_bytes (total {total})"
    );

    let node_modules = dirs
        .iter()
        .find(|d| d.pattern_name == "node_modules")
        .expect("node_modules must be found");
    assert_eq!(
        node_modules.size_bytes, 1,
        "only the 1-byte fixture inside node_modules may be counted"
    );
}

// --- Case 5: from_str_ignore_case -------------------------------------------------

#[test]
fn strategy_id_from_str_ignore_case_is_case_insensitive() {
    assert_eq!(
        StrategyId::from_str_ignore_case("Node"),
        Some(StrategyId::Node)
    );
    assert_eq!(
        StrategyId::from_str_ignore_case("node"),
        Some(StrategyId::Node)
    );
    assert_eq!(
        StrategyId::from_str_ignore_case("GENERIC"),
        Some(StrategyId::Generic)
    );
    assert_eq!(
        StrategyId::from_str_ignore_case("python"),
        Some(StrategyId::Python)
    );
    assert_eq!(
        StrategyId::from_str_ignore_case("RUST"),
        Some(StrategyId::Rust)
    );
    assert_eq!(StrategyId::from_str_ignore_case("nope"), None);
    assert_eq!(StrategyId::from_str_ignore_case(""), None);
}

// --- Case 6: Default strategies ------------------------------------------------------

#[test]
fn default_strategies_contain_rust_node_python() {
    let ids: Vec<&str> = DEFAULT_STRATEGIES.iter().map(|s| s.id.as_str()).collect();
    for expected in ["rust", "node", "python"] {
        assert!(
            ids.contains(&expected),
            "DEFAULT_STRATEGIES must contain `{expected}`: {ids:?}"
        );
    }
}

#[test]
fn rust_strategy_has_target_pattern_and_node_has_node_modules() {
    let rust = DEFAULT_STRATEGIES
        .iter()
        .find(|s| s.id == StrategyId::Rust)
        .expect("rust strategy must exist");
    assert!(
        rust.patterns.iter().any(|p| p.name == "target"),
        "rust strategy must contain pattern `target`: {:?}",
        rust.patterns.iter().map(|p| p.name).collect::<Vec<_>>()
    );

    let node = DEFAULT_STRATEGIES
        .iter()
        .find(|s| s.id == StrategyId::Node)
        .expect("node strategy must exist");
    assert!(
        node.patterns.iter().any(|p| p.name == "node_modules"),
        "node strategy must contain pattern `node_modules`: {:?}",
        node.patterns.iter().map(|p| p.name).collect::<Vec<_>>()
    );
}

// --- Case 7: Unknown strategy in config (strict) --------------------------------------

#[test]
fn config_load_from_path_unknown_cleanup_strategy_is_strict_error() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    fs::write(
        &path,
        "[cleanup]\nenabled_strategies = [\"rust\", \"nope\"]\n",
    )
    .unwrap();

    let err = RaccConfig::load_from_path(&path).expect_err("unknown strategy must fail validation");
    assert!(
        matches!(err, ConfigError::UnknownCleanupStrategy { ref id } if id == "nope"),
        "expected UnknownCleanupStrategy for `nope`, got {err:?}"
    );
}

#[test]
fn config_validate_manual_struct_unknown_strategy_is_error() {
    let cfg = RaccConfig {
        cleanup: CleanupConfig {
            enabled_strategies: vec!["rust".into(), "nope".into()],
        },
        ..RaccConfig::default()
    };

    let err = cfg
        .validate()
        .expect_err("manual config with an unknown strategy must fail");
    assert!(
        matches!(err, ConfigError::UnknownCleanupStrategy { ref id } if id == "nope"),
        "expected UnknownCleanupStrategy for `nope`, got {err:?}"
    );
}

// --- Case 8: Config defaults -----------------------------------------------------------

#[test]
fn cleanup_config_default_enabled_strategies_are_rust_node_python() {
    assert_eq!(
        RaccConfig::default().cleanup.enabled_strategies,
        vec!["rust".to_string(), "node".to_string(), "python".to_string()]
    );
    assert_eq!(
        CleanupConfig::default().enabled_strategies,
        vec!["rust".to_string(), "node".to_string(), "python".to_string()]
    );
}

#[test]
fn config_load_from_path_without_cleanup_section_uses_default_strategies() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    fs::write(&path, "[paths]\nscan_root = \"/tmp/projects\"\n").unwrap();

    let cfg = RaccConfig::load_from_path(&path).expect("config without [cleanup] must parse");
    assert_eq!(
        cfg.cleanup.enabled_strategies,
        vec!["rust".to_string(), "node".to_string(), "python".to_string()]
    );
}

// --- Case 9: Config case-insensitivity ---------------------------------------------------

#[test]
fn config_load_from_path_cleanup_strategies_case_insensitive_ok() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("config.toml");
    fs::write(
        &path,
        "[cleanup]\nenabled_strategies = [\"Node\", \"RUST\"]\n",
    )
    .unwrap();

    let cfg = RaccConfig::load_from_path(&path).expect("mixed-case strategy ids must validate");
    assert!(cfg.validate().is_ok());
    assert_eq!(
        cfg.cleanup.enabled_strategies,
        vec!["Node".to_string(), "RUST".to_string()]
    );
}

// --- Case 10: Pruning -----------------------------------------------------------------

#[test]
fn find_trash_dirs_prunes_matched_dirs() {
    let proj = TempDir::new().unwrap();
    write(proj.path(), "target/node_modules/x");

    let dirs = find(proj.path(), vec![StrategyId::Rust, StrategyId::Node]);

    assert_eq!(
        dirs.len(),
        1,
        "the nested node_modules inside a matched target must be pruned: {dirs:?}"
    );
    assert_eq!(dirs[0].path, proj.path().join("target"));
    assert_eq!(dirs[0].pattern_name, "target");
}

// --- Case 11: Root not recorded -----------------------------------------------------------

#[test]
fn find_trash_dirs_root_named_like_trash_pattern_not_recorded() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("build");
    fs::create_dir(&root).expect("create scan root named `build`");
    write(&root, "node_modules/a");

    // `build` matches the JVM strategy; the depth-0 entry must not appear even
    // though its own name is a trash pattern.
    let dirs = find(&root, vec![StrategyId::Jvm, StrategyId::Node]);

    assert!(
        !dirs.iter().any(|d| d.path == root),
        "the scan root itself must never be recorded: {dirs:?}"
    );
}

// --- Case 12: Sorted output ---------------------------------------------------------------

#[test]
fn find_trash_dirs_results_sorted_by_path() {
    let proj = TempDir::new().unwrap();
    // Create in reverse alphabetical order; results must still be sorted by path.
    write(proj.path(), "target/x");
    write(proj.path(), "node_modules/a");

    let dirs = find(proj.path(), vec![StrategyId::Rust, StrategyId::Node]);

    let paths: Vec<PathBuf> = dirs.iter().map(|d| d.path.clone()).collect();
    assert!(
        paths.windows(2).all(|w| w[0] <= w[1]),
        "results must be sorted by path: {paths:?}"
    );
    assert_eq!(paths[0], proj.path().join("node_modules"));
    assert_eq!(paths[1], proj.path().join("target"));
}

// --- Case 13: No strategies ----------------------------------------------------------------

#[test]
fn find_trash_dirs_no_strategies_returns_empty() {
    let proj = TempDir::new().unwrap();
    write(proj.path(), "target/x");

    let dirs = find(proj.path(), vec![]);
    assert!(
        dirs.is_empty(),
        "no strategies must yield no results: {dirs:?}"
    );
}

// --- Case 14: Missing / non-dir target -------------------------------------------------------

#[test]
fn find_trash_dirs_missing_target_is_path_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("does-not-exist");

    let err = find_trash_dirs(&opts(&missing, vec![StrategyId::Rust]))
        .expect_err("a nonexistent target must fail");
    assert!(matches!(err, Error::PathNotFound { .. }));
}

#[test]
fn find_trash_dirs_file_target_is_not_a_directory() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a-file.txt");
    fs::write(&file, "not a directory").unwrap();

    let err =
        find_trash_dirs(&opts(&file, vec![StrategyId::Rust])).expect_err("a file target must fail");
    assert!(matches!(err, Error::NotADirectory { .. }));
}

// --- Extras -------------------------------------------------------------------------------------

#[test]
fn find_trash_dirs_respects_max_depth() {
    let proj = TempDir::new().unwrap();
    write(proj.path(), "deep/target/x");

    let shallow = find_trash_dirs(&DetectTrashOptions {
        target: proj.path().to_path_buf(),
        strategy_ids: vec![StrategyId::Rust],
        max_depth: 1,
        compute_size: false,
        scope_filter: None,
    })
    .expect("no error at max_depth 1");
    assert!(
        shallow.iter().all(|d| d.pattern_name != "target"),
        "target at depth 2 must be invisible at max_depth 1: {shallow:?}"
    );

    let deep = find_trash_dirs(&DetectTrashOptions {
        target: proj.path().to_path_buf(),
        strategy_ids: vec![StrategyId::Rust],
        max_depth: 2,
        compute_size: false,
        scope_filter: None,
    })
    .expect("no error at max_depth 2");
    assert!(
        deep.iter().any(|d| d.pattern_name == "target"),
        "target at depth 2 must be found at max_depth 2: {deep:?}"
    );
}

#[test]
fn strategy_id_as_str_roundtrip_for_all_variants() {
    for id in [
        StrategyId::Rust,
        StrategyId::Node,
        StrategyId::Python,
        StrategyId::Jvm,
        StrategyId::Go,
        StrategyId::Generic,
    ] {
        let s = id.as_str();
        assert_eq!(
            StrategyId::from_str_ignore_case(s),
            Some(id),
            "as_str/from_str_ignore_case roundtrip must hold for `{s}`"
        );
    }
    assert_eq!(StrategyId::Rust.as_str(), "rust");
    assert_eq!(StrategyId::Node.as_str(), "node");
    assert_eq!(StrategyId::Python.as_str(), "python");
    assert_eq!(StrategyId::Jvm.as_str(), "jvm");
    assert_eq!(StrategyId::Go.as_str(), "go");
    assert_eq!(StrategyId::Generic.as_str(), "generic");
}

#[test]
fn config_validate_ok_for_default_and_mixed_case_strategies() {
    let cfg = RaccConfig::default();
    assert!(cfg.validate().is_ok());

    let mixed = RaccConfig {
        cleanup: CleanupConfig {
            enabled_strategies: vec!["Node".into(), "python".into()],
        },
        ..RaccConfig::default()
    };
    assert!(mixed.validate().is_ok());
}
