# TUI — интерактивный терминал

**Бинарник:** `racc-tui` · **Статус:** Beta

TUI вызывает тот же core, что CLI. Секреты — masked; reveal — opt-in.

---

## Запуск

```bash
cargo run -p raccpack-tui
# или
racc-tui

export RACCPACK_CONFIG=~/.config/raccpack/config.toml
# только для CI-тестов TUI:
export RACCPACK_PASSPHRASE='…'
```

---

## Клавиши

### Общие

| Key | Action |
|-----|--------|
| `q` | выход |
| `?` | справка |
| `1` | Sniff |
| `2` | Dig |
| `3` | Raid |

### Sniff

| Key | Action |
|-----|--------|
| `r` | rescan |
| `o` | scan root |
| `j`/`k` | выбор |
| `Enter` | dig |

### Dig

| Key | Action |
|-----|--------|
| `f` | min-risk filter |
| `c` | content scan toggle |
| `v` | reveal (confirm) |
| `Esc` | назад |

### Raid

| Key | Action |
|-----|--------|
| `R` | raid scenario |
| `y`/`n` | confirm / cancel |
| `K` | keep sources |
| `S` | skip stash |

Сначала dry-run preview, затем Commit. Atomic по умолчанию; при ошибке — `rolled_back`.

### Reveal

| Key | Action |
|-----|--------|
| `v` | ephemeral reveal (confirm) |
| `Esc` | close + wipe |

Сырое значение не остаётся в state.

---

## Эквивалент CLI

```bash
racc sniff --root …
racc dig --project …
racc raid --project … --den … --yes
racc reveal --project … --file …
```
