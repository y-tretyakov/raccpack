# docs/

Внутренняя (dev) документация. **Не** публикуется в wiki / GitHub Pages.

## Структура

```
docs/
  README.md                 # этот файл
  config.example.toml       # пример конфига (runtime)
  alpha/                    # спеки этапов Alpha (A1–A4) — по мере появления
  archive/
    WORKLOG_MVP.md          # полный журнал закрытого MVP 0.1.0
    mvp/                    # спеки закрытых этапов M1–M4
```

## Корневые knowledge-docs (не переносить сюда без нужды)

| Файл | Назначение |
|------|------------|
| `AGENTS.md` | Единственная памятка Orchestrator: правила, backlog Alpha, модульность, git, wiki, инварианты |
| `WORKLOG.md` | Текущий журнал (Alpha+) |

Устаревшие knowledge-docs (`raccpack-agent-workflow.md`, `raccpack-roadmap-v1.md`,
`raccpack-architecture-vision.md`, `raccpack-facade-and-den.md`, `raccpack-modularity.md`,
`raccpack-markers-detect-modularity.md`, `FOLLOWUPS_FROM_MVP.md`) **удалены** — их содержание
консолидировано в `AGENTS.md` (см. §4/§10). Не восстанавливать.

## Правила для агента

- Спеки этапов (`docs/alpha/…`, `docs/archive/mvp/…`) читаются **только** по
  явной ссылке человека перед этапом.
- Пользовательская документация — только `wiki/` (VitePress). Сюда её не дублировать.
- Архив MVP не править (история). Новые решения — в текущий `WORKLOG.md`.
