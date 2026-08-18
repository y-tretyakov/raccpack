//! Stage presentation for raid: stage results and mode-aware summaries.
//!
//! These helpers build [`RaidStageResult`]s (ok / failed / skipped / disabled)
//! and the human summaries that both the stage results and the raid-level
//! completion events reuse, so CLI output and the result stay in sync.

use crate::app::pack::PackResult;
use crate::app::rinse::RinseResult;
use crate::app::stash::StashResult;

use super::{RaidStageResult, SKIPPED_MESSAGE};

/// Build a successful stage.
pub(super) fn ok_stage(name: &str, message: impl Into<String>) -> RaidStageResult {
    RaidStageResult {
        name: name.to_string(),
        success: true,
        message: message.into(),
        skipped: false,
    }
}

/// Build a failed stage (the phase ran and errored).
pub(super) fn failed_stage(name: &str, message: impl Into<String>) -> RaidStageResult {
    RaidStageResult {
        name: name.to_string(),
        success: false,
        message: message.into(),
        skipped: false,
    }
}

/// Build a stage for a phase short-circuited by an earlier failure.
pub(super) fn skipped_stage(name: &str) -> RaidStageResult {
    RaidStageResult {
        name: name.to_string(),
        success: false,
        message: SKIPPED_MESSAGE.to_string(),
        skipped: true,
    }
}

/// Build a stage for a disabled phase.
pub(super) fn disabled_stage(name: &str) -> RaidStageResult {
    RaidStageResult {
        name: name.to_string(),
        success: true,
        message: "disabled".to_string(),
        skipped: true,
    }
}

/// Stash stage summary, mode-aware ("would stash N files" / "stashed N files").
pub(super) fn stash_message(result: &StashResult, dry_run: bool) -> String {
    if dry_run {
        format!("would stash {} files", result.files_archived)
    } else {
        format!("stashed {} files", result.files_archived)
    }
}

/// Rinse stage summary, mode-aware.
pub(super) fn rinse_message(result: &RinseResult, dry_run: bool) -> String {
    if dry_run {
        format!("found {} directories", result.removed.len())
    } else {
        format!("removed {} directories", result.removed.len())
    }
}

/// Pack stage summary, mode-aware.
pub(super) fn pack_message(result: &PackResult, dry_run: bool) -> String {
    if dry_run {
        "would pack project".to_string()
    } else {
        format!("packed {} files", result.file_count)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::app::pack::PackResult;
    use crate::app::rinse::RinseResult;
    use crate::app::stash::StashResult;
    use crate::clean::TrashDir;

    use super::*;

    fn stash_result(files: usize, dry_run: bool) -> StashResult {
        StashResult {
            archive_path: PathBuf::from("/tmp/den/secrets/1.age"),
            files_archived: files,
            bytes_archived: 0,
            removed_sources: 0,
            dry_run,
            manifest: Vec::new(),
        }
    }

    fn rinse_result(removed: usize, dry_run: bool) -> RinseResult {
        let trash = (0..removed)
            .map(|i| TrashDir {
                path: PathBuf::from(format!("/tmp/app/dir-{i}")),
                strategy: "node".to_string(),
                pattern_name: "pattern".to_string(),
                size_bytes: 0,
            })
            .collect();
        RinseResult {
            removed: trash,
            bytes_freed: 0,
            dry_run,
        }
    }

    fn pack_result(file_count: usize, dry_run: bool) -> PackResult {
        PackResult {
            source: PathBuf::from("/tmp/app"),
            output: PathBuf::from("/tmp/den/packs/1.tar.zst"),
            size_bytes: 0,
            file_count,
            skipped_secret_files: 0,
            dry_run,
        }
    }

    #[test]
    fn stage_helpers_produce_expected_shapes() {
        let ok = ok_stage("pack", "packed 3 files");
        assert!(ok.success);
        assert!(!ok.skipped);
        assert_eq!(ok.message, "packed 3 files");

        let failed = failed_stage("stash", "no files");
        assert!(!failed.success);
        assert!(!failed.skipped);
        assert_eq!(failed.message, "no files");

        let skipped = skipped_stage("rinse");
        assert!(!skipped.success);
        assert!(skipped.skipped);
        assert_eq!(skipped.message, SKIPPED_MESSAGE);

        let disabled = disabled_stage("pack");
        assert!(disabled.success);
        assert!(disabled.skipped);
        assert_eq!(disabled.message, "disabled");
    }

    #[test]
    fn phase_messages_are_mode_aware() {
        assert_eq!(
            stash_message(&stash_result(3, true), true),
            "would stash 3 files"
        );
        assert_eq!(
            stash_message(&stash_result(3, false), false),
            "stashed 3 files"
        );
        assert_eq!(
            rinse_message(&rinse_result(2, true), true),
            "found 2 directories"
        );
        assert_eq!(
            rinse_message(&rinse_result(2, false), false),
            "removed 2 directories"
        );
        assert_eq!(
            pack_message(&pack_result(5, true), true),
            "would pack project"
        );
        assert_eq!(
            pack_message(&pack_result(5, false), false),
            "packed 5 files"
        );
    }
}
