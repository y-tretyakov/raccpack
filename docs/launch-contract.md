# Launch Contract

Shared semantic flags for `racc-tui` and, in the future, the Desktop app.
Parsed **before** any terminal init (raw mode / alternate screen / ratatui).

| Flag | Meaning |
|------|---------|
| `--version` / `-V` | Print version and exit 0 (no terminal init). |
| `--help` / `-h` | Print help and exit 0 (no terminal init). |
| `--root <PATH>` | Project scan root. Default: `~/DEV/PROJS` if it exists, else cwd. |
| `--den <PATH>` | Den directory. Overrides `RACCPACK_DEN`; else default `~/.raccpack/den`. |
| `--config <PATH>` | Config file. Parsed/stored for later; not wired into the worker yet. |
| `--view <overview\|projects\|findings\|operations>` | Initial view. Default: `overview`. |
| `--refresh` | Run a sniff refresh on startup when landing on the Projects view. |
| `-v` / `--verbose` | Increase verbosity (`-v` info, `-vv` debug, `-vvv` trace). Stored for later. |

Non-interactive invocation (`stdout` or `stdin` is not a TTY) is refused with
`racc-tui requires an interactive terminal` on stderr and a non-zero exit code,
before any raw-mode output.
