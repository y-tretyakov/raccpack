---
title: Что поддерживается
description: Полный каталог того, что raccpack поддерживает на MVP 0.1.0 — маркеры проектов, фреймворки, секреты, skip-политика и deny при упаковке.
---

# Что поддерживается

::: info
Этот список снят с кода `raccpack-core` на момент **MVP 0.1.0** (ветка `dev`).
Добавление нового языка или секрета = изменение кода **и** обновление этой страницы в том же PR.
:::

## 1. Обнаружение проектов (sniff)

### 1.1 Маркеры по экосистемам

Проект определяется по **маркерам** — характерным файлам/каталогам в корне директории.
Реестр собран в [`scan/markers/`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/scan/markers) в стабильном порядке групп: rust → node → go → python → jvm → ruby → php → cpp → make → git.

| Экосистема | Маркер | Kind | language_hint |
|------------|--------|------|----------------|
| Rust | `Cargo.toml` | file | Rust |
| Node.js / JS / TS | `package.json` | file | JavaScript |
| Go | `go.mod` | file | Go |
| Python | `pyproject.toml` | file | Python |
| Python | `setup.py` | file | Python |
| Python | `requirements.txt` | file | Python |
| JVM (Java) | `pom.xml` | file | Java |
| JVM (Java) | `build.gradle` | file | Java |
| JVM (Kotlin) | `build.gradle.kts` | file | Kotlin |
| Ruby | `Gemfile` | file | Ruby |
| PHP | `composer.json` | file | PHP |
| C/C++ | `CMakeLists.txt` | file | C++ |
| Make | `Makefile` | file | — |
| Git | `.git` | dir | — |

**Всего 14 маркеров.** Маркер `Makefile` несёт `None` как `language_hint` (язык не определяется); маркер `.git` — директория, без языкового сигнала.

::: tip
Реестр экосистемно-модульный: один язык = один файл (`rust.rs`, `node.rs`, …) + одна строка в реестре `scan/markers/mod.rs`. `candidates.rs` ничего не знает о языках.
:::

### 1.2 Как выбирается язык

Язык резолвится из `language_hint` найденных маркеров по таблице приоритетов из [`detect/types.rs` `LANGUAGE_PRIORITY_GROUPS`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/detect/types.rs) (выше — приоритетнее):

```
Cargo.toml > go.mod > (pom.xml, build.gradle, build.gradle.kts)
  > package.json > pyproject.toml > setup.py > requirements.txt
  > Gemfile > composer.json > CMakeLists.txt > Makefile
```

- Маркеры **внутри одной группы** имеют равный приоритет; при равенстве побеждает первый хит в порядке `hits`.
- `.git` в таблице **отсутствует** — он не несёт языкового сигнала.
- Хит из таблицы с `None`-подсказкой (например `Makefile`) даёт `None`, даже если рядом есть другой хит с подсказкой.
- Если ни один хит не попал в таблицу — берётся подсказка первого хита (покрывает пользовательские `extra_markers`), при её отсутствии — `None`.

### 1.3 Фреймворки (shallow detect)

Фреймворки определяются **поверхностно** — по именам файлов в **top-level** листинге каталога проекта (`read_dir` одного уровня, без рекурсии и без чтения lock-файлов). Ровно один пик вглубь делает только Ruby-детектор (один уровень `config/`, чтобы отличить Rails).

| Экосистема | Признак (имя файла) | Значение в `stack.frameworks` |
|------------|----------------------|-------------------------------|
| Node.js | `next.config.{js,mjs,ts}` | `Next.js` |
| Node.js | `nuxt.config.*` | `Nuxt` |
| Node.js | `angular.json` | `Angular` |
| Node.js | `vite.config.*` | `Vite` |
| Node.js | `deno.json` | `Deno` |
| Python | `manage.py` | `Django` |
| JVM | `build.sbt` | `Scala/sbt` |
| Ruby | `Gemfile` **и** `config/application.rb` | `Rails` |

- Go, PHP, C/C++, Make, Rust и Git — **правил фреймворков в MVP нет**.
- Ruby: `Rails` срабатывает только при наличии `Gemfile` (иначе детектор возвращает пустой стек) и настоящего каталога `config/` (симлинк не «прочитывается», за пределы корня проект не выходит).
- Не выполняются последующие проверки, если каталог `config` — симлинк.
- Результаты детерминированы: имена в listing сортируются до сравнения.

::: info
Глубокого парсинга `package.json`, `Cargo.toml` (workspace, зависимости для определения фреймворков, вроде Axum) на MVP **нет** — см. [раздел 5](#5-что-пока-не-поддерживается-честно).
:::

### 1.4 Git

`.git` распознаётся как маркер **DirName** ([`scan/markers/git.rs`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/scan/markers/git.rs)). Он даёт:

- `is_git_repo = true` у проекта ([`candidates.rs`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/scan/candidates.rs));
- `stack.markers` пополняется именем `.git`;
- **язык и фреймворки не меняются** (`.git` не несёт языкового сигнала).

::: tip
`.git` находится в списке пропускаемых каталогов, но это не мешает: маркер распознаётся по содержимому **родительской** директории, а не через обход самого `.git`.
:::

## 2. Секреты (dig)

### 2.1 Уровни риска

[`SensitiveRisk`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/domain/risk.rs), упорядочены `Critical > High > Medium > Low`, в JSON — PascalCase:

| Уровень | Sense |
|---------|-------|
| `Low` | Информационный / низкая уверенность |
| `Medium` | Стоит проверить |
| `High` | Вероятно, секрет (умолчание для stash/deny) |
| `Critical` | Почти наверняка ключ / credential |

### 2.2 По имени файла

Полная таблица [`DEFAULT_FILENAME_PATTERNS`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/secrets/filename.rs) — **28 строк**, все id уникальны. Сопоставление по `file_name()` без чтения содержимого, без regex/glob; kind-ы: `Exact` / `Prefix` / `Suffix` / `Contains` (все case-sensitive).

**Окружение (env):**

| id | pattern | kind | risk | label |
|----|---------|------|------|-------|
| `env_file` | `.env` | Exact | High | Environment file |
| `env_local` | `.env.local` | Exact | High | Environment file (local) |
| `env_prod` | `.env.production` | Exact | Critical | Environment file (production) |
| `env_prefix` | `.env.` | Prefix | High | Environment file (prefixed) |

**SSH / приватные ключи:**

| id | pattern | kind | risk | label |
|----|---------|------|------|-------|
| `id_rsa` | `id_rsa` | Exact | Critical | SSH private key (RSA) |
| `id_ed25519` | `id_ed25519` | Exact | Critical | SSH private key (Ed25519) |
| `id_ecdsa` | `id_ecdsa` | Exact | Critical | SSH private key (ECDSA) |
| `private_key_pem` | `.pem` | Suffix | High | Private key (PEM) |
| `private_key_key` | `.key` | Suffix | High | Private key |
| `ppk` | `.ppk` | Suffix | High | PuTTY private key |

**Keystore / хранилища ключей:**

| id | pattern | kind | risk | label |
|----|---------|------|------|-------|
| `p12` | `.p12` | Suffix | High | PKCS#12 keystore |
| `pfx` | `.pfx` | Suffix | High | PKCS#12 certificate store |
| `keystore` | `.jks` | Suffix | High | Java keystore (JKS) |

**Credentials / сервис-аккаунты:**

| id | pattern | kind | risk | label |
|----|---------|------|------|-------|
| `aws_credentials` | `credentials` | Exact | High | AWS credentials |
| `aws_credentials_path` | `credentials` | Exact | High | AWS credentials |
| `service_account` | `service-account` | Contains | High | Service account |
| `google_sa` | `-sa.json` | Suffix | High | Google service account |
| `git_credentials` | `.git-credentials` | Exact | Critical | Git credentials |
| `netrc` | `.netrc` | Exact | High | netrc credentials |
| `htpasswd` | `.htpasswd` | Exact | High | htpasswd credentials |

::: info
Строки `aws_credentials` и `aws_credentials_path` — **одинаковый** паттерн `credentials` (Exact, High), два разных id по историческим причинам. Оба попадают в таблицу и сохраняются в отчётах.
:::

**Конфиги реестров / Kubernetes:**

| id | pattern | kind | risk | label |
|----|---------|------|------|-------|
| `kubeconfig` | `kubeconfig` | Exact | High | Kubernetes kubeconfig |
| `docker_config` | `config.json` | Exact | Medium | Docker config |
| `npmrc` | `.npmrc` | Exact | High | npm registry config |
| `pypirc` | `.pypirc` | Exact | High | PyPI registry config |

**Файлы секретов / кошелёк:**

| id | pattern | kind | risk | label |
|----|---------|------|------|-------|
| `secrets_json` | `secrets.json` | Exact | High | Secrets file (JSON) |
| `secrets_yaml` | `secrets.yaml` | Exact | High | Secrets file (YAML) |
| `secrets_yml` | `secrets.yml` | Exact | High | Secrets file (YAML) |
| `wallet` | `wallet.dat` | Contains | Critical | Wallet data |

См. также [Основные понятия → по имени файла](/concepts#по-имени-файла).

### 2.3 По содержимому

Полная таблица [`DEFAULT_CONTENT_MARKERS`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/secrets/content.rs) — **12 строк**. Все помечены `text_only: true`. `Prefix` — токен начинается с префикса (расширяется ASCII alnum/`-`/`_`); `Regex` — компилируется один раз при старте.

| id | kind | Что ищет | risk | label |
|----|------|----------|------|-------|
| `aws_access_key` | Prefix | токен `AKIA…` | Critical | AWS access key |
| `aws_secret_assign` | Regex | присваивание `aws_secret_access_key = …` | Critical | AWS secret access key assignment |
| `generic_api_key_assign` | Regex | присваивание `api_key` / `apikey = …` (≥16 символов значения) | High | API key assignment |
| `generic_secret_assign` | Regex | присваивание `secret` / `password` / `passwd` / `token = …` (≥8 символов) | High | Secret assignment |
| `private_key_header` | Regex | заголовок `-----BEGIN … PRIVATE KEY-----` (RSA / EC / DSA / OPENSSH / ENCRYPTED) | Critical | Private key (PEM header) |
| `github_pat` | Prefix | токен `ghp_…` | Critical | GitHub personal access token |
| `github_oauth` | Prefix | токен `gho_…` | Critical | GitHub OAuth token |
| `slack_token` | Prefix | токен `xoxb-…` | High | Slack token |
| `stripe_live` | Prefix | ключ `sk_live_…` | Critical | Stripe live key |
| `stripe_test` | Prefix | ключ `sk_test_…` | Medium | Stripe test key |
| `connection_string` | Regex | строка подключения `postgres://user:pass@…`, `mysql://…`, `mongodb://…` (присутствие `:` и `@`) | Critical | Database connection string |
| `jwt_like` | Regex | JWT-подобный токен `eyJ…eyJ….…` | Medium | JWT-like token |

См. также [Основные понятия → по содержимому](/concepts#по-содержимому).

### 2.4 Лимиты content-scan

[`ContentScanLimits`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/secrets/content.rs) (значения по умолчанию):

| Параметр | По умолчанию |
|----------|--------------|
| `max_file_bytes` | **1 MiB** (1_048_576) — файлы больше пропускаются |
| `max_read_bytes` | = `max_file_bytes` (максимум читается на файл) |
| `skip_binary` | `true` — пропуск по нулевому байту в первых **8 KiB** |

Поведение `scan_file_content`:

- Пустой файл / файл больше `max_file_bytes` → пропуск (не ошибка).
- Бинарный пропуск: первый min(8 KiB, len) читается и проверяется на `0x00`; найден — файл пропускается целиком (поэтому `text_only` маркеры никогда не срабатывают на бинарниках).
- Сканирование **построчное**, номера строк **1-байтовые** (начинаются с 1), `Read::take(max_read_bytes)` + lossy UTF-8.
- Результаты детерминированы: строки по возрастанию, маркеры — в порядке таблицы.

::: warning
Raw-значения секретов никогда не попадают в JSON и отчёты. Каждое значение маскируется (`mask_secret`): `scan_file_content` возвращает `MaskedValue` — **маскированное превью `AAAA…99`, blake3-хеш и длину**, но не сам секрет.
:::

### 2.5 Merge filename + content

[`scan_secrets`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/secrets/scan.rs) за один обход объединяет оба источника по пути:

- риск = fold `upgrade_risk` (только **максимум**, никогда не понижается) по всем сохранённым источникам;
- `sources` = filename-совпадения (порядок таблицы), затем content-хиты (порядок строк/маркеров); `labels` согласованы;
- `content_match` = маскированное значение самого рискованного content-хита;
- один путь может быть найден и по имени, и по содержимому — будут оба источника;

### 2.6 CLI

```
racc dig [--project PATH] [--no-content] [--repeated]
         [--fail-on ignore|critical|high] [--max-depth N]
```

- `--no-content` — только по именам файлов (содержимое не читается);
- `--repeated` — агрегация повторяющихся значений по blake3-хешу (только те, что в ≥2 файлах);
- `--fail-on` — политика выхода: `ignore` (никогда), `critical` (по умолчанию), `high`;
- **exit code `2` — только у `dig`** (найдены секреты выше порога политики); `sniff`/`pack` используют `0`/`1`.

## 3. Пропуск каталогов (SkipPolicy)

[`DEFAULT_DIR_NAMES`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/scan/skip.rs) — **18** имён; `*.egg-info` — суффиксное совпадение, остальные — точное.

| Категория | Имена |
|-----------|-------|
| Зависимости | `node_modules` |
| Build trash | `target`, `dist`, `build` |
| VCS | `.git`, `.svn`, `.hg` |
| Python | `__pycache__`, `*.egg-info` |
| Виртуальные окружения | `.venv`, `venv`, `.tox` |
| Кэши | `.mypy_cache`, `.pytest_cache`, `.cache` |
| IDE | `.idea`, `.vscode` |
| Den | `.raccpack` |

Прочее:

- `skip_hidden_dirs` — **opt-in** через `SkipPolicy::with_skip_hidden_dirs(true)`; по умолчанию выключен (`default_scan()`). При включении пропускаются любые имена, начинающиеся с `.`.
- Порядок reasons детерминирован: DefaultDirName → CustomPattern → Hidden.
- **Корневая директория сама никогда не пропускается по имени**, но если корень — скрытый каталог (начинается с `.`) и включён `skip_hidden_dirs`, фильтр применяется и к корню: **обход выдаёт пустой результат** (это подтверждено тестом `walk_skips_hidden_dirs_when_enabled` — он обходит не-hidden субкаталог как корень).

## 4. Упаковка (pack) и deny

Проверено по [`archive/deny.rs`](https://github.com/y-tretyakov/raccpack/tree/dev/crates/raccpack-core/src/archive/deny.rs) и фасаду `app/pack.rs`:

- **Name deny — всегда**: файл исключается из архива, если его имя даёт риск **≥ High** (`should_deny_file_in_pack`, порог фиксирован). Medium (`config.json`) и ниже — не исключаются.
- **Content deny — включено по умолчанию** (фасад `PackOptions::deny_content_secrets: true`), порог **Critical**; `--no-content-deny` отключает **только** content denial. Не читаемые файлы fail-closed (в архив не попадают).
- **Symlink** — не следуются и не архивируются (INVARIANT, `follow_links(false)` + явная классификация `file_type()`).
- **SkipPolicy** применяется при обходе (все 18 имён + опциональная скрытая политика).
- **Архив = содержимое project root**, а не обёртка с именем папки (`src/main.rs`, не `proj/src/main.rs`).
- **Формат**: tar + zstd (`tar.zst`, уровень по умолчанию 3); путь в den — `packs/{yyyy}/{mm}/{slug}__{ts}.tar.zst`; имя `{name}.tar.zst` при `--output-name`.
- Пустые директории и `output` внутри `source` не поддерживаются; записи сортируются по именам (детерминированный архив).

## 5. Что пока не поддерживается (честно)

На MVP 0.1.0 **нет**:

- `stash` (age-шифрование секретов), `rinse` (очистка мусора), `raid` (полный цикл) — планируется в Alpha;
- **кастомные marker/secret наборы из конфига** — в `RaccConfig` только `[paths]` и `[scanner]`; `extra_markers` в коде (`CandidateOptions`) есть, но из конфига/CLI **не выставляется**;
- глубокий парсинг `package.json` / `Cargo.toml` (workspace, зависимости → фреймворки). В частности, **Axum** для Rust отложен (нужен разбор `Cargo.toml`) — см. комментарий в `detect/rust.rs`;
- content-маркер **`telegram_bot`** из спеки — **осознанно отложен**: шумный и требует ограничения длины значения; вернётся с юнит-тестами, когда matcher поддержит лимиты длины;
- **Windows-специфика путей** (тесты/поведение заточены под POSIX; архивные имена собираются POSIX-стилем через `/`);
- сторонние rule-pack-плагины;
- энтропийные heuristics (`DigOptions.use_heuristics` **принят в API, но не используется**, просто игнорируется).

## 6. Как расширять (для контрибьюторов)

Модульность обязательна (см. dev-документы `raccpack-markers-detect-modularity.md` и `raccpack-modularity.md` в корне репозитория):

- **Новый язык** → `scan/markers/<eco>.rs` (набор `MarkerDef`) + `detect/<eco>.rs` (детектор фреймворков) + **одна строка** в реестре `scan/markers/mod.rs` и `detect/mod.rs`. Комментарии в коде называют Axum-детектор и telegram-маркер как цель будущих PR.
- **Новый filename-secret** → одна строка в `secrets/filename.rs` (+ тесты: таблица `table_has_expected_row_count` и уникальность id обновляются).
- **Новый content-маркер** → одна строка в `secrets/content.rs` (+ тест порядка `table_has_expected_rows_in_order`).

::: warning
Обновите **эту страницу в том же PR**, что и код: числа в 1.1 (14 маркеров), 2.2 (28), 2.3 (12), 3 (18) и таблицы фреймворков должны точно отражать код.
:::

## Дальнейшее чтение

- [Основные понятия](/concepts) — риски, маскирование, den.
- [Facade API](/facade-api) — публичный контракт ядра.
- [Дорожная карта](/roadmap) — что появится в Alpha и дальше.