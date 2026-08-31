//! Operations hub state — the list of runnable operations (Pack, Stash,
//! Rinse, Raid) plus the project the next operation will target.
//!
//! The selected project is derived from the sniff table selection
//! (`SniffScreenState::selected_project`) and mirrored into the hub state so
//! the screen owns a display copy (and later stages can replace it with a
//! manually entered path).

use std::path::{Path, PathBuf};

/// Available operations launched from the Operations hub.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OperationKind {
    /// Package a project into the den (`tar.zst` + age archive).
    #[default]
    Pack,
    /// Move secrets out of a project into age archives.
    Stash,
    /// Clean build/runtime junk from a project.
    Rinse,
    /// Run the full pipeline: stash → rinse → pack → move (live flow).
    Raid,
}

/// Ordered registry of every operation, in list order.
pub const ALL_OPERATIONS: [OperationKind; 4] = [
    OperationKind::Pack,
    OperationKind::Stash,
    OperationKind::Rinse,
    OperationKind::Raid,
];

impl OperationKind {
    /// Display label for the operations list.
    pub fn label(self) -> &'static str {
        match self {
            Self::Pack => "Pack",
            Self::Stash => "Stash",
            Self::Rinse => "Rinse",
            Self::Raid => "Raid",
        }
    }

    /// One-key shortcut that jumps the list selection to this operation.
    pub fn key(self) -> char {
        match self {
            Self::Pack => 'p',
            Self::Stash => 's',
            Self::Rinse => 'r',
            Self::Raid => 'd',
        }
    }

    /// Inverse of [`Self::key`]: the operation bound to `c`, if any.
    pub fn from_key(c: char) -> Option<Self> {
        ALL_OPERATIONS.iter().copied().find(|k| k.key() == c)
    }

    /// Next operation in list order (cycles back to the first).
    pub fn next(self) -> Self {
        match self {
            Self::Pack => Self::Stash,
            Self::Stash => Self::Rinse,
            Self::Rinse => Self::Raid,
            Self::Raid => Self::Pack,
        }
    }

    /// Previous operation in list order (cycles back to the last).
    pub fn prev(self) -> Self {
        match self {
            Self::Pack => Self::Raid,
            Self::Stash => Self::Pack,
            Self::Rinse => Self::Stash,
            Self::Raid => Self::Rinse,
        }
    }

    /// Future stage that owns this operation's real flow. `None` when the
    /// flow already exists (Raid).
    pub fn planned_stage(self) -> Option<&'static str> {
        match self {
            Self::Pack => Some("T-02"),
            Self::Stash => Some("T-03"),
            Self::Rinse => Some("T-04"),
            Self::Raid => None,
        }
    }
}

/// Selection + project for the Operations hub screen.
#[derive(Debug, Default)]
pub struct OperationsScreenState {
    /// Operation currently highlighted in the list.
    pub selected: OperationKind,
    /// Display copy of the sniff-selected project (refreshed each render).
    pub project: Option<PathBuf>,
    /// Operation whose real flow is still a stub; shown as a notice until
    /// dismissed with Esc / Enter.
    pub stub: Option<OperationKind>,
}

impl OperationsScreenState {
    /// Move the selection to the next operation (cycles).
    pub fn select_next(&mut self) {
        self.selected = self.selected.next();
    }

    /// Move the selection to the previous operation (cycles).
    pub fn select_previous(&mut self) {
        self.selected = self.selected.prev();
    }

    /// Move the selection to the first operation.
    pub fn select_first(&mut self) {
        self.selected = ALL_OPERATIONS[0];
    }

    /// Move the selection to the last operation.
    pub fn select_last(&mut self) {
        self.selected = ALL_OPERATIONS[ALL_OPERATIONS.len() - 1];
    }

    /// Align the displayed project with the sniff table selection.
    pub fn refresh_project(&mut self, project: Option<&Path>) {
        self.project = project.map(Path::to_path_buf);
    }

    /// Open `kind` in its stub (placeholder) form. The real flows are stages
    /// T-02..T-04; until then activating them only shows a notice.
    pub fn open_stub(&mut self, kind: OperationKind) {
        self.stub = Some(kind);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_operations_are_four_in_order() {
        let labels: Vec<&str> = ALL_OPERATIONS.iter().map(|k| k.label()).collect();
        assert_eq!(labels, vec!["Pack", "Stash", "Rinse", "Raid"]);
    }

    #[test]
    fn labels_and_keys_are_unique() {
        let labels: Vec<&str> = ALL_OPERATIONS.iter().map(|k| k.label()).collect();
        let keys: Vec<char> = ALL_OPERATIONS.iter().map(|k| k.key()).collect();
        let unique_labels: std::collections::BTreeSet<_> = labels.iter().collect();
        let unique_keys: std::collections::BTreeSet<_> = keys.iter().collect();
        assert_eq!(unique_labels.len(), ALL_OPERATIONS.len());
        assert_eq!(unique_keys.len(), ALL_OPERATIONS.len());
    }

    #[test]
    fn from_key_round_trips_every_key() {
        for kind in ALL_OPERATIONS {
            assert_eq!(OperationKind::from_key(kind.key()), Some(kind));
        }
        assert_eq!(OperationKind::from_key('x'), None);
    }

    #[test]
    fn next_prev_round_trip() {
        for kind in ALL_OPERATIONS {
            assert_eq!(kind.next().prev(), kind, "next().prev() must round-trip");
            assert_eq!(kind.prev().next(), kind, "prev().next() must round-trip");
        }
    }

    #[test]
    fn next_cycles_through_list() {
        for (i, kind) in ALL_OPERATIONS.iter().enumerate() {
            let expected = ALL_OPERATIONS[(i + 1) % ALL_OPERATIONS.len()];
            assert_eq!(kind.next(), expected, "next() must cycle in list order");
        }
        assert_eq!(
            OperationKind::Raid.next(),
            OperationKind::Pack,
            "cycles back to the first"
        );
    }

    #[test]
    fn planned_stage_maps_future_stages_only() {
        assert_eq!(OperationKind::Pack.planned_stage(), Some("T-02"));
        assert_eq!(OperationKind::Stash.planned_stage(), Some("T-03"));
        assert_eq!(OperationKind::Rinse.planned_stage(), Some("T-04"));
        assert_eq!(OperationKind::Raid.planned_stage(), None);
    }

    #[test]
    fn state_selection_navigation() {
        let mut state = OperationsScreenState::default();
        assert_eq!(state.selected, OperationKind::Pack);

        state.select_next();
        assert_eq!(state.selected, OperationKind::Stash);
        state.select_next();
        assert_eq!(state.selected, OperationKind::Rinse);
        state.select_previous();
        assert_eq!(state.selected, OperationKind::Stash);
        state.select_first();
        assert_eq!(state.selected, OperationKind::Pack);
        state.select_last();
        assert_eq!(state.selected, OperationKind::Raid);
    }

    #[test]
    fn refresh_project_syncs_display_copy() {
        let mut state = OperationsScreenState::default();
        assert_eq!(state.project, None);

        let path = Path::new("/tmp/proj");
        state.refresh_project(Some(path));
        assert_eq!(state.project, Some(PathBuf::from("/tmp/proj")));

        state.refresh_project(None);
        assert_eq!(
            state.project, None,
            "clearing the sniff selection clears it"
        );
    }

    #[test]
    fn open_stub_marks_kind_and_default_is_clear() {
        let mut state = OperationsScreenState::default();
        assert_eq!(state.stub, None);
        state.open_stub(OperationKind::Pack);
        assert_eq!(state.stub, Some(OperationKind::Pack));
    }
}
