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
| `AGENTS.md` | Памятка Orchestrator: правила, backlog Alpha, формат отчёта |
| `raccpack-agent-workflow.md` | Workflow Orchestrator / Dev / Test / Docs |
| `raccpack-roadmap-v1.md` | Дорожная карта до 1.0.0 |
| `raccpack-architecture-vision.md` | Слои, доверие, DTO |
| `raccpack-facade-and-den.md` | Facade API + den layout |
| `raccpack-modularity.md` | Secrets / archive backends |
| `raccpack-markers-detect-modularity.md` | Markers / detect по экосистемам |
| `WORKLOG.md` | Текущий журнал (Alpha+) |

## Правила для агента

- Спеки этапов (`docs/alpha/…`, `docs/archive/mvp/…`) читаются **только** по
  явной ссылке человека перед этапом.
- Пользовательская документация — только `wiki/` (VitePress). Сюда её не дублировать.
- Архив MVP не править (история). Новые решения — в текущий `WORKLOG.md`.
