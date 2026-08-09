# Использование CLI

`racc` — командная строка raccpack. Подходит для повседневной работы, скриптов и CI.

## Глобальные флаги

Флаги можно указывать до или после подкоманды.

| Флаг | Описание |
|------|----------|
| `--config PATH` | Путь к файлу конфигурации (переопределяет `RACCPACK_CONFIG`) |
| `--root PATH` | Переопределить `scan_root` на этот запуск |
| `--den PATH` | Переопределить `den_dir` на этот запуск |
| `--json` | Машиночитаемый вывод в JSON вместо человекочитаемой таблицы |

> **Note** `--root` и `--den` не изменяют конфигурацию на диске — только на текущий запуск.

## Команды

### `racc sniff` — найти проекты

Сканирует `scan_root` и выводит таблицу проектов: имя, стек, размер, признак git-репозитория, путь.

```
racc sniff [--force-refresh] [--max-depth N]
```

| Флаг | Описание |
|------|----------|
| `--force-refresh` | Игнорировать кэш и пересканировать с нуля |
| `--max-depth N` | Максимальная глубина обхода (переопределяет `scanner.max_depth`) |

Примеры:

```bash
# Полный обзор папки с проектами
racc sniff

# Принудительное пересканирование
racc sniff --force-refresh

# Не глубже 3 уровней
racc sniff --max-depth 3

# Машиночитаемый результат для скриптов
racc sniff --json
```

Пример JSON-вывода (`--json`):

```json
{
  "report": {
    "root": "/home/user/DEV/PROJS",
    "projects": [
      {
        "path": "/home/user/DEV/PROJS/app-api",
        "name": "app-api",
        "stack": { "language": "Rust", "frameworks": ["Axum"], "markers": ["Cargo.toml"] },
        "size_bytes": 432523264,
        "is_git_repo": true
      }
    ],
    "total_size_bytes": 432523264,
    "schema_version": 1
  },
  "from_cache": false,
  "duration_ms": 210
}
```

### `racc dig` — найти секреты

Сканирует `scan_root` (или один проект) на чувствительные файлы и возвращает отчёт с уровнями риска.

```
racc dig [--project PATH] [--no-content] [--repeated]
         [--fail-on ignore|critical|high] [--max-depth N]
```

| Флаг | Описание |
|------|----------|
| `--project PATH` | Ограничить поиск одним проектом |
| `--no-content` | Только по именам файлов, без чтения содержимого |
| `--repeated` | Искать повторяющиеся значения секретов между файлами |
| `--fail-on POLICY` | Политика выхода: `ignore`, `critical` (по умолчанию), `high` |
| `--max-depth N` | Максимальная глубина обхода |

Примеры:

```bash
# Проверить все проекты
racc dig

# Проверить один проект
racc dig --project ~/DEV/PROJS/app-api

# Только по именам файлов (быстрее, без чтения содержимого)
racc dig --no-content

# Искать повторяющиеся секреты
racc dig --repeated

# Проваливать запуск уже при High-находках
racc dig --fail-on high
```

**Политика выхода.** По умолчанию `dig` завершается с кодом `2`, если найдены секреты уровня `Critical` (и выше — то есть только Critical). Значения:

- `ignore` — не завершаться с ошибкой из-за находок (только `0`/`1`);
- `critical` — код `2` при находках уровня `Critical`;
- `high` — код `2` при находках уровня `High` и выше.

### Что пока ещё в разработке

Следующие команды планируются в ближайших версиях (см. [Дорожную карту](roadmap.md)):

| Команда | Назначение | Статус |
|---------|------------|--------|
| `racc pack` | Упаковать проект в `tar.zst` | В разработке (ядро готово, CLI ожидается в MVP M4.4) |
| `racc stash` | Вынести секреты в age-архив | Планируется (Alpha) |
| `racc rinse` | Очистить мусор сборки | Планируется (Alpha) |
| `racc raid` | Полный цикл одним действием | Планируется (Alpha) |
| `racc den …` | Управление den (список, очистка staging) | Планируется (Beta) |
| `racc init` | Сгенерировать стартовую конфигурацию | Планируется |

## Выходные данные

### Человекочитаемый вывод

По умолчанию `racc` печатает аккуратные таблицы. Пример `dig`:

```text
Dig root: /home/user/DEV/PROJS
Files scanned: 1204  |  Findings: 4  |  Repeated: 1  |  180 ms

RISK      LABEL                    PATH
Critical  AWS Access Key           /home/user/DEV/PROJS/app-api/app/config/aws.env
Critical  Private key PEM          /home/user/DEV/PROJS/app-api/certs/server.key
High      Env file                 /home/user/DEV/PROJS/app-api/app/.env
```

Находки сортируются по риску (убывание), затем по пути. Блок «Repeated secrets» печатается только при включённом `--repeated` и наличии повторений.

### JSON

Флаг `--json` печатает полный serde-результат команды. Это стабильный контракт для скриптов и CI. Отчёты содержат поле `schema_version` — для проверок совместимости.

> **Warning** В JSON-выводе никогда не содержится сырых значений секретов — только маскированные превью и хеши.

## Использование в CI

Типовой паттерн — проверять, что в проектах нет критичных секретов:

```bash
racc dig --project . --fail-on critical --json
```

Код выхода `2` сигнализирует о находках. Пример скрипта:

```bash
racc dig --root "$ROOT" --fail-on critical --json > dig.json
code=$?
if [ "$code" -eq 2 ]; then
  echo "Найдены CRITICAL секреты" >&2
  exit 2
fi
exit "$code"
```

## Дальнейшее чтение

- [Конфигурация](configuration.md) — пути, глубина и переменные окружения.
- [Основные понятия](concepts.md) — риски, маскирование, den.
