use std::fs;
use tempfile::TempDir;

use raccpack_core::{
    default_config_path, default_toml, init_config, ConfigError, InitOptions, RaccConfig,
    CURRENT_CONFIG_VERSION,
};

#[test]
fn default_toml_parses_and_validates() {
    let toml_str = default_toml(Some("/custom/root"), Some("/custom/den"));
    let cfg: RaccConfig = toml::from_str(&toml_str).expect("default_toml should parse");
    assert_eq!(cfg.config_version, CURRENT_CONFIG_VERSION);
    assert_eq!(cfg.paths.scan_root.as_deref(), Some("/custom/root"));
    assert_eq!(cfg.paths.den_dir.as_deref(), Some("/custom/den"));
    cfg.validate().expect("default_toml should validate");
}

#[test]
fn init_config_creates_file_and_den_skeleton() {
    let tmp = TempDir::new().unwrap();
    let config_file = tmp.path().join("sub/config.toml");
    let den_dir = tmp.path().join("my_den");

    let opts = InitOptions {
        config_path: config_file.clone(),
        force: false,
        scan_root: Some(tmp.path().join("projects")),
        den_dir: Some(den_dir.clone()),
        ensure_den: true,
    };

    let res = init_config(&opts).expect("init_config should succeed");
    assert_eq!(res.config_path, config_file);
    assert_eq!(res.den_dir, Some(den_dir.clone()));

    assert!(config_file.exists());
    let loaded = RaccConfig::load_from_path(&config_file).expect("load_from_path must succeed");
    assert_eq!(loaded.config_version, 1);
    assert_eq!(
        loaded.paths.scan_root.as_deref(),
        Some(tmp.path().join("projects").to_str().unwrap())
    );
    assert_eq!(
        loaded.paths.den_dir.as_deref(),
        Some(den_dir.to_str().unwrap())
    );

    // Verify den skeleton
    assert!(den_dir.join(".den-version").exists());
    assert!(den_dir.join("README.txt").exists());
}

#[test]
fn init_config_refuses_overwrite_without_force() {
    let tmp = TempDir::new().unwrap();
    let config_file = tmp.path().join("config.toml");
    fs::write(&config_file, "existing").unwrap();

    let opts = InitOptions {
        config_path: config_file.clone(),
        force: false,
        scan_root: None,
        den_dir: None,
        ensure_den: false,
    };

    let err = init_config(&opts).expect_err("should refuse without force");
    match err {
        ConfigError::AlreadyExists { path } => assert_eq!(path, config_file),
        other => panic!("expected AlreadyExists, got {other:?}"),
    }
}

#[test]
fn init_config_overwrites_with_force() {
    let tmp = TempDir::new().unwrap();
    let config_file = tmp.path().join("config.toml");
    fs::write(&config_file, "corrupt toml content [[[}}}").unwrap();

    let opts = InitOptions {
        config_path: config_file.clone(),
        force: true,
        scan_root: None,
        den_dir: None,
        ensure_den: false,
    };

    let res = init_config(&opts).expect("init_config with force should succeed");
    assert_eq!(res.config_path, config_file);

    let loaded = RaccConfig::load_from_path(&config_file).expect("overwritten config must parse");
    assert_eq!(loaded.config_version, 1);
}

#[test]
fn load_from_path_migrates_v0_and_missing_version() {
    let tmp = TempDir::new().unwrap();
    let config_file = tmp.path().join("old_config.toml");
    fs::write(
        &config_file,
        r#"
[paths]
scan_root = "/tmp/legacy"
[scanner]
max_depth = 5
"#,
    )
    .unwrap();

    let loaded = RaccConfig::load_from_path(&config_file).expect("legacy config should load");
    assert_eq!(loaded.config_version, 1);
    assert_eq!(loaded.paths.scan_root.as_deref(), Some("/tmp/legacy"));
    assert_eq!(loaded.scanner.max_depth, 5);
}

#[test]
fn load_from_path_rejects_incompatible_future_version() {
    let tmp = TempDir::new().unwrap();
    let config_file = tmp.path().join("future_config.toml");
    fs::write(
        &config_file,
        r#"
config_version = 42
[paths]
scan_root = "/tmp"
"#,
    )
    .unwrap();

    let err = RaccConfig::load_from_path(&config_file).expect_err("future config must fail");
    match err {
        ConfigError::IncompatibleVersion { found, current } => {
            assert_eq!(found, 42);
            assert_eq!(current, CURRENT_CONFIG_VERSION);
        }
        other => panic!("expected IncompatibleVersion, got {other:?}"),
    }
}

#[test]
fn default_config_path_returns_xdg_or_home() {
    let path = default_config_path();
    assert!(path.is_ok());
    let path = path.unwrap();
    assert!(path.ends_with("raccpack/config.toml"));
}
