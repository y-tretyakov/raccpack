//! Encrypt secrets with age (scrypt passphrase identity).
//!
//! [`encrypt_bytes_to_file`] and [`encrypt_file_to_age`] write age-encrypted
//! vault files keyed by a human-supplied passphrase. The age backend uses the
//! **binary** age file format (no ASCII armor) to keep vaults small; the `armor`
//! feature of the `age` crate is intentionally not enabled.
//!
//! INVARIANTS:
//!
//! - **Zeroize**: the passphrase is copied into a `secrecy::SecretString` for
//!   the age backend; both the caller's `Zeroizing<String>` and the internal
//!   `SecretString` wipe their bytes on drop. No copy of the passphrase is
//!   stored in any struct beyond the duration of one call.
//! - **No passphrase in errors**: `Error::Encrypt` / `Error::Io` messages never
//!   contain the passphrase; age errors are mapped to their `Display` text,
//!   which is passphrase-free. An empty passphrase is rejected with a static
//!   [`Error::Encrypt`] message.
//! - **Atomic write**: ciphertext is written to a sibling temp file
//!   (`<output>.tmp` in the same directory) and renamed over `output` on
//!   success; the temp file is removed on any failure. `output` is overwritten
//!   if it already exists. On Unix `rename` overwrites atomically; as a
//!   fallback, if `rename` fails and `output` exists, `output` is removed and
//!   the rename retried.
//! - **Core policy**: the core does not enforce a minimum passphrase length
//!   (A1.1); only emptiness is rejected. The CLI may warn about weak
//!   passphrases.
//! - **Synchronous**: functions are plain synchronous calls; the passphrase is
//!   never held in a `static`.
//!
//! Decryption ([`decrypt_file_from_age`]) exists only for internal roundtrip
//! tests; it is compiled under `#[cfg(any(test, feature = "age-decrypt"))]` and
//! is not re-exported from the crate root.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use zeroize::Zeroizing;

use crate::domain::{Error, Result};

#[cfg(any(test, feature = "age-decrypt"))]
use std::io::Read;

/// Encrypt `plaintext` into `output` as a binary age file.
///
/// Overwrites `output` if it exists; writes atomically via a sibling temp file
/// (see module invariants). `passphrase` is copied into a zeroizing
/// `SecretString` and never appears in errors.
pub fn encrypt_bytes_to_file(
    plaintext: &[u8],
    output: &Path,
    passphrase: &Zeroizing<String>,
) -> Result<()> {
    ensure_nonempty_passphrase(passphrase)?;
    let secret = secret_string(passphrase);
    let recipient = age::scrypt::Recipient::new(secret);
    let ciphertext = age::encrypt(&recipient, plaintext).map_err(|err| Error::Encrypt {
        message: err.to_string(),
    })?;
    write_atomically(output, |tmp, file| {
        let mut file = file;
        file.write_all(&ciphertext).map_err(|source| Error::Io {
            path: tmp.to_path_buf(),
            source,
        })
    })
}

/// Encrypt `source` into `output` as a binary age file, returning the number
/// of plaintext bytes read from `source`.
///
/// Streams the file through the age encryptor without buffering the whole
/// plaintext in memory. Overwrites `output` if it exists and writes atomically
/// via a sibling temp file (see module invariants). `passphrase` is copied into
/// a zeroizing `SecretString` and never appears in errors.
pub fn encrypt_file_to_age(
    source: &Path,
    output: &Path,
    passphrase: &Zeroizing<String>,
) -> Result<u64> {
    ensure_nonempty_passphrase(passphrase)?;
    let secret = secret_string(passphrase);

    let mut source_file = File::open(source).map_err(|source_err| Error::Io {
        path: source.to_path_buf(),
        source: source_err,
    })?;

    let mut bytes_read = 0u64;
    write_atomically(output, |tmp, file| {
        let encryptor = age::Encryptor::with_user_passphrase(secret.clone());
        let mut writer = encryptor.wrap_output(file).map_err(|source| Error::Io {
            path: tmp.to_path_buf(),
            source,
        })?;
        bytes_read = std::io::copy(&mut source_file, &mut writer).map_err(|source| Error::Io {
            path: tmp.to_path_buf(),
            source,
        })?;
        writer.finish().map_err(|source| Error::Io {
            path: tmp.to_path_buf(),
            source,
        })?;
        Ok(())
    })?;
    Ok(bytes_read)
}

/// Decrypt a binary age file produced by [`encrypt_bytes_to_file`] /
/// [`encrypt_file_to_age`] back to its plaintext bytes.
///
/// Test / internal helper only (see module invariants). A wrong passphrase or a
/// malformed file yields [`Error::Encrypt`] without the passphrase in the
/// message.
#[cfg(any(test, feature = "age-decrypt"))]
pub fn decrypt_file_from_age(source: &Path, passphrase: &Zeroizing<String>) -> Result<Vec<u8>> {
    ensure_nonempty_passphrase(passphrase)?;
    let secret = secret_string(passphrase);

    let mut file = File::open(source).map_err(|source_err| Error::Io {
        path: source.to_path_buf(),
        source: source_err,
    })?;
    let decryptor = age::Decryptor::new(&mut file).map_err(|err| Error::Encrypt {
        message: format!("decryption setup failed: {err}"),
    })?;
    let identity = age::scrypt::Identity::new(secret);
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|err| Error::Encrypt {
            message: format!("decryption failed: {err}"),
        })?;
    let mut plaintext = Vec::new();
    reader
        .read_to_end(&mut plaintext)
        .map_err(|source_err| Error::Io {
            path: source.to_path_buf(),
            source: source_err,
        })?;
    Ok(plaintext)
}

/// Copy the passphrase into the age backend's `SecretString`.
///
/// Both the caller's `Zeroizing<String>` and the returned `SecretString`
/// zeroize on drop, so the value is wiped once it goes out of scope after the
/// call returns.
fn secret_string(passphrase: &Zeroizing<String>) -> age::secrecy::SecretString {
    age::secrecy::SecretString::from(passphrase.as_str().to_owned())
}

/// Reject an empty passphrase before any encryption work begins.
///
/// The core intentionally enforces no minimum length (A1.1); emptiness only.
fn ensure_nonempty_passphrase(passphrase: &Zeroizing<String>) -> Result<()> {
    if passphrase.is_empty() {
        return Err(Error::Encrypt {
            message: "passphrase must not be empty".to_string(),
        });
    }
    Ok(())
}

/// `output`'s sibling temp path: `<output>.tmp` in the same directory.
fn tmp_sibling_path(output: &Path) -> PathBuf {
    let mut name = output.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

/// Write `output` atomically: create the sibling temp file, run `write`, then
/// rename it over `output`. The temp file is removed on any failure.
fn write_atomically(output: &Path, write: impl FnOnce(&Path, File) -> Result<()>) -> Result<()> {
    let tmp = tmp_sibling_path(output);
    let mut guard = TempFileGuard {
        path: tmp.clone(),
        armed: true,
    };
    let file = File::create(&tmp).map_err(|source| Error::Io {
        path: tmp.clone(),
        source,
    })?;
    write(&tmp, file)?;
    replace_output(&tmp, output)?;
    guard.armed = false;
    Ok(())
}

/// Rename `tmp` over `output`, deleting a pre-existing `output` if a plain
/// rename fails (e.g. on platforms where `rename` does not overwrite).
fn replace_output(tmp: &Path, output: &Path) -> Result<()> {
    match fs::rename(tmp, output) {
        Ok(()) => Ok(()),
        Err(first_err) => {
            if output.exists() {
                fs::remove_file(output).map_err(|source| Error::Io {
                    path: output.to_path_buf(),
                    source,
                })?;
                fs::rename(tmp, output).map_err(|source| Error::Io {
                    path: tmp.to_path_buf(),
                    source,
                })
            } else {
                Err(Error::Io {
                    path: tmp.to_path_buf(),
                    source: first_err,
                })
            }
        }
    }
}

/// Removes the temp file on drop unless disarmed (after a successful rename).
struct TempFileGuard {
    path: PathBuf,
    armed: bool,
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;
    use zeroize::Zeroizing;

    const TEST_PASSPHRASE: &str = "correct horse battery staple a1";
    const LEAK_TEST_PASSPHRASE: &str = "SUPER-SECRET-PHRASE-x9";

    fn passphrase(p: &str) -> Zeroizing<String> {
        Zeroizing::new(p.to_string())
    }

    #[test]
    fn roundtrip_bytes_encrypt_decrypt_preserves_plaintext() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("roundtrip.age");
        let plaintext: &[u8] = b"raccpack secret payload \x00\x01\xff binary-safe";

        encrypt_bytes_to_file(plaintext, &output, &passphrase(TEST_PASSPHRASE)).unwrap();

        let decrypted = decrypt_file_from_age(&output, &passphrase(TEST_PASSPHRASE)).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_passphrase_fails_to_decrypt() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("wrong.age");

        encrypt_bytes_to_file(b"guarded data", &output, &passphrase(TEST_PASSPHRASE)).unwrap();

        let result = decrypt_file_from_age(&output, &passphrase("definitely-not-the-passphrase"));
        assert!(result.is_err(), "decrypt with wrong passphrase must fail");
    }

    #[test]
    fn empty_passphrase_is_rejected_on_encrypt() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("empty.age");

        let result = encrypt_bytes_to_file(b"data", &output, &Zeroizing::new(String::new()));
        assert!(result.is_err(), "empty passphrase must be rejected");
    }

    #[test]
    fn file_roundtrip_matches_source_and_reports_bytes_read() {
        let dir = TempDir::new().unwrap();
        let source = dir.path().join("source.txt");
        let content: &[u8] = b"known file content\nwith unicode: caf\xc3\xa9\n";
        {
            let mut f = fs::File::create(&source).unwrap();
            f.write_all(content).unwrap();
        }
        let output = dir.path().join("file.age");

        let bytes_read =
            encrypt_file_to_age(&source, &output, &passphrase(TEST_PASSPHRASE)).unwrap();
        assert_eq!(bytes_read, content.len() as u64);

        let decrypted = decrypt_file_from_age(&output, &passphrase(TEST_PASSPHRASE)).unwrap();
        assert_eq!(decrypted, content);
    }

    #[test]
    fn error_display_does_not_contain_passphrase() {
        let dir = TempDir::new().unwrap();
        // parent is a regular file -> output path unusable -> encrypt must fail
        let blocker = dir.path().join("blocker");
        fs::write(&blocker, b"i am a file, not a directory").unwrap();
        let output = blocker.join("out.age");

        let err =
            encrypt_bytes_to_file(b"data", &output, &passphrase(LEAK_TEST_PASSPHRASE)).unwrap_err();

        let display = format!("{err}");
        assert!(
            !display.contains(LEAK_TEST_PASSPHRASE),
            "error Display leaked passphrase: {display}"
        );
    }

    #[test]
    fn overwrite_existing_output_keeps_latest_data() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("overwrite.age");

        encrypt_bytes_to_file(b"first version", &output, &passphrase(TEST_PASSPHRASE)).unwrap();
        encrypt_bytes_to_file(
            b"second, latest version",
            &output,
            &passphrase(TEST_PASSPHRASE),
        )
        .unwrap();

        let decrypted = decrypt_file_from_age(&output, &passphrase(TEST_PASSPHRASE)).unwrap();
        assert_eq!(decrypted, b"second, latest version");
    }

    #[test]
    fn age_output_has_binary_format_magic_header() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("header.age");

        encrypt_bytes_to_file(b"payload", &output, &passphrase(TEST_PASSPHRASE)).unwrap();

        let bytes = fs::read(&output).unwrap();
        assert!(
            bytes.starts_with(b"age-encryption.org/v1"),
            "expected age binary magic header, got: {:?}",
            &bytes[..bytes.len().min(32)]
        );
    }
}
