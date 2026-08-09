//! Den skeleton: version gate, README, and layout directories.
//!
//! [`ensure_den`] creates `{root}/{packs,staging,manifests,secrets}`, writes
//! `.den-version` and `README.txt` when they are missing, and refuses to work
//! with an existing den whose major version is incompatible.
//!
//! INVARIANTS:
//!
//! - [`ensure_den`] is idempotent: a second call on an initialized den is a
//!   no-op for `.den-version` / `README.txt` and never fails on version "1".
//! - The version gate is a major-only check: `"1"`, `"1.5"` pass; `"2"`,
//!   `"99"`, and unparsable content fail with
//!   [`crate::Error::DenVersion`].
//! - Permissions are best-effort: a failed `chmod` on Unix is ignored, never
//!   fatal; on non-Unix platforms it is skipped entirely.
//!
//! [`staging_pack_path`] is the layout path used for in-progress archive
//! writes before [`crate::den::place_pack`] moves them into `packs/`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{Error, Result};

/// Current den layout major version (content of `.den-version`).
pub const DEN_VERSION: &str = "1";

/// Name of the version marker file at the den root.
const DEN_VERSION_FILE: &str = ".den-version";

/// Name of the human-readable README at the den root.
const README_FILE: &str = "README.txt";

/// Fixed README template (facade-and-den §9.6 / M4.2 §4.1).
const README_TEMPLATE: &str = "This directory is a raccpack den (output vault).\n\
- secrets/  encrypted secret batches (age)\n\
- packs/    project archives (no secrets)\n\
- manifests/ JSON metadata for each raid\n\
\n\
Do not commit this tree to git.\n\
Keep passphrase offline.\n";

/// Paths of the den skeleton, all rooted at the given den root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenPaths {
    /// The den root itself.
    pub root: PathBuf,
    /// `root/packs` — finished project archives.
    pub packs: PathBuf,
    /// `root/staging` — temporary in-progress files.
    pub staging: PathBuf,
    /// `root/manifests` — JSON metadata per raid (A3).
    pub manifests: PathBuf,
    /// `root/secrets` — encrypted secret batches (Alpha).
    pub secrets: PathBuf,
}

/// Create the den skeleton under `den_root` if missing. Idempotent.
///
/// Writes `.den-version` and `README.txt` only when they are absent, and
/// rejects an existing den whose major version differs from [`DEN_VERSION`]
/// with [`Error::DenVersion`]. Returns the resolved [`DenPaths`].
///
/// # Errors
///
/// - [`Error::Io`] on any filesystem failure while creating directories or
///   writing the marker/README files.
/// - [`Error::DenVersion`] when `.den-version` exists with an incompatible
///   major version or unparsable content.
pub fn ensure_den(den_root: &Path) -> Result<DenPaths> {
    create_dir_all(den_root)?;
    set_mode_best_effort(den_root, 0o700);
    check_version_gate(den_root)?;
    write_readme_if_absent(den_root)?;

    let packs = den_root.join("packs");
    let staging = den_root.join("staging");
    let manifests = den_root.join("manifests");
    let secrets = den_root.join("secrets");
    for dir in [&packs, &staging, &manifests, &secrets] {
        create_dir_all(dir)?;
    }

    Ok(DenPaths {
        root: den_root.to_path_buf(),
        packs,
        staging,
        manifests,
        secrets,
    })
}

/// `staging/{short_id}/pack.tar.zst` — path for in-progress archive writes.
///
/// The caller creates the parent directory and writes a partial archive here;
/// [`crate::den::place_pack`] later renames it into `packs/…`.
pub fn staging_pack_path(den_root: &Path, short_id: &str) -> PathBuf {
    den_root.join("staging").join(short_id).join("pack.tar.zst")
}

/// Create `path` (and parents) mapping failures to [`Error::Io`].
pub(crate) fn create_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Best-effort `chmod` on Unix; no-op elsewhere. Failures are ignored.
pub(crate) fn set_mode_best_effort(path: &Path, mode: u32) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(mode);
            let _ = fs::set_permissions(path, permissions);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
}

/// Write `.den-version` when absent, otherwise verify its major version.
fn check_version_gate(den_root: &Path) -> Result<()> {
    let version_file = den_root.join(DEN_VERSION_FILE);
    if !version_file.exists() {
        let content = format!("{DEN_VERSION}\n");
        return fs::write(&version_file, content).map_err(|source| Error::Io {
            path: version_file,
            source,
        });
    }

    let found = fs::read_to_string(&version_file).map_err(|source| Error::Io {
        path: version_file,
        source,
    })?;
    let found = found.trim();
    let Some(found_major) = parse_major(found) else {
        return Err(Error::DenVersion {
            found: found.to_string(),
            expected: DEN_VERSION,
        });
    };
    let expected_major = parse_major(DEN_VERSION).unwrap_or(0);
    if found_major == expected_major {
        Ok(())
    } else {
        Err(Error::DenVersion {
            found: found.to_string(),
            expected: DEN_VERSION,
        })
    }
}

/// Write `README.txt` when absent; existing files are left untouched.
fn write_readme_if_absent(den_root: &Path) -> Result<()> {
    let readme = den_root.join(README_FILE);
    if readme.exists() {
        return Ok(());
    }
    fs::write(&readme, README_TEMPLATE).map_err(|source| Error::Io {
        path: readme,
        source,
    })
}

/// Integer part before the first `.` of a version string, if it parses.
fn parse_major(version: &str) -> Option<u64> {
    let head = version.trim().split('.').next().unwrap_or("");
    head.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_den_creates_skeleton_and_is_idempotent() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        let first = ensure_den(root).unwrap();
        assert_eq!(first.root, root);
        assert_eq!(first.packs, root.join("packs"));
        assert_eq!(first.staging, root.join("staging"));
        assert_eq!(first.manifests, root.join("manifests"));
        assert_eq!(first.secrets, root.join("secrets"));
        for dir_name in ["packs", "staging", "manifests", "secrets"] {
            assert!(root.join(dir_name).is_dir());
        }

        assert_eq!(
            std::fs::read_to_string(root.join(".den-version")).unwrap(),
            "1\n"
        );
        let readme = std::fs::read_to_string(root.join("README.txt")).unwrap();
        assert!(readme.contains("This directory is a raccpack den (output vault)."));
        assert!(readme.contains("- manifests/ JSON metadata for each raid"));
        assert!(readme.contains("Do not commit this tree to git."));
        assert!(readme.contains("Keep passphrase offline."));

        let second = ensure_den(root).unwrap();
        assert_eq!(second.packs, first.packs);
        assert_eq!(
            std::fs::read_to_string(root.join(".den-version")).unwrap(),
            "1\n"
        );
    }

    #[test]
    fn ensure_den_rejects_incompatible_major() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join(".den-version"), "99\n").unwrap();

        match ensure_den(root) {
            Err(Error::DenVersion { found, expected }) => {
                assert_eq!(found, "99");
                assert_eq!(expected, DEN_VERSION);
            }
            Err(other) => panic!("expected DenVersion, got {other:?}"),
            Ok(_) => panic!("expected DenVersion error"),
        }
    }

    #[test]
    fn ensure_den_accepts_same_major_with_suffix() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join(".den-version"), "1.5\n").unwrap();
        ensure_den(root).unwrap();
    }

    #[test]
    fn ensure_den_rejects_garbage_version() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join(".den-version"), "garbage\n").unwrap();
        assert!(matches!(ensure_den(root), Err(Error::DenVersion { .. })));
    }

    #[test]
    fn ensure_den_never_rewrites_existing_readme() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("README.txt"), "custom\n").unwrap();

        ensure_den(root).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("README.txt")).unwrap(),
            "custom\n"
        );
    }

    #[test]
    fn staging_path_is_under_staging() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = staging_pack_path(dir.path(), "a1b2c3d4");
        assert_eq!(p, dir.path().join("staging/a1b2c3d4/pack.tar.zst"));
    }
}
