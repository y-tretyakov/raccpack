# MVP 0.1.0 — Фаза M4: Pack + den layout (минимум)

Индекс подробных спецификаций этапов. Каждый этап — отдельный документ.

| Этап | Файл | Кратко |
|------|------|--------|
| **M4.1** | [m4.1-pack-tar-zstd.md](m4.1-pack-tar-zstd.md) | Pack tar+zstd, deny-list по имени, SkipPolicy |
| **M4.2** | [m4.2-den-layout.md](m4.2-den-layout.md) | `den/packs/…`, `.den-version`, README, place_pack |
| **M4.3** | [m4.3-facade-pack.md](m4.3-facade-pack.md) | Facade `pack` + DryRun/Commit |
| **M4.4** | [m4.4-cli-pack-e2e.md](m4.4-cli-pack-e2e.md) | CLI `racc pack` + E2E MVP |

## Порядок выполнения

```text
M4.1 → M4.2 → M4.3 → M4.4
```

- M4.1: низкоуровневая упаковка в файл.
- M4.2: den skeleton + atomic place.
- M4.3: facade склеивает + DryRun.
- M4.4: CLI + закрытие MVP.

Зависимости: M1.4 walk/SkipPolicy, M3.1 filename deny, M2.3 AppContext/RunMode.

## Exit criteria фазы M4 / MVP 0.1.0

- Валидный `.tar.zst` без symlink-follow и без High+ secret filenames.
- Den v1: `.den-version`, README, `packs/yyyy/mm/slug__ts.tar.zst`.
- `pack` DryRun не пишет; Commit пишет в den.
- `racc pack --project … --den … --yes` работает.
- E2E: sniff + dig + pack на fixture.

**Вне MVP:** age stash, rinse, raid, TUI, Desktop, git status в dig.

## Инварианты

1. `follow_links(false)` на walk и pack.
2. Absolute paths / `..` не попадают в tar entries.
3. DryRun не создаёт файлы в `packs/`.
4. Name-deny secrets в pack по умолчанию on.
5. Den permissions best-effort `0700` / files `0600`.

## Полный путь MVP

```text
M1 каркас → M2 sniff → M3 dig → M4 pack+den → 0.1.0
```

## Связанные документы

- [m1-index.md](m1-index.md) · [m2-index.md](m2-index.md) · [m3-index.md](m3-index.md)
- `raccpack-roadmap-v1.md`
- `raccpack-facade-and-den.md` §7 pack, §9 den layout
- `raccpack-architecture-vision.md`
- `raccpack-agent-workflow.md`
