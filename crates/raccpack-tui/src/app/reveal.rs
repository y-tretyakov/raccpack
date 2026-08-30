//! Reveal modal state: a state machine driving confirm → reveal → show-once.
//!
//! This module owns the modal's local state and key handling only. The actual
//! reveal (re-reading the file, path containment, hash matching) happens in
//! `raccpack_core::secrets::reveal_finding`, called from the worker. The raw
//! value arrives in the [`RevealPhase::Ready`] phase wrapped in a
//! [`WorkerRevealSecret`] and is dropped the moment the modal closes.
//!
//! **Invariant:** the raw value never lives in `App`, `DigScreenState`, or any
//! long-lived struct — only in `Ready { secret }` on this modal, and only until
//! it is closed (which drops/zeroizes it).

use std::path::PathBuf;

use crossterm::event::KeyCode;
use raccpack_core::secrets::FindingRef;

use crate::worker::WorkerRevealSecret;

/// Command the modal hands back to the app layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevealCommand {
    /// The user confirmed the reveal; dispatch it to the worker.
    Confirm,
    /// Close the modal (returning to the list). Drops any in-flight secret.
    Close,
    /// The key only changed local state; nothing to dispatch.
    None,
}

/// Current phase of the reveal modal.
#[derive(Debug)]
pub enum RevealPhase {
    /// Awaiting an explicit confirm before anything is revealed.
    Confirm,
    /// The reveal has been dispatched to the worker; awaiting the result.
    Revealing,
    /// The raw value arrived and is shown once. `secret` is dropped on close.
    Ready {
        /// The revealed raw value (zeroized on drop, shown exactly once).
        secret: WorkerRevealSecret,
    },
    /// Reveal failed; no value was or will be shown.
    Failed {
        /// Human-readable error message (never the raw value).
        message: String,
    },
}

/// State machine for one reveal modal.
#[derive(Debug)]
pub struct RevealModal {
    /// Sensitive file being revealed.
    pub path: PathBuf,
    /// Reference pinning the exact value to reveal (marker + line + hash).
    pub reference: FindingRef,
    /// Current phase.
    pub phase: RevealPhase,
}

impl RevealModal {
    /// Open a reveal modal in the `Confirm` phase.
    pub fn new(path: PathBuf, reference: FindingRef) -> Self {
        Self {
            path,
            reference,
            phase: RevealPhase::Confirm,
        }
    }

    /// Handle one keypress inside the modal.
    pub fn handle_key(&mut self, key: KeyCode) -> RevealCommand {
        match &mut self.phase {
            RevealPhase::Confirm => match key {
                KeyCode::Char('y') | KeyCode::Enter => {
                    self.phase = RevealPhase::Revealing;
                    RevealCommand::Confirm
                }
                KeyCode::Char('n') | KeyCode::Esc => RevealCommand::Close,
                _ => RevealCommand::None,
            },
            // The worker is resolving; swallow every key.
            RevealPhase::Revealing => RevealCommand::None,
            // Shown once: any key closes and drops (zeroizes) the secret.
            RevealPhase::Ready { .. } => RevealCommand::Close,
            RevealPhase::Failed { .. } => match key {
                KeyCode::Enter | KeyCode::Esc => RevealCommand::Close,
                _ => RevealCommand::None,
            },
        }
    }

    /// Move to `Ready` after the worker delivered the value.
    pub fn set_ready(&mut self, secret: WorkerRevealSecret) {
        self.phase = RevealPhase::Ready { secret };
    }

    /// Move to `Failed` with a message (never the raw value).
    pub fn set_failed(&mut self, message: String) {
        self.phase = RevealPhase::Failed { message };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> FindingRef {
        FindingRef {
            path: PathBuf::from("/repo/.env"),
            marker_id: "aws_access_key".to_string(),
            line: 1,
            value_hash: "abc".to_string(),
        }
    }

    #[test]
    fn confirm_y_dispatches_and_moves_to_revealing() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        assert_eq!(modal.handle_key(KeyCode::Char('y')), RevealCommand::Confirm);
        assert!(matches!(modal.phase, RevealPhase::Revealing));
    }

    #[test]
    fn confirm_enter_dispatches() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        assert_eq!(modal.handle_key(KeyCode::Enter), RevealCommand::Confirm);
        assert!(matches!(modal.phase, RevealPhase::Revealing));
    }

    #[test]
    fn confirm_n_esc_close_without_dispatch() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        assert_eq!(modal.handle_key(KeyCode::Char('n')), RevealCommand::Close);
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        assert_eq!(modal.handle_key(KeyCode::Esc), RevealCommand::Close);
    }

    #[test]
    fn other_confirm_keys_are_noop() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        assert_eq!(modal.handle_key(KeyCode::Char('x')), RevealCommand::None);
        assert!(matches!(modal.phase, RevealPhase::Confirm));
    }

    #[test]
    fn revealing_swallows_every_key() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        modal.phase = RevealPhase::Revealing;
        for code in [
            KeyCode::Char('y'),
            KeyCode::Char('n'),
            KeyCode::Esc,
            KeyCode::Enter,
            KeyCode::Down,
        ] {
            assert_eq!(modal.handle_key(code), RevealCommand::None);
        }
    }

    #[test]
    fn ready_close_drops_secret_and_debug_never_leaks() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        modal.set_ready(WorkerRevealSecret::new("AKIASUPERSECRET123".to_string()));

        let debug = format!("{modal:?}");
        assert!(
            !debug.contains("AKIASUPERSECRET123"),
            "modal Debug must not leak the revealed value: {debug}"
        );
        assert!(debug.contains("(**)"), "redacted payload expected: {debug}");

        assert_eq!(modal.handle_key(KeyCode::Esc), RevealCommand::Close);
        assert_eq!(modal.handle_key(KeyCode::Char('n')), RevealCommand::Close);
        assert_eq!(modal.handle_key(KeyCode::Enter), RevealCommand::Close);
    }

    #[test]
    fn failed_closes_on_enter_or_esc() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        modal.set_failed("file changed since dig".to_string());
        assert_eq!(modal.handle_key(KeyCode::Enter), RevealCommand::Close);
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        modal.set_failed("boom".to_string());
        assert_eq!(modal.handle_key(KeyCode::Esc), RevealCommand::Close);
    }

    #[test]
    fn failed_other_keys_are_noop() {
        let mut modal = RevealModal::new(PathBuf::from("/repo/.env"), reference());
        modal.set_failed("boom".to_string());
        assert_eq!(modal.handle_key(KeyCode::Char('x')), RevealCommand::None);
    }
}
