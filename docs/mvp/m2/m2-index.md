# MVP 0.1.0 — Фаза M2: Sniff (обнаружение проектов)

Индекс подробных спецификаций этапов. Каждый этап — отдельный документ.

| Этап | Файл | Кратко |
|------|------|--------|
| **M2.1** | [m2.1-markers-candidates.md](m2.1-markers-candidates.md) | Marker files + skip dirs → `ProjectCandidate` |
| **M2.2** | [m2.2-detect-stack.md](m2.2-detect-stack.md) | Detect languages/frameworks → `Stack`, size |
| **M2.3** | [m2.3-facade-sniff-cache.md](m2.3-facade-sniff-cache.md) | Facade `sniff` + versioned cache + ProgressSink |
| **M2.4** | [m2.4-cli-sniff.md](m2.4-cli-sniff.md) | CLI `racc sniff --root …` (text + `--json`) |

## Порядок выполнения

```text
M2.1 → M2.2 → M2.3 → M2.4
```

- M2.1 блокирует detect (нужны candidates).
- M2.2 блокирует полный report (нужен Stack + size).
- M2.3 блокирует CLI (нужен facade).
- Dev + Test параллельно на каждом этапе с M2.1.

Зависимости от M1:

- M1.4 walk + SkipPolicy — обязательно для M2.1.
- M1.2 DTO — обязательно.
- M1.3 config — обязательно для M2.3/M2.4 (`scan_root`, `max_depth`).

## Exit criteria фазы M2

- `find_candidates` находит проекты по markers, не заходя в `node_modules`/`target`.
- `Stack` заполняется детерминированно (language + frameworks + markers).
- `sniff` возвращает `ScanReport`, поддерживает cache и `force_refresh`.
- `racc sniff --root PATH` и `--json` работают end-to-end.
- Нет content-secret scan и нет записи в den (это M3/M4).

## Связь с MVP

```text
M1 workspace/DTO/config/walk
 → M2 sniff          ← вы здесь
 → M3 dig
 → M4 pack + den
```

После M2 пользователь уже видит список проектов и стеки через CLI.

## Связанные документы

- [m1-index.md](m1-index.md) — предыдущая фаза
- `raccpack-roadmap-v1.md` — вехи
- `raccpack-architecture-vision.md` — границы core/UI
- `raccpack-facade-and-den.md` — сигнатуры `sniff` / `SniffResult`
- `raccpack-agent-workflow.md` — Orchestrator / Dev / Test
