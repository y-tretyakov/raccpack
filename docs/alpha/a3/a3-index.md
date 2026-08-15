# Alpha — Фаза A3: Raid orchestration

Индекс спецификаций. Каждый этап — отдельный документ.

| Этап | Файл | Кратко |
|------|------|--------|
| **A3.1** | [a3.1-facade-raid.md](a3.1-facade-raid.md) | Facade `raid`: stash → rinse → pack → move, fail-fast |
| **A3.2** | [a3.2-progress.md](a3.2-progress.md) | ProgressSink фазы raid + CLI progress |
| **A3.3** | [a3.3-manifest-json.md](a3.3-manifest-json.md) | Manifest JSON в `den/manifests/…` |
| **A3.4** | [a3.4-cli-raid-e2e.md](a3.4-cli-raid-e2e.md) | CLI `racc raid --yes` + E2E alpha |
| **Wiki** | [wiki-raid.md](wiki-raid.md) | Пользовательская документация + примеры CLI |

## Порядок

```text
A3.1 → A3.2 → A3.3 → A3.4
```

Зависимости: **A1 stash**, **A2 rinse**, **M4 pack** + den layout.  
A3.2 можно частично параллелить с A3.1 (события progress уже в facade).  
A3.3 — запись manifest после успешного/частичного raid.  
A3.4 — CLI и E2E.

## Exit criteria A3

- Один вызов `raid` выполняет включённые фазы: stash → rinse → pack → move (finalize).
- Fail-fast: первая failed enabled-фаза останавливает последующие; `success: false`.
- DryRun: ничего не пишет в secrets/packs/manifests и не удаляет.
- Progress по фазам с `OperationKind::Raid`.
- Manifest JSON в `den/manifests/yyyy/mm/…`.
- `racc raid --project … --den … --yes` (+ passphrase для stash).
- E2E на fixture: secrets.age + pack + manifest.

## Модульность (сводка)

```text
raccpack-core/src/
  app/
    raid.rs              # facade raid()
  den/
    manifest.rs          # write_raid_manifest, naming
  # reuse: app/stash, app/rinse, app/pack, den/place*
raccpack-cli/src/
  commands/raid.rs
  # reuse: passphrase.rs
docs/wiki/raid.md
```

## Follow-ups из MVP (для A3)

Источник: [FOLLOWUPS_FROM_MVP.md](FOLLOWUPS_FROM_MVP.md).

| ID | Что сделать в A3 |
|----|------------------|
| **F-PATH-1/3** | Raid не ослабляет containment stash/pack; staging только under den |
| **F-SKIP-1** | Pack-фаза с согласованным name-deny + SkipPolicy |

## Связь с Alpha

```text
A1 stash → A2 rinse → A3 raid (оркестрация) → A4 git+CI
```

**Alpha exit:** одной командой `raid` секреты в `.age`, мусор чистится, pack в den, manifest на месте.
