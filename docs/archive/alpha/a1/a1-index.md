# Alpha 0.2–0.3 — Фаза A1: Stash (age)

Индекс спецификаций этапов. Каждый этап — отдельный документ.

| Этап | Файл | Кратко |
|------|------|--------|
| **A1.1** | [a1.1-age-integration-zeroize.md](a1.1-age-integration-zeroize.md) | age + passphrase, zeroize ключа |
| **A1.2** | [a1.2-stash-manifest-remove.md](a1.2-stash-manifest-remove.md) | Manifest без raw; удаление источников в Commit |
| **A1.3** | [a1.3-facade-stash-den.md](a1.3-facade-stash-den.md) | Facade `stash` + `den/secrets/…` |
| **A1.4** | [a1.4-cli-stash.md](a1.4-cli-stash.md) | CLI `racc stash` (prompt / env) |
| **Wiki** | [wiki-stash.md](wiki-stash.md) | Пользовательская документация (совпадает с реализацией) |

## Порядок

```text
A1.1 → A1.2 → A1.3 → A1.4
```

Зависимости: M3 dig (список секретов), M4.2 den layout (`ensure_den`, naming), M1.2 risk/DTO.

## Exit criteria A1

- Секреты уезжают в `.age` в `den/secrets/yyyy/mm/`.
- Passphrase/материал ключа zeroize после use.
- Manifest entries **без raw**.
- DryRun не пишет age и не удаляет источники.
- Commit: encrypt → optional remove sources.
- `racc stash` с prompt или `RACCPACK_PASSPHRASE` / stdin для CI.

## Модульность (сводка по crate)

```text
raccpack-core/src/
  archive/
    age_vault.rs      # A1.1 encrypt/decrypt primitives, zeroize
  secrets/
    stash_select.rs   # A1.2 выбор файлов по dig/min_risk
  den/
    secrets_place.rs  # A1.3 place .age under den/secrets/
  app/
    stash.rs          # A1.3 facade stash()
raccpack-cli/src/
  commands/stash.rs   # A1.4
docs/wiki/
  stash.md            # user-facing (копия/синк с wiki-stash.md)
```

## Follow-ups из MVP (обязательные для A1)

Источник: [FOLLOWUPS_FROM_MVP.md](FOLLOWUPS_FROM_MVP.md).

| ID | Что сделать в A1 | Статус |
|----|------------------|--------|
| **F-PATH-1** | Единый path-containment: stash-файлы только under `target`; нет escape через symlink/`..` | ✅ closed (A1.2 `stash_select` + `is_path_under_root`) |
| **F-PATH-3** | Staging `.age` только под den, не внутри project tree | ✅ closed (A1.3: `den/staging/…`, guard в `stash`) |
| **F-SKIP-2** | File-deny секретов ≠ dir SkipPolicy (уже name-based в secrets) | ✅ name-based (A1.2) |

## Связь с Alpha

```text
MVP pack (без age)
 → A1 stash/age     ← вы здесь
 → A2 rinse
 → A3 raid (stash→rinse→pack→manifest)
 → A4 git + CI
```
