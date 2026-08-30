//! B1.4 raid modal flow: state machine driving preview → passphrase → run.
//!
//! This module owns the flow's local state and key handling only. Core phase
//! logic stays in `raccpack_core::app::raid` (called exclusively from the
//! worker); the two-step passphrase entry lives in [`passphrase`]; the modal
//! renderer lives in `ui/screens/raid.rs`.

use std::path::PathBuf;

use crossterm::event::KeyCode;
use raccpack_core::app::{OrchestrationMode, ProgressEvent, RaidResult};
use zeroize::Zeroizing;

use self::passphrase::EnterOutcome;
pub use self::passphrase::{PassphraseInput, PassphraseStep};

/// Options editable from the flow's Preview; mirrored to the worker on run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RaidFlowOptions {
    /// Keep source files after a Commit (`remove_sources=false`).
    pub keep_sources: bool,
    /// Skip the stash phase entirely.
    pub skip_stash: bool,
    /// Orchestration mode: Atomic (default) or FailFast.
    pub mode: OrchestrationMode,
}

impl Default for RaidFlowOptions {
    fn default() -> Self {
        Self {
            keep_sources: false,
            skip_stash: false,
            mode: OrchestrationMode::Atomic,
        }
    }
}

impl RaidFlowOptions {
    /// Toggle `keep_sources`; updates only the Preview display and the final
    /// options passed to the worker.
    pub fn toggle_keep_sources(&mut self) {
        self.keep_sources = !self.keep_sources;
    }

    /// Toggle `skip_stash`; updates only the Preview display and the final
    /// options passed to the worker.
    pub fn toggle_skip_stash(&mut self) {
        self.skip_stash = !self.skip_stash;
    }

    /// Toggle `mode` between Atomic and FailFast.
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            OrchestrationMode::Atomic => OrchestrationMode::FailFast,
            OrchestrationMode::FailFast => OrchestrationMode::Atomic,
        };
    }
}

/// Current phase of the raid modal flow.
#[derive(Clone, PartialEq, Eq)]
pub enum FlowPhase {
    /// Worker is computing the dry-run preview.
    Preparing,
    /// Dry-run preview is available.
    Preview(RaidResult),
    /// Passphrase entry (two inputs).
    Passphrase(PassphraseInput),
    /// Commit run is in progress.
    Running,
    /// Commit run finished.
    Done(RaidResult),
    /// The flow failed or was interrupted with a message.
    Failed(String),
}

impl std::fmt::Debug for FlowPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Preparing => f.write_str("Preparing"),
            Self::Preview(_) => f.write_str("Preview(..)"),
            Self::Passphrase(input) => f.debug_tuple("Passphrase").field(input).finish(),
            Self::Running => f.write_str("Running"),
            Self::Done(_) => f.write_str("Done(..)"),
            Self::Failed(message) => f.debug_tuple("Failed").field(message).finish(),
        }
    }
}

/// Command the flow hands back to the app/event layer.
///
/// Deliberately not `Copy`: the confirmed passphrase is moved out through
/// [`RaidCommand::PassphraseConfirm`] and stored on the flow, so the command
/// passed through the app loop never carries raw material twice.
#[derive(Clone, PartialEq, Eq)]
pub enum RaidCommand {
    /// Preview confirmed; stash enabled — proceed (passphrase or direct run).
    PreviewConfirm,
    /// Preview cancelled.
    PreviewCancel,
    /// Passphrase entered and confirmed (match verified).
    PassphraseConfirm(Zeroizing<String>),
    /// Passphrase entry cancelled.
    PassphraseCancel,
    /// Run immediately (stash skipped).
    Run,
    /// Close the flow (Done / Failed).
    Close,
}

impl std::fmt::Debug for RaidCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreviewConfirm => f.write_str("PreviewConfirm"),
            Self::PreviewCancel => f.write_str("PreviewCancel"),
            Self::PassphraseConfirm(_) => f.write_str("PassphraseConfirm(..)"),
            Self::PassphraseCancel => f.write_str("PassphraseCancel"),
            Self::Run => f.write_str("Run"),
            Self::Close => f.write_str("Close"),
        }
    }
}

/// One pipeline row on the Running screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseLine {
    /// Phase name: `stash` | `rinse` | `pack` | `move`.
    pub name: String,
    /// `phase_complete` from the latest event for this phase.
    pub done: bool,
    /// Whether this is the first not-yet-completed phase.
    pub current: bool,
    /// Latest progress message for this phase.
    pub message: String,
}

/// State machine for one raid flow.
///
/// `Debug` redacts both the typed passphrase and any confirmed passphrase, so
/// `App` (which embeds this) never leaks raw material in its own `Debug`.
pub struct RaidFlow {
    /// Project being raided.
    pub project: PathBuf,
    /// Options shown and toggled in the Preview.
    pub options: RaidFlowOptions,
    /// Current phase of the modal.
    pub phase: FlowPhase,
    /// Resolved den directory (for den-relative artifact paths).
    pub den_dir: PathBuf,
    /// Phase pipeline derived from raid progress events.
    pub pipeline: Vec<PhaseLine>,
    /// Progress within the current phase, 0–100 (from core, never invented).
    pub percent: u8,
    /// Overall progress, 0–100 (from core, never invented).
    pub overall_percent: u8,
    /// Latest progress message.
    pub message: String,
    /// Passphrase confirmed in the modal, kept out of `Command`.
    confirmed_passphrase: Option<Zeroizing<String>>,
}

impl std::fmt::Debug for RaidFlow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RaidFlow")
            .field("project", &self.project)
            .field("options", &self.options)
            .field("phase", &self.phase)
            .field("den_dir", &self.den_dir)
            .field("pipeline", &self.pipeline)
            .field("percent", &self.percent)
            .field("overall_percent", &self.overall_percent)
            .field("message", &self.message)
            .field("confirmed_passphrase", &"(hidden)")
            .finish()
    }
}

impl RaidFlow {
    /// Create a flow in the `Preparing` phase with default options.
    pub fn new(project: PathBuf, den_dir: PathBuf, options: RaidFlowOptions) -> Self {
        Self {
            project,
            options,
            phase: FlowPhase::Preparing,
            den_dir,
            pipeline: Vec::new(),
            percent: 0,
            overall_percent: 0,
            message: String::new(),
            confirmed_passphrase: None,
        }
    }

    /// Planned phase names in run order, honoring the current options.
    pub fn planned_names(&self) -> Vec<&'static str> {
        let mut names = Vec::with_capacity(4);
        if !self.options.skip_stash {
            names.push("stash");
        }
        names.push("rinse");
        names.push("pack");
        names.push("move");
        names
    }

    /// Handle one keypress inside the flow. Returns the command that the
    /// caller executes, or `None` when the key only updated local state.
    pub fn handle_key(&mut self, key: KeyCode) -> Option<RaidCommand> {
        match &mut self.phase {
            FlowPhase::Preparing | FlowPhase::Preview(_) => match key {
                KeyCode::Char('y') | KeyCode::Enter => Some(if self.options.skip_stash {
                    RaidCommand::Run
                } else {
                    RaidCommand::PreviewConfirm
                }),
                KeyCode::Char('n') | KeyCode::Esc => Some(RaidCommand::PreviewCancel),
                KeyCode::Char('K') => {
                    self.options.toggle_keep_sources();
                    None
                }
                KeyCode::Char('S') => {
                    self.options.toggle_skip_stash();
                    None
                }
                KeyCode::Char('m') => {
                    self.options.toggle_mode();
                    None
                }
                _ => None,
            },
            FlowPhase::Passphrase(input) => match key {
                KeyCode::Esc => Some(RaidCommand::PassphraseCancel),
                KeyCode::Enter => match input.enter() {
                    EnterOutcome::AwaitConfirm | EnterOutcome::Mismatch => None,
                    EnterOutcome::Confirmed(passphrase) => {
                        Some(RaidCommand::PassphraseConfirm(passphrase))
                    }
                },
                KeyCode::Backspace => {
                    input.backspace();
                    None
                }
                KeyCode::Char(c) if !c.is_control() => {
                    input.push_char(c);
                    None
                }
                _ => None,
            },
            // Esc must not interrupt or cancel a running commit: the core has
            // no cancel, so the modal blocks until the run reports back.
            FlowPhase::Running => None,
            FlowPhase::Done(_) | FlowPhase::Failed(_) => match key {
                KeyCode::Enter | KeyCode::Esc => Some(RaidCommand::Close),
                _ => None,
            },
        }
    }

    /// Fold the latest raid progress event into the Running state.
    pub fn on_progress(&mut self, event: &ProgressEvent) {
        self.percent = event.percent;
        self.overall_percent = event.overall_percent;
        self.message = event.message.clone();

        if self.pipeline.is_empty() {
            self.pipeline = self
                .planned_names()
                .into_iter()
                .map(|name| PhaseLine {
                    name: name.to_string(),
                    done: false,
                    current: false,
                    message: String::new(),
                })
                .collect();
        }
        if let Some(line) = self.pipeline.iter_mut().find(|l| l.name == event.phase) {
            line.done = event.phase_complete;
            line.message = event.message.clone();
        }
        let mut first_pending = true;
        for line in self.pipeline.iter_mut() {
            if line.done {
                line.current = false;
            } else {
                line.current = first_pending;
                first_pending = false;
            }
        }
    }

    /// Move to the passphrase entry phase.
    pub fn start_passphrase(&mut self) {
        self.phase = FlowPhase::Passphrase(PassphraseInput::new());
    }

    /// Move to the Running phase (the worker message has been dispatched).
    pub fn start_running(&mut self) {
        self.phase = FlowPhase::Running;
    }

    /// Store a passphrase confirmed by the modal; the app keeps it here so it
    /// never travels inside a `Command`.
    pub fn store_confirmed(&mut self, passphrase: Zeroizing<String>) {
        self.confirmed_passphrase = Some(passphrase);
    }

    /// Take the confirmed passphrase out of the flow (drops it after use).
    pub fn take_passphrase(&mut self) -> Option<Zeroizing<String>> {
        self.confirmed_passphrase.take()
    }
}

mod passphrase;

#[cfg(test)]
mod tests;
