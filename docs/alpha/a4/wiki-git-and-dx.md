# Git, init и DX (Alpha)

**Для пользователей CLI.**  
Команды и флаги фазы **A4**: git status в dig, `racc init`, `--verbose`.

Нет в этой странице — нет в текущей версии.

---

## Что появилось в A4

| Возможность | Зачем |
|-------------|--------|
| Git status у находок dig | Понять, tracked / untracked / ignored ли секрет |
| `racc init` | Быстро создать `config.toml` и опционально den |
| `-v` / `-vv` / `-vvv` | Подробные логи **без** сырых секретов и passphrase |
| CI / тесты | Стабильный headless CLI (Alpha) |

---

## Примеры команд CLI

### dig + git status

```bash
# Обычный dig — в human-выводе виден git status (если репозиторий git)
racc dig --project ~/DEV/PROJS/my-api

# JSON: поле git_status у каждого файла
racc dig --project ~/DEV/PROJS/my-api --json | jq '.files[] | {path, risk, git_status}'

# Только Critical + статус
racc dig --project ~/DEV/PROJS/my-api --json \
  | jq '[.files[] | select(.risk=="Critical") | {path, git_status}]'

# Не git-репозиторий — dig всё равно работает, git_status обычно null
racc dig --project /tmp/not-a-git-project --json
```

Возможные значения `git_status` (строки в JSON):

`tracked`, `untracked`, `ignored`, `modified`, `staged`, `deleted`, `unknown`  
или отсутствие поля / `null`, если git недоступен.

```bash
# sniff: проекты с is_git_repo
racc sniff --root ~/DEV/PROJS --json | jq '.report.projects[] | {name, is_git_repo}'
```

---

### racc init

```bash
# Config по умолчанию: ~/.config/raccpack/config.toml
racc init

# С путями
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den

# Создать и каркас den (.den-version, README, secrets/, packs/, …)
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den --ensure-den

# Перезаписать существующий config
racc init --force --scan-root ~/DEV/PROJS --den ~/.raccpack/den

# Свой файл config
racc init --config /tmp/my-racc.toml --scan-root /tmp/proj --den /tmp/den

# JSON: путь к созданному файлу
racc init --scan-root ~/DEV/PROJS --json

# Типичный онбординг
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den --ensure-den
racc sniff --root ~/DEV/PROJS
racc dig --project ~/DEV/PROJS/my-api
```

**Ошибки:**

```bash
# Повторный init без --force → ошибка «уже существует»
racc init
racc init
```

Пример содержимого после init:

```toml
config_version = 1

[paths]
scan_root = "~/DEV/PROJS"
den_dir = "~/.raccpack/den"

[scanner]
max_depth = 6

[cleanup]
enabled_strategies = ["rust", "node", "python"]
```

---

### Verbose / логи

```bash
# Тихий режим (по умолчанию)
racc sniff --root ~/DEV/PROJS

# Info
racc sniff --root ~/DEV/PROJS -v
racc dig --project ~/DEV/PROJS/my-api -v
racc rinse --project ~/DEV/PROJS/my-api --yes -v

# Debug
racc pack --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes -vv
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes -vv

# Trace
racc dig --project ~/DEV/PROJS/my-api -vvv

# RUST_LOG (имеет приоритет, если задан)
RUST_LOG=raccpack_core=debug racc sniff --root ~/DEV/PROJS

# JSON в stdout, логи в stderr
racc dig --project ~/DEV/PROJS/my-api --json -v 2>dig-verbose.log

# Все основные команды с -v
racc init --scan-root ~/DEV/PROJS -v
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes -v
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --json -v
```

**Passphrase не должна попадать в логи:**

```bash
RACCPACK_PASSPHRASE='unique-pass-xyz' \
  racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes -vv 2>&1 \
  | grep -F 'unique-pass-xyz' && echo 'LEAK — баг' || echo 'OK — пароля в логах нет'
```

---

### Полный Alpha-сценарий одной сессией

```bash
export RACCPACK_PASSPHRASE='your-strong-passphrase'
PROJ=~/DEV/PROJS/my-api
DEN=~/.raccpack/den

racc init --scan-root ~/DEV/PROJS --den "$DEN" --ensure-den --force
racc sniff --root ~/DEV/PROJS -v
racc dig --project "$PROJ" --json | jq '.files[] | {path, risk, git_status}'
racc raid --project "$PROJ" --den "$DEN" --yes -v

# Артефакты
find "$DEN/secrets" -name '*.age'
find "$DEN/packs" -name '*.tar.zst'
find "$DEN/manifests" -name '*.json'
```

---

### Локальная проверка «как CI»

```bash
cargo test --workspace
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

---

## Синтаксис: `racc init`

```text
racc init [OPTIONS]
```

| Параметр | Описание |
|----------|----------|
| `--config <PATH>` | Куда писать config (default XDG) |
| `--force` | Перезаписать существующий |
| `--scan-root <PATH>` | Заполнить `paths.scan_root` |
| `--den <PATH>` | Заполнить `paths.den_dir` |
| `--ensure-den` | Создать каркас den |
| `--json` | Вывести путь к config |

## Глобально: verbose

| Флаг | Уровень |
|------|---------|
| *(нет)* | warn |
| `-v` | info |
| `-vv` | debug |
| `-vvv` | trace |

Также: `RUST_LOG=…`.

---

## Связанные страницы

- [Stash](wiki-stash.md) · [Rinse](wiki-rinse.md) · [Raid](wiki-raid.md)

---

*Соответствует A4 (a4.1–a4.4). Меняете флаги — обновляйте wiki в том же PR.*
