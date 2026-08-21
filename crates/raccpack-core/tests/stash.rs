//! Integration tests for A1.2 — stash: file selection, batch encrypt,
//! manifest without raw values, and removal of source files.
//!
//! Covers the 8 required cases from
//! `docs/alpha/a1/a1.2-stash-manifest-remove.md` §5:
//! 1. High `.env` selected while a clean `notes.txt` is ignored;
//! 2. `min_risk: Critical` filters a High-only env with no content hit;
//! 3. `only_files` limits the set and rejects a path outside `target`;
//! 4. `write_stash_age` writes a binary age archive (magic header, correct
//!    counts, sources kept) and, with `--features age-decrypt`, a decrypt +
//!    untar roundtrip restores the exact contents;
//! 5. `manifest.len() == files_archived` and manifest serde JSON has no raw;
//! 6. `remove_stash_sources` deletes files (fail-fast) and fails on a second
//!    call;
//! 7. empty select → `Error::StashEmpty`;
//! 8. `relative_path` contains no `..` components.
//!
//! All fixtures are hermetic `tempfile::TempDir`s; no network, no real git.
//! `bytes_archived` / `size_bytes` are derived from file metadata, so there is
//! no timing-dependent flakiness.
//!
//! `stash_batch_roundtrip_decrypt_untar_restores` requires the `age-decrypt`
//! feature (it calls `decrypt_file_from_age`, which is compiled only under
//! `cfg(any(test, feature = "age-decrypt"))`). Run the full stash suite with:
//!
//! ```text
//! cargo test -p raccpack-core stash --features age-decrypt
//! ```
//!
//! Without the feature, the roundtrip test is excluded; all other stash tests
//! still run with plain `cargo test -p raccpack-core stash`.

use std::fs;
use std::path::{Component, Path, PathBuf};

use raccpack_core::secrets::stash_batch::{write_stash_age, StashBatchResult, StashManifestEntry};
use raccpack_core::secrets::stash_remove::remove_stash_sources;
use raccpack_core::secrets::stash_select::{
    select_files_for_stash, StashFileEntry, StashSelectOptions,
};
use raccpack_core::{Error, SensitiveRisk};
use tempfile::TempDir;
use zeroize::Zeroizing;

/// A long, distinctive password value used to prove the manifest JSON never
/// serializes raw content.
const PASSWORD_VALUE: &str = "SUPERSECRETVALUE_xyz987";

/// Test passphrase for age encryption (must be non-empty).
const PASSPHRASE: &str = "raccpack a1.2 stash test passphrase";

// --- Test helpers -----------------------------------------------------------

/// Create parent directories and write a file at `root/rel`, returning its path.
fn write(root: &Path, rel: &str, contents: &str) -> PathBuf {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("rel has a parent")).expect("create parent dirs");
    fs::write(&path, contents).expect("write fixture file");
    path
}

/// Wrap a passphrase in a zeroizing buffer the way `write_stash_age` expects.
fn passphrase(p: &str) -> Zeroizing<String> {
    Zeroizing::new(p.to_string())
}

/// Default stash options: whole `target`, content scan on, `min_risk: High`.
fn stash_options(target: &Path) -> StashSelectOptions {
    StashSelectOptions {
        target: target.to_path_buf(),
        only_files: None,
        min_risk: SensitiveRisk::High,
        scan_content: true,
    }
}

/// Build an empty `project` directory under a fresh `TempDir`.
fn project_dir() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("create work dir");
    let project = temp.path().join("project");
    fs::create_dir_all(&project).expect("create project dir");
    (temp, project)
}

/// Select all stashable files in `project` and write them into one age stash.
fn stash_project(project: &Path, age_path: &Path) -> StashBatchResult {
    let entries = select_files_for_stash(&stash_options(project)).expect("select files");
    write_stash_age(&entries, age_path, &passphrase(PASSPHRASE)).expect("write stash age")
}

// --- Case 1: High `.env` selected, clean `notes.txt` ignored ------------------

#[test]
fn stash_select_high_env_only() {
    let (temp, project) = project_dir();
    let _temp = temp;
    let env = write(&project, ".env", "APP=local\n");
    write(&project, "notes.txt", "just a plain text file\n");

    let entries = select_files_for_stash(&stash_options(&project)).unwrap();

    assert_eq!(entries.len(), 1, "only .env must match: {entries:#?}");
    assert_eq!(entries[0].path, env);
    assert_eq!(entries[0].relative_path, Path::new(".env"));
    assert_eq!(entries[0].risk, SensitiveRisk::High);
    assert_eq!(entries[0].size_bytes, fs::metadata(&env).unwrap().len());
}

// --- Case 2: `min_risk: Critical` filters a High-only env ---------------------

#[test]
fn stash_select_min_risk_critical_filters_high_only_env() {
    let (temp, project) = project_dir();
    let _temp = temp;
    write(&project, ".env", "APP=local\n");

    let opts = StashSelectOptions {
        min_risk: SensitiveRisk::Critical,
        ..stash_options(&project)
    };
    let entries = select_files_for_stash(&opts).unwrap();

    assert!(
        entries.is_empty(),
        "High-only .env (no content hit) must be filtered at Critical: {entries:#?}"
    );
}

// --- Case 3: `only_files` limits the set; outside-target is an error ----------

#[test]
fn stash_select_only_files_limits_set() {
    let (temp, project) = project_dir();
    let _temp = temp;
    let env = write(&project, ".env", "APP=local\n");
    write(&project, "notes.txt", "just a plain text file\n");

    let opts = StashSelectOptions {
        only_files: Some(vec![env.clone()]),
        ..stash_options(&project)
    };
    let entries = select_files_for_stash(&opts).unwrap();

    assert_eq!(
        entries.len(),
        1,
        "only_files must limit the set: {entries:#?}"
    );
    assert_eq!(entries[0].path, env);
}

#[test]
fn stash_select_only_files_outside_target_is_error() {
    let (temp, project) = project_dir();
    let _ = &temp;
    let outside = write(temp.path(), "outside.txt", "not under target\n");

    let opts = StashSelectOptions {
        only_files: Some(vec![outside]),
        ..stash_options(&project)
    };
    let result = select_files_for_stash(&opts);

    assert!(
        result.is_err(),
        "a path outside target must be rejected (path containment)"
    );
}

// --- Case 4: batch writes an age file; decrypt+untar restores content ---------

#[test]
fn stash_batch_writes_age_file_and_keeps_sources() {
    let (temp, project) = project_dir();
    let env = write(&project, ".env", "PASSWORD=SUPERSECRETVALUE_xyz987\n");
    let env_local = write(
        &project,
        ".env.local",
        "PASSWORD=SUPERSECRETVALUE_xyz987_local\n",
    );
    let age_path = temp.path().join("stash.age");

    let entries: Vec<StashFileEntry> = select_files_for_stash(&stash_options(&project)).unwrap();
    let result = write_stash_age(&entries, &age_path, &passphrase(PASSPHRASE)).unwrap();

    assert_eq!(result.age_path, age_path);
    assert_eq!(result.files_archived, entries.len());
    assert_eq!(result.manifest.len(), entries.len());
    let expected_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();
    assert_eq!(result.bytes_archived, expected_bytes);

    assert!(age_path.is_file(), "age archive must exist on disk");
    let head = fs::read(&age_path).unwrap();
    assert!(
        head.starts_with(b"age-encryption.org/v1"),
        "age binary magic header expected, got: {:?}",
        &head[..head.len().min(24)]
    );

    // write_stash_age must NOT remove the source files.
    assert!(env.is_file(), ".env must still exist after batch write");
    assert!(
        env_local.is_file(),
        ".env.local must still exist after batch write"
    );
}

#[cfg(feature = "age-decrypt")]
#[test]
fn stash_batch_roundtrip_decrypt_untar_restores_content() {
    use std::collections::BTreeMap;
    use std::io::Read;

    let (temp, project) = project_dir();
    let env = write(&project, ".env", "PASSWORD=SUPERSECRETVALUE_xyz987\n");
    let env_local = write(
        &project,
        ".env.local",
        "PASSWORD=SUPERSECRETVALUE_xyz987_local\n",
    );
    let age_path = temp.path().join("stash.age");

    let result = stash_project(&project, &age_path);
    assert_eq!(result.files_archived, 2);

    let plaintext = raccpack_core::archive::age_vault::decrypt_file_from_age(
        &age_path,
        &passphrase(PASSPHRASE),
    )
    .unwrap();

    let mut archive = tar::Archive::new(&plaintext[..]);
    let mut contents: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for item in archive.entries().unwrap() {
        let mut entry = item.unwrap();
        let name = entry.path().unwrap().to_string_lossy().into_owned();
        assert!(
            !name.contains(".."),
            "tar entry path must not escape the stash root: {name}"
        );
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).unwrap();
        contents.insert(name, buf);
    }

    assert_eq!(contents.len(), result.files_archived);

    let env_name = env
        .strip_prefix(&project)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let local_name = env_local
        .strip_prefix(&project)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(contents.get(&env_name).unwrap(), &fs::read(&env).unwrap());
    assert_eq!(
        contents.get(&local_name).unwrap(),
        &fs::read(&env_local).unwrap()
    );
}

// --- Case 5: manifest length matches; serde JSON has no raw -------------------

#[test]
fn stash_batch_manifest_is_raw_free_and_counts_match() {
    let (temp, project) = project_dir();
    write(&project, ".env", &format!("PASSWORD={PASSWORD_VALUE}\n"));
    write(&project, ".env.local", "PASSWORD=another-local-value\n");
    let age_path = temp.path().join("stash.age");

    let entries = select_files_for_stash(&stash_options(&project)).unwrap();
    assert_eq!(entries.len(), 2);
    let result = write_stash_age(&entries, &age_path, &passphrase(PASSPHRASE)).unwrap();

    assert_eq!(result.manifest.len(), result.files_archived);
    assert_eq!(result.manifest.len(), entries.len());

    let json = serde_json::to_string(&result.manifest).expect("serialize manifest");
    assert!(
        !json.contains(PASSWORD_VALUE),
        "manifest JSON must never contain raw .env value: {json}"
    );
    assert!(
        !json.contains("another-local-value"),
        "manifest JSON must never contain raw .env.local value: {json}"
    );

    for entry in &result.manifest {
        let file_entry = entries
            .iter()
            .find(|f| f.path == entry.original_path)
            .expect("manifest path must reference an archived entry");
        assert_eq!(entry.original_path, file_entry.path);
        assert_eq!(entry.risk, file_entry.risk);
        assert_eq!(entry.size_bytes, file_entry.size_bytes);
        assert_eq!(
            entry.size_bytes,
            fs::metadata(&entry.original_path).unwrap().len()
        );

        let one = serde_json::to_string(entry).expect("serialize one manifest entry");
        let decoded: StashManifestEntry =
            serde_json::from_str(&one).expect("deserialize one entry");
        assert_eq!(decoded.original_path, entry.original_path);
        assert_eq!(decoded.risk, entry.risk);
        assert_eq!(decoded.size_bytes, entry.size_bytes);
    }
}

// --- Case 6: remove deletes files; second call fails --------------------------

#[test]
fn stash_remove_deletes_sources_and_fails_on_second_call() {
    let (temp, project) = project_dir();
    let env = write(&project, ".env", "PASSWORD=SUPERSECRETVALUE_xyz987\n");
    let env_local = write(
        &project,
        ".env.local",
        "PASSWORD=SUPERSECRETVALUE_xyz987_local\n",
    );
    let age_path = temp.path().join("stash.age");

    let result = stash_project(&project, &age_path);
    assert_eq!(result.manifest.len(), 2);

    let removed = remove_stash_sources(&result.manifest).expect("remove sources");
    assert_eq!(removed, result.manifest.len());
    assert!(!env.exists(), ".env must be deleted");
    assert!(!env_local.exists(), ".env.local must be deleted");

    let second = remove_stash_sources(&result.manifest);
    assert!(
        second.is_err(),
        "re-removing already-deleted sources must fail"
    );
}

// --- Case 7: empty select yields a clear error --------------------------------

#[test]
fn stash_batch_empty_entries_is_stash_empty_error() {
    let temp = TempDir::new().unwrap();
    let age_path = temp.path().join("empty.age");

    let err = write_stash_age(&[], &age_path, &passphrase(PASSPHRASE)).unwrap_err();

    assert!(matches!(err, Error::StashEmpty { .. }), "got: {err}");
    assert!(
        err.to_string().starts_with("nothing to stash"),
        "StashEmpty Display must start with 'nothing to stash': {err}"
    );
    assert!(
        !age_path.exists(),
        "no archive may be written for empty entries"
    );
}

// --- Case 8: relative_path has no `..` components -----------------------------

#[test]
fn stash_select_relative_path_has_no_parent_dir() {
    let (temp, project) = project_dir();
    let _temp = temp;
    let nested = write(&project, "nested/.env", "APP=local\n");

    let entries = select_files_for_stash(&stash_options(&project)).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, nested);
    assert_eq!(entries[0].relative_path, Path::new("nested/.env"));
    assert!(
        entries[0]
            .relative_path
            .components()
            .all(|c| !matches!(c, Component::ParentDir)),
        "relative_path must contain no '..' components"
    );
}

// --- Extras ------------------------------------------------------------------

#[test]
fn stash_batch_empty_passphrase_is_encrypt_error() {
    let (temp, project) = project_dir();
    let env = write(&project, ".env", "PASSWORD=SUPERSECRETVALUE_xyz987\n");
    let age_path = temp.path().join("stash.age");

    let entries = select_files_for_stash(&stash_options(&project)).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, env);

    let result = write_stash_age(&entries, &age_path, &Zeroizing::new(String::new()));
    assert!(
        matches!(result, Err(Error::Encrypt { .. })),
        "an empty passphrase must be rejected: {result:?}"
    );
    assert!(
        !age_path.exists(),
        "no archive may be written for an empty passphrase"
    );
}

#[test]
fn stash_select_result_is_sorted_by_path() {
    let (temp, project) = project_dir();
    let _temp = temp;
    write(&project, ".env", "APP=root\n");
    write(&project, "a/.env", "APP=one\n");
    write(&project, "b/.env", "APP=two\n");

    let entries = select_files_for_stash(&stash_options(&project)).unwrap();
    assert_eq!(entries.len(), 3);

    let paths: Vec<PathBuf> = entries.iter().map(|e| e.path.clone()).collect();
    let mut expected = paths.clone();
    expected.sort();
    assert_eq!(paths, expected, "select must return entries sorted by path");
}
