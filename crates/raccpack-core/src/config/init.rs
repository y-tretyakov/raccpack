//! Configuration initialization and template generation for raccpack.

use std::path::PathBuf;

use super::error::ConfigError;
use super::paths::{resolve_path, DEFAULT_DEN_DIR};

/// Options for initializing a new raccpack configuration.
#[derive(Debug, Clone)]
pub struct InitOptions {
    /// Target path for the configuration file (e.g. ~/.config/raccpack/config.toml).
    pub config_path: PathBuf,
    /// Overwrite an existing configuration file if present.
    pub force: bool,
    /// Optional pre-filled scan root directory.
    pub scan_root: Option<PathBuf>,
    /// Optional pre-filled den output directory.
    pub den_dir: Option<PathBuf>,
    /// Ensure the den skeleton (.den-version, README.txt) exists.
    pub ensure_den: bool,
}

/// Result of a successful configuration initialization.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InitResult {
    /// Absolute path to the generated configuration file.
    pub config_path: PathBuf,
    /// Absolute path to the initialized den directory, if `ensure_den` was requested.
    pub den_dir: Option<PathBuf>,
}

/// Generate default commented TOML configuration string with official wiki links.
pub fn default_toml(scan_root: Option<&str>, den_dir: Option<&str>) -> String {
    let scan_root_str = scan_root.unwrap_or("~/DEV/PROJS");
    let den_dir_str = den_dir.unwrap_or(DEFAULT_DEN_DIR);

    format!(
        r#"# =============================================================================
# raccpack configuration file
# =============================================================================
# Documentation & Wiki:
#   Overview & Quickstart: https://y-tretyakov.github.io/raccpack/
#   Configuration guide:   https://y-tretyakov.github.io/raccpack/configuration.html
#   Core concepts (Den):   https://y-tretyakov.github.io/raccpack/concepts.html
#   Project discovery:     https://y-tretyakov.github.io/raccpack/sniff.html
#   Cleanup strategies:    https://y-tretyakov.github.io/raccpack/rinse.html
#   Supported stacks:      https://y-tretyakov.github.io/raccpack/supported.html
# =============================================================================

# Configuration schema version. Used for forward compatibility and automated migrations.
config_version = 1

# -----------------------------------------------------------------------------
# [paths] — Workspace paths
# -----------------------------------------------------------------------------
# Paths can be absolute, relative (to current working directory), or use `~`
# for user home directory.
[paths]
# Directory containing source projects to scan (e.g. ~/DEV/PROJS).
# If not set here, can be provided via --root CLI flag.
# Docs: https://y-tretyakov.github.io/raccpack/configuration.html#paths
scan_root = "{scan_root_str}"

# Central storage vault (Den) for packed archives and encrypted secrets.
# Default: ~/.raccpack/den
# Docs: https://y-tretyakov.github.io/raccpack/concepts.html#den
den_dir = "{den_dir_str}"

# -----------------------------------------------------------------------------
# [scanner] — Project discovery & depth limits
# -----------------------------------------------------------------------------
[scanner]
# Maximum directory depth to traverse when discovering projects with `racc sniff`.
# Must be at least 1. Default: 6.
# Docs: https://y-tretyakov.github.io/raccpack/sniff.html
max_depth = 6

# -----------------------------------------------------------------------------
# [cleanup] — Build artifact cleanup (rinse) strategies
# -----------------------------------------------------------------------------
# Enabled strategies when running `racc rinse` without explicit --strategy flags.
#
# Available strategies:
#   - "rust"    : target/
#   - "node"    : node_modules/, .next/, .nuxt/, .turbo/, dist/, build/
#   - "python"  : __pycache__/, .venv/, venv/, env/, .pytest_cache/, .ruff_cache/
#   - "jvm"     : target/, .gradle/, build/ (opt-in)
#   - "go"      : vendor/, bin/ (opt-in)
#   - "generic" : tmp/, temp/, .cache/ (opt-in)
#
# Safe default enabled strategies: ["rust", "node", "python"]
# Docs: https://y-tretyakov.github.io/raccpack/rinse.html
# Supported catalogs: https://y-tretyakov.github.io/raccpack/supported.html
[cleanup]
enabled_strategies = ["rust", "node", "python"]

# -----------------------------------------------------------------------------
# [detect] — Language & framework detection pipeline for `racc sniff`
# -----------------------------------------------------------------------------
# mode = "priority_table"   # default; "composite_dag" lands in Detect v2 (0.4.x)
"#
    )
}

/// Initialize a new raccpack configuration file and optional den storage skeleton.
pub fn init_config(opts: &InitOptions) -> Result<InitResult, ConfigError> {
    let resolved_config_path = resolve_path(&opts.config_path.to_string_lossy())?;

    if resolved_config_path.exists() && !opts.force {
        return Err(ConfigError::AlreadyExists {
            path: resolved_config_path,
        });
    }

    if let Some(parent) = resolved_config_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    let scan_root_str = opts.scan_root.as_ref().map(|p| p.to_string_lossy());
    let den_dir_str = opts.den_dir.as_ref().map(|p| p.to_string_lossy());
    let toml_text = default_toml(scan_root_str.as_deref(), den_dir_str.as_deref());

    std::fs::write(&resolved_config_path, toml_text).map_err(|source| ConfigError::Write {
        path: resolved_config_path.clone(),
        source,
    })?;

    let den_result = if opts.ensure_den {
        let resolved_den = match &opts.den_dir {
            Some(p) => resolve_path(&p.to_string_lossy())?,
            None => resolve_path(DEFAULT_DEN_DIR)?,
        };
        crate::den::ensure_den(&resolved_den).map_err(|err| ConfigError::Write {
            path: resolved_den.clone(),
            source: std::io::Error::other(err.to_string()),
        })?;
        Some(resolved_den)
    } else {
        None
    };

    Ok(InitResult {
        config_path: resolved_config_path,
        den_dir: den_result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_toml_parses_into_valid_racc_config() {
        let text = default_toml(Some("/custom/projs"), Some("/custom/den"));
        let parsed: crate::config::RaccConfig = toml::from_str(&text).unwrap();
        assert_eq!(parsed.config_version, 1);
        assert_eq!(parsed.paths.scan_root.as_deref(), Some("/custom/projs"));
        assert_eq!(parsed.paths.den_dir.as_deref(), Some("/custom/den"));
        assert_eq!(parsed.scanner.max_depth, 6);
        assert_eq!(
            parsed.cleanup.enabled_strategies,
            vec!["rust", "node", "python"]
        );
        parsed.validate().unwrap();
    }

    #[test]
    fn default_toml_contains_all_wiki_links() {
        let text = default_toml(None, None);
        assert!(text.contains("https://y-tretyakov.github.io/raccpack/"));
        assert!(text.contains("https://y-tretyakov.github.io/raccpack/configuration.html"));
        assert!(text.contains("https://y-tretyakov.github.io/raccpack/concepts.html"));
        assert!(text.contains("https://y-tretyakov.github.io/raccpack/sniff.html"));
        assert!(text.contains("https://y-tretyakov.github.io/raccpack/rinse.html"));
        assert!(text.contains("https://y-tretyakov.github.io/raccpack/supported.html"));
    }

    #[test]
    fn init_config_creates_new_file_and_den() {
        let tmp = TempDir::new().unwrap();
        let cfg_path = tmp.path().join("sub/config.toml");
        let den_path = tmp.path().join("vault");

        let opts = InitOptions {
            config_path: cfg_path.clone(),
            force: false,
            scan_root: Some(tmp.path().join("src")),
            den_dir: Some(den_path.clone()),
            ensure_den: true,
        };

        let result = init_config(&opts).unwrap();
        assert_eq!(result.config_path, cfg_path);
        assert_eq!(result.den_dir, Some(den_path.clone()));
        assert!(cfg_path.exists());
        assert!(den_path.join(".den-version").exists());
        assert!(den_path.join("README.txt").exists());
    }

    #[test]
    fn init_config_refuses_to_overwrite_without_force() {
        let tmp = TempDir::new().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        std::fs::write(&cfg_path, "existing").unwrap();

        let opts = InitOptions {
            config_path: cfg_path.clone(),
            force: false,
            scan_root: None,
            den_dir: None,
            ensure_den: false,
        };

        let err = init_config(&opts).unwrap_err();
        match err {
            ConfigError::AlreadyExists { path } => assert_eq!(path, cfg_path),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn init_config_overwrites_with_force() {
        let tmp = TempDir::new().unwrap();
        let cfg_path = tmp.path().join("config.toml");
        std::fs::write(&cfg_path, "existing").unwrap();

        let opts = InitOptions {
            config_path: cfg_path.clone(),
            force: true,
            scan_root: None,
            den_dir: None,
            ensure_den: false,
        };

        let result = init_config(&opts).unwrap();
        assert_eq!(result.config_path, cfg_path);
        let content = std::fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("config_version = 1"));
    }
}
