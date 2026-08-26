# TUI — операционный терминал

**Команда:** `racc tui`  
**Статус:** Beta  

TUI — **thin client** над `raccpack-core`. Не вызывает CLI через shell.

Формула:

> **Inspect → Preview → Execute → Verify**

---

## Запуск

```bash
racc tui
# config missing → init wizard
```

---

## Навигация (views)

| View | Смысл |
|------|--------|
| Overview | состояние workspace |
| Projects | таблица проектов |
| Findings | находки (masked) |
| Den | packs / secrets / manifests |
| Operations | running + history |
| Config | RaccConfig + UI prefs отдельно |

**Raid — не отдельная «страница»**, а **Operation** из Projects / palette.

---

## Клавиши (основные)

| Key | Action |
|-----|--------|
| `1`–`6` | views |
| `j`/`k` | список |
| `/` | filter |
| `:` | command palette |
| `Enter` | primary / confirm |
| `Esc` | back / close overlay |
| `d` | dig |
| `r` | raid preview |
| `?` | help |
| `q` | quit |
| `v` | reveal finding (opt-in, ephemeral) |

Destructive: Preview, затем ввод `yes` (не одно `y`).

---

## Операции

```text
sniff / dig     — analyze
stash / pack    — write (preview)
rinse / raid    — destructive (preview + explicit confirm)
```

Progress — только события core (`ProgressEvent`).  
Raid pipeline: stash → rinse → pack → move. Режим **ATOMIC** (default) или Fail-Fast.

Отмена mid-phase: в текущем core **нет** cooperative cancel — Esc убирает с экрана операции / не обещает rollback.

---

## Безопасность

- Нет plaintext secrets в UI
- Passphrase скрыт, не в логах
- Reveal — opt-in, значение не хранится в state после закрытия

---

## Эквивалент CLI

```bash
racc sniff --root …
racc dig --project …
racc raid --project … --den … --yes
racc reveal --project … --file …
```
