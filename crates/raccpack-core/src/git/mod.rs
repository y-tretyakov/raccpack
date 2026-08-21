//! Git integration for raccpack-core.
//!
//! Contract in [`client`], subprocess implementation in [`process`],
//! deterministic [`MockGitClient`] in [`mock`]. This module never imports
//! app/report types; `app → git` is the only allowed dependency direction.

mod client;
mod mock;
mod process;

pub use client::{find_repo_root, GitClient, GitFileStatus, GitState};
pub use mock::MockGitClient;
pub use process::ProcessGitClient;
