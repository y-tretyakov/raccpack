---
title: Конфигурация
description: "Настройка raccpack через TOML-файл и переменные окружения: пути, сканер, стратегии очистки, ошибки конфигурации."
---

# Конфигурация

raccpack настраивается через TOML-файл и несколько переменных окружения.

## Где искать конфигурацию

Порядок разрешения:

1. Переменная окружения **`RACCPACK_CONFIG`** — явный путь к файлу. Если задана, файл **обязан** существовать.
2. Стандартный путь XDG: `$XDG_CONFIG_HOME/raccpack/config.toml`, а если `XDG_CONFIG_HOME` не задан — `~/.config/raccpack/config.toml`.
3. Если файла нигде нет — используется конфигурация по умолчанию (пути можно задать флагами `--root` и `--den`).

Самый простой способ создать файл — команда [`racc init`](/ru/init): она записывает комментированный шаблон в стандартный путь (или в путь из `--config`):

```bash
racc init --scan-root ~/DEV/PROJS
```

Пример:

::: code-group

```bash [bash]
# bash / zsh — явный путь через переменную окружения
export RACCPACK_CONFIG=/path/to/raccpack.toml
racc sniff
```

```fish [fish]
set -gx RACCPACK_CONFIG /path/to/raccpack.toml
racc sniff
```

```nu [nu]
$env.RACCPACK_CONFIG = "/path/to/raccpack.toml"
racc sniff
```

```powershell [pwsh]
$env:RACCPACK_CONFIG = "/path/to/raccpack.toml"
racc sniff
```

:::

## Формат файла

```toml
# Конфигурация raccpack
config_version = 1

[paths]
# Каталог, содержащий ваши проекты (вход)
scan_root = "~/DEV/PROJS"
# Каталог-хранилище den (выход)
den_dir = "~/.raccpack/den"

[scanner]
# Максимальная глубина обхода дерева
max_depth = 6

[cleanup]
# Стратегии rinse по умолчанию (если CLI не передал --strategy)
enabled_strategies = ["rust", "node", "python"]
# Opt-in при необходимости: "jvm", "go", "generic"
```

### Секция `[paths]`

| Ключ | Обязательный | Описание |
|------|--------------|----------|
| `scan_root` | Да (для сканирования) | Папка с проектами. Должна существовать |
| `den_dir` | Нет | Папка-хранилище. По умолчанию `~/.raccpack/den`. Создаётся при первой записи |

Пути могут содержать `~` и относительные компоненты — raccpack приводит их к абсолютным относительно домашнего каталога.

### Секция `[scanner]`

| Ключ | По умолчанию | Описание |
|------|--------------|----------|
| `max_depth` | `6` | Максимальная глубина обхода. Должна быть ≥ 1 |

### Секция `[detect]`

| Ключ | По умолчанию | Описание |
|------|--------------|----------|
| `mode` | `"priority_table"` | Конвейер определения стека для `racc sniff`. Значения: `"priority_table"`, `"composite_dag"` |

::: warning
`"composite_dag"` **пока недоступен** (Detect v2, `0.4.x`). Неизвестное значение завершается ошибкой с перечислением валидных.
:::

### Секция `[cleanup]`

| Ключ | По умолчанию | Описание |
|------|--------------|----------|
| `enabled_strategies` | `["rust", "node", "python"]` | Id стратегий для `racc rinse`, когда не передан флаг `--strategy` |

`racc rinse` удаляет каталоги артефактов сборки по наборам правил — **стратегиям**. Каждая стратегия — это набор имён каталогов, считающихся мусором. Зарегистрированные стратегии:

| Id | В defaults | Типовые каталоги |
|----|------------|------------------|
| `rust` | да | `target` |
| `node` | да | `node_modules`, `.next`, `dist`, `.nuxt`, `coverage` |
| `python` | да | `__pycache__`, `.venv`, `venv`, `.tox`, `.mypy_cache`, `.pytest_cache`, `*.egg-info`, `.ruff_cache` |
| `jvm` | **opt-in** | `build`, `.gradle`, `.m2` |
| `go` | **opt-in** | `vendor` |
| `generic` | **opt-in** | `.cache`, `tmp`, `temp` |

По умолчанию включены только `rust`, `node` и `python`. Причина — «осторожные» имена: `dist` (node) и `build` (jvm) иногда содержат настоящие исходники, `vendor` (go) может быть намеренной копией зависимостей, а `tmp` / `temp` (generic) — пользовательские данные. Поэтому `jvm`, `go` и `generic` подключаются **явно** — через `enabled_strategies` в конфигурации или флаг `--strategy` (см. [Rinse](/ru/rinse)).

Флаг `--strategy` перекрывает конфигурацию на текущий запуск:

```bash
# Вместо config.cleanup.enabled_strategies — только node и rust
racc rinse --project ~/DEV/PROJS/my-api --strategy node --strategy rust --yes
```

Неизвестный id в TOML — ошибка при загрузке конфигурации (см. [Ошибки конфигурации](#ошибки-конфигурации)); неизвестный `--strategy` в CLI — ошибка, код выхода `1`.

::: info
Имена cleanup-стратегий и списки пропускаемых каталогов при обходе/упаковке согласованы по смыслу, но пока живут раздельно. Единый источник правил — в планах (follow-up).
:::

### Будущие секции

В конфигурацию будут добавлены секции для групп секретов и производительности:

- `[sensitive]` — какие группы секретов включены;
- `[advanced]` — параллельность (`parallel_jobs`), уровень zstd-сжатия.

::: info
Неизвестные ключи в TOML не ломают загрузку — будущие секции не сломают существующие конфигурации.
:::

## config_version и миграция

Актуальная схема конфигурации имеет версию **1** (`config_version = 1`). Именно эта строка записывается в файл командой [`racc init`](/ru/init).

При загрузке конфигурации raccpack проверяет поле `config_version`:

| Значение в файле | Поведение |
|------------------|-----------|
| Поле отсутствует или `0` | Автоматическая миграция до v1 **in-memory**: конфиг загружается как v1 без изменений на диске |
| `1` | Загружается как есть |
| Больше текущей (например, `2`) | Ошибка `incompatible config version: found N, current version is 1`, код выхода `1` |

Пояснения:

- миграция не переписывает файл — правки появляются только в памяти на время запуска;
- конфиг из «будущей» версии означает, что файл создан более новой версией raccpack; подсказка CLI предлагает обновить raccpack;
- старые конфиги (без `config_version`) продолжают работать без ручных правок.

## Переменные окружения

| Переменная | Назначение |
|------------|------------|
| `RACCPACK_CONFIG` | Явный путь к TOML-файлу. Если задана — файл обязан существовать |
| `RACCPACK_PASSPHRASE` | Passphrase для `racc stash` (шифрование age-архива). **Не храните её в TOML** — задаётся через окружение, интерактивный ввод или stdin |

Passphrase не читается из конфигурационного файла и не попадает в вывод/отчёты.

## Переопределение через CLI

Глобальные флаги переопределяют конфигурацию только на текущий запуск:

```bash
# Временное сканирование другой папки
racc sniff --root /tmp/other --max-depth 4

# Использовать временный den для этого запуска
racc sniff --root ~/DEV/PROJS --den /tmp/den
```

`--root` и `--den` не изменяют файл на диске — они действуют один запуск. Для `racc rinse` флаг `--den` принимается, но не влияет на работу: очистка мусора не пишет в den (см. [Rinse](/ru/rinse)).

## Минимальная конфигурация для первого запуска

Самый быстрый путь — [`racc init`](/ru/init):

```bash
racc init --scan-root ~/DEV/PROJS
racc sniff
```

Вариант вручную:

```bash
mkdir -p ~/.config/raccpack
cat > ~/.config/raccpack/config.toml <<'EOF'
[paths]
scan_root = "~/DEV/PROJS"
EOF

racc sniff
```

::: info
Без `scan_root` в конфигурации и без флага `--root` `racc` завершится с ошибкой (`missing scan_root: …`).
:::

## Ошибки конфигурации

Типичные ошибки и подсказки, которые выводит `racc`:

| Ошибка | Причина | Подсказка |
|--------|---------|-----------|
| `missing scan_root: set paths.scan_root in config or pass --root` | `scan_root` не задан | Укажите `scan_root` в TOML или флаг `--root` |
| `scan_root does not exist: <path>` / `path not found: <path>` | Путь не существует | Проверьте, что папка существует |
| `not a directory: <path>` | Указан файл, а не папка | Укажите каталог |
| `invalid max_depth: <value> (must be >= 1)` | `max_depth < 1` | Поставьте значение ≥ 1 |
| `unknown cleanup strategy `foo`` | Неизвестный id в `cleanup.enabled_strategies` | Используйте известные id: `rust`, `node`, `python`, `jvm`, `go`, `generic` |
| `invalid configuration: unknown cleanup strategy `foo`` | Неизвестный `--strategy foo` в CLI | Используйте известные id; ошибка, код выхода `1` |
| `incompatible config version: found N, current version is 1` | Конфиг создан более новой версией raccpack (`config_version` > 1) | Обновите raccpack (см. [config_version и миграция](#config-version-и-миграция)) |

Ошибка в конфигурации выводится с подсказкой `hint: …`, код выхода — `1`.

## Дальнейшее чтение

- [Init](/ru/init) — команда создания стартового конфига и скелета den.
- [Rinse](/ru/rinse) — стратегии очистки и флаг `--strategy`.
- [Что поддерживается](/ru/supported) — полный каталог возможностей (маркеры, секреты, стратегии).
- [Использование CLI](/ru/cli-usage) — все флаги команд.
- [Основные понятия](/ru/concepts) — что такое den и как устроен вывод.
