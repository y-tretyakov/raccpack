# A3 — Raid orchestration (от текущей позиции)

**Код в `dev`:** A3.1 + A3.2 ✅  
Спека стыкует shipped fail-fast с целевым **Atomic** (vision/roadmap).

---

## Уже в коде (не переделывать с нуля)

| Этап | Статус | Документ |
|------|--------|----------|
| **A3.1** facade fail-fast | ✅ | [a3.1-facade-raid-SHIPPED.md](a3.1-facade-raid-SHIPPED.md) |
| **A3.2** progress + thin CLI | ✅ | [a3.2-progress-SHIPPED.md](a3.2-progress-SHIPPED.md) |

Кратко A3.1: stash→rinse→pack→move; fail-fast; `StashEmpty` no-op; **нет** WAL.  
Кратко A3.2: completion events; `racc raid --project/--yes/--dry-run`; exit 0 даже при `success:false`.

---

## Дальше

| Этап | Файл | Суть |
|------|------|------|
| **A3.3** | [a3.3-atomic-upgrade.md](a3.3-atomic-upgrade.md) | Staging + WAL + rollback; default **Atomic** |
| **A3.4** | [a3.4-manifest-after-commit.md](a3.4-manifest-after-commit.md) | Manifest только после success |
| **A3.5** | [a3.5-cli-e2e-wiki.md](a3.5-cli-e2e-wiki.md) | Toggles, exit 1, E2E, orphan, wiki |

```text
A3.3 atomic → A3.4 manifest → A3.5 CLI/E2E/wiki → A4
```

---

## Модульность (после A3.3)

```text
app/raid/{mod,stages,progress}.rs   # есть
app/raid/{staging,wal,rollback}.rs  # A3.3
den/manifest.rs                     # A3.4
cli/commands/raid.rs                # expand A3.5
```

Черновики `a3.0`…`a3.7` (vision до shipped) **не** использовать как порядок — смысл слит в A3.3–A3.5.
