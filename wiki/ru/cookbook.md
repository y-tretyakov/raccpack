---
title: Usage cookbook — сценарии и скрипты
description: "Практические сценарии raccpack: onboarding, dry-run безопасность, полный raid, raid по всем проектам из config, JSON-пайплайны jq, отладка без утечки секретов — со скриптами для bash, fish, Nushell и PowerShell."
---

# Usage cookbook — сценарии и скрипты

Статус: актуально для **raccpack 0.4.0** (Detect v2 завершена: композитные DAG-детекторы, пакетный рейд `racc raid --root`).

Готовые рецепты поверх командной поверхности. Если флаг не описан здесь или на
странице команды — его нет в текущей версии.

Обзор команд: [Использование CLI](/ru/cli-usage) ·
Отдельные команды: [Sniff](/ru/sniff) · [Dig](/ru/dig) · [Pack](/ru/pack) ·
[Stash](/ru/stash) · [Rinse](/ru/rinse) · [Raid](/ru/raid) · [Init](/ru/init) ·
Конфиг: [Конфигурация](/ru/configuration) · [Git, init и DX](/ru/git-and-dx)

::: warning Dry-run по умолчанию
`pack`, `stash`, `rinse` и `raid` **ничего не пишут и не удаляют** без `--yes`.
Если переданы и `--yes`, и `--dry-run` — побеждает `--dry-run`.
:::

## 1. Onboarding: init → sniff → dig

Первый запуск: создать конфиг, посмотреть, что нашлось, проверить утечки.

```bash
racc init --scan-root ~/DEV/PROJS --ensure-den   # конфиг + den одним шагом
racc sniff                                        # таблица проектов (кэш)
racc sniff --force-refresh                        # пересканировать мимо кэша
racc dig                                          # чувствительные файлы по всему scan_root
```

Ожидаемый эффект: `init` пишет `~/.config/raccpack/config.toml` (пути можно
переопределить), создаёт den при `--ensure-den`. `sniff` показывает проекты,
стек и размер; `dig` — список находок с рисками Critical…Low.

Подробнее: [Init](/ru/init), [Sniff](/ru/sniff), [Dig](/ru/dig).

## 2. Dry-run safety

Любая опасная операция сначала запускается «вхолостую» — это поведение по
умолчанию:

```bash
racc pack  --project ~/DEV/PROJS/my-api     # покажет план архива, ничего не пишёт
racc rinse --project ~/DEV/PROJS/my-api     # покажет, что удалил бы
racc raid  --project ~/DEV/PROJS/my-api     # полный прогон без последствий
```

Ожидаемый эффект: отчёт в stdout, den не меняется, файлы в проекте целы.
Коммит — только явным `--yes`.

## 3. Полный raid одного проекта

Passphrase нужна только если включён stash (по умолчанию включён) и идёт
Commit. Из TTY она запрашивается дважды с подтверждением; в скриптах — через
env `RACCPACK_PASSPHRASE`.

```bash
export RACCPACK_PASSPHRASE='…'   # placeholder — подставьте свой секрет
racc raid --project ~/DEV/PROJS/my-api --yes
unset RACCPACK_PASSPHRASE
```

Порядок фаз: stash → rinse → pack → move. Любая упавшая фаза откатывает
операцию целиком (atomic по умолчанию; `--fail-fast` — режим отладки).

::: danger Passphrase
Passphrase не восстанавливается. Потеряли — age-архивы секретов не читаются.
`racc` никогда не логирует и не сохраняет её; в примерах выше — placeholder.
:::

## 4. Raid всех проектов из scan_root

Простейший способ отрейдить все проекты под директорией — `racc raid --root`:

```bash
# Сначала dry-run (по умолчанию)
racc raid --root ~/DEV/PROJS

# По-настоящему
racc raid --root ~/DEV/PROJS --yes
```

`--root` находит проекты под указанной директорией (те же маркеры, что и `sniff`),
затем рейдит их последовательно. Комбинируйте с `--only` и `--limit` для фильтрации:

```bash
# Только Rust-проекты
racc raid --root ~/DEV/PROJS --only rust --yes

# Первые 5 проектов, остановка при первой ошибке
racc raid --root ~/DEV/PROJS --limit 5 --stop-on-error --yes
```

::: tip Без stash
Если ни в одном проекте нет секретов, отключите stash чтобы не запрашивать passphrase:
`racc raid --root ~/DEV/PROJS --yes --no-stash`.
:::

::: details Продвинутое: кастомный фильтр через скрипт
Когда нужен фильтр, который `--only` не может выразить (например исключить
конкретный проект или использовать внешние метаданные), зациклите `racc raid --project` сами.

Скрипты ниже читают проекты из `racc sniff --json --force-refresh`, проходят
циклом и рейдят каждый. Установите `EXTRA_RAID_ARGS="--no-stash"` если stash
не нужен.

Переменные окружения:

| Переменная | Смысл | По умолчанию |
|------------|-------|--------------|
| `RACCPACK_PASSPHRASE` | passphrase для stash-фазы | пусто (обязательно, если не `--no-stash`) |
| `DRY_RUN=1` | гонять без `--yes` | `0` (реальный Commit с `--yes`) |
| `EXTRA_RAID_ARGS` | доп. аргументы raid, напр. `--no-stash --keep-sources` | пусто |
| `CONTINUE_ON_ERROR=1` | не останавливаться на первой ошибке проекта | `1` |

### bash

```bash
#!/usr/bin/env bash
# raid-all.sh — raid по всем проектам из sniff (scan_root/den из config)
set -u

command -v racc >/dev/null || { echo "need: racc" >&2; exit 1; }
command -v jq    >/dev/null || { echo "need: jq" >&2; exit 1; }

DRY_RUN="${DRY_RUN:-0}"
CONTINUE_ON_ERROR="${CONTINUE_ON_ERROR:-1}"
EXTRA_RAID_ARGS="${EXTRA_RAID_ARGS:-}"

if [ -z "${RACCPACK_PASSPHRASE:-}" ] && [[ "$EXTRA_RAID_ARGS" != *--no-stash* ]]; then
  echo "ERROR: set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS=--no-stash)" >&2
  exit 1
fi
export RACCPACK_PASSPHRASE

mapfile -t PROJECTS < <(racc sniff --json --force-refresh | jq -r '.report.projects[].path')
[ "${#PROJECTS[@]}" -eq 0 ] && { echo "No projects found."; exit 0; }

echo "Found ${#PROJECTS[@]} project(s):"
printf '  - %s\n' "${PROJECTS[@]}"

ok=0; fail=0; failed=()
for proj in "${PROJECTS[@]}"; do
  echo "==> raid: $proj"
  mode=(--yes); [ "$DRY_RUN" = "1" ] && mode=(--dry-run)
  # shellcheck disable=SC2086
  if racc raid --project "$proj" "${mode[@]}" $EXTRA_RAID_ARGS; then
    ok=$((ok+1))
  else
    echo "FAIL: $proj" >&2; fail=$((fail+1)); failed+=("$proj")
    [ "$CONTINUE_ON_ERROR" != "1" ] && exit 1
  fi
done

echo "Done. ok=$ok fail=$fail total=${#PROJECTS[@]}"
[ "$fail" -gt 0 ] && { printf 'Failed: %s\n' "${failed[@]}"; exit 1; }
```

### fish

```fish
#!/usr/bin/env fish
# raid-all.fish — raid по всем проектам из sniff (scan_root/den из config)

set -q DRY_RUN; or set DRY_RUN 0
set -q CONTINUE_ON_ERROR; or set CONTINUE_ON_ERROR 1
set -q EXTRA_RAID_ARGS; or set EXTRA_RAID_ARGS ""

if not command -q racc
    echo "need: racc" >&2
    exit 1
end
if not command -q jq
    echo "need: jq" >&2
    exit 1
end

set -q RACCPACK_PASSPHRASE; or set RACCPACK_PASSPHRASE ""
if test -z "$RACCPACK_PASSPHRASE"; and not string match -q '*--no-stash*' -- $EXTRA_RAID_ARGS
    echo "ERROR: set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS=--no-stash)" >&2
    exit 1
end
set -gx RACCPACK_PASSPHRASE $RACCPACK_PASSPHRASE

echo "==> sniff --json --force-refresh"
set PROJECTS (racc sniff --json --force-refresh | jq -r '.report.projects[].path')

if test (count $PROJECTS) -eq 0
    echo "No projects found."
    exit 0
end

echo "Found "(count $PROJECTS)" project(s):"
for p in $PROJECTS
    echo "  - $p"
end

set ok 0
set fail 0
set failed_list

for proj in $PROJECTS
    echo "==> raid: $proj"
    set RAID_ARGS raid --project $proj
    if test "$DRY_RUN" = "1"
        set -a RAID_ARGS --dry-run
    else
        set -a RAID_ARGS --yes
    end
    if test -n "$EXTRA_RAID_ARGS"
        set -a RAID_ARGS (string split ' ' -- $EXTRA_RAID_ARGS)
    end

    if racc $RAID_ARGS
        echo "OK: $proj"
        set ok (math $ok + 1)
    else
        echo "FAIL: $proj" >&2
        set fail (math $fail + 1)
        set -a failed_list $proj
        if test "$CONTINUE_ON_ERROR" != "1"
            echo "Stopping on first error." >&2
            exit 1
        end
    end
end

echo "Done. ok=$ok fail=$fail total="(count $PROJECTS)
if test $fail -gt 0
    for p in $failed_list
        echo "  - $p"
    end
    exit 1
end
```

### Nushell

```nu
#!/usr/bin/env nu
# raid-all.nu — raid по всем проектам из sniff (scan_root/den из config)
# Зависимости: racc в PATH; JSON парсится встроенными средствами nu (jq не нужен)

def main [
  --dry-run   # гонять без --yes (аналог DRY_RUN=1)
] {
  # CONTINUE_ON_ERROR=0 останавливает цикл на первой ошибке (по умолчанию — продолжать)
  let stop_on_error = (($env | get -i CONTINUE_ON_ERROR | default '1') == '0')
  let extra = ($env | get -i EXTRA_RAID_ARGS | default '')
  let pass  = ($env | get -i RACCPACK_PASSPHRASE | default '')
  if ($pass | is-empty) and not ($extra | str contains '--no-stash') {
    error make {msg: "set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS='--no-stash')"}
  }

  let projects = (racc sniff --json --force-refresh
    | from json
    | get report.projects.path)

  if ($projects | is-empty) {
    print 'No projects found.'
    return
  }
  print $"Found ($projects | length) project(s):"
  for p in $projects { print $"  - ($p)" }

  mut ok = 0
  mut fail = 0
  mut failed = []
  for proj in $projects {
    print $"==> raid: ($proj)"
    let mode = if $dry_run { '--dry-run' } else { '--yes' }
    let extra_args = ($extra | split row ' ' | where {|x| $x != ''})
    let outcome = (do -i { ^racc raid --project $proj $mode ...$extra_args } | complete)
    if $outcome.exit_code == 0 {
      print $"OK: ($proj)"
      $ok += 1
    } else {
      print -e $"FAIL: ($proj)"
      $fail += 1
      $failed = ($failed | append $proj)
      if $stop_on_error {
        error make {msg: 'stopping on first error'}
      }
    }
  }

  print $"Done. ok=($ok) fail=($fail) total=($projects | length)"
  if $fail > 0 {
    for p in $failed { print $"  - ($p)" }
    exit 1
  }
}
```

### PowerShell 7+

```powershell
#!/usr/bin/env pwsh
# raid-all.ps1 — raid по всем проектам из sniff (scan_root/den из config)

$ErrorActionPreference = 'Continue'

if (-not (Get-Command racc -ErrorAction SilentlyContinue)) { Write-Error 'need: racc'; exit 1 }
if (-not (Get-Command jq    -ErrorAction SilentlyContinue)) { Write-Error 'need: jq';    exit 1 }

$DRY_RUN            = if ($env:DRY_RUN)            { $env:DRY_RUN }            else { '0' }
$CONTINUE_ON_ERROR  = if ($env:CONTINUE_ON_ERROR)  { $env:CONTINUE_ON_ERROR }  else { '1' }
$EXTRA_RAID_ARGS    = if ($env:EXTRA_RAID_ARGS)    { $env:EXTRA_RAID_ARGS }    else { '' }

if (-not $env:RACCPACK_PASSPHRASE -and $EXTRA_RAID_ARGS -notmatch '--no-stash') {
  Write-Error 'ERROR: set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS=--no-stash)'
  exit 1
}

$projects = (racc sniff --json --force-refresh | jq -r '.report.projects[].path')
if (-not $projects) { Write-Host 'No projects found.'; exit 0 }

$projects = @($projects)
Write-Host "Found $($projects.Count) project(s):"
$projects | ForEach-Object { Write-Host "  - $_" }

$ok = 0; $fail = 0; $failed = @()
foreach ($proj in $projects) {
  Write-Host "==> raid: $proj"
  $mode = if ($DRY_RUN -eq '1') { '--dry-run' } else { '--yes' }
  $extra = @($EXTRA_RAID_ARGS -split ' ' | Where-Object { $_ })
  racc raid --project $proj $mode @extra
  if ($LASTEXITCODE -eq 0) {
    $ok++
  } else {
    Write-Error "FAIL: $proj"
    $fail++; $failed += $proj
    if ($CONTINUE_ON_ERROR -ne '1') { exit 1 }
  }
}

Write-Host "Done. ok=$ok fail=$fail total=$($projects.Count)"
if ($fail -gt 0) { $failed | ForEach-Object { Write-Host "  - $_" }; exit 1 }
```

::: info Про одинаковые env во всех вариантах
Скрипты намеренно повторяют одну семантику: sniff из config → цикл raid →
сводка. Отличается только синтаксис оболочки; в nu вместо `jq` — встроенный
`from json`.
:::
:::

## 5. Точечные операции: только stash / только rinse / только pack

Фазы raid доступны и по отдельности — когда нужен только один эффект:

```bash
# Только секреты в age-архив (+ удалить исходники после успешного commit)
racc stash --project ~/DEV/PROJS/my-api --yes --remove-sources

# Только чистка мусора сборки
racc rinse --project ~/DEV/PROJS/my-api --yes

# Только архив проекта в den
racc pack --project ~/DEV/PROJS/my-api --yes
```

Полезные уточнители stash: `--min-risk critical` (брать только критичные),
`--only path/to/file` (конкретный файл, повторяемый), `--batch-id release-x`
(имя артефакта вместо timestamp). Уточнители rinse: `--strategy ID`
(повторяемый; по умолчанию стратегии из config).

Страницы: [Stash](/ru/stash) · [Rinse](/ru/rinse) · [Pack](/ru/pack).

## 6. Raid без stash (--no-stash), когда нет passphrase

Архив и чистка работают без шифрования секретов:

```bash
racc raid --project ~/DEV/PROJS/my-api --yes --no-stash
```

С passphrase не задаётся вовсе — stash-фаза выключена. Вариант для «холодных»
проектов без чувствительных файлов или когда secrets-фаза будет отдельным
проходом.

## 7. JSON-пайплайны

`--json` у каждой команды; структура стабильна (`schema_version`).

```bash
# Пути только Critical-находок
racc dig --project "$PROJ" --json | jq -r '.files[] | select(.risk=="Critical") | .path'

# Находки High+ с git-статусом (git_status есть только в JSON)
racc dig --project "$PROJ" --json \
  | jq '.files[] | select(.risk=="Critical" or .risk=="High") | {path, risk, git_status}'

# Повторяющиеся секреты (одинаковое значение в нескольких файлах)
racc dig --project "$PROJ" --repeated --json | jq '.repeated'

# Проекты больше 100 MiB
racc sniff --json | jq '.report.projects[] | select(.size_bytes > 104857600) | .path'

# Git-репозитории без языка
racc sniff --json | jq '.report.projects[] | select(.is_git_repo and (.stack.language == null)) | .name'
```

Коды выхода: `dig` возвращает `2` при срабатывании политики `--fail-on
critical|high` — удобно для CI.

## 8. Отладка без утечки секретов

Логи (`tracing`) всегда идут в **stderr**, машинный вывод (`--json`) — в
**stdout**, поэтому пайпы не смешиваются, а логи не попадают в JSON:

```bash
racc dig --project "$PROJ" --json 2>dig.log          # stdout чистый JSON, логи в файл
racc raid --project "$PROJ" -vv --yes                # debug-логи в терминал
racc pack --project "$PROJ" -v                       # info-логи
```

Уровни: `-v` info · `-vv` debug · `-vvv` trace. В логах нет raw-секретов,
passphrase и содержимого файлов — это инвариант продукта.

## 9. Кастомные config / den / root

```bash
# Разовый override путей
racc sniff --root ~/other/projects --den /mnt/vault/den

# Альтернативный конфиг целиком
RACCPACK_CONFIG=~/.config/raccpack/work.toml racc raid --project "$PROJ" --yes
racc --config ~/.config/raccpack/work.toml sniff
```

Пути с `~` и относительные резолвятся в абсолютные при загрузке конфига.
Что внутри конфига — [Конфигурация](/ru/configuration); миграция версий конфига —
[Git, init и DX](/ru/git-and-dx).

## 10. Monorepo awareness

`sniff` может показать и корень монорепо, и вложенные пакеты (у каждого свои
маркеры). Перед массовым raid отфильтруйте «листья», чтобы не упаковать одно и
то же дважды.

::: tip Проще через `--root`
`racc raid --root ~/path/to/monorepo --only subpkg` обычно достаточно для рейда
конкретных вложенных пакетов без shell-цикла. Скрипты ниже нужны только когда
нужен кастомный фильтр (например исключить конкретную подпапку по пути).
:::

```bash
# Показать дерево кандидатов
racc sniff --json | jq '.report.projects[] | {name, path}'

# Рейдить только вложенные пакеты, исключив корень
racc sniff --json \
  | jq -r '.report.projects[].path' \
  | grep -v '/monorepo-root$' \
  | while read -r p; do racc raid --project "$p" --yes --no-stash; done
```

::: warning Двойная упаковка монорепо
Рейд корня и его подпапок даёт пересекающиеся архивы. Решите заранее уровень
«единицы бэкапа»: обычно листья (пакеты/сервисы) или корень, но не оба.
:::

## 11. Проверка den после raid

```text
~/.raccpack/den/
├── packs/{yyyy}/{mm}/      # {slug}__{UTC}__.tar.zst
├── secrets/{yyyy}/{mm}/    # {slug}__{UTC}__.age
└── manifests/{yyyy}/{mm}/  # JSON-манифесты raid
```

Быстрая сверка:

```bash
ls -lh ~/.raccpack/den/packs/*/* | tail
ls -lh ~/.raccpack/den/secrets/*/* | tail
jq '{project, success, phases}' ~/.raccpack/den/manifests/*/*.json | tail -40
```

Манифесты содержат метаданные операции (пути относительно den, фазы, счётчики)
— без raw-секретов. Layout и соглашения имён: [Concepts](/ru/concepts).

## 12. Проверка контрольных сумм и установка бинарника из Release

```bash
# Скачать tarball и подпись суммы (см. GitHub Release v0.3.0)
curl -LO https://github.com/y-tretyakov/raccpack/releases/download/v0.3.0/raccpack-0.3.0-linux-x86_64.tar.gz
curl -LO https://github.com/y-tretyakov/raccpack/releases/download/v0.3.0/raccpack-0.3.0-linux-x86_64.tar.gz.sha256

sha256sum -c raccpack-0.3.0-linux-x86_64.tar.gz.sha256   # OK
tar xzf raccpack-0.3.0-linux-x86_64.tar.gz               # внутри: racc (0755)
./racc --version                                         # racc 0.3.0
install -m 0755 racc ~/.local/bin/racc                   # или ~/.cargo/bin
racc init --scan-root ~/DEV/PROJS --ensure-den
```

Для ARM64/Raspberry Pi/Graviton возьмите `linux-aarch64`; для Alpine — суффикс
`-musl` (если сборка присутствует в релизе).
