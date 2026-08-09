//! Den layout: initialize the output vault and place pack archives into it.
//!
//! A den is a directory tree that raccpack treats as its output vault
//! (see `raccpack-facade-and-den.md` §9):
//!
//! ```text
//! {den_dir}/
//! ├── README.txt
//! ├── .den-version          # "1"
//! ├── manifests/yyyy/mm/
//! ├── secrets/yyyy/mm/
//! ├── packs/yyyy/mm/{slug}__{ts}.tar.zst
//! └── staging/{short_id}/
//! ```
//!
//! [`ensure_den`] creates the directory tree, writes `.den-version` and
//! `README.txt` when absent, and enforces the major version gate. The naming
//! conventions (slug, UTC timestamp, short id, pack relative path) are owned by
//! [`project_slug`], [`utc_timestamp_now`], [`short_id`] and
//! [`pack_relative_path`]. [`place_pack`] moves a completed archive produced by
//! `crate::archive::pack_tree` into `packs/…` atomically.
//!
//! INVARIANTS:
//!
//! - [`ensure_den`] is idempotent: repeated calls never rewrite existing
//!   `.den-version` / `README.txt` and never fail on an already-initialized den.
//! - The version gate rejects any den whose major version differs from
//!   [`DEN_VERSION`] with [`crate::Error::DenVersion`] instead of writing.
//! - Pack placement never writes outside `den_root`: the relative target is
//!   checked for escaping components before any filesystem mutation.
//! - No symlink handling is involved here (this module does not walk
//!   directories; it only creates dirs and renames a single archive).

mod layout;
mod names;
mod place;

pub use layout::{ensure_den, staging_pack_path, DenPaths, DEN_VERSION};
pub use names::{pack_relative_path, project_slug, short_id, utc_timestamp_now};
pub use place::{place_pack, PlacePackRequest, PlacePackResult};
