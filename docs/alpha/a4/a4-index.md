# Alpha — Фаза A4: Git и DX alpha

Индекс спецификаций. Каждый этап — отдельный документ.

| Этап | Файл | Кратко |
|------|------|--------|
| **A4.1** | [a4.1-git-client.md](a4.1-git-client.md) | GitClient (process) + git status в dig |
| **A4.2** | [a4.2-config-migrate-init.md](a4.2-config-migrate-init.md) | Config migrate chain + `racc init` |
| **A4.3** | [a4.3-tracing-verbose.md](a4.3-tracing-verbose.md) | tracing без секретов; `--verbose` |
| **A4.4** | [a4.4-integration-ci.md](a4.4-integration-ci.md) | Integration tests core + CI `cargo test` |
| **Wiki** | [wiki-git-and-dx.md](wiki-git-and-dx.md) | Пользовательская документация + **примеры CLI** |

## Порядок

```text
A4.1 → A4.2 → A4.3 → A4.4
```

A4.1 и A4.2 можно частично параллелить (разные подсистемы).  
A4.3 затрагивает CLI глобально.  
A4.4 — финализация Alpha quality gate.

## Exit criteria A4 / Alpha

- `GitClient` за интерфейсом; dig показывает git status sensitive files (tracked/untracked/ignored/dirty).
- `racc init` создаёт config; migrate chain для schema version.
- Логи tracing **без** raw secrets; `--verbose` / `-v`.
- CI job: `cargo test --workspace` green.
- Alpha: headless CLI feature-complete (sniff/dig/stash/rinse/pack/raid + init + verbose).

## Модульность (сводка)

```text
raccpack-core/src/
  git/
    mod.rs
    client.rs          # trait GitClient
    process.rs         # ProcessGitClient
    status.rs          # GitFileStatus mapping
  config/
    migrate.rs         # version chain
  app/dig.rs           # enrich SensitiveFile.git_status
raccpack-cli/src/
  commands/init.rs
  main.rs              # --verbose → tracing subscriber
.github/workflows/ci.yml  # or docs for CI
docs/wiki/git-and-dx.md
```

## Follow-ups из MVP (для A4)

Источник: [FOLLOWUPS_FROM_MVP.md](FOLLOWUPS_FROM_MVP.md).

| ID | Что сделать в A4 |
|----|------------------|
| **F-ERR-1** | Map `ConfigError` → CLI/`Error` (**A4.2**) |
| **F-CFG-4** | `config_version` + migrate (**A4.2**) |
| **F-CFG-2** | MSRV/toolchain в CI (**A4.4**) |

Beta/RC: Windows paths, pub-use audit, EN wiki, EnabledGroups.

## Связь

```text
A3 raid → A4 git+DX → Alpha 0.3 exit → Beta (TUI/Desktop)
```
