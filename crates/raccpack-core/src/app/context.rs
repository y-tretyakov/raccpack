use std::path::PathBuf;

use crate::config::{ConfigError, RaccConfig};

/// Resolved, ready-to-use workspace paths.
///
/// Filled by [`AppContext::from_config`] from the raw path settings of the
/// [`RaccConfig`]. Both paths are absolute (config resolution expands `~` and
/// relative values) but not canonicalized.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkspacePaths {
    /// Absolute directory that contains the projects to scan.
    pub scan_root: PathBuf,
    /// Absolute directory that stores packs and (later) secret archives.
    pub den_dir: PathBuf,
}

/// Execution mode of a facade use-case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RunMode {
    /// Report what a run would do without performing destructive side effects.
    DryRun,
    /// Perform the operation and report back.
    Commit,
}

impl RunMode {
    /// Whether this mode is a dry run.
    pub fn is_dry_run(&self) -> bool {
        matches!(self, Self::DryRun)
    }
}

/// Policy for how sensitive findings affect the run / exit status.
///
/// Used by the secret phases (M3.x); `sniff` never inspects content, so the
/// policy has no effect on this stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SecretExitPolicy {
    /// Sensitive findings never change the outcome.
    Ignore,
    /// A CRITICAL-severity finding fails the run.
    FailOnCritical,
    /// Any HIGH-severity-or-above finding fails the run.
    FailOnHighOrAbove,
}

/// Application context passed to every facade use-case.
///
/// # Note on `secret_groups_override`
///
/// The field `secret_groups_override: Option<EnabledGroups>` from the facade
/// spec is intentionally **not** present yet: the `EnabledGroups` type does not
/// exist until the secret phase (M3.x). It will be added additively together
/// with that type; no placeholder types are introduced on this stage.
#[derive(Debug, Clone)]
pub struct AppContext {
    /// Resolved configuration.
    pub config: RaccConfig,
    /// Resolved absolute workspace paths.
    pub paths: WorkspacePaths,
    /// Execution mode.
    pub mode: RunMode,
    /// Exit-policy for sensitive findings (used by later phases).
    pub exit_policy: SecretExitPolicy,
}

impl AppContext {
    /// Build an [`AppContext`] from a resolved config.
    ///
    /// Resolves `scan_root` and `den_dir` via [`RaccConfig::scan_root_dir`] /
    /// [`RaccConfig::den_dir`]; a missing or unusable `scan_root` fails with
    /// [`ConfigError::MissingScanRoot`] / [`ConfigError::ScanRootMissing`].
    /// `exit_policy` defaults to [`SecretExitPolicy::FailOnCritical`].
    pub fn from_config(config: RaccConfig, mode: RunMode) -> Result<Self, ConfigError> {
        let paths = WorkspacePaths {
            scan_root: config.scan_root_dir()?,
            den_dir: config.den_dir()?,
        };
        Ok(Self {
            config,
            paths,
            mode,
            exit_policy: SecretExitPolicy::FailOnCritical,
        })
    }
}
