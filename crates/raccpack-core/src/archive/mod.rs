//! Pack project trees into tar+zstd archives with deny rules.
//!
//! [`pack_tree`] is the low-level packing API: it walks a `source` directory
//! with [`SkipPolicy`] pruning and name/content deny rules, then writes a
//! single tar+zstd stream to `output`. [`deny`] holds the name and content deny
//! helpers reused by the packer.
//!
//! The archive root is the *contents* of `source` (entries like `src/main.rs`,
//! NOT `myproject/src/main.rs`). Symlinks are never followed and never
//! archived. `output` is written directly (created/overwritten); atomicity
//! (temp + rename) is the caller's / facade's responsibility (M4.2/M4.3).
//! `output` must NOT be inside `source` — the caller guarantees a staging path
//! outside the tree.
//!
//! [`age_vault`] holds the age (scrypt passphrase) encrypt backend used for
//! secret vaults; it writes binary age files atomically and zeroizes passphrase
//! material.
//!
//! [`SkipPolicy`]: crate::scan::SkipPolicy

pub mod age_vault;
pub mod deny;
pub mod pack;

pub use age_vault::{encrypt_bytes_to_file, encrypt_file_to_age};
pub use deny::{content_deny_hit, should_deny_file_in_pack, ContentDenyOptions};
pub use pack::{pack_tree, PackTreeOptions, PackTreeResult};
