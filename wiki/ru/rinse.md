---
title: Rinse — очистка мусора сборки
description: Команда racc rinse — удалить известные каталоги артефактов сборки (target, node_modules, кэши) по стратегиям, dry-run по умолчанию.
---

# Rinse - очистка мусора сборки

Команда: `racc rinse`  
Статус: реализовано (Alpha).

Эта страница описывает **ровно то поведение**, которое реализует `raccpack` сейчас. Если флаг или путь не указаны здесь — их нет в текущей версии.

Вернуться к обзору команд: [Использование CLI](/ru/cli-usage).

## Что делает rinse (и чего не делает)

`racc rinse` удаляет **известные каталоги артефактов сборки** внутри проекта по наборам правил — **стратегиям**:

- `target` (Rust),
- `node_modules`, `.next`, `dist`, … (Node),
- `__pycache__`, `.venv`, … (Python),
- и другие включённые стратегии (см. [Стратегии](#стратегии-strategy)).

По умолчанию команда работает в **dry-run**: только показывает, что было бы удалено. Реальное удаление — только с `--yes`.

Чего rinse **не** делает:

- не ищет и не трогает секреты — для этого `racc stash`;
- не создаёт архивы — для этого `racc pack`;
- не удаляет произвольные файлы пользователя вне таблицы стратегий;
- не использует код выхода `2` (это особенность только `dig`);
- не требует passphrase и не имеет флагов `--remove-sources` / `--only` (это параметры `stash`).

## Быстрый старт

```bash
# 1) Посмотреть, что будет удалено (безопасно, ничего не удаляется)
racc rinse --project ~/DEV/PROJS/my-api

# 2) Удалить найденный мусор
racc rinse --project ~/DEV/PROJS/my-api --yes
```

::: warning
По умолчанию `rinse` работает в **dry-run**: ничего не удаляется. Реальное удаление каталогов — только с `--yes`.
:::

::: info
Перед `--yes` всегда имеет смысл прогнать dry-run и прочитать список путей: `dist`, `build` и `vendor` — «осторожные» имена, в default-наборе стратегий их нет (см. [Стратегии](#стратегии-strategy)).
:::

## Синтаксис

```text
racc rinse --project <PATH> [OPTIONS]
```

`--project <PATH>` — **обязательный** параметр: каталог проекта (или поддерево), в котором ищем мусор сборки.

## Параметры и флаги

### Проект (обязательно)

| Параметр | Описание |
|----------|----------|
| `--project <PATH>` | Каталог проекта (или поддерево), в котором ищем мусор. Может быть относительным — например `--project .` из каталога проекта |

### Режим записи

| Параметр | Поведение |
|----------|-----------|
| *(по умолчанию)* | **Dry-run**: только отчёт, каталоги не удаляются |
| `--dry-run` | Явный dry-run |
| `--yes` | **Commit**: реально удалить найденные каталоги |

**Приоритет:** если указаны и `--dry-run`, и `--yes`, побеждает dry-run — ничего не удаляется.

### Стратегии

| Параметр | По умолчанию | Описание |
|----------|--------------|----------|
| `--strategy <ID>` | из `config.cleanup.enabled_strategies` | Повторяемый фильтр стратегий. Без флага используются стратегии из конфигурации |

### Вывод

| Параметр | Описание |
|----------|----------|
| `--json` | Печать `RinseResult` в JSON (см. [JSON](#json-json)) |

### Глобальные флаги

| Флаг | Описание |
|------|----------|
| `-c, --config <PATH>` | Файл конфигурации (переопределяет `RACCPACK_CONFIG`) |
| `--root <PATH>` | Переопределить `scan_root` на этот запуск; относительный `--project` резолвится относительно него |
| `--den <PATH>` | Переопределить `den_dir` на этот запуск. Для `rinse` **не используется** — rinse не пишет в den |
| `--json` | Машиночитаемый вывод JSON |

::: info
`--den` принимается (это глобальный флаг), но на работу `rinse` не влияет: очистка мусора не затрагивает den.
:::

## Стратегии (`--strategy`)

| ID | Что обычно удаляется |
|----|----------------------|
| `rust` | `target` |
| `node` | `node_modules`, `.next`, `dist`, `.nuxt`, `coverage` |
| `python` | `__pycache__`, `.venv`, `venv`, `.tox`, `.mypy_cache`, `.pytest_cache`, `*.egg-info`, `.ruff_cache` |
| `jvm` | `build`, `.gradle`, `.m2` |
| `go` | `vendor` |
| `generic` | `.cache`, `tmp`, `temp` |

`dist`, `build`, `vendor`, `tmp`/`temp` — «осторожные» имена: иногда это не мусор, а настоящие исходники или пользовательские данные. Поэтому по умолчанию включены только `rust`, `node` и `python`; `jvm`, `go` и `generic` подключаются явно — флагом `--strategy` или в конфигурации.

### Конфиг (`config.toml`)

```toml
[cleanup]
enabled_strategies = ["rust", "node", "python"]
```

Это default, если CLI не передал `--strategy`. Неизвестный id (в конфиге или в CLI) — ошибка, exit `1`.

## Режим Dry-run vs Commit

| Режим | Флаг | Файловая система |
|-------|------|------------------|
| Dry-run | по умолчанию или `--dry-run` | Каталоги **не** удаляются; отчёт — полный список найденного |
| Commit | `--yes` | Найденные trash-каталоги удаляются (см. [Безопасность](#безопасность)) |

## Вывод

### Человекочитаемый (human)

Dry-run:

```text
Rinse (dry-run)
  Project: /home/user/DEV/PROJS/my-api
  Would remove 2 directories (140.2 MiB)
    node_modules  [node]  120.0 MiB
    target        [rust]   20.2 MiB
  (nothing deleted)
```

Commit:

```text
Rinse complete
  Removed 2 directories, freed 140.2 MiB
```

### JSON (`--json`)

| Поле | Смысл |
|------|--------|
| `removed` | Массив объектов `{ path, strategy, pattern_name, size_bytes }` |
| `bytes_freed` | Сумма размеров (в dry-run — оценка; в commit — реально освобождено) |
| `dry_run` | `true` / `false` |

В dry-run `removed` — это **кандидаты** (то, что было бы удалено), а не «уже удалённые».

Пример:

```json
{
  "removed": [
    {
      "path": "/home/user/DEV/PROJS/my-api/node_modules",
      "strategy": "node",
      "pattern_name": "node_modules",
      "size_bytes": 125829120
    },
    {
      "path": "/home/user/DEV/PROJS/my-api/target",
      "strategy": "rust",
      "pattern_name": "target",
      "size_bytes": 21181235
    }
  ],
  "bytes_freed": 147010355,
  "dry_run": true
}
```

## Коды выхода

| Код | Когда |
|-----|--------|
| 0 | Успех (в т.ч. dry-run) |
| 1 | Ошибка: нет `--project` (usage), неизвестная стратегия, IO при удалении |

Код `2` (как у dig для Critical) **не** используется для rinse.

## Примеры

```bash
# Локально: dry-run — показать, что будет удалено (ничего не удаляется)
racc rinse --project ~/DEV/PROJS/my-api

# Явный dry-run
racc rinse --project ~/DEV/PROJS/my-api --dry-run

# Commit: реально удалить найденный мусор
racc rinse --project ~/DEV/PROJS/my-api --yes

# Только Cargo target/
racc rinse --project ~/DEV/PROJS/my-api --strategy rust --yes

# Только Node-мусор (node_modules, .next, …)
racc rinse --project ~/DEV/PROJS/my-api --strategy node --yes

# Rust + Node за один проход (флаг повторяется)
racc rinse --project ~/DEV/PROJS/my-api --strategy rust --strategy node --yes

# JVM build-каталоги (в default-наборе выключены — только явно)
racc rinse --project ~/DEV/PROJS/my-api --strategy jvm --yes

# Go vendor/ (по умолчанию выключен — только явно)
racc rinse --project ~/DEV/PROJS/my-api --strategy go --yes

# Generic: .cache, tmp, temp (по умолчанию выключены — только явно)
racc rinse --project ~/DEV/PROJS/my-api --strategy generic --yes

# --dry-run всегда побеждает --yes: ничего не удаляется
racc rinse --project ~/DEV/PROJS/my-api --yes --dry-run

# Project относительно текущей директории
cd ~/DEV/PROJS/my-api
racc rinse --project . --yes
```

### Примеры для CI

```bash
# Проверить, есть ли что чистить (dry-run JSON)
racc rinse --project "$CI_PROJECT_DIR" --json

# Удалить только node_modules на CI-агенте после build
racc rinse --project "$CI_PROJECT_DIR" --strategy node --yes --json

# Подсчёт «сколько бы освободили» без удаления (jq)
racc rinse --project ~/DEV/PROJS/my-api --json | jq '.bytes_freed'

# Список путей-кандидатов
racc rinse --project ~/DEV/PROJS/my-api --json | jq -r '.removed[].path'
```

## Частые ошибки

| Ситуация | Что сделать |
|----------|-------------|
| `error: invalid configuration: unknown cleanup strategy \`foo\`` | Проверьте id стратегии: `rust`, `node`, `python`, `jvm`, `go`, `generic` |
| Ничего не удалилось | Нужен `--yes` (Commit); стратегия не включена (default — только `rust`, `node`, `python`); или имени каталога нет в таблице стратегий |
| `--project` обязателен | Укажите `--project <PATH>`; ошибка парсинга, exit `1` |
| Можно ли вернуть `node_modules`? | Только переустановкой зависимостей (`npm install` и т.д.). Rinse не делает бэкап |
| Секреты в `.env` удалятся? | Нет — `.env` не является trash-каталогом ни одной стратегии. Для секретов: `racc stash` |

## Безопасность

- Удаляются только каталоги, совпавшие со **стратегиями**, внутри `--project` (path containment).
- Обход **без** follow symlinks (`follow_links(false)`); симлинки на каталоги не удаляются и не обходятся — внешние деревья не задеваются.
- По умолчанию dry-run — сначала смотрите отчёт.
- «Осторожные» имена (`dist`, `build`, `vendor`) входят в стратегии, но не в default-набор: `jvm`, `go` и `generic` включайте явно.
- Rinse — не «удалить всё кроме `src`» и не замена антивирусу: только таблица стратегий.

## Связанные команды

| Команда | Роль |
|---------|------|
| `racc dig` | Найти секреты (read-only) |
| `racc stash` | Убрать секреты в `.age` |
| `racc pack` | Упаковать проект **без** секретов в `packs/` |
| `racc raid` | Полный цикл одной командой: stash → rinse → pack → move |

---

*Документ соответствует реализации; при изменении флагов CLI обновляйте страницу в том же PR.*