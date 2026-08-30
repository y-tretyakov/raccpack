//! Two-step passphrase entry: typed material stays in [`Zeroizing`] and every
//! `Debug` output is redacted, so nothing embedding this type can leak raw
//! material.

use zeroize::Zeroizing;

/// Which of the two passphrase inputs is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassphraseStep {
    /// First entry.
    First,
    /// Repeat to confirm.
    Confirm,
}

impl PassphraseStep {
    /// Whether the user is on the repeat entry.
    pub fn is_confirm(self) -> bool {
        matches!(self, Self::Confirm)
    }
}

/// Two-step passphrase entry.
#[derive(Clone, PartialEq, Eq)]
pub struct PassphraseInput {
    first: Zeroizing<String>,
    confirm: Zeroizing<String>,
    step: PassphraseStep,
    error: Option<String>,
}

impl Default for PassphraseInput {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PassphraseInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PassphraseInput")
            .field("first", &"(hidden)")
            .field("confirm", &"(hidden)")
            .field("step", &self.step)
            .field("error", &self.error)
            .finish()
    }
}

impl PassphraseInput {
    /// Create an empty input on the first entry.
    pub fn new() -> Self {
        Self {
            first: Zeroizing::new(String::new()),
            confirm: Zeroizing::new(String::new()),
            step: PassphraseStep::First,
            error: None,
        }
    }

    /// The currently active entry step.
    pub fn step(&self) -> PassphraseStep {
        self.step
    }

    /// Number of characters typed into the first entry.
    pub fn first_len(&self) -> usize {
        self.first.len()
    }

    /// Number of characters typed into the repeat entry.
    pub fn confirm_len(&self) -> usize {
        self.confirm.len()
    }

    /// Validation error message from the last `Enter`.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Append one character to the active input.
    pub fn push_char(&mut self, c: char) {
        self.error = None;
        match self.step {
            PassphraseStep::First => self.first.push(c),
            PassphraseStep::Confirm => self.confirm.push(c),
        }
    }

    /// Remove the last character from the active input.
    pub fn backspace(&mut self) {
        self.error = None;
        match self.step {
            PassphraseStep::First => {
                self.first.pop();
            }
            PassphraseStep::Confirm => {
                self.confirm.pop();
            }
        }
    }

    /// Advance on `Enter`: move to the confirm entry, or — when both entries
    /// match and are non-empty — produce the confirmed passphrase.
    pub(crate) fn enter(&mut self) -> EnterOutcome {
        match self.step {
            PassphraseStep::First => {
                self.step = PassphraseStep::Confirm;
                self.error = None;
                EnterOutcome::AwaitConfirm
            }
            PassphraseStep::Confirm => {
                if self.first.is_empty() {
                    self.error = Some("passphrase must not be empty".to_string());
                    EnterOutcome::Mismatch
                } else if *self.first == *self.confirm {
                    let passphrase = std::mem::take(&mut self.first);
                    EnterOutcome::Confirmed(passphrase)
                } else {
                    self.reset_to_first();
                    self.error = Some("passphrases do not match".to_string());
                    EnterOutcome::Mismatch
                }
            }
        }
    }

    /// Reset both entries and return to the first input (mismatch recovery);
    /// the old buffers are dropped and zeroized.
    fn reset_to_first(&mut self) {
        self.first = Zeroizing::default();
        self.confirm = Zeroizing::default();
        self.step = PassphraseStep::First;
    }
}

/// Result of advancing [`PassphraseInput::enter`].
pub(crate) enum EnterOutcome {
    /// Move to the repeat entry and wait.
    AwaitConfirm,
    /// Both entries matched; the confirmed passphrase.
    Confirmed(Zeroizing<String>),
    /// Validation failed; the input was reset to the first entry.
    Mismatch,
}
