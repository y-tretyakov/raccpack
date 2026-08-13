# MVP 0.1.0 — Фаза M3: Dig (секреты, read-only)

Индекс подробных спецификаций этапов. Каждый этап — отдельный документ.

| Этап | Файл | Кратко |
|------|------|--------|
| **M3.1** | [m3.1-filename-patterns-risk.md](m3.1-filename-patterns-risk.md) | Filename patterns + risk model (severity API) |
| **M3.2** | [m3.2-content-markers.md](m3.2-content-markers.md) | Content markers (regex/prefix) + size limits + mask |
| **M3.3** | [m3.3-facade-dig.md](m3.3-facade-dig.md) | Facade `dig` — masked output, `exit_code_for_secrets` |
| **M3.4** | [m3.4-cli-dig.md](m3.4-cli-dig.md) | CLI `racc dig` + exit policy (`FailOnCritical`) |

## Порядок выполнения

```text
M3.1 → M3.2 → M3.3 → M3.4
```

- M3.1 блокирует content merge (нужны findings + risk).
- M3.2 блокирует полный dig (нужны mask + content hits).
- M3.3 блокирует CLI.
- Dev + Test параллельно на каждом этапе.

Зависимости от предыдущих фаз:

- M1.2 `SensitiveRisk`, M1.4 walk/SkipPolicy — обязательно.
- M2.3 `AppContext` / ProgressSink — для M3.3.
- M2.4 CLI skeleton — для M3.4.

## Exit criteria фазы M3

- Filename patterns ловят `.env`, keys, pem и т.п. с корректным risk.
- Content markers + size/binary limits; masked + fingerprint без raw в report.
- `dig` возвращает `DigResult` только с masked данными.
- `racc dig --root …` / `--json` / `--fail-on` работают; exit 2 при Critical (default policy).
- Read-only: ничего не удаляется и не шифруется (stash — Alpha A1).

## Инварианты безопасности (M3)

1. Raw secret не в JSON, human output, `Display` public DTO, tracing default.
2. Masking policy зафиксирована тестами.
3. Risk upgrade только через severity API (`max` / `upgrade_risk`).
4. Walk: `follow_links(false)` + SkipPolicy.

## Связь с MVP

```text
M1 каркас
 → M2 sniff
 → M3 dig          ← вы здесь
 → M4 pack + den   → MVP exit
```

## Связанные документы

- [m1-index.md](m1-index.md) · [m2-index.md](m2-index.md)
- `raccpack-roadmap-v1.md`
- `raccpack-architecture-vision.md`
- `raccpack-facade-and-den.md` — `dig`, `DigResult`, `SecretExitPolicy`
- `raccpack-agent-workflow.md`
