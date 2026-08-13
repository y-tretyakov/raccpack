# Raid — полный цикл: секреты, очистка, упаковка

**Для пользователей CLI.**  
Команда: `racc raid`  
Статус: Alpha.

Описывает **фактическое** поведение фазы A3. Нет в таблице — нет в текущей версии.

---

## Что делает raid

Одна команда последовательно:

1. **stash** — находит секреты, шифрует в `den/secrets/…/*.age`, опционально удаляет исходники  
2. **rinse** — удаляет мусор сборки (`node_modules`, `target`, …)  
3. **pack** — упаковывает проект в `den/packs/…/*.tar.zst` (без секретов)  
4. **move / finalize** — пишет **manifest** JSON в `den/manifests/…`

По умолчанию — **dry-run** (ничего не меняет на диске).  
Реальная запись и удаления — только с **`--yes`**.

Если включён stash, нужна **passphrase** (`RACCPACK_PASSPHRASE` или интерактивный ввод).

---

## Быстрый старт

```bash
# План без изменений на диске
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den

# Полный цикл
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes
```

---

## Примеры команд CLI

### Dry-run (безопасно, ничего не пишет)

```bash
# Текстовый отчёт: какие фазы и что сделали бы
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den

# Явный dry-run
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --dry-run

# JSON-план (удобно для CI-проверки)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --json

# Dry-run + JSON
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --dry-run --json

# Dry-run без stash (passphrase не нужна)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --no-stash --json
```

### Полный Commit

```bash
export RACCPACK_PASSPHRASE='your-strong-passphrase'

# Полный цикл: stash → rinse → pack → manifest
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# С JSON-результатом
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --json

# Рекомендуемый порядок: сначала dry-run, потом commit
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes
```

### Отключение фаз

```bash
# Без stash (passphrase не нужна)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-stash

# Без очистки node_modules/target
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-rinse

# Без pack (только stash + rinse + manifest)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-pack

# Только pack «через» raid
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-stash --no-rinse

# Только stash + pack (без rinse)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-rinse

# Только rinse + pack (секреты уже убраны раньше)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-stash
```

### Stash-опции внутри raid

```bash
export RACCPACK_PASSPHRASE='…'

# Не удалять исходные секретные файлы после шифрования
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --keep-sources

# Только Critical в stash
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --min-risk critical

# High и выше (default)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --min-risk high

# Более широкий захват (включая Medium)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --min-risk medium
```

### Pack-опции

```bash
# Не сканировать content на секреты при pack (только name-deny)
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-content-deny
```

### Config и пути

```bash
# Проект = текущая директория
cd ~/DEV/PROJS/my-api
racc raid --project . --den ~/.raccpack/den --yes

# Свой config
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den \
  --config ~/.config/raccpack/config.toml --yes

# Den из config (если paths.den_dir задан) — всё равно можно переопределить
racc raid --project ~/DEV/PROJS/my-api --yes
```

### Passphrase

```bash
# Через env (CI / неинтерактивно)
export RACCPACK_PASSPHRASE='your-strong-passphrase'
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# Одной строкой
RACCPACK_PASSPHRASE='…' racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# Интерактивно: не задавайте env, запустите в TTY с --yes — CLI запросит пароль
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# Stash выключен — passphrase не нужна
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --no-stash
```

### CI и jq

```bash
export RACCPACK_PASSPHRASE="$STASH_SECRET"

racc raid \
  --project "$CI_PROJECT_DIR" \
  --den "$DEN_PATH" \
  --yes \
  --json

# Проверить success
racc raid --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes --json | jq '.success'

# Список стадий
racc raid --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes --json | jq '.stages'

# Пути артефактов
racc raid --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --yes --json | jq -r '.den_artifacts[]'

# Только dry-run проверка в pipeline
racc raid --project "$CI_PROJECT_DIR" --den "$DEN_PATH" --json | jq '{success, dry_run, stages}'
```

### Проверка den после raid

```bash
DEN=~/.raccpack/den

# Версия den
cat "$DEN/.den-version"

# Секреты
find "$DEN/secrets" -name '*.age'

# Packs
find "$DEN/packs" -name '*.tar.zst'

# Manifests
find "$DEN/manifests" -name '*.json'
jq . "$(find "$DEN/manifests" -name '*.json' | sort | tail -1)"
```

### Типичные ошибки (что не сработает)

```bash
# Нет --project → ошибка
racc raid --den ~/.raccpack/den --yes

# Stash включён, нет passphrase и нет TTY → ошибка
RACCPACK_PASSPHRASE= racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes

# --dry-run вместе с --yes → удаление/запись НЕ выполняются
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --dry-run --yes

# Неверный min-risk
racc raid --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --min-risk ultra --yes
```

### Порядок «вручную» vs raid

Эквивалент полного raid по шагам:

```bash
export RACCPACK_PASSPHRASE='…'
racc stash --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes --remove-sources
racc rinse --project ~/DEV/PROJS/my-api --yes
racc pack  --project ~/DEV/PROJS/my-api --den ~/.raccpack/den --yes
# manifest пишет только raid (отдельной команды manifest в A3 нет)
```

Предпочтительно: **`racc raid --yes`**.

---

## Синтаксис

```text
racc raid --project <PATH> [OPTIONS]
```

| Параметр | По умолчанию | Описание |
|----------|--------------|----------|
| `--project <PATH>` | обязателен | Корень проекта |
| `--den <PATH>` | из config | Каталог den |
| `--yes` | выкл. | Commit (запись и удаления) |
| `--dry-run` | вкл. без `--yes` | Только план |
| `--json` | выкл. | Печать `RaidResult` |
| `--no-stash` | stash вкл. | Пропустить stash |
| `--no-rinse` | rinse вкл. | Пропустить rinse |
| `--no-pack` | pack вкл. | Пропустить pack |
| `--keep-sources` | исходники удаляются при stash | Не удалять файлы секретов |
| `--min-risk <LEVEL>` | `high` | Порог для stash: low\|medium\|high\|critical |
| `--no-content-deny` | content-deny вкл. | Pack без content-scan deny |
| `--config <PATH>` | XDG/default | Файл конфигурации |

Passphrase: env **`RACCPACK_PASSPHRASE`** или prompt (если stash включён).

---

## Fail-fast

Порядок фаз фиксирован: **stash → rinse → pack → move**.

Если enabled-фаза **упала**:

- следующие фазы **не** запускаются;
- уже созданные `.age` / `.tar.zst` **не** откатываются автоматически;
- в результате `success: false`, stages описывают, что успело выполниться.

---

## Что появляется в den

```text
{den}/
├── .den-version
├── README.txt
├── secrets/yyyy/mm/{slug}__{ts}__secrets.age
├── packs/yyyy/mm/{slug}__{ts}.tar.zst
└── manifests/yyyy/mm/{slug}__{ts}__{short_id}.json
```

Manifest **без** сырых секретов; пути артефактов — **относительно** den.

---

## JSON (`RaidResult`) — основные поля

| Поле | Смысл |
|------|--------|
| `success` | Все enabled-фазы успешны |
| `dry_run` | Режим |
| `stages` | `{ name, success, message, skipped }[]` |
| `stash` / `rinse` / `pack` | Вложенные результаты или null |
| `den_artifacts` | Абсолютные пути созданных файлов |
| `project_path` | Исходный проект |

---

## Коды выхода

| Код | Когда |
|-----|--------|
| 0 | `success == true` |
| 1 | Ошибка preconditions **или** `success == false` |

---

## Безопасность

- Default dry-run.
- Passphrase не в JSON и не в логах.
- Удаление секретов с диска только если stash включён и нет `--keep-sources`.
- Rinse удаляет только известные trash-dirs (см. [wiki-rinse](wiki-rinse.md)).
- Pack не кладёт High+ secret filenames (и content Critical при deny).

---

## Связанные страницы

- [Stash](wiki-stash.md) — только секреты  
- [Rinse](wiki-rinse.md) — только мусор  
- Pack — `racc pack` (MVP)

---

*Соответствует A3 (a3.1–a3.4). Изменение флагов CLI — обновление этой страницы в том же PR.*
