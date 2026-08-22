//! The [`StackDetector`] contract shared by every ecosystem detector module.

use std::path::Path;

use crate::domain::{Error, Stack};
use crate::scan::MarkerHit;

/// A detector contributing framework knowledge for exactly one ecosystem.
///
/// Detectors never walk or scan; they read at most one level of the project
/// directory (see `read_dir_names`, plus a single `config/` peek in the
/// Ruby detector) to look for framework marker files. The orchestrator
/// ([`super::detect_stack`]) collects their `frameworks`, resolves `language`
/// centrally by priority and sets `markers` from the hits, so detectors must
/// leave both fields empty/unset.
///
/// `detect` returns a [`Result`] so shallow-read IO failures surface as
/// [`Error::Io`] (spec §5), which a bare `-> Stack` signature cannot express.
pub trait StackDetector: Send + Sync {
    /// Stable identifier used for diagnostics and future configuration.
    fn id(&self) -> &'static str;

    /// Whether this detector applies to the given marker hits.
    ///
    /// A detector matches when at least one of its ecosystem markers appears
    /// in `hits`. The orchestrator additionally probes *all* detectors when
    /// `hits` is empty (path-only detection), so this method only gates the
    /// marker-driven case.
    fn matches(&self, hits: &[MarkerHit]) -> bool;

    /// Produce a [`Stack`] contribution for the project at `project_dir`.
    ///
    /// Only the returned `frameworks` are used by the orchestrator. A detector
    /// does its shallow read here and only when the orchestrator decided to
    /// consult it. Errors from reading the directory map to [`Error::Io`].
    fn detect(&self, hits: &[MarkerHit], project_dir: &Path) -> Result<Stack, Error>;
}
