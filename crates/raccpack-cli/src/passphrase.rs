//! Passphrase acquisition for `racc stash`.
//!
//! Read order (used only in `RunMode::Commit`; DryRun must NOT call this):
//!
//! 1. `RACCPACK_PASSPHRASE` env var when set and non-empty (CI-friendly).
//! 2. Interactive double prompt on a TTY, echo disabled.
//! 3. A single line read from piped/non-TTY stdin.
//! 4. Otherwise an error with a hint about `RACCPACK_PASSPHRASE`.
//!
//! The value is returned zeroizing and is never printed or logged.

use std::io::{BufRead, IsTerminal};

use zeroize::Zeroizing;

use crate::error::CliError;

/// Read the stash passphrase following the priority order above.
pub fn read_passphrase() -> Result<Zeroizing<String>, CliError> {
    if let Ok(value) = std::env::var("RACCPACK_PASSPHRASE") {
        if !value.is_empty() {
            return Ok(Zeroizing::new(value));
        }
    }

    if std::io::stdin().is_terminal() {
        read_interactive()
    } else {
        read_pipe()
    }
}

/// Prompt twice on the controlling terminal, requiring a matching confirmation.
fn read_interactive() -> Result<Zeroizing<String>, CliError> {
    let first = rpassword::prompt_password("Passphrase: ")
        .map_err(|err| passphrase_error(format!("failed to read passphrase: {err}")))?;
    let second = rpassword::prompt_password("Confirm passphrase: ")
        .map_err(|err| passphrase_error(format!("failed to read confirmation: {err}")))?;
    if first != second {
        return Err(passphrase_error("passphrases do not match".to_string()));
    }
    Ok(Zeroizing::new(first))
}

/// Read a single line from piped stdin and strip the trailing newline.
fn read_pipe() -> Result<Zeroizing<String>, CliError> {
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|err| passphrase_error(format!("failed to read passphrase from stdin: {err}")))?;
    let value = line.trim_end_matches(['\n', '\r']).to_string();
    if value.is_empty() {
        return Err(passphrase_error(
            "no passphrase provided; set RACCPACK_PASSPHRASE or run `racc stash --yes` in an interactive terminal".to_string(),
        ));
    }
    Ok(Zeroizing::new(value))
}

fn passphrase_error(message: String) -> CliError {
    CliError::Passphrase { message }
}
