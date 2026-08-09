//! Pack a project tree into a tar+zstd archive.
//!
//! [`pack_tree`] writes `source`'s directory tree to `output` as a single
//! tar+zstd stream, honoring a [`SkipPolicy`] for directories and name/content
//! deny rules for files. The archive root is the *contents* of `source`
//! (entries like `src/main.rs`, not `myproject/src/main.rs`).
//!
//! INVARIANT: symlinks are never followed and never archived. `output` is
//! written directly (created/overwritten); atomicity (temp + rename) is the
//! caller's / facade's responsibility (M4.2/M4.3). `output` must NOT be inside
//! `source` — the archive could otherwise include itself while growing; the
//! caller guarantees a staging path outside the tree.

use std::fs::File;
use std::path::{Component, Path, PathBuf};

use walkdir::WalkDir;

use crate::domain::{Error, Result};
use crate::scan::skip::SkipPolicy;
use crate::scan::walk::{ensure_scan_root, map_walk_error};

use super::deny::{content_deny_hit, should_deny_file_in_pack, ContentDenyOptions};

/// Options controlling [`pack_tree`].
#[derive(Debug, Clone)]
pub struct PackTreeOptions {
    /// Policy deciding which directories are pruned from the archive.
    pub policy: SkipPolicy,
    /// Maximum directory depth to descend (0 packs only files at the root).
    pub max_depth: usize,
    /// zstd compression level passed to the encoder.
    pub zstd_level: i32,
    /// When true, omit files whose name matches a secret pattern (risk ≥ High).
    pub deny_name_secrets: bool,
    /// Optional content-based deny (off by default in M4.1).
    pub content_deny: ContentDenyOptions,
}

impl Default for PackTreeOptions {
    fn default() -> Self {
        Self {
            policy: SkipPolicy::default_scan(),
            max_depth: 64,
            zstd_level: 3,
            deny_name_secrets: true,
            content_deny: ContentDenyOptions::default(),
        }
    }
}

/// Statistics from a [`pack_tree`] run.
#[derive(Debug, Clone)]
pub struct PackTreeResult {
    /// Path of the written archive (caller-provided destination).
    pub output: PathBuf,
    /// Byte size of the finished archive on disk.
    pub size_bytes: u64,
    /// Number of regular files appended to the archive.
    pub file_count: usize,
    /// Number of files omitted by name or content deny rules.
    pub skipped_secret_files: usize,
    /// Number of directories pruned by `policy`.
    pub skipped_dir_names: usize,
}

/// Pack `source` into `output` as tar+zstd.
///
/// # Contract
///
/// - Archive root = contents of `source` (entries like `src/main.rs`, NOT
///   `myproject/src/main.rs`).
/// - Symlinks are never followed and never archived (skipped).
/// - `output` is written directly (created/overwritten); atomicity (temp +
///   rename) is the caller's / facade's responsibility (M4.2/M4.3).
/// - `output` must NOT be inside `source` — the archive could include itself
///   while growing; the caller guarantees a staging path outside the tree.
/// - [`PackTreeOptions::policy`] prunes directories; name-deny omits High+
///   secret filenames; content-deny (off by default in M4.1) omits files with a
///   content hit ≥ `min_risk`.
/// - Empty directories are not preserved (M4.1 archives regular files only).
///
/// # Errors
///
/// `source` missing → [`Error::PathNotFound`]; `source` not a directory →
/// [`Error::NotADirectory`]; output creation, archive I/O, or mid-pack walk
/// failures → [`Error::Io`] (a partially written `output` may remain; the
/// caller deletes it on error).
pub fn pack_tree(source: &Path, output: &Path, opts: &PackTreeOptions) -> Result<PackTreeResult> {
    ensure_scan_root(source)?;

    let file = File::create(output).map_err(|source_err| Error::Io {
        path: output.to_path_buf(),
        source: source_err,
    })?;
    let encoder =
        zstd::stream::write::Encoder::new(file, opts.zstd_level).map_err(|source_err| {
            Error::Io {
                path: output.to_path_buf(),
                source: source_err,
            }
        })?;
    let mut builder = tar::Builder::new(encoder);

    let mut skipped_dir_names = 0usize;
    let mut skipped_secret_files = 0usize;
    let mut file_count = 0usize;

    let walker = WalkDir::new(source)
        .follow_links(false)
        .max_depth(opts.max_depth)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() > 0
                && entry.file_type().is_dir()
                && opts.policy.should_skip_dir(entry.path())
            {
                skipped_dir_names += 1;
                return false;
            }
            true
        });

    for item in walker {
        let entry = item.map_err(|err| map_walk_error(err, source))?;
        if entry.depth() == 0 {
            continue;
        }
        let file_type = entry.file_type();
        if file_type.is_dir() {
            continue;
        }
        if file_type.is_symlink() {
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        if opts.deny_name_secrets && should_deny_file_in_pack(entry.path()) {
            skipped_secret_files += 1;
            continue;
        }
        if content_deny_hit(entry.path(), &opts.content_deny)? {
            skipped_secret_files += 1;
            continue;
        }
        let name = relative_posix_name(source, entry.path())?;
        builder
            .append_path_with_name(entry.path(), name)
            .map_err(|source_err| Error::Io {
                path: entry.path().to_path_buf(),
                source: source_err,
            })?;
        file_count += 1;
    }

    builder.finish().map_err(|source_err| Error::Io {
        path: output.to_path_buf(),
        source: source_err,
    })?;
    let encoder = builder.into_inner().map_err(|source_err| Error::Io {
        path: output.to_path_buf(),
        source: source_err,
    })?;
    let file = encoder.finish().map_err(|source_err| Error::Io {
        path: output.to_path_buf(),
        source: source_err,
    })?;
    let size_bytes = file
        .metadata()
        .map_err(|source_err| Error::Io {
            path: output.to_path_buf(),
            source: source_err,
        })?
        .len();

    Ok(PackTreeResult {
        output: output.to_path_buf(),
        size_bytes,
        file_count,
        skipped_secret_files,
        skipped_dir_names,
    })
}

/// Build the POSIX relative archive name for `path` under `source`.
///
/// Components are rejoined with `/`. Any path that would contain `ParentDir`,
/// `RootDir`, or `Prefix` components after stripping `source` is rejected with
/// [`Error::Other`] so archive paths never escape the pack root.
fn relative_posix_name(source: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(source).map_err(|_| Error::Other {
        message: format!("path escapes pack root: {}", path.display()),
    })?;
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(Error::Other {
                    message: format!("archive path would escape pack root: {}", path.display()),
                });
            }
        }
    }
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_options_defaults() {
        let opts = PackTreeOptions::default();
        assert_eq!(opts.max_depth, 64);
        assert_eq!(opts.zstd_level, 3);
        assert!(opts.deny_name_secrets);
        assert!(!opts.content_deny.enabled);
        assert_eq!(
            opts.content_deny.min_risk,
            crate::domain::SensitiveRisk::Critical
        );
    }

    #[test]
    fn relative_name_is_posix() {
        let root = Path::new("/tmp/proj");
        assert_eq!(
            relative_posix_name(root, Path::new("/tmp/proj/src/main.rs")).unwrap(),
            "src/main.rs"
        );
    }

    #[test]
    fn relative_name_rejects_escape() {
        let root = Path::new("/tmp/proj");
        assert!(relative_posix_name(root, Path::new("/tmp/proj/../escape")).is_err());
        assert!(relative_posix_name(root, Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn pack_tree_writes_valid_archive_with_deny() {
        let dir = tempfile::TempDir::new().unwrap();
        let source = dir.path().join("proj");
        std::fs::create_dir_all(source.join("src")).unwrap();
        std::fs::write(source.join("src/main.rs"), b"fn main() {}\n").unwrap();
        std::fs::write(source.join(".env"), b"TOKEN=secret\n").unwrap();
        std::fs::write(source.join("notes.txt"), b"hi\n").unwrap();

        let output = dir.path().join("out.tar.zst");
        let result = pack_tree(&source, &output, &PackTreeOptions::default()).unwrap();

        assert_eq!(result.output, output);
        assert_eq!(result.file_count, 2);
        assert_eq!(result.skipped_secret_files, 1);
        assert!(result.size_bytes > 0);

        let file = File::open(&output).unwrap();
        let decoder = zstd::stream::read::Decoder::new(file).unwrap();
        let mut archive = tar::Archive::new(decoder);
        let mut names: Vec<String> = archive
            .entries()
            .unwrap()
            .map(|entry| {
                entry
                    .unwrap()
                    .path()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        assert_eq!(names, vec!["notes.txt", "src/main.rs"]);
    }
}
