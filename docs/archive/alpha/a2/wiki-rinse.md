# Rinse — очистка мусора сборки

**Для пользователей CLI.**  
Команда: `racc rinse`  
Статус: Alpha.

Страница описывает **фактическое** поведение реализации фазы A2. Нет флага в этой таблице — нет в текущей версии.

---

## Что делает rinse

Удаляет **известные каталоги артефактов** внутри проекта:

- `target` (Rust),
- `node_modules`, `.next`, … (Node),
- `__pycache__`, `.venv`, … (Python),
- и другие включённые **стратегии**.

Не делает:

- не ищет и не трогает секреты (для этого `racc stash`);
- не создаёт архивы (`racc pack`);
- не удаляет произвольные файлы пользователя вне списка стратегий.

По умолчанию команда работает в **dry-run**: только показывает, что было бы удалено.

---

## Быстрый старт

```bash
# Посмотреть, что будет удалено (безопасно)
racc rinse --project ~/DEV/PROJS/my-api

# Удалить найденный мусор
racc rinse --project ~/DEV/PROJS/my-api --yes
```

---

## Примеры команд CLI

### Только отчёт (dry-run)

```bash
# Все стратегии из config (по умолчанию rust + node + python)
racc rinse --project ~/DEV/PROJS/my-api

# Явный dry-run
racc rinse --project ~/DEV/PROJS/my-api --dry-run

# Dry-run в JSON (удобно для скриптов и CI)
racc rinse --project ~/DEV/PROJS/my-api --json

# Dry-run + JSON одной командой
racc rinse --project ~/DEV/PROJS/my-api --dry-run --json
```

### Реальное удаление (Commit)

```bash
# Удалить всё, что нашли стратегии из config
racc rinse --project ~/DEV/PROJS/my-api --yes

# Сначала dry-run, потом commit (рекомендуемый порядок)
racc rinse --project ~/DEV/PROJS/my-api
racc rinse --project ~/DEV/PROJS/my-api --yes
```

### Фильтр по стратегиям

```bash
# Только Cargo target/
racc rinse --project ~/DEV/PROJS/my-api --strategy rust --yes

# Только Node-мусор (node_modules, .next, …)
racc rinse --project ~/DEV/PROJS/my-api --strategy node --yes

# Rust + Node за один проход
racc rinse --project ~/DEV/PROJS/my-api --strategy rust --strategy node --yes

# Python-кэши и venv
racc rinse --project ~/DEV/PROJS/my-api --strategy python --yes

# JVM (build, .gradle)
racc rinse --project ~/DEV/PROJS/my-api --strategy jvm --yes

# Go vendor (в default config обычно выключен — только явно)
racc rinse --project ~/DEV/PROJS/my-api --strategy go --yes

# Generic: .cache, tmp, temp
racc rinse --project ~/DEV/PROJS/my-api --strategy generic --yes

# Несколько «осторожных» стратегий сразу
racc rinse --project ~/DEV/PROJS/my-api \
  --strategy rust \
  --strategy node \
  --strategy python \
  --yes
```

### Config и пути

```bash
# Свой файл конфигурации
racc rinse --project ~/DEV/PROJS/my-api --config ~/.config/raccpack/config.toml

# Project относительно текущей директории
cd ~/DEV/PROJS/my-api
racc rinse --project .

# Абсолютный путь
racc rinse --project /home/user/DEV/PROJS/my-api --yes
```

### Скрипты и CI

```bash
# Проверить, есть ли что чистить (dry-run JSON)
racc rinse --project "$CI_PROJECT_DIR" --json

# Пример: удалить только node_modules на CI-агенте после build
racc rinse --project "$CI_PROJECT_DIR" --strategy node --yes --json

# Подсчёт «сколько бы освободили» без удаления (jq)
racc rinse --project ~/DEV/PROJS/my-api --json | jq '.bytes_freed'

# Список путей-кандидатов
racc rinse --project ~/DEV/PROJS/my-api --json | jq -r '.removed[].path'
```

### Типичные ошибки (что не сработает)

```bash
# Нет --project → ошибка
racc rinse --yes

# Неизвестная стратегия → ошибка, exit 1
racc rinse --project ~/DEV/PROJS/my-api --strategy foo

# --dry-run вместе с --yes → удаление НЕ выполняется
racc rinse --project ~/DEV/PROJS/my-api --dry-run --yes
```

### Связка с другими командами

```bash
# 1) Найти секреты
racc dig --project ~/DEV/PROJS/my-api

# 2) Убрать секреты в age (если нужно сохранить)
export RACCPACK_PASSPHRASE='…'
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --remove-sources

# 3) Почистить мусор
racc rinse --project ~/DEV/PROJS/my-api --yes

# 4) Упаковать проект в den
racc pack --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes
```

---

## Синтаксис

```text
racc rinse --project <PATH> [OPTIONS]
```

### Параметры

| Параметр | По умолчанию | Описание |
|----------|--------------|----------|
| `--project <PATH>` | *(обязательно)* | Корень проекта |
| `--yes` | выкл. | **Commit** — реально удалить каталоги |
| `--dry-run` | вкл. (если нет `--yes`) | Только отчёт |
| `--strategy <ID>` | из config | Повторяемый фильтр стратегий (см. ниже) |
| `--json` | выкл. | Машиночитаемый `RinseResult` |

Если указаны и `--dry-run`, и `--yes` — удаление **не** выполняется.

### Стратегии (`--strategy`)

| ID | Что обычно удаляется |
|----|----------------------|
| `rust` | `target` |
| `node` | `node_modules`, `.next`, `dist`, `.nuxt`, `coverage` |
| `python` | `__pycache__`, `.venv`, `venv`, `.tox`, `.mypy_cache`, `.pytest_cache`, `.ruff_cache` |
| `jvm` | `build`, `.gradle` |
| `go` | `vendor` |
| `generic` | `.cache`, `tmp`, `temp` |

Без `--strategy` используются стратегии из конфигурации.

### Конфиг (`config.toml`)

```toml
[cleanup]
enabled_strategies = ["rust", "node", "python"]
```

Это default, если CLI не передал `--strategy`.  
Неизвестный id в конфиге или в CLI → ошибка.

---

## Режим Dry-run vs Commit

| Режим | Флаг | ФС |
|-------|------|-----|
| Dry-run | по умолчанию или `--dry-run` | Каталоги **не** удаляются |
| Commit | `--yes` | Найденные trash-dirs удаляются через `remove_dir_all` |

Перед `--yes` всегда имеет смысл прогнать dry-run и прочитать список путей.

---

## Пример вывода

**Dry-run:**

```text
Rinse (dry-run)
  Project: /home/user/DEV/PROJS/my-api
  Would remove 2 directories (140.2 MiB)
    node_modules  [node]  120.0 MiB
    target        [rust]   20.2 MiB
  (nothing deleted)
```

**Commit:**

```text
Rinse complete
  Removed 2 directories, freed 140.2 MiB
```

---

## JSON (`--json`)

Поля результата:

| Поле | Смысл |
|------|--------|
| `removed` | Массив объектов: `path`, `strategy`, `pattern_name`, `size_bytes` |
| `bytes_freed` | Сумма размеров (оценка до удаления / фактическая логика реализации) |
| `dry_run` | `true` или `false` |

В dry-run `removed` — это **кандидаты** (то, что было бы удалено), не «уже удалённые».

---

## Коды выхода

| Код | Когда |
|-----|--------|
| 0 | Успех |
| 1 | Ошибка (нет project, неизвестная strategy, IO при удалении) |

---

## Безопасность

- Удаляются только директории, совпавшие со **стратегиями**, внутри `--project`.
- Симлинки на каталоги **снаружи** проекта не должны приводить к удалению внешнего дерева (реализация пропускает symlink-dirs).
- Обход без follow symlinks (`follow_links(false)`).
- Осторожные имена вроде `dist` / `build` / `vendor` входят в стратегии; default config **не** включает `go` и `generic` — включайте явно, если нужно.
- Rinse **не** замена антивирусу и не «удалить всё кроме src».

---

## Частые вопросы

**Почему не удалилось?**  
Нет `--yes`, или стратегия не включена, или имя каталога не в таблице patterns.

**Можно ли вернуть `node_modules`?**  
Только переустановкой зависимостей (`npm install` и т.д.). Rinse не делает бэкап.

**Секреты в `.env` удалятся?**  
Нет — `.env` не является trash-dir стратегией. Для секретов: `racc stash`.

**Порядок с pack/stash?**  
Типичный полный цикл позже: `raid` = stash → rinse → pack. Вручную: сначала stash (если нужно сохранить секреты), потом rinse, потом pack.

---

## Связанные команды

| Команда | Роль |
|---------|------|
| `racc dig` | Найти секреты |
| `racc stash` | Убрать секреты в `.age` |
| `racc pack` | Упаковать проект без секретов и без (уже почищенного) мусора |
| `racc raid` | Оркестрация (фаза A3) |

---

*Соответствует реализации A2 (спеки a2.1–a2.3). Меняете флаги CLI — обновите эту страницу в том же PR.*
