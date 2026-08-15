# Alpha — Фаза A2: Rinse (очистка)

Индекс спецификаций. Каждый этап — отдельный документ.

| Этап | Файл | Кратко |
|------|------|--------|
| **A2.1** | [a2.1-cleanup-strategies.md](a2.1-cleanup-strategies.md) | Стратегии cleanup + config toggles |
| **A2.2** | [a2.2-facade-rinse.md](a2.2-facade-rinse.md) | Facade `rinse` DryRun/Commit + bytes freed |
| **A2.3** | [a2.3-cli-rinse.md](a2.3-cli-rinse.md) | CLI `racc rinse` |
| **Wiki** | [wiki-rinse.md](wiki-rinse.md) | Пользовательская документация |

## Порядок

```text
A2.1 → A2.2 → A2.3
```

Зависимости: M1.4 (SkipPolicy/walk), M1.3 (config), M2.3 (AppContext, RunMode, ProgressSink).  
Не зависит от age/stash (A1) — rinse можно вести параллельно с A1, если не правят одни и те же файлы.

## Exit criteria A2

- Стратегии: rust / node / python / … (data-driven).
- Config: включение/выключение стратегий.
- `rinse` DryRun только перечисляет; Commit удаляет trash-dirs.
- Подсчёт `bytes_freed`.
- `racc rinse --project …` (+ `--yes` / `--dry-run` / `--json`).
- **Не** трогает секретные файлы (это stash) и **не** пакует.

## Модульность (сводка)

```text
raccpack-core/src/
  clean/
    mod.rs
    strategy.rs       # StrategyId, TrashPattern, DEFAULT_STRATEGIES
    detect.rs         # match dirs under target
    remove.rs         # delete dir tree + bytes
  config/             # extend: cleanup.enabled_strategies
  app/
    rinse.rs          # facade rinse()
raccpack-cli/src/
  commands/rinse.rs
docs/wiki/rinse.md    # = wiki-rinse.md
```

## Follow-ups из MVP (для A2)

Источник: AGENTS.md (follow-ups консолидированы; `FOLLOWUPS_FROM_MVP.md` удалён).

| ID | Что сделать в A2 |
|----|------------------|
| **F-SKIP-1** | Patterns rinse (`.next`, `coverage`, …) согласовать с будущим `SkipPolicy::default_pack()` |

## Связь с Alpha

```text
A1 stash/age
A2 rinse        ← вы здесь
A3 raid (stash → rinse → pack → manifest)
```
