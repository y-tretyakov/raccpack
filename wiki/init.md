---
title: Init — стартовая конфигурация
description: Команда racc init — создание config.toml с комментированным шаблоном и, по желанию, скелета den.
---

# Init - стартовая конфигурация

Команда: `racc init`  
Статус: реализовано.

Эта страница описывает **ровно то поведение**, которое реализует `raccpack` сейчас. Если флаг или путь не указаны здесь — их нет в текущей версии.

> Вернуться к обзору команд: [Использование CLI](/cli-usage).

## Что делает

1. Создаёт конфигурационный файл с комментированным шаблоном (`config_version = 1`, секции `[paths]`, `[scanner]`, `[cleanup]`). По умолчанию — XDG-путь `~/.config/raccpack/config.toml`; недостающие каталоги создаются.
2. С флагом `--ensure-den` дополнительно создаёт скелет den: `.den-version` и `README.txt`.

Чего **не** делает:

- не перезаписывает существующий конфиг без явного `--force`;
- не проверяет существование `scan_root` — путь только записывается в шаблон;
- ничего не мигрирует на диске (авто-миграция конфига v0 → v1 выполняется in-memory при загрузке, см. [Конфигурация](/configuration#config-version-и-миграция)).

## Быстрый старт

```bash
# Создать ~/.config/raccpack/config.toml с шаблоном по умолчанию
racc init

# Сразу указать папку проектов и создать скелет den
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den --ensure-den
```

## Синтаксис

```text
racc init [OPTIONS]
```

Обязательных параметров нет.

## Параметры и флаги

### Флаги команды

| Флаг | По умолчанию | Описание |
|------|--------------|----------|
| `--force` | выкл. | Перезаписать существующий конфигурационный файл |
| `--scan-root <PATH>` | `~/DEV/PROJS` | Prefill `paths.scan_root` в генерируемом шаблоне |
| `--ensure-den` | выкл. | Создать скелет den: `.den-version`, `README.txt` |

### Глобальные флаги

| Флаг | Описание |
|------|----------|
| `-c, --config <PATH>` | Куда писать конфиг (по умолчанию — XDG-путь) |
| `--root <PATH>` | Альтернатива `--scan-root`: prefill `paths.scan_root` |
| `--den <PATH>` | Prefill `paths.den_dir`; также место создания den при `--ensure-den` |
| `--json` | JSON-вывод вместо человекочитаемого |

Приоритеты:

- если указаны и `--scan-root`, и глобальный `--root` — побеждает `--scan-root`;
- без `--scan-root` / `--root` в шаблон подставляется `~/DEV/PROJS`;
- без `--den` в шаблон подставляется `~/.raccpack/den`.

## Что генерируется

Краткий вид шаблона (в файле он дополнен комментариями и ссылками на wiki):

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

Значения `scan_root` и `den_dir` подставляются из флагов; остальные поля — значения по умолчанию. Сгенерированный файл проходит валидацию конфигурации.

При `--ensure-den` в den создаётся:

```text
{den_dir}/
├── .den-version
└── README.txt
```

Подробно о формате файла и секциях: [Конфигурация](/configuration).

## Вывод

### Человекочитаемый

```text
Created config file: /home/user/.config/raccpack/config.toml
Initialized den vault: /home/user/.raccpack/den
```

Вторая строка печатается только при `--ensure-den`.

### JSON (`--json`)

```json
{
  "config_path": "/home/user/.config/raccpack/config.toml",
  "den_dir": "/home/user/.raccpack/den"
}
```

Поле `den_dir` равно `null`, если `--ensure-den` не передан.

## Коды выхода

| Код | Когда |
|-----|--------|
| `0` | Успех |
| `1` | Ошибка: конфиг уже существует (без `--force`), IO-ошибка записи, не удалось создать den |

Код `2` (как у `dig`) для `init` **не** используется.

## Негативный сценарий: конфиг уже существует

Без `--force` команда отказывается перезаписывать файл:

```text
$ racc init
error: config file already exists: /home/user/.config/raccpack/config.toml
hint: Use --force to overwrite the existing configuration file.
$ echo $?
1
```

Перезапись — только явно:

```bash
racc init --force
```

::: warning
`--force` перезаписывает файл целиком: ручные правки в `config.toml` будут потеряны.
:::

## Примеры

```bash
# Базовый: комментированный шаблон в XDG-путь
racc init

# Со своими путями в шаблоне
racc init --scan-root ~/DEV/PROJS --den /mnt/backup/den

# Конфиг в нестандартном месте + создать скелет den
racc init --config ~/cfg/raccpack.toml --ensure-den

# Перезаписать существующий конфиг новым шаблоном
racc init --force

# Машиночитаемый вывод для скриптов
racc init --scan-root ~/DEV/PROJS --json
```

## Связанные страницы

| Страница | Роль |
|----------|------|
| [Конфигурация](/configuration) | Формат `config.toml`, `config_version` и миграция |
| [Основные понятия](/concepts) | Den и раскладка хранилища |
| [Быстрый старт](/quick-start) | Первый прогон за пять минут |
| [Использование CLI](/cli-usage) | Обзор всех команд |

*Документ соответствует реализации; при изменении флагов CLI обновляйте страницу в том же PR.*
