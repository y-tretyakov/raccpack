//! Integration tests for the M1.3 config loading / validation.
//!
//! Covers `RaccConfig`, `PathsConfig`, `ScannerConfig`, `ConfigError`, the
//! `load*` entry points, `~` expansion, relative-path resolution, defaults and
//! strict error variants as specified in docs/mvp/m1/m1.3-config.md.
//!
//! Env-dependent tests (HOME / XDG_CONFIG_HOME / RACCPACK_CONFIG /
//! set_current_dir) are marked `#[serial]` and manage all three variables
//! explicitly so the order in which they run does not matter.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use raccpack_core::{CleanupConfig, ConfigError, PathsConfig, RaccConfig, ScannerConfig};
use serial_test::serial;
use tempfile::TempDir;

// --- Test helpers -----------------------------------------------------------

/// Saves the three env vars that affect config loading and restores them on
/// drop, even if the test panics.
struct EnvGuard {
    home: Option<OsString>,
    xdg: Option<OsString>,
    raccpack_config: Option<OsString>,
}

impl EnvGuard {
    fn new() -> Self {
        Self {
            home: env::var_os("HOME"),
            xdg: env::var_os("XDG_CONFIG_HOME"),
            raccpack_config: env::var_os("RACCPACK_CONFIG"),
        }
    }

    /// Remove all three variables (used to start from a clean slate).
    fn clear(&mut self) {
        env::remove_var("HOME");
        env::remove_var("XDG_CONFIG_HOME");
        env::remove_var("RACCPACK_CONFIG");
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        let values = [
            ("HOME", &self.home),
            ("XDG_CONFIG_HOME", &self.xdg),
            ("RACCPACK_CONFIG", &self.raccpack_config),
        ];
        for (name, value) in values {
            match value {
                Some(value) => env::set_var(name, value),
                None => env::remove_var(name),
            }
        }
    }
}

/// Capture the env, clear all three config vars, and set `HOME`.
fn set_home(home: &Path) -> EnvGuard {
    let mut guard = EnvGuard::new();
    guard.clear();
    env::set_var("HOME", home);
    guard
}

/// Capture the env and clear all three config vars (e.g. to test `~` without
/// HOME).
fn clear_env() -> EnvGuard {
    let mut guard = EnvGuard::new();
    guard.clear();
    guard
}

/// Restores the previous working directory on drop, even on panic.
struct CwdGuard(PathBuf);

impl CwdGuard {
    fn set(dir: &Path) -> Self {
        let previous = env::current_dir().expect("read current_dir before switching");
        env::set_current_dir(dir).expect("set_current_dir to tempdir");
        Self(previous)
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        env::set_current_dir(&self.0).expect("restore original current_dir");
    }
}

/// Write a TOML config file into `dir` and return its path.
fn write_config(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).expect("write config file");
    path
}

// --- Case 1: Defaults ------------------------------------------------------

#[test]
#[serial]
fn config_default_config_den_dir_resolves_to_home_raccpack_den() {
    let temp = TempDir::new().unwrap();
    let _guard = set_home(temp.path());

    let cfg = RaccConfig::default();
    assert_eq!(cfg.scanner.max_depth, 6);

    let den = cfg
        .den_dir()
        .expect("den_dir should resolve when HOME is set");
    assert_eq!(den, temp.path().join(".raccpack").join("den"));
}

// --- Case 2: Parse minimal TOML ----------------------------------------------

#[test]
fn config_load_from_path_parses_minimal_toml() {
    let temp = TempDir::new().unwrap();
    let path = write_config(
        temp.path(),
        "config.toml",
        r#"
[paths]
scan_root = "/tmp/projects"
den_dir = "/tmp/den"

[scanner]
max_depth = 4
"#,
    );

    let cfg = RaccConfig::load_from_path(&path).expect("minimal TOML should parse");

    assert_eq!(cfg.paths.scan_root.as_deref(), Some("/tmp/projects"));
    assert_eq!(cfg.paths.den_dir.as_deref(), Some("/tmp/den"));
    assert_eq!(cfg.scanner.max_depth, 4);
}

// --- Case 3: Missing file ----------------------------------------------------

#[test]
fn config_load_from_path_missing_file_is_file_not_found() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("does-not-exist.toml");

    let err = RaccConfig::load_from_path(&missing).expect_err("missing file must fail");
    assert!(matches!(err, ConfigError::FileNotFound { .. }));
}

// --- Case 4: Invalid TOML -----------------------------------------------------

#[test]
fn config_load_from_path_broken_toml_is_parse_error() {
    let temp = TempDir::new().unwrap();
    let path = write_config(temp.path(), "broken.toml", "this is not toml = [[[");

    let err = RaccConfig::load_from_path(&path).expect_err("broken TOML must fail");
    assert!(matches!(err, ConfigError::Parse { .. }));
}

// --- Case 5: `~` expansion -----------------------------------------------------

#[test]
#[serial]
fn config_scan_root_with_tilde_expands_to_home() {
    let temp = TempDir::new().unwrap();
    let _guard = set_home(temp.path());
    fs::create_dir(temp.path().join("proj")).expect("create ~/proj");

    let path = write_config(
        temp.path(),
        "config.toml",
        "[paths]\nscan_root = \"~/proj\"\n",
    );
    let cfg = RaccConfig::load_from_path(&path).expect("config should parse");

    let root = cfg
        .scan_root_dir()
        .expect("scan_root should resolve with HOME set");
    assert_eq!(root, temp.path().join("proj"));
}

#[test]
#[serial]
fn config_scan_root_tilde_without_home_is_path_resolve_error() {
    let _guard = clear_env();
    let temp = TempDir::new().unwrap();
    let path = write_config(
        temp.path(),
        "config.toml",
        "[paths]\nscan_root = \"~/proj\"\n",
    );
    let cfg = RaccConfig::load_from_path(&path).expect("load_from_path must not resolve paths");

    let err = cfg
        .scan_root_dir()
        .expect_err("no HOME means `~` cannot expand");
    assert!(matches!(err, ConfigError::PathResolve { .. }));
}

// --- Case 6: Missing scan_root -------------------------------------------------

#[test]
fn config_default_config_scan_root_dir_is_missing_scan_root() {
    let cfg = RaccConfig::default();
    let err = cfg
        .scan_root_dir()
        .expect_err("default config has no scan_root");
    assert!(matches!(err, ConfigError::MissingScanRoot));
}

// --- Case 7: Relative path ------------------------------------------------------

#[test]
#[serial]
fn config_relative_scan_root_resolves_from_current_dir() {
    let temp = TempDir::new().unwrap();
    fs::create_dir(temp.path().join("projects")).expect("create projects dir");
    let _cwd = CwdGuard::set(temp.path());

    let path = write_config(
        temp.path(),
        "config.toml",
        "[paths]\nscan_root = \"projects\"\n",
    );
    let cfg = RaccConfig::load_from_path(&path).expect("config should parse");

    let expected = env::current_dir().unwrap().join("projects");
    let root = cfg
        .scan_root_dir()
        .expect("relative scan_root should resolve");
    assert_eq!(root, expected);
}

// --- Case 8: max_depth default -------------------------------------------------

#[test]
fn config_scanner_max_depth_defaults_to_six() {
    assert_eq!(RaccConfig::default().scanner.max_depth, 6);
    assert_eq!(ScannerConfig::default().max_depth, 6);
}

// --- Case 9: Empty file / empty paths -------------------------------------------

#[test]
fn config_load_from_path_empty_file_returns_defaults() {
    let temp = TempDir::new().unwrap();
    let path = write_config(temp.path(), "empty.toml", "");

    let cfg = RaccConfig::load_from_path(&path).expect("empty file should give defaults");
    assert_eq!(cfg.paths.scan_root, None);
    assert_eq!(cfg.paths.den_dir, None);
    assert_eq!(cfg.scanner.max_depth, 6);
}

#[test]
fn config_empty_scan_root_string_is_treated_as_missing() {
    let temp = TempDir::new().unwrap();
    let path = write_config(temp.path(), "config.toml", "[paths]\nscan_root = \"\"\n");
    let cfg = RaccConfig::load_from_path(&path).expect("empty scan_root must not panic");

    let err = cfg.scan_root_dir().expect_err("empty scan_root is missing");
    assert!(matches!(err, ConfigError::MissingScanRoot));
}

// --- Case 10: validate() max_depth = 0 --------------------------------------------

#[test]
fn config_validate_rejects_zero_max_depth_from_toml() {
    let temp = TempDir::new().unwrap();
    let path = write_config(temp.path(), "config.toml", "[scanner]\nmax_depth = 0\n");

    let err = RaccConfig::load_from_path(&path).expect_err("validate() runs after parse");
    assert!(matches!(err, ConfigError::InvalidMaxDepth { value: 0 }));
}

#[test]
fn config_validate_rejects_zero_max_depth_manual_struct() {
    let cfg = RaccConfig {
        paths: PathsConfig {
            scan_root: Some("/tmp/ignored".into()),
            den_dir: None,
        },
        scanner: ScannerConfig { max_depth: 0 },
        cleanup: CleanupConfig::default(),
        ..RaccConfig::default()
    };

    let err = cfg
        .validate()
        .expect_err("manual struct with max_depth 0 must fail");
    assert!(matches!(err, ConfigError::InvalidMaxDepth { value: 0 }));
}

// --- Case 11: load() with RACCPACK_CONFIG ----------------------------------------

#[test]
#[serial]
fn config_load_uses_raccpack_config_env_when_set() {
    let temp = TempDir::new().unwrap();
    let _guard = set_home(temp.path());
    let path = write_config(temp.path(), "config.toml", "[scanner]\nmax_depth = 3\n");
    env::set_var("RACCPACK_CONFIG", &path);

    let cfg = RaccConfig::load().expect("load should read RACCPACK_CONFIG");
    assert_eq!(cfg.scanner.max_depth, 3);
}

#[test]
#[serial]
fn config_load_with_raccpack_config_missing_file_is_error() {
    let temp = TempDir::new().unwrap();
    let _guard = set_home(temp.path());
    let missing = temp.path().join("nope.toml");
    env::set_var("RACCPACK_CONFIG", &missing);

    let err = RaccConfig::load().expect_err("RACCPACK_CONFIG pointing at a missing file must fail");
    assert!(matches!(err, ConfigError::FileNotFound { .. }));
}

// --- Case 12: load() with XDG_CONFIG_HOME ----------------------------------------

#[test]
#[serial]
fn config_load_reads_config_from_xdg_config_home() {
    let temp = TempDir::new().unwrap();
    let _guard = set_home(temp.path());
    env::set_var("XDG_CONFIG_HOME", temp.path());
    let config_path = temp.path().join("raccpack").join("config.toml");
    fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    fs::write(&config_path, "[scanner]\nmax_depth = 2\n").unwrap();

    let cfg = RaccConfig::load().expect("load should read the XDG default location");
    assert_eq!(cfg.scanner.max_depth, 2);
}

#[test]
#[serial]
fn config_load_without_config_file_returns_default() {
    let temp = TempDir::new().unwrap();
    let _guard = set_home(temp.path());
    env::set_var("XDG_CONFIG_HOME", temp.path().join("no-config-here"));

    let cfg = RaccConfig::load().expect("missing default file should give defaults");
    assert_eq!(cfg.scanner.max_depth, 6);
    assert_eq!(cfg.paths.scan_root, None);
    assert_eq!(cfg.paths.den_dir, None);
}

// --- Case 13: with_scan_root / with_den_dir overrides ---------------------------

#[test]
fn config_builder_overrides_override_toml_paths() {
    let temp = TempDir::new().unwrap();
    let cli_root = temp.path().join("cli-root");
    fs::create_dir(&cli_root).expect("create cli scan_root");
    let cli_den = temp.path().join("cli-den");

    let path = write_config(
        temp.path(),
        "config.toml",
        "[paths]\nscan_root = \"/tmp/from-toml\"\nden_dir = \"/tmp/from-toml-den\"\n",
    );
    let cfg = RaccConfig::load_from_path(&path)
        .expect("TOML should parse")
        .with_scan_root(&cli_root)
        .with_den_dir(&cli_den);

    assert_eq!(cfg.scan_root_dir().unwrap(), cli_root);
    assert_eq!(cfg.den_dir().unwrap(), cli_den);
}

// --- Case 14: den_dir existence not required --------------------------------------

#[test]
fn config_den_dir_does_not_require_existence() {
    let temp = TempDir::new().unwrap();
    let not_yet = temp.path().join("not-yet-created").join("den");
    let cfg = RaccConfig::default().with_den_dir(&not_yet);

    let den = cfg
        .den_dir()
        .expect("den_dir may point at a missing directory");
    assert_eq!(den, not_yet);
}

// --- Case 15: scan_root is a file -------------------------------------------------

#[test]
fn config_scan_root_that_is_a_file_is_scan_root_missing() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("a-file.txt");
    fs::write(&file, "not a directory").unwrap();

    let cfg = RaccConfig::default().with_scan_root(&file);
    let err = cfg
        .scan_root_dir()
        .expect_err("a file is not a valid scan_root");
    assert!(matches!(err, ConfigError::ScanRootMissing { .. }));
}

#[test]
fn config_scan_root_that_does_not_exist_is_scan_root_missing() {
    let temp = TempDir::new().unwrap();
    let missing = temp.path().join("no-such-dir");
    let cfg = RaccConfig::default().with_scan_root(&missing);

    let err = cfg
        .scan_root_dir()
        .expect_err("missing scan_root must fail");
    assert!(matches!(err, ConfigError::ScanRootMissing { .. }));
}

// --- Case 16: suggestion() ---------------------------------------------------------

#[test]
fn config_suggestion_covers_missing_scan_root_and_scan_root_missing() {
    assert!(ConfigError::MissingScanRoot.suggestion().is_some());
    assert!(ConfigError::ScanRootMissing {
        path: PathBuf::from("/nonexistent")
    }
    .suggestion()
    .is_some());
}
