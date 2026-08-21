---
title: Raid — полный цикл одной командой
description: "Команда racc raid — stash → rinse → pack → move за один вызов: секреты в age-архив, чистка мусора сборки, архив проекта и манифест в den."
---

# Raid - полный цикл одной командой

Команда: `racc raid`  
Статус: реализовано (Alpha).

Эта страница описывает **ровно то поведение**, которое реализует `raccpack` сейчас. Если флаг или путь не указаны здесь — их нет в текущей версии.

Вернуться к обзору команд: [Использование CLI](/cli-usage).

## Что делает raid

`racc raid` запускает весь конвейер по проекту одной командой, в фиксированном порядке:

```text
stash  →  rinse  →  pack  →  move
```

1. **stash** — находит чувствительные файлы (те же правила, что у `racc dig`) и шифрует их в age-архив в `den/secrets/…`, по умолчанию удаляя исходники;
2. **rinse** — удаляет мусор сборки (`node_modules`, `target`, … по стратегиям);
3. **pack** — упаковывает проект **без** секретов в `den/packs/…`;
4. **move (commit)** — финализирует размещение и, после успеха, пишет манифест.

Результат одного успешного запуска:

```text
{den}/secrets/{год}/{месяц}/{slug}__{время}__secrets.age
{den}/packs/{год}/{месяц}/{slug}__{время}.tar.zst
{den}/manifests/{год}/{месяц}/{slug}__{время}__{id}.json
```

Манифест — JSON-запись для аудита: стадии, пути артефактов (относительно den), raw-free stash-manifest, версия инструмента, `success`, `dry_run`, `created_at`. Пишется **только** после успешного commit и только если артефакты реально размещены.

По умолчанию `racc raid` работает в **dry-run**: ничего не пишет и не удаляет.

::: info
По умолчанию используется **atomic** режим: все промежуточные файлы живут в `den/staging/{id}/`, удаление исходников и мусора откладывается в commit, а каждый шаг commit записывается в журнал (WAL). Если commit падает на середине — размещённые артефакты **откатываются** (`rolled_back`). См. [Orphan green](#orphan-green-и-флаги).
:::

## Быстрый старт

```bash
# 1) Посмотреть, что будет сделано (ничего не пишет и не удаляет)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den

# 2) Полный commit (stash + rinse + pack + manifest)
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes
```

::: warning
По умолчанию `racc raid` работает в **dry-run**: не пишет в den, не удаляет исходники и мусор. Commit — только с `--yes`.
:::

## Синтаксис

```text
racc raid --project <PATH> [OPTIONS]
```

`--project <PATH>` — **обязательный**: каталог проекта, над которым выполняется конвейер.

## Параметры и флаги

### Проект и den

| Параметр | Описание |
|----------|----------|
| `--project <PATH>` | Каталог проекта (обязательно) |
| `--den <PATH>` | Корень den. Если не указан — из config (`paths.den_dir`) |

### Режим записи

| Параметр | Поведение |
|----------|-----------|
| *(по умолчанию)* | **Dry-run**: только отчёт, файлы не создаются и не удаляются |
| `--dry-run` | Явный dry-run |
| `--yes` | **Commit**: записать артефакты в den, применить удаления |

**Приоритет:** если указаны и `--dry-run`, и `--yes`, побеждает dry-run.

### Фазы

| Параметр | По умолчанию | Описание |
|----------|--------------|----------|
| *(без флага)* | все фазы включены | stash → rinse → pack |
| `--no-stash` | — | Выключить stash (не искать/шифровать секреты, не удалять исходники) |
| `--no-rinse` | — | Выключить rinse (не чистить мусор сборки) |
| `--no-pack` | — | Выключить pack (не создавать `tar.zst`) |
| `--fail-fast` | — | Режим `FailFast` вместо atomic: остановиться на первой упавшей фазе (см. ниже) |

### Stash / pack тонкая настройка

| Параметр | По умолчанию | Описание |
|----------|--------------|----------|
| `--min-risk <LEVEL>` | `high` | Минимальный уровень риска для stash: `low`, `medium`, `high`, `critical` |
| `--keep-sources` | выкл. | Не удалять исходные секреты после успешного stash (`remove_sources` выключен) |
| `--no-content-deny` | выкл. | Не исключать из pack файлы с секретом в содержимом (deny по имени остаётся) |

### Вывод

| Параметр | Описание |
|----------|----------|
| `--json` | Печать `RaidResult` в JSON (стадии, `success`, `rolled_back`, артефакты) |

### Глобальные флаги

| Флаг | Описание |
|------|----------|
| `-c, --config <PATH>` | Файл конфигурации (переопределяет `RACCPACK_CONFIG`) |
| `--root <PATH>` | Переопределить `scan_root` на этот запуск |
| `--den <PATH>` | Переопределить `den_dir` на этот запуск |
| `--json` | Машиночитаемый вывод JSON |

## Passphrase

Нужна **только** если stash включён **и** выполняется Commit. Если задан `--no-stash`, passphrase не запрашивается даже с `--yes`.

Порядок выбора (как у `racc stash`):

1. Переменная окружения **`RACCPACK_PASSPHRASE`** — если задана и не пустая.
2. Иначе интерактивный ввод в TTY (два раза, без отображения символов).
3. Если stdin — не терминал, берётся **одна строка из stdin**.
4. Если нет ни env, ни TTY, ни stdin — ошибка с подсказкой задать `RACCPACK_PASSPHRASE`.

::: warning
Не коммитьте `RACCPACK_PASSPHRASE` и не храните passphrase в открытых скриптах. В CI задавайте переменную через secrets store.
:::

## Atomic vs fail-fast (orphan green)

### Atomic (по умолчанию)

- Все промежуточные артефакты живут в `den/staging/{id}/`.
- Удаление исходников (`remove_sources`) и мусора (`rinse`) откладывается в **move (commit)**.
- Каждый эффект commit записывается в журнал **до** применения; падение на середине откатывает размещённые артефакты.
- При откате человекочитаемый вывод показывает `Failed` и `rolled back (N warnings)`; в JSON — `rolled_back: true`.
- **Гарантия:** неудачный raid не оставляет `.age` / `.tar.zst` / манифест в den (только временный `staging/`, который очищается).
- **Audit-policy:** если manifest записан, но сам commit уже прошёл успешно — это `success: false` без отката (артефакты остаются в den, откатывать нечего). Откат не восстанавливает удалённые исходники/мусор (эффекты move с `remove_sources` необратимы) — они попадают в `rollback_warnings`.

### Fail-fast (`--fail-fast`)

- Легаси-поведение: останавливается на первой упавшей фазе.
- Уже размещённые артефакты **остаются** в den (это документированное отличие от atomic — «orphan»).
- Используется для отладки; в обычной работе предпочтителен atomic.

## Коды выхода

| Код | Когда |
|-----|--------|
| 0 | `Ok` и `success == true` (в т.ч. dry-run) |
| 1 | Ошибка CLI/конфига/фазы **или** `Ok` с `success == false` (вкл. откат commit) |

Код `2` (как у dig для Critical) **не** используется для raid.

## Вывод

### Человекочитаемый (human)

Во время работы печатаются строки фаз (`→ stash: …`, `→ rinse: …`, `→ pack: …`, `→ move: …`), затем итог:

```text
Success
  placed 2 artifact(s):
    /tmp/den/secrets/2026/08/my-api__20260804T155230Z__secrets.age
    /tmp/den/packs/2026/08/my-api__20260804T155230Z.tar.zst
```

При откате:

```text
Failed
  rolled back (1 warnings)
```

### JSON (`--json`)

Поля `RaidResult`: `stages` (имя/успех/сообщение), `stash`/`rinse`/`pack` подрезультаты, `den_artifacts`, `success`, `dry_run`, `rolled_back`, `rollback_warnings`. Raw-секретов в JSON нет.

## Примеры

```bash
# Dry-run: показать весь конвейер, ничего не писать
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den

# Полный atomic commit
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# Без stash (не трогать секреты; passphrase не нужна)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-stash

# Не удалять исходные секреты
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --keep-sources

# Debug fail-fast (orphan возможен)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --fail-fast

# JSON для CI + проверка rollback-полей
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --json \
  | jq '{success, rolled_back, stages}'

# Секреты только Critical
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --min-risk critical

# Exit code: success=false → 1
racc raid --project /bad --den /tmp/den --yes ; echo $?
```

## Безопасность

- По умолчанию dry-run — сначала смотрите отчёт.
- Удаление исходников и мусора — только в commit (`--yes`) и **после** успешного размещения артефактов.
- В atomic-режиме неудачный commit откатывается: артефакты не остаются в den.
- Passphrase не пишется в логи и JSON; материал ключа очищается (zeroize).
- Файл манифеста и `.age` создаются с правами `0600` (best-effort на Unix).
- Не коммитьте каталог den в git.

## Связанные команды

| Команда | Роль |
|---------|------|
| `racc sniff` / `racc dig` | Найти проекты / секреты (read-only) |
| `racc stash` | Только секреты → age-архив |
| `racc rinse` | Только чистка мусора |
| `racc pack` | Только архив проекта без секретов |

---

*Документ соответствует реализации; при изменении флагов CLI обновляйте страницу в том же PR.*