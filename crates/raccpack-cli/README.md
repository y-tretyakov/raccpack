# raccpack-cli

Command-line binary `racc` for `raccpack`.

Implemented subcommands:

- `racc sniff --root PATH` — project discovery (text or `--json`).
- `racc dig --project PATH` — secret findings with `--fail-on` exit policy.
- `racc pack --project PATH --yes` — archive a project into the den as `tar.zst` (dry-run by default, commit with `--yes`).

`stash` / `rinse` / `raid` come later (Alpha).

Full supported catalog (markers, secrets, skip dirs, deny rules): [Что поддерживается](../../wiki/supported.md).
