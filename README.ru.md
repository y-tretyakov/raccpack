<p align="center">
  <img src="RaccPack.webp" alt="raccpack" width="435"/>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85-orange?style=flat-square&logo=rust" alt="Rust"/></a>
  <a href="https://doc.rust-lang.org/cargo/"><img src="https://img.shields.io/badge/Cargo-workspace-blue?style=flat-square&logo=cargo" alt="Cargo"/></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/version-0.4.3-blue?style=flat-square" alt="version"/></a>
  <a href="https://github.com/y-tretyakov/raccpack/actions/workflows/wiki.yml"><img src="https://img.shields.io/badge/CI-wiki-success?style=flat-square" alt="CI"/></a>
  <a href="https://github.com/y-tretyakov/raccpack/releases"><img src="https://img.shields.io/badge/OS-Linux-success?style=flat-square" alt="Linux"/></a>
  <a href="https://clap.rs"><img src="https://img.shields.io/badge/CLI-clap-ee4b2b?style=flat-square" alt="CLI"/></a>
  <a href="https://github.com/FiloSottile/age"><img src="https://img.shields.io/badge/secrets-age--encrypted-0a0a0a?style=flat-square" alt="age"/></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue?style=flat-square" alt="License"/></a>
</p>

<p align="center">
  <a href="README.md">🇬🇧 English</a>
</p>

# raccpack

CLI-инструмент для сканирования директорий с проектами, поиска секретов, очистки мусора сборки и упаковки проектов в безопасное **хранилище (den)** — архивы секретов, зашифрованные age, и сжатые пакеты проектов.

**Документация:** [https://y-tretyakov.github.io/raccpack/](https://y-tretyakov.github.io/raccpack/ru/)

## Статус

**Версия `0.4.3`** — MVP `0.1.0` закрыт; **Alpha `0.3.0` закрыта** (stash / rinse / raid / git+DX). **Detect v2 `0.4.0` закрыт** (DAG-детекторы, очистка по экосистеме, пакетный raid `racc raid --root`, wiki + E2E). **Beta B1.3: TUI-экран dig готов** (Findings — маскированные детали, фильтр риска, переключение content-скана; неблокирующий worker). Следующий: Beta `0.5.0` (TUI + Desktop).

| Команда | Статус | Роль |
|---------|--------|------|
| **sniff** | Доступна | Обнаружение проектов по маркерам, стеку, размерам, кэш |
| **dig** | Доступна | Поиск секретов (имена файлов + содержимое), маскирование, уровни риска, exit policy, git-статус |
| **pack** | Доступна | `tar.zst` в den (`packs/…`), name/content deny, DryRun по умолчанию / `--yes` |
| **stash** | Доступна (Alpha) | Age-зашифрованные архивы секретов в den (`secrets/…`), опциональное удаление исходников |
| **rinse** | Доступна (Alpha) | Очистка артефактов сборки по стратегиям (`rust`/`node`/`python` по умолчанию), DryRun / `--yes` |
| **raid** | Доступна (Alpha) | Оркестрация stash → rinse → pack → move; атомарность по умолчанию (staging + WAL + rollback), manifest JSON, `--fail-fast`, `--root` для пакетного режима |
| **init** | Доступна (Alpha) | Создание конфигурации по умолчанию (`config_version = 1`), опциональный скелет den (`--ensure-den`), `--force` |
| **TUI** | Beta (экраны sniff + dig) | Ratatui-таблица проектов, неблокирующий worker sniff/dig, Findings с маскированными деталями, фильтр риска `f`, content-переключение `c`, навигация j/k (с 0.4.2, dig с 0.4.3) |
| **Desktop** | Планируется (Beta) | Tauri + React |

Подробности и флаги: [wiki / CLI](https://y-tretyakov.github.io/raccpack/ru/cli-usage.html).

## Установка

Скачайте с [GitHub Releases](https://github.com/y-tretyakov/raccpack/releases/latest):

```bash
# Debian / Ubuntu
sudo dpkg -i raccpack-0.4.0-1-amd64.deb

# Fedora / RHEL / Rocky
sudo rpm -i raccpack-0.4.0-1.x86_64.rpm

# Arch Linux / Manjaro
sudo pacman -U raccpack-0.4.0-1-x86_64.pkg.tar.zst

# Любая Linux (musl, универсальный)
tar --zstd -xf raccpack-0.4.0-linux-x86_64.tar.zst
sudo cp raccpack-0.4.0/racc /usr/local/bin/

# Из исходников
cargo install raccpack-cli
```

Пакеты для ARM64 доступны во всех форматах.

## Быстрый старт

```bash
# Создать конфигурацию
racc init

# Найти проекты
racc sniff

# Проверить на секреты
racc dig --project ~/DEV/PROJS/my-app

# Упаковать проект
racc pack --project ~/DEV/PROJS/my-app --yes

# Зашифровать секреты
racc stash --project ~/DEV/PROJS/my-app --yes

# Удалить мусор сборки
racc rinse --project ~/DEV/PROJS/my-app --yes
```

Добавьте `--json` к любой команде для машиночитаемого вывода.

## Что поддерживается

Полные таблицы: [wiki / Что поддерживается](https://y-tretyakov.github.io/raccpack/ru/supported.html)

- **14 маркеров проектов:** `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `setup.py`, `requirements.txt`, `pom.xml`, `build.gradle`, `build.gradle.kts`, `Gemfile`, `composer.json`, `CMakeLists.txt`, `Makefile`, `.git`
- **28 паттернов имён секретов:** семейство `.env`, SSH-ключи, хранилища ключей, реестры, JSON-сервисных аккаунтов и т.д.
- **12 маркеров содержимого:** ключи AWS, токены GitHub, Slack, Stripe, PEM-заголовки, строки подключения, JWT, универсальные `api_key` / `secret`
- **6 стратегий очистки:** `rust`, `node`, `python` (по умолчанию) + опционально `jvm`, `go`, `generic`

## Структура den

```
~/.raccpack/den/
├── packs/2026/08/     # tar.zst архивы проектов
├── secrets/2026/08/   # age-зашифрованные архивы секретов
├── manifests/2026/08/ # манифесты операций
└── staging/            # временные файлы (можно удалять)
```

Не коммитьте den в git. Храните пароли оффлайн.

## Сборка и тесты

```bash
cargo build
cargo test -p raccpack-core
cargo fmt --check
cargo clippy -p raccpack-core --all-targets -- -D warnings
```

## Лицензия

Двойная лицензия: [Apache License, Version 2.0](LICENSE-APACHE) или [MIT license](LICENSE-MIT) на ваш выбор.
