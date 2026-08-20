//! Pack artifact naming: final relative den path and collision handling.
//!
//! [`artifact_rel`] derives the final `packs/{yyyy}/{mm}/…` relative path,
//! honoring a custom output name; [`resolve_artifact_name`] appends a short-id
//! suffix on collision so two packs in the same second never overwrite each
//! other.

use std::path::{Path, PathBuf};

use crate::den::{pack_relative_path, short_id};
use crate::domain::{Error, Result};

/// Final relative den path for a pack, honoring a custom output name.
///
/// `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst`, or `packs/{yyyy}/{mm}/{name}.tar.zst`
/// when `output_name` is set (year/month still derived from `ts`).
pub(super) fn artifact_rel(slug: &str, ts: &str, output_name: Option<&str>) -> PathBuf {
    let rel = pack_relative_path(slug, ts);
    match output_name {
        Some(name) => rel.with_file_name(format!("{name}.tar.zst")),
        None => rel,
    }
}

/// Resolve the final artifact naming, appending a short-id suffix on collision.
///
/// Returns `(final_ts, final_output_name)`. When the expected target already
/// exists, the short-id suffix is appended to the timestamp (auto-name) or to
/// the custom name; an existing target after that (astronomically unlikely)
/// fails with [`Error::Other`].
pub(super) fn resolve_artifact_name(
    den: &Path,
    slug: &str,
    ts: &str,
    output_name: Option<&str>,
) -> Result<(String, Option<String>)> {
    if !den.join(artifact_rel(slug, ts, output_name)).exists() {
        return Ok((ts.to_string(), output_name.map(str::to_string)));
    }
    let (final_ts, final_name) = match output_name {
        Some(name) => (ts.to_string(), Some(format!("{name}__{}", short_id()))),
        None => (format!("{ts}__{}", short_id()), None),
    };
    let rel = artifact_rel(slug, &final_ts, final_name.as_deref());
    if den.join(rel).exists() {
        return Err(Error::Other {
            message: "pack artifact name collision under den".to_string(),
        });
    }
    Ok((final_ts, final_name))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn artifact_rel_honors_custom_name() {
        assert_eq!(
            artifact_rel("my-api", "20260804T155230Z", None),
            PathBuf::from("packs/2026/08/my-api__20260804T155230Z.tar.zst")
        );
        assert_eq!(
            artifact_rel("my-api", "20260804T155230Z", Some("snapshot")),
            PathBuf::from("packs/2026/08/snapshot.tar.zst")
        );
    }

    #[test]
    fn resolve_keeps_name_when_target_is_free() {
        let den = tempfile::TempDir::new().unwrap();
        let (ts, name) =
            resolve_artifact_name(den.path(), "my-api", "20260804T155230Z", Some("snapshot"))
                .unwrap();
        assert_eq!(ts, "20260804T155230Z");
        assert_eq!(name.as_deref(), Some("snapshot"));
    }

    #[test]
    fn resolve_appends_suffix_on_auto_name_collision() {
        let den = tempfile::TempDir::new().unwrap();
        let target = den
            .path()
            .join(artifact_rel("my-api", "20260804T155230Z", None));
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"existing").unwrap();

        let (ts, name) =
            resolve_artifact_name(den.path(), "my-api", "20260804T155230Z", None).unwrap();
        assert_ne!(ts, "20260804T155230Z");
        assert!(ts.contains("Z__"), "suffix must trail the Z: {ts}");
        assert!(name.is_none());
    }

    #[test]
    fn resolve_appends_suffix_on_custom_name_collision() {
        let den = tempfile::TempDir::new().unwrap();
        let target = den
            .path()
            .join(artifact_rel("my-api", "20260804T155230Z", Some("snapshot")));
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, b"existing").unwrap();

        let (ts, name) =
            resolve_artifact_name(den.path(), "my-api", "20260804T155230Z", Some("snapshot"))
                .unwrap();
        assert_eq!(ts, "20260804T155230Z");
        let name = name.expect("custom name survives the suffix");
        assert!(
            name.starts_with("snapshot__"),
            "name must gain a suffix: {name}"
        );
    }
}
