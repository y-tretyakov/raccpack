//! Pack a project tree into a tar+zstd archive.
//!
//! [`pack_tree`] writes `source`'s directory tree to `output` as a single
//! tar+zstd stream, honoring a [`SkipPolicy`] for directories and name/content
//! deny rules for files. The archive root is the *contents* of `source`
//! (entries like `src/main.rs`, not `myproject/src/main.rs`).
//!
//! The tree is walked with an explicit depth-first traversal (`fs::read_dir`,
//! symlinks never followed). Each directory's entries are sorted by name, so
//! archive entry order is deterministic regardless of the OS readdir order.
//!
//! INVARIANT: symlinks are never followed and never archived. `output` is
//! written directly (created/overwritten); atomicity (temp + rename) is the
//! caller's / facade's responsibility (M4.2/M4.3). `output` must NOT be inside
//! `source` — the archive could otherwise include itself while growing; the
//! caller guarantees a staging path outside the tree.

use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

use crate::domain::{Error, Result};
use crate::scan::skip::SkipPolicy;
use crate::scan::walk::ensure_scan_root;

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

    // Explicit DFS with an own stack: each frame lists one directory's entries
    // sorted by lossy name (ascending), so the archive bytes are identical
    // regardless of the OS readdir order. Directories are visited in pre-order
    // (descend immediately when encountered), which keeps entries ascending.
    // Classification uses `DirEntry::file_type()` (does not follow symlinks),
    // so symlinked dirs/files are never descended into and never archived.
    let mut stack = vec![PlanDir {
        path: source.to_path_buf(),
        depth: 0,
        entries: read_sorted_entries(source)?,
        cursor: 0,
    }];
    while let Some(mut plan) = stack.pop() {
        if plan.cursor >= plan.entries.len() {
            continue;
        }
        let (name, file_type) = plan.entries[plan.cursor].clone();
        plan.cursor += 1;
        let parent_path = plan.path.clone();
        let parent_depth = plan.depth;
        stack.push(plan);

        let path = parent_path.join(&name);
        let depth = parent_depth + 1;

        if file_type.is_dir() {
            if opts.policy.should_skip_dir(&path) {
                skipped_dir_names += 1;
            } else if depth < opts.max_depth {
                let entries = read_sorted_entries(&path)?;
                stack.push(PlanDir {
                    path,
                    depth,
                    entries,
                    cursor: 0,
                });
            }
            continue;
        }
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }
        if depth > opts.max_depth {
            continue;
        }
        if opts.deny_name_secrets && should_deny_file_in_pack(&path) {
            skipped_secret_files += 1;
            continue;
        }
        if content_deny_hit(&path, &opts.content_deny)? {
            skipped_secret_files += 1;
            continue;
        }
        let name = relative_posix_name(source, &path)?;
        builder
            .append_path_with_name(&path, name)
            .map_err(|source_err| Error::Io {
                path: path.clone(),
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

/// One directory frame of the depth-first walk.
///
/// `depth` is the directory's own depth (0 = `source`); its children are
/// processed at `depth + 1`. `entries` is the sorted readdir listing.
struct PlanDir {
    path: PathBuf,
    depth: usize,
    entries: Vec<(OsString, fs::FileType)>,
    cursor: usize,
}

/// List `dir`'s entries as `(name, type)` pairs sorted by lossy name ascending.
///
/// Types come from [`std::fs::DirEntry::file_type`], which does NOT follow
/// symlinks. Read-dir and per-entry `file_type` failures map to [`Error::Io`].
fn read_sorted_entries(dir: &Path) -> Result<Vec<(OsString, fs::FileType)>> {
    let mut entries: Vec<(OsString, fs::FileType)> = fs::read_dir(dir)
        .map_err(|source| Error::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .map(|item| {
            let entry = item.map_err(|source| Error::Io {
                path: dir.to_path_buf(),
                source,
            })?;
            let file_type = entry.file_type().map_err(|source| Error::Io {
                path: entry.path(),
                source,
            })?;
            Ok((entry.file_name(), file_type))
        })
        .collect::<Result<_>>()?;
    entries.sort_by(|a, b| a.0.to_string_lossy().cmp(&b.0.to_string_lossy()));
    Ok(entries)
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

    fn unpack_names(path: &Path) -> Vec<String> {
        let file = File::open(path).unwrap();
        let decoder = zstd::stream::read::Decoder::new(file).unwrap();
        let mut archive = tar::Archive::new(decoder);
        archive
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
            .collect()
    }

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

    #[test]
    fn policy_pruned_dir_is_counted_and_omitted() {
        let dir = tempfile::TempDir::new().unwrap();
        let source = dir.path().join("proj");
        std::fs::create_dir_all(source.join("target/debug")).unwrap();
        std::fs::write(source.join("target/debug/x"), b"bytes\n").unwrap();
        std::fs::create_dir_all(source.join("src")).unwrap();
        std::fs::write(source.join("src/main.rs"), b"fn main() {}\n").unwrap();

        let output = dir.path().join("out.tar.zst");
        let result = pack_tree(&source, &output, &PackTreeOptions::default()).unwrap();

        assert_eq!(result.skipped_dir_names, 1);
        let names = unpack_names(&output);
        assert_eq!(
            names,
            vec!["src/main.rs"],
            "skipped dir subtree must be absent"
        );
    }

    #[test]
    fn archive_entries_are_deterministically_sorted() {
        let dir = tempfile::TempDir::new().unwrap();
        let source = dir.path().join("proj");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(source.join("src")).unwrap();
        // Created in reverse-sorted order so an unwrapped readdir would return
        // them unsorted; the walker must still emit ascending names.
        std::fs::write(source.join("z.txt"), b"z\n").unwrap();
        std::fs::write(source.join("m.txt"), b"m\n").unwrap();
        std::fs::write(source.join("a.txt"), b"a\n").unwrap();
        std::fs::write(source.join("src/main.rs"), b"fn main() {}\n").unwrap();

        let output = dir.path().join("out.tar.zst");
        let result = pack_tree(&source, &output, &PackTreeOptions::default()).unwrap();

        assert_eq!(result.file_count, 4);
        let names = unpack_names(&output);
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(
            names, sorted,
            "archive entry order must be ascending: {names:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_dir_is_neither_descended_nor_archived() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::TempDir::new().unwrap();
        let source = dir.path().join("proj");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(source.join("real")).unwrap();
        std::fs::write(source.join("real/in.txt"), b"ok\n").unwrap();
        std::fs::write(source.join("root.txt"), b"r\n").unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("leak.txt"), b"SECRET\n").unwrap();
        symlink(&outside, source.join("link")).unwrap();

        let output = dir.path().join("out.tar.zst");
        let result = pack_tree(&source, &output, &PackTreeOptions::default()).unwrap();

        let names = unpack_names(&output);
        assert!(
            !names
                .iter()
                .any(|n| n.starts_with("link") || n == "leak.txt"),
            "symlinked dir must not be followed or archived: {names:?}"
        );
        assert_eq!(result.skipped_dir_names, 0, "a symlink is not a pruned dir");
        assert_eq!(result.file_count, 2);
    }
}
