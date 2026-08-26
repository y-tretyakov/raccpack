<p align="center">
  <img src="RaccPack.webp" alt="raccpack" width="435"/>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.85-orange?style=flat-square&logo=rust" alt="Rust"/></a>
  <a href="https://doc.rust-lang.org/cargo/"><img src="https://img.shields.io/badge/Cargo-workspace-blue?style=flat-square&logo=cargo" alt="Cargo"/></a>
  <a href="Cargo.toml"><img src="https://img.shields.io/badge/version-0.4.0-blue?style=flat-square" alt="version"/></a>
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

**Документация:** [https://y-tretyakov.github.io/raccpack/](https://y-tretyakov.github.io/raccpack/)

---

## Установка

Скачайте последний релиз для вашей системы с [GitHub Releases](https://github.com/y-tretyakov/raccpack/releases/latest).

**Debian / Ubuntu:**
```bash
sudo dpkg -i raccpack-0.4.0-1-amd64.deb
```

**Fedora / RHEL / Rocky:**
```bash
sudo rpm -i raccpack-0.4.0-1.x86_64.rpm
```

**Arch Linux / Manjaro:**
```bash
sudo pacman -U raccpack-0.4.0-1-x86_64.pkg.tar.zst
```

**Любая Linux (musl, универсальный):**
```bash
tar --zstd -xf raccpack-0.4.0-linux-x86_64.tar.zst
sudo cp raccpack-0.4.0/racc /usr/local/bin/
```

**Из исходников:**
```bash
cargo install raccpack-cli
```

Пакеты для ARM64 доступны во всех форматах.

---

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

---

## Команды

| Команда | Что делает |
|---------|-----------|
| `racc sniff` | Находит проекты по маркерам языков и фреймворков |
| `racc dig` | Ищет секреты (имена файлов + содержимое) |
| `racc pack` | Архивирует проект в `tar.zst` в вашем den |
| `racc stash` | Шифрует секреты через age и сохраняет в den |
| `racc rinse` | Удаляет директории артефактов сборки |
| `racc raid` | Запускает stash → rinse → pack за один раз |
| `racc init` | Создаёт конфигурацию по умолчанию |

Полный справочник: [wiki / CLI](https://y-tretyakov.github.io/raccpack/cli-usage.html)

---

## Что поддерживается

- **14 маркеров проектов:** `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `setup.py`, `requirements.txt`, `pom.xml`, `build.gradle`, `build.gradle.kts`, `Gemfile`, `composer.json`, `CMakeLists.txt`, `Makefile`, `.git`
- **28 паттернов имён секретов:** семейство `.env`, SSH-ключи, хранилища ключей, реестры, JSON-сервисных аккаунтов и т.д.
- **12 маркеров содержимого:** ключи AWS, токены GitHub, Slack/Stripe, PEM-заголовки, строки подключения, JWT, универсальные `api_key` / `secret`
- **6 стратегий очистки:** `rust`, `node`, `python` (по умолчанию) + опционально `jvm`, `go`, `generic`

Полная таблица: [wiki / Что поддерживается](https://y-tretyakov.github.io/raccpack/supported.html)

---

## Структура den

```
~/.raccpack/den/
├── packs/2026/08/     # tar.zst архивы проектов
├── secrets/2026/08/   # age-зашифрованные архивы секретов
├── manifests/2026/08/ # манифесты операций
└── staging/            # временные файлы (можно удалять)
```

Не коммитьте den в git. Храните пароли оффлайн.

---

## Сборка и тесты

```bash
cargo build
cargo test -p raccpack-core
cargo fmt --check
cargo clippy -p raccpack-core --all-targets -- -D warnings
```

---

## Лицензия

Двойная лицензия: [Apache License, Version 2.0](LICENSE-APACHE) или [MIT license](LICENSE-MIT) на ваш выбор.
