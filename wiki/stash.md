---
title: Stash — вынос секретов в зашифрованный архив
description: Команда racc stash — собрать чувствительные файлы проекта в зашифрованный age-архив в den, при необходимости удалив исходники.
---

# Stash - вынос секретов в зашифрованный архив (age)

Команда: `racc stash`  
Статус: реализовано (Alpha).

Эта страница описывает **ровно то поведение**, которое реализует `raccpack` сейчас. Если флаг или путь не указаны здесь — их нет в текущей версии.

Вернуться к обзору команд: [Использование CLI](/cli-usage).

## Что делает stash (и чего не делает)

`racc stash` выносит чувствительные файлы проекта в зашифрованный архив:

1. Находит чувствительные файлы в проекте (по имени и, при необходимости, по содержимому — те же правила, что у `racc dig`).
2. Упаковывает их в один архив **tar**, затем шифрует **age** с **passphrase**.
3. Кладёт файл в **den**:

   ```text
   {den}/secrets/{год}/{месяц}/{slug}__{UTC-время}__secrets.age
   ```

4. По желанию **удаляет** исходные файлы с диска (только после успешной записи архива и только с явным флагом).

Сырые секреты **не** печатаются в терминал и **не** попадают в JSON-отчёт.

Чего stash **не** делает:

- без `--yes` ничего не пишет и не удаляет (по умолчанию — **dry-run**);
- не расшифровывает архивы — для этого вне `racc` используйте `age` (см. [Примеры](#расшифровка-вручную-age--d));
- не использует код выхода `2` (это особенность только `dig`).

## Быстрый старт

```bash
# 1) Посмотреть, что будет сделано (ничего не пишет и не удаляет)
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den

# 2) Реально создать .age в den (исходники НЕ удаляются)
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# 3) Создать .age и удалить исходные секретные файлы
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --remove-sources
```

::: warning
По умолчанию `stash` работает в **dry-run**: не создаёт `.age`, не трогает den и не удаляет файлы. Запись в den — только с `--yes`.
:::

::: danger
`--remove-sources` **удаляет** исходные секретные файлы с диска после **успешного** Commit (`--yes`). Сначала выполните dry-run без `--yes`. В CI не передавайте `--remove-sources`, если файлы ещё нужны job'у.
:::

Интерактивно (без env): запустите с `--yes` в терминале — CLI запросит passphrase дважды (без отображения символов).

## Синтаксис

```text
racc stash --project <PATH> [OPTIONS]
```

`--project <PATH>` — **обязательный** параметр: каталог проекта (или поддерево), в котором ищем секреты.

## Параметры и флаги

### Проект (обязательно)

| Параметр | Описание |
|----------|----------|
| `--project <PATH>` | Каталог проекта (или поддерево), в котором ищем секреты |

### Den

| Параметр | Описание |
|----------|----------|
| `--den <PATH>` | Корень den. Если не указан — берётся из config (`paths.den_dir`), обычно `~/.raccpack/den` |

### Режим записи

| Параметр | Поведение |
|----------|-----------|
| *(по умолчанию)* | **Dry-run**: только отчёт, файлы не создаются и не удаляются |
| `--dry-run` | Явный dry-run |
| `--yes` | **Commit**: записать `.age` в den |

**Приоритет:** если указаны и `--dry-run`, и `--yes`, побеждает dry-run — ничего не пишется и не удаляется.

### Секреты и удаление

| Параметр | По умолчанию | Описание |
|----------|--------------|----------|
| `--min-risk <LEVEL>` | `high` | Минимальный уровень риска: `low`, `medium`, `high`, `critical` |
| `--remove-sources` | выкл. | После **успешного** Commit удалить исходные файлы |
| `--only <PATH>` | все найденные | Можно повторять: архивировать только перечисленные файлы (должны лежать внутри `--project`) |
| `--batch-id <ID>` | нет | Заменяет UTC-временной токен в имени файла: `{slug}__{ID}__secrets.age` |

`--remove-sources` в dry-run **игнорируется** (удаления не будет).

### Вывод

| Параметр | Описание |
|----------|----------|
| `--json` | Печать `StashResult` в JSON (пути, счётчики, manifest **без** содержимого секретов) |

### Глобальные флаги

| Флаг | Описание |
|------|----------|
| `-c, --config <PATH>` | Файл конфигурации (переопределяет `RACCPACK_CONFIG`) |
| `--root <PATH>` | Переопределить `scan_root` на этот запуск |
| `--den <PATH>` | Переопределить `den_dir` на этот запуск |
| `--json` | Машиночитаемый вывод JSON |

## Поведение

### Режимы: dry-run и commit

- По умолчанию — **dry-run**: ничего не создаётся и не удаляется, `archive_path` в отчёте — ожидаемый путь.
- **Commit** — только с `--yes`. Порядок операций fail-safe: шифрование → размещение в den → (опционально) удаление исходников. Ошибка шифрования или размещения **никогда** не приводит к удалению исходников.

### Passphrase

Нужна только в **Commit**; в dry-run не запрашивается. Порядок выбора:

1. Переменная окружения **`RACCPACK_PASSPHRASE`** — если задана и не пустая.
2. Иначе интерактивный ввод в TTY (два раза для подтверждения, без отображения символов).
3. Если stdin — не терминал (например, пайп в CI), берётся **одна строка из stdin**.
4. Если нет ни env, ни TTY, ни stdin — ошибка с подсказкой задать `RACCPACK_PASSPHRASE`.

Рекомендации:

::: warning
Не коммитьте `RACCPACK_PASSPHRASE` и не храните passphrase в открытых скриптах. В CI задавайте переменную через secrets store.
:::

- После команды процесс не обязан хранить пароль; в core материал ключа очищается (zeroize). Значение не пишется в логи и JSON.

### Структура den после stash

```text
~/.raccpack/den/
├── .den-version          # 1
├── README.txt
├── secrets/
│   └── 2026/
│       └── 08/
│           └── my-api__20260804T155230Z__secrets.age
├── packs/                # от racc pack
├── staging/              # временное; после успеха очищается
└── …
```

Имя файла:

```text
{project_slug}__{YYYYMMDDThhmmssZ}__secrets.age
```

- `project_slug` — имя папки проекта, безопасные символы `[a-zA-Z0-9._-]`, пробелы → `-`, длина ≤ 80.
- Время — **UTC**.
- С `--batch-id <ID>` временной токен в имени заменяется на `ID`: `{slug}__{ID}__secrets.age`. Год/месяц каталога (`secrets/{yyyy}/{mm}`) при этом по-прежнему берутся из текущего UTC-времени.

### Что попадает в архив

- Файлы, которые нашёл бы `racc dig` с риском **≥ `--min-risk`** (по умолчанию High и Critical).
- Типичные примеры имён: `.env`, `.env.*`, `id_rsa`, `*.pem`, `.npmrc`, `credentials`, …  
  Точный набор совпадает с filename/content rules движка secrets (см. dig).

Не попадает:

- каталоги вроде `node_modules`, `target` (skip policy);
- файлы ниже порога risk;
- при `--only` — всё, что не перечислено.

## Вывод

### Человекочитаемый (human)

Dry-run:

```text
Stash (dry-run)
  Would archive: 1 files → /tmp/den/secrets/2026/08/app__20260815T141227Z__secrets.age
  Would remove sources: no (--remove-sources not set)
  (nothing written or deleted)
```

Если задан `--remove-sources`, вторая строка меняется на `Would remove sources: yes (requires --yes)`.

Commit:

```text
Stash complete
  Archive: /tmp/den/secrets/2026/08/app__20260815T141227Z__secrets.age
  Files: 1  (21 B plaintext)
  Removed sources: 0
```

### JSON (`--json`)

| Поле | Смысл |
|------|--------|
| `archive_path` | Путь к `.age` (в dry-run — ожидаемый путь) |
| `files_archived` | Число файлов |
| `bytes_archived` | Суммарный размер plaintext |
| `removed_sources` | Сколько исходников удалено (0 в dry-run) |
| `dry_run` | `true` / `false` |
| `manifest` | Список `{ original_path, risk, size_bytes }` **без** содержимого файлов |

Пример:

```json
{
  "archive_path": "/tmp/den/secrets/2026/08/app__20260815T141227Z__secrets.age",
  "files_archived": 2,
  "bytes_archived": 91,
  "removed_sources": 0,
  "dry_run": false,
  "manifest": [
    { "original_path": "/tmp/app/.env", "risk": "High", "size_bytes": 21 }
  ]
}
```

## Коды выхода

| Код | Когда |
|-----|--------|
| 0 | Успех (в т.ч. dry-run) |
| 1 | Ошибка: нет project/den, пустой passphrase, нечего архивировать, IO, encrypt |

Код `2` (как у dig для Critical) **не** используется для stash.

## Примеры

```bash
# Локально: dry-run — показать, что попадёт в архив (ничего не пишется)
racc stash --project ~/DEV/PROJS/my-api

# Commit: создать .age в den (исходники не удаляются)
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# Commit и удалить исходные секретные файлы
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --remove-sources

# JSON для скриптов и CI
racc stash --project ~/DEV/PROJS/my-api --yes --json

# Архивировать только конкретные файлы (повторяемый флаг)
racc stash --project ~/DEV/PROJS/my-api --yes --only ~/DEV/PROJS/my-api/.env --only ~/DEV/PROJS/my-api/id_rsa

# Своё имя артефакта вместо timestamp
racc stash --project ~/DEV/PROJS/my-api --yes --batch-id release-42
# → …/secrets/2026/08/my-api__release-42__secrets.age

# Понизить порог риска (архивировать и Medium)
racc stash --project ~/DEV/PROJS/my-api --min-risk medium --dry-run

# --dry-run всегда побеждает --yes: ничего не пишется
racc stash --project ~/DEV/PROJS/my-api --yes --dry-run
```

### Примеры для CI

::: code-group

```bash [bash]
# bash / zsh
export RACCPACK_PASSPHRASE="$STASH_SECRET"   # из CI secrets
racc stash --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes --json
# исходники на CI-агенте обычно не удаляют:
# не передавайте --remove-sources, если артефакты ещё нужны job'у
```

```fish [fish]
set -gx RACCPACK_PASSPHRASE $STASH_SECRET   # из CI secrets
racc stash --project $CI_PROJECT_DIR --den $DEN_PATH --yes --json
```

```nu [nu]
$env.RACCPACK_PASSPHRASE = $env.STASH_SECRET   # из CI secrets
racc stash --project $env.CI_PROJECT_DIR --den $env.DEN_PATH --yes --json
```

```powershell [pwsh]
$env:RACCPACK_PASSPHRASE = $env:STASH_SECRET   # из CI secrets
racc stash --project $env:CI_PROJECT_DIR --den $env:DEN_PATH --yes --json
```

:::

### Расшифровка вручную (`age -d`)

Расшифровка архива средствами `racc` в Alpha A1 **не** входит в CLI (используйте официальный инструмент [age](https://github.com/FiloSottile/age) при необходимости). Внутри архива после расшифровки — **tar** с относительными путями файлов.

```bash
age -d -o secrets.tar /path/to/…__secrets.age
tar -tf secrets.tar
tar -xf secrets.tar -C /safe/restore/dir
```

## Частые ошибки

| Ситуация | Что сделать |
|----------|-------------|
| `nothing to stash: no files matched the current min-risk threshold` | Понизьте `--min-risk` или проверьте `racc dig --project …` |
| Нет passphrase | Задайте `RACCPACK_PASSPHRASE` или запустите в интерактивном терминале |
| `--remove-sources` не удалило | Нужен `--yes` (Commit) и успешное завершение без ошибки |
| Path outside project для `--only` | Укажите пути строго внутри `--project` |

## Безопасность

- По умолчанию dry-run — сначала смотрите отчёт.
- Удаление исходников только с `--yes --remove-sources`, причём строго **после** успешного размещения архива в den.
- Passphrase не пишется в логи и JSON; в core материал ключа очищается (zeroize).
- Файл `.age` создаётся с правами **`0600`** (best-effort на Unix).
- Права den: при создании лучше `0700`.
- Не коммитьте каталог den в git.

## Связанные команды

| Команда | Роль |
|---------|------|
| `racc dig` | Найти секреты (read-only) |
| `racc sniff` | Найти проекты под `scan_root` |
| `racc pack` | Упаковать проект **без** секретов в `packs/` |
| `racc raid` | Полный цикл одной командой: stash → rinse → pack → move |

---

*Документ соответствует реализации; при изменении флагов CLI обновляйте страницу в том же PR.*
