# WORKLOG — raccpack

Журнал статусов этапов. Orchestrator: y-tretyakov.

**MVP 0.1.0 закрыт.** Полный журнал M1–M4 и docs-миграции:
[`docs/archive/WORKLOG_MVP.md`](docs/archive/WORKLOG_MVP.md).
Спеки закрытых этапов: [`docs/archive/mvp/`](docs/archive/mvp/).

## Backlog (Alpha → 0.3.0)

```
[ ] A1.1 age + zeroize passphrase
[ ] A1.2 stash manifest (без raw) + remove sources в Commit
[ ] A1.3 facade stash + den/secrets/…
[ ] A1.4 CLI racc stash
[ ] A2.1 cleanup strategies + config toggles
[ ] A2.2 facade rinse DryRun/Commit
[ ] A2.3 CLI racc rinse
[ ] A3.1 facade raid (stash→rinse→pack→move, fail-fast)
[ ] A3.2 ProgressSink + CLI progress
[ ] A3.3 manifest JSON в den/manifests/
[ ] A3.4 CLI racc raid --yes; E2E alpha
[ ] A4.1 GitClient (process) + status sensitive files в dig
[ ] A4.2 Config migrate chain + racc init
[ ] A4.3 tracing без секретов; --verbose
[ ] A4.4 integration tests core + CI cargo test
```

## Этапы

_(пусто — первый этап Alpha ещё не стартовал)_

## Принятые решения (Alpha+)

| Дата | Решение |
|------|---------|
| 2026-08-13 | Релиз-подготовка MVP: agent tooling (`.agents/`, `skills-lock.json`) убран из репо; WORKLOG MVP → `docs/archive/WORKLOG_MVP.md`; спеки M1–M4 → `docs/archive/mvp/`; one-shot промпты Writerside/VitePress icons удалены. Новый `WORKLOG.md` только для Alpha+. |
| 2026-08-13 | Документы агента: `AGENTS.md` переписан под Alpha; knowledge base (architecture / facade / modularity / roadmap / workflow) остаётся в корне. |

## Tracked с MVP (не блокеры Alpha start)

См. конец `docs/archive/WORKLOG_MVP.md` и таблицу решений там:

- P1-4 `SkipPolicy::default_pack()` (расширенный список) — позже.
- P2-5 `zstd_level` из `[advanced]` при появлении секции.
- P2-6 cost content-deny (extensions / size-cap) — оптимизация позже.
- P2-7 сужение public API (`lib.rs`) — с фазой hygiene / R1.
- P2-8 типизация `Error::Other` (DenInsideProject / InvalidOutputName) — CLI UX.
- `is_under_root` / path-containment — перед stash destructive paths.
- ConfigError ↔ domain Error merge — на facade maturity.
- Windows HOME/XDG — best-effort после Linux primary.
