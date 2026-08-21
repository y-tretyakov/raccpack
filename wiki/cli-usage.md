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
| `-v, --verbose` | Подробные логи в stderr (повторяемый: `-v` info, `-vv` debug, `-vvv` trace) |

::: info
`--root` и `--den` переопределяют конфигурацию только на текущий запуск и не изменяют её на диске.
:::

::: tip Вербозность и логи
Без `-v` racc почти молчит (уровень warn). Логи всегда пишутся в **stderr** — stdout остаётся чистым для данных и JSON, поэтому `racc dig --json -v` безопасно использовать в скриптах. Переменная `RUST_LOG`, если задана, имеет приоритет над `-v` (например, `RUST_LOG=raccpack_core=debug`). В логи никогда не попадают значения секретов и passphrase — только пути, счётчики и источник passphrase.
:::

Справка и версия доступны в любом месте: `-h, --help` и `-V, --version`.

## Типовой сценарий

Перед первым запуском создайте конфигурацию одной командой — [racc init](/init):

```bash
racc init --scan-root ~/DEV/PROJS
```

Полный цикл работы с проектами:

```bash
racc sniff
racc dig --project <PATH>
racc stash --project <PATH> --yes
racc rinse --project <PATH> --yes
racc pack --project <PATH> --yes
```

- **init** — создать стартовый `config.toml` (один раз, перед первым запуском);
- **sniff** — найти проекты под `scan_root`;
- **dig** — найти секреты (read-only, ничего не пишет);
- **stash** — вынести секреты в зашифрованный age-архив в den;
- **rinse** — удалить мусор сборки (`target`, `node_modules`, …);
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

Ищет чувствительные файлы в `scan_root` (или в одном проекте с `--project`) и возвращает отчёт с уровнями риска. В JSON-выводе каждая находка содержит git-статус файла (`git_status`; `null`, если статус определить нельзя). Команда read-only: ничего не пишет и не удаляет. По умолчанию завершается с кодом `2`, если найдены секреты уровня Critical и выше; порог задаёт `--fail-on`.

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

# Commit и удалить исходные секретные файлы
racc stash --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes --remove-sources
```

Commit с env-passphrase для CI (исходники не удаляются):

::: code-group

```bash [bash]
# bash / zsh
export RACCPACK_PASSPHRASE="$STASH_SECRET"
racc stash --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes
```

```fish [fish]
set -gx RACCPACK_PASSPHRASE $STASH_SECRET
racc stash --project $CI_PROJECT_DIR --den $DEN_PATH --yes
```

```nu [nu]
$env.RACCPACK_PASSPHRASE = $env.STASH_SECRET
racc stash --project $env.CI_PROJECT_DIR --den $env.DEN_PATH --yes
```

```powershell [pwsh]
$env:RACCPACK_PASSPHRASE = $env:STASH_SECRET
racc stash --project $env:CI_PROJECT_DIR --den $env:DEN_PATH --yes
```

:::

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

### `racc raid`

Запускает весь конвейер по проекту одной командой: **stash → rinse → pack → move**. По умолчанию — **atomic**: промежуточные файлы в `den/staging/{id}/`, удаления откладываются в commit, падение commit откатывается (`rolled_back`). После успешного commit пишется манифест в `den/manifests/{yyyy}/{mm}/`. По умолчанию работает в **dry-run** — commit только с `--yes`.

```text
racc raid --project PATH [--den PATH] [--yes] [--dry-run] [--no-stash] [--no-rinse] [--no-pack] [--min-risk LEVEL] [--keep-sources] [--no-content-deny] [--fail-fast]
```

```bash
# Dry-run: показать весь конвейер (ничего не пишется)
racc raid --project ~/DEV/PROJS/app-api --den ~/.raccpack/den

# Полный commit (stash + rinse + pack + manifest)
export RACCPACK_PASSPHRASE="$STASH_SECRET"
racc raid --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes

# Без stash: не трогать секреты, passphrase не нужна
racc raid --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes --no-stash

# Не удалять исходные секреты
racc raid --project ~/DEV/PROJS/app-api --den ~/.raccpack/den --yes --keep-sources
```

Exit: `0` при `success == true`, `1` при ошибке или `success == false` (в т.ч. откат commit).

::: warning
По умолчанию `raid` работает в **dry-run** и ничего не пишет/не удаляет. Commit — только с `--yes`.
:::

Подробно: [Raid](/raid)

### `racc init`

Создаёт стартовый конфигурационный файл с комментированным шаблоном (`config_version = 1`) — по умолчанию в `~/.config/raccpack/config.toml`. С `--ensure-den` дополнительно создаёт скелет den (`.den-version`, `README.txt`). Существующий файл перезаписывается только с `--force`.

```text
racc init [--force] [--scan-root PATH] [--ensure-den]
```

```bash
# Шаблон в ~/.config/raccpack/config.toml
racc init

# Сразу указать папку проектов и создать den
racc init --scan-root ~/DEV/PROJS --den ~/.raccpack/den --ensure-den
```

Подробно: [Init](/init)

## В разработке

Следующие команды планируются в ближайших версиях (см. [Дорожную карту](/roadmap)):

| Команда | Назначение | Статус |
|---------|------------|--------|
| `racc den` | Управление den | Планируется |

## Примечания

- JSON-вывод никогда не содержит raw-значений секретов — только маскированные превью и хеши (подробнее: [Dig](/dig)).
- Код выхода `2` используется только у `dig` (политика `--fail-on`) и означает сработавшую политику, а не сбой CLI; у `pack`, `stash`, `rinse` и `raid` коды выхода — только `0` (успех) и `1` (ошибка).
