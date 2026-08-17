---
title: Использование CLI
description: Обзор команд racc — глобальные флаги и типовой сценарий; подробно по командам — на отдельных страницах.
---

# Использование CLI

`racc` — командная строка raccpack. Подходит для повседневной работы, скриптов и CI. На этой странице — краткий обзор: глобальные флаги, типовой сценарий и список команд; подробные страницы по каждой команде — внизу раздела.

## Глобальные флаги

Флаги можно указывать до или после подкоманды.

| Флаг | Описание |
|------|----------|
| `-c, --config <PATH>` | Файл конфигурации (переопределяет `RACCPACK_CONFIG`) |
| `--root <PATH>` | Переопределить `scan_root` на текущий запуск (доступен и как флаг отдельных команд) |
| `--den <PATH>` | Переопределить `den_dir` на текущий запуск (для `sniff` необязателен) |
| `--json` | Машиночитаемый вывод JSON вместо человекочитаемой таблицы |

::: info
`--root` и `--den` переопределяют конфигурацию только на текущий запуск и не изменяют её на диске.
:::

Справка и версия доступны в любом месте: `-h, --help` и `-V, --version`.

## Типовой сценарий

Полный цикл работы с проектами:

```bash
racc sniff
racc dig --project <PATH>
racc stash --project <PATH> --yes
racc pack --project <PATH> --yes
```

- **sniff** — найти проекты под `scan_root`;
- **dig** — найти секреты (read-only, ничего не пишет);
- **stash** — вынести секреты в зашифрованный age-архив в den;
- **pack** — упаковать проект БЕЗ секретов в `packs/`.

## Команды

### `racc sniff`

Сканирует `scan_root`, находит проекты и печатает таблицу: имя, стек, размер, признак git-репозитория, путь. Результат кэшируется; `--force-refresh` игнорирует кэш, `--max-depth N` ограничивает глубину обхода.

```text
racc sniff [--force-refresh] [--max-depth N]
```

```bash
# Полный обзор папки с проектами
racc sniff

# Принудительное пересканирование без кэша
racc sniff --force-refresh

# Не глубже 3 уровней
racc sniff --max-depth 3

# Машиночитаемый результат для скриптов
racc sniff --json
```

Подробно: [Sniff](/sniff)

### `racc dig`

Ищет чувствительные файлы в `scan_root` (или в одном проекте с `--project`) и возвращает отчёт с уровнями риска. Команда read-only: ничего не пишет и не удаляет. По умолчанию завершается с кодом `2`, если найдены секреты уровня Critical и выше; порог задаёт `--fail-on`.

```text
racc dig [--project PATH] [--no-content] [--repeated] [--fail-on ignore|critical|high] [--max-depth N]
```

```bash
# Проверить все проекты
racc dig

# Проверить один проект
racc dig --project ~/DEV/PROJS/app-api

# Только по именам файлов (быстрее, без чтения содержимого)
racc dig --no-content

# Проваливать запуск уже при High-находках
racc dig --fail-on high
```

Подробно: [Dig](/dig)

### `racc pack`

Упаковывает каталог проекта в архив `tar.zst` и кладёт его в den по раскладке `packs/{yyyy}/{mm}/`, исключая секреты. По умолчанию работает в **dry-run** и ничего не пишет — commit только с `--yes`.

```text
racc pack --project PATH [--den PATH] [--yes] [--dry-run] [--no-content-deny] [--zstd-level N] [--output-name NAME]
```

```bash
# Dry-run: показать, что будет упаковано (ничего не пишется)
racc pack --project ~/DEV/PROJS/app-api

# Commit: создать архив в den
racc pack --project ~/DEV/PROJS/app-api --yes

# Своё имя артефакта вместо slug__timestamp
racc pack --project ~/DEV/PROJS/app-api --yes --output-name snapshot
```

::: warning
По умолчанию `pack` работает в **dry-run** и ничего не пишет. Запись в den — только с флагом `--yes`.
:::

Подробно: [Pack](/pack)

### `racc stash`

Собирает чувствительные файлы проекта в один зашифрованный **age**-архив и кладёт его в den по раскладке `secrets/{yyyy}/{mm}/`, при желании удаляя исходники. По умолчанию работает в **dry-run** — commit только с `--yes`. Passphrase берётся из `RACCPACK_PASSPHRASE`, интерактивного ввода или stdin. Сырые секреты не печатаются и не попадают в вывод.

```text
racc stash --project PATH [--den PATH] [--yes] [--dry-run] [--remove-sources] [--min-risk LEVEL] [--only PATH] [--batch-id ID]
```

```bash
# Dry-run: показать, что попадёт в архив (ничего не пишется)
racc stash --project ~/DEV/PROJS/app-api

# Commit с env-passphrase для CI (исходники не удаляются)
export RACCPACK_PASSPHRASE="$STASH_SECRET"
racc stash --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes

# Commit и удалить исходные секретные файлы
racc stash --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes --remove-sources
```

::: danger
Пример с `--remove-sources` удаляет исходные секреты после успешного stash. Подробности и ограничения: [Stash](/stash).
:::

::: warning
По умолчанию `stash` работает в **dry-run** и ничего не пишет. Commit — только с `--yes`.
:::

Подробно: [Stash](/stash)

### `racc rinse`

Удаляет из проекта известные каталоги артефактов сборки по **стратегиям** (`target`, `node_modules`, `__pycache__`, …). По умолчанию работает в **dry-run** и ничего не удаляет — commit только с `--yes`. Стратегии без флага берутся из `config.cleanup.enabled_strategies` (по умолчанию `rust`, `node`, `python`).

```text
racc rinse --project PATH [--strategy ID ...] [--yes] [--dry-run]
```

```bash
# Dry-run: показать, что было бы удалено (ничего не удаляется)
racc rinse --project ~/DEV/PROJS/app-api

# Commit: реально удалить найденный мусор
racc rinse --project ~/DEV/PROJS/app-api --yes

# Только Node-мусор (node_modules, .next, …)
racc rinse --project ~/DEV/PROJS/app-api --strategy node --yes
```

::: warning
По умолчанию `rinse` работает в **dry-run** и ничего не удаляет. Удаление каталогов — только с `--yes`.
:::

Подробно: [Rinse](/rinse)

## В разработке

Следующие команды планируются в ближайших версиях (см. [Дорожную карту](/roadmap)):

| Команда | Назначение | Статус |
|---------|------------|--------|
| `racc raid` | Полный цикл одной командой | Планируется |
| `racc den` | Управление den | Планируется |
| `racc init` | Стартовая конфигурация | Планируется |

## Примечания

- JSON-вывод никогда не содержит raw-значений секретов — только маскированные превью и хеши (подробнее: [Dig](/dig)).
- Код выхода `2` используется только у `dig` (политика `--fail-on`) и означает сработавшую политику, а не сбой CLI; у `pack`, `stash` и `rinse` коды выхода — только `0` (успех) и `1` (ошибка).
