//! tracing subscriber initialization for the `racc` binary.
//!
//! Verbosity maps to log filters as follows (when `RUST_LOG` is unset):
//!
//! | Flags  | Filter                                        |
//! |--------|-----------------------------------------------|
//! | (none) | `warn`                                        |
//! | `-v`   | `info,raccpack_core=info,raccpack_cli=info`   |
//! | `-vv`  | `debug,raccpack_core=debug,raccpack_cli=debug`|
//! | `-vvv+`| `trace,raccpack_core=trace,raccpack_cli=trace`|
//!
//! `RUST_LOG` wins when set to a non-empty value. Logs always go to
//! **stderr** so `--json` output on stdout stays machine-clean.

use std::io::IsTerminal;

use tracing_subscriber::EnvFilter;

/// Initialize the global tracing subscriber exactly once.
///
/// Safe to call repeatedly: subsequent calls are no-ops instead of panicking.
pub fn init_tracing(verbose: u8) {
    let filter = resolve_filter(verbose, std::env::var("RUST_LOG").ok().as_deref());
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(true)
        .with_ansi(std::io::stderr().is_terminal())
        .with_writer(std::io::stderr)
        .try_init();
}

/// Pure filter selection from the `-v` count (no environment involved).
fn filter_for_verbosity(verbose: u8) -> String {
    match verbose {
        0 => "warn".to_string(),
        1 => "info,raccpack_core=info,raccpack_cli=info".to_string(),
        2 => "debug,raccpack_core=debug,raccpack_cli=debug".to_string(),
        _ => "trace,raccpack_core=trace,raccpack_cli=trace".to_string(),
    }
}

/// Effective filter string: a non-empty `RUST_LOG` overrides the verbosity.
fn resolve_filter(verbose: u8, rust_log: Option<&str>) -> String {
    match rust_log.map(str::trim) {
        Some(log) if !log.is_empty() => log.to_string(),
        _ => filter_for_verbosity(verbose),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_default_verbosity_is_warn() {
        assert_eq!(filter_for_verbosity(0), "warn");
    }

    #[test]
    fn filter_single_v_is_info() {
        assert_eq!(
            filter_for_verbosity(1),
            "info,raccpack_core=info,raccpack_cli=info"
        );
    }

    #[test]
    fn filter_double_v_is_debug() {
        assert_eq!(
            filter_for_verbosity(2),
            "debug,raccpack_core=debug,raccpack_cli=debug"
        );
    }

    #[test]
    fn filter_triple_v_and_beyond_is_trace() {
        assert_eq!(
            filter_for_verbosity(3),
            "trace,raccpack_core=trace,raccpack_cli=trace"
        );
        assert_eq!(
            filter_for_verbosity(4),
            "trace,raccpack_core=trace,raccpack_cli=trace"
        );
        assert_eq!(
            filter_for_verbosity(u8::MAX),
            "trace,raccpack_core=trace,raccpack_cli=trace"
        );
    }

    #[test]
    fn rust_log_overrides_verbosity() {
        for verbose in [0u8, 1, 2, 3, 9] {
            assert_eq!(
                resolve_filter(verbose, Some("raccpack_core=debug")),
                "raccpack_core=debug",
                "RUST_LOG must win at every verbosity level"
            );
        }
    }

    #[test]
    fn missing_rust_log_falls_back_to_verbosity() {
        assert_eq!(resolve_filter(0, None), "warn");
        assert_eq!(
            resolve_filter(2, None),
            "debug,raccpack_core=debug,raccpack_cli=debug"
        );
    }

    #[test]
    fn empty_rust_log_is_treated_as_unset() {
        assert_eq!(resolve_filter(0, Some("")), "warn");
        assert_eq!(
            resolve_filter(1, Some("   ")),
            "info,raccpack_core=info,raccpack_cli=info"
        );
    }

    #[test]
    fn init_tracing_does_not_panic_on_repeated_calls() {
        init_tracing(0);
        init_tracing(2);
    }
}
