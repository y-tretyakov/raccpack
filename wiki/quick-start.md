---
title: Быстрый старт
description: Первый прогон raccpack за пять минут — конфигурация, поиск проектов и проверка на секреты.
---

# Быстрый старт

За пять минут: настроить raccpack, найти проекты, проверить их на секреты.

## 1. Убедитесь, что `racc` установлен

```bash
racc --version
```

Если команды нет — см. [Установка](/installation).

## 2. Создайте конфигурацию

Укажите папку с проектами и папку для «den»:

```bash
mkdir -p ~/.config/raccpack
cat > ~/.config/raccpack/config.toml <<'EOF'
[paths]
scan_root = "~/DEV/PROJS"
den_dir = "~/.raccpack/den"
EOF
```

::: info
Пути могут содержать `~` и относительные компоненты — raccpack сам приведёт их к абсолютным.
:::

## 3. Найдите проекты (`sniff`)

```bash
racc sniff
```

Пример вывода:

```text
Scan root: /home/user/DEV/PROJS
Projects: 3  |  Total size: 1.2 GiB  |  210 ms  |  cache: miss

NAME        STACK                 SIZE    GIT  PATH
app-api     Rust + Axum           412.5 MiB  yes  /home/user/DEV/PROJS/app-api
web-dashboard TypeScript + Next.js 730.1 MiB  yes  /home/user/DEV/PROJS/web-dashboard
scripts     -                      1.8 MiB   no   /home/user/DEV/PROJS/scripts
```

Если проектов нет — проверьте `scan_root` и глубину сканирования (см. [Конфигурация](/configuration)).

## 4. Проверьте проекты на секреты (`dig`)

```bash
racc dig
```

или по одному проекту:

```bash
racc dig --project ~/DEV/PROJS/app-api
```

Пример вывода:

```text
Dig root: /home/user/DEV/PROJS
Files scanned: 1204  |  Findings: 4  |  Repeated: 1  |  180 ms

RISK      LABEL                    PATH
Critical  AWS Access Key           /home/user/DEV/PROJS/app-api/app/config/aws.env
Critical  Private key PEM          /home/user/DEV/PROJS/app-api/certs/server.key
High      Env file                 /home/user/DEV/PROJS/app-api/app/.env
Medium    JWT-like token           /home/user/DEV/PROJS/scripts/token.txt
```

::: info
В выводе никогда не появляются исходные значения — только маскированные превью и уровень риска.
:::

## 5. Поймите код выхода

`racc dig` возвращает код выхода, пригодный для CI:

- `0` — ошибок нет;
- `1` — произошла ошибка выполнения;
- `2` — найдены секреты выше порога политики (по умолчанию `Critical`).

Это удобно для проверок в скриптах:

```bash
racc dig --fail-on high
code=$?
if [ "$code" -eq 2 ]; then
  echo "Найдены секреты High и выше"
fi
```

## 6. Упакуйте проект (`pack`)

```bash
racc pack --project ~/DEV/PROJS/app-api --yes
```

По умолчанию `pack` — **dry-run** (ничего не пишет); флаг `--yes` — явное подтверждение, которое записывает архив `.tar.zst` в den. Секреты из архива исключаются автоматически (по имени — всегда, по содержимому — по умолчанию).

## 7. Вынесите секреты (`stash`)

```bash
racc stash --project ~/DEV/PROJS/app-api
racc stash --project ~/DEV/PROJS/app-api --yes
```

Первый запуск — **dry-run** (ничего не пишет); флаг `--yes` переносит чувствительные файлы в зашифрованный age-архив в `den/secrets/`. Пароль задаётся через `RACCPACK_PASSPHRASE` или вводится интерактивно.

## 8. Что дальше

Сейчас CLI умеет `sniff`, `dig`, `pack` и `stash`; по roadmap — `rinse`, `raid`, `den`. Обзор команд — в [Использование CLI](/cli-usage), подробности по каждой — на страницах `/sniff`, `/dig`, `/pack` и `/stash`:

- [Использование CLI](/cli-usage) — полный справочник команд.
- [Основные понятия](/concepts) — что такое den, риски и фазы.
- [Конфигурация](/configuration) — все настройки.
