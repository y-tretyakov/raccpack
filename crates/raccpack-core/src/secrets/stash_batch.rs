//! Assemble selected files into one age-encrypted tar batch.
//!
//! [`write_stash_age`] packs [`StashFileEntry`]s into a single ustar archive in
//! memory (entry names are the relative, POSIX-style paths — never absolute,
//! never containing `..`), encrypts the whole tar with [`crate::archive::age_vault`],
//! and returns a [`StashBatchResult`] with stats plus a raw-free
//! [`StashManifestEntry`] list for later JSON serialization by the facade.
//!
//! Sources are NOT removed here: that is the explicit,
//! separate [`crate::secrets::stash_remove::remove_stash_sources`] call (Commit
//! semantics only). If encryption fails, `encrypt_bytes_to_file` already
//! leaves no partial `output_age` behind.
//!
//! INVARIANTS:
//!
//! - **No TOCTOU size drift**: the ustar header size is taken from the *open*
//!   file's `metadata().len()`, not from the selection-time
//!   `StashFileEntry::size_bytes`, so a file changed between select and batch
//!   cannot produce a truncated/corrupt tar. The manifest mirrors the actual
//!   archived size.
//! - **0600 end-to-end**: tar entry mode is `0o600` (secrets) and `output_age`
//!   is `chmod`ed `0o600` after encryption (best-effort on Unix).
//! - The whole tar is buffered in memory (Alpha scope); streaming
//!   tar → age writer is a later optimization.

use std::fs;
use std::fs::File;
use std::path::{Component, Path, PathBuf};

use zeroize::Zeroizing;

use crate::archive::encrypt_bytes_to_file;
use crate::domain::{Error, SensitiveRisk};

use super::stash_select::StashFileEntry;

/// One raw-free manifest record for an archived source file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StashManifestEntry {
    /// Absolute path of the archived source file.
    pub original_path: PathBuf,
    /// Folded severity at selection time.
    pub risk: SensitiveRisk,
    /// Plaintext byte size of the source file.
    pub size_bytes: u64,
}

/// Statistics for a completed stash batch.
#[derive(Debug, Clone)]
pub struct StashBatchResult {
    /// Where the encrypted `.age` file was written (staging path).
    pub age_path: PathBuf,
    /// Number of files appended to the tar.
    pub files_archived: usize,
    /// Sum of plaintext source sizes, in bytes.
    pub bytes_archived: u64,
    /// Raw-free manifest aligned with the tar entries.
    pub manifest: Vec<StashManifestEntry>,
}

/// Pack `entries` into a ustar tar, encrypt it with `passphrase`, and write the
/// result to `output_age`.
///
/// # Algorithm
///
/// 1. An empty `entries` slice is [`Error::StashEmpty`].
/// 2. A `tar::Builder` over an in-memory buffer appends each entry under a
///    freshly built [`tar::Header::new_ustar`] header (POSIX ustar), using its
///    POSIX-relative name (components joined with `/`, no `..`, no leading `/`,
///    no `./`). The header size is the *open file's* `metadata().len()` (not
///    the selection-time size), so a file mutated between select and batch
///    cannot truncate the tar. Open, metadata, and tar errors map to
///    [`Error::Io`] on the offending path. Entry mode is `0o600`.
/// 3. `finish()` + `into_inner()` yield the tar bytes.
/// 4. [`encrypt_bytes_to_file`] writes the age file atomically; the passphrase
///    is zeroized by the age backend, never appears in errors. The finished
///    `output_age` is then `chmod`ed `0o600` (best-effort on Unix).
/// 5. The manifest mirrors `entries` using the actual archived sizes;
///    `bytes_archived` is the sum of those sizes.
///
/// The `output_age` staging path is returned in
/// [`StashBatchResult::age_path`]; the caller decides where to place it
/// (e.g. den `secrets/`) and whether to remove the sources.
pub fn write_stash_age(
    entries: &[StashFileEntry],
    output_age: &Path,
    passphrase: &Zeroizing<String>,
) -> Result<StashBatchResult, Error> {
    if entries.is_empty() {
        return Err(Error::StashEmpty {
            message: "no files matched the current min-risk threshold".into(),
        });
    }

    let mut builder = tar::Builder::new(Vec::new());

    let mut bytes_archived = 0u64;
    let mut manifest = Vec::with_capacity(entries.len());
    for entry in entries {
        let mut file = File::open(&entry.path).map_err(|source| Error::Io {
            path: entry.path.clone(),
            source,
        })?;
        let actual_len = file
            .metadata()
            .map_err(|source| Error::Io {
                path: entry.path.clone(),
                source,
            })?
            .len();
        let name = posix_archive_name(&entry.relative_path)?;
        let mut header = tar::Header::new_ustar();
        header.set_size(actual_len);
        header.set_mode(0o600);
        header.set_mtime(0);
        header.set_entry_type(tar::EntryType::Regular);
        builder
            .append_data(&mut header, name, &mut file)
            .map_err(|source| Error::Io {
                path: entry.path.clone(),
                source,
            })?;
        bytes_archived += actual_len;
        manifest.push(StashManifestEntry {
            original_path: entry.path.clone(),
            risk: entry.risk,
            size_bytes: actual_len,
        });
    }

    builder.finish().map_err(|source| Error::Io {
        path: output_age.to_path_buf(),
        source,
    })?;
    let tar_bytes = builder.into_inner().map_err(|source| Error::Io {
        path: output_age.to_path_buf(),
        source,
    })?;

    encrypt_bytes_to_file(&tar_bytes, output_age, passphrase)?;
    set_secrets_file_mode(output_age);

    Ok(StashBatchResult {
        age_path: output_age.to_path_buf(),
        files_archived: entries.len(),
        bytes_archived,
        manifest,
    })
}

/// Convert a relative `PathBuf` to a POSIX archive name.
///
/// Components are joined with `/`; `ParentDir`, `RootDir`, and drive prefixes
/// are rejected with [`Error::Other`] so archive entries never escape the stash
/// target even if a caller hand-builds a [`StashFileEntry`].
fn posix_archive_name(relative: &Path) -> Result<String, Error> {
    let mut parts: Vec<String> = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(Error::Other {
                    message: format!("path escapes stash target: {}", relative.display()),
                });
            }
        }
    }
    Ok(parts.join("/"))
}

/// Best-effort `chmod 0600` for a finished `.age` file (Unix only).
///
/// Mirrors `den::layout::set_mode_best_effort`: a failed `chmod` is ignored,
/// never fatal — the vault is still encrypted. On non-Unix platforms this is a
/// no-op (permissions are documented as a Unix recommendation). The facade's
/// den `secrets/` placement (A1.3) applies the authoritative `0600`.
fn set_secrets_file_mode(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(path) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o600);
            let _ = fs::set_permissions(path, permissions);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Read;

    use super::*;
    use crate::archive::age_vault::decrypt_file_from_age;
    use crate::secrets::stash_select::{select_files_for_stash, StashSelectOptions};
    use tempfile::TempDir;

    const TEST_PASSPHRASE: &str = "correct horse battery staple a1";

    fn passphrase() -> Zeroizing<String> {
        Zeroizing::new(TEST_PASSPHRASE.to_string())
    }

    #[test]
    fn empty_selection_is_stash_empty() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("empty.age");

        let err = write_stash_age(&[], &output, &passphrase()).unwrap_err();
        assert!(matches!(err, Error::StashEmpty { .. }));
    }

    #[test]
    fn roundtrip_tar_encrypt_preserves_content_and_relative_name() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(target.join("deep/sub")).unwrap();
        fs::write(target.join("deep/sub/.env"), b"TOKEN=super-secret\n").unwrap();

        let opts = StashSelectOptions {
            target: target.clone(),
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();
        assert_eq!(entries.len(), 1);

        let output = dir.path().join("stash.age");
        let result = write_stash_age(&entries, &output, &passphrase()).unwrap();

        assert_eq!(result.age_path, output);
        assert_eq!(result.files_archived, 1);
        assert_eq!(result.bytes_archived, entries[0].size_bytes);
        assert_eq!(result.manifest.len(), 1);
        assert_eq!(result.manifest[0].original_path, entries[0].path);
        assert_eq!(result.manifest[0].risk, entries[0].risk);

        let plaintext = decrypt_file_from_age(&output, &passphrase()).unwrap();
        let mut archive = tar::Archive::new(&plaintext[..]);
        let mut entries_in_tar = Vec::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let name = entry.path().unwrap().to_string_lossy().into_owned();
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).unwrap();
            entries_in_tar.push((name, contents));
        }

        assert_eq!(entries_in_tar.len(), 1);
        let (name, contents) = &entries_in_tar[0];
        assert_eq!(name, "deep/sub/.env");
        assert!(
            !name.contains(".."),
            "tar entry must not contain `..`: {name}"
        );
        assert_eq!(contents, b"TOKEN=super-secret\n");
    }

    #[test]
    fn manifest_serde_has_no_raw_values() {
        let dir = TempDir::new().unwrap();
        let entry = StashManifestEntry {
            original_path: dir.path().join("proj/.env"),
            risk: SensitiveRisk::High,
            size_bytes: 16,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("proj/.env"));
        assert!(json.contains("High"));
        assert!(json.contains("16"));
    }

    #[test]
    fn posix_name_rejects_parent_dir() {
        let err = posix_archive_name(Path::new("a/../b")).unwrap_err();
        assert!(err.to_string().contains("escapes"));
    }

    #[test]
    fn header_size_tracks_actual_file_len() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        let source = target.join(".env");
        fs::write(&source, b"TOKEN=abc\n").unwrap();

        let opts = StashSelectOptions {
            target: target.clone(),
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();
        assert_eq!(entries[0].size_bytes, 10);

        // Selection-time size differs from the on-disk size: the batch must
        // use the open file's length so the tar stays consistent.
        fs::write(&source, b"TOKEN=abc\nTOKEN=def\n").unwrap();

        let output = dir.path().join("stash.age");
        let result = write_stash_age(&entries, &output, &passphrase()).unwrap();

        assert_eq!(result.bytes_archived, 20);
        assert_eq!(result.manifest[0].size_bytes, 20);

        let plaintext = decrypt_file_from_age(&output, &passphrase()).unwrap();
        let mut archive = tar::Archive::new(&plaintext[..]);
        for item in archive.entries().unwrap() {
            let mut entry = item.unwrap();
            assert_eq!(entry.size(), 20);
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).unwrap();
            assert_eq!(contents, b"TOKEN=abc\nTOKEN=def\n");
        }
    }

    #[cfg(unix)]
    #[test]
    fn age_output_mode_is_0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().unwrap();
        let target = dir.path().join("proj");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join(".env"), b"TOKEN=secret\n").unwrap();

        let opts = StashSelectOptions {
            target: target.clone(),
            ..StashSelectOptions::default()
        };
        let entries = select_files_for_stash(&opts).unwrap();
        let output = dir.path().join("stash.age");
        write_stash_age(&entries, &output, &passphrase()).unwrap();

        let mode = fs::metadata(&output).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "staged .age must be 0600, got {mode:o}");
    }
}
