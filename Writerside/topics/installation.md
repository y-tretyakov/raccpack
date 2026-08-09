# Установка

## Требования

- **Linux** — основная поддерживаемая платформа (macOS и Windows работают через те же механизмы Rust, но проверяются «best-effort»).
- **Rust toolchain** версии **1.75+** для сборки из исходников.
- Для сборки: компилятор `cargo` и `rustc`.

> **Note** Сборка из исходников пока единственный способ установки: релизные бинарники и системные пакеты появятся на этапе 1.0.0.

## Сборка из исходников

Склонируйте репозиторий и соберите workspace:

```bash
git clone https://github.com/y-tretyakov/raccpack.git
cd raccpack

# Сборка всего workspace (core + CLI)
cargo build --release
```

Бинарник появится в `target/release/racc`. Можно установить его в системный каталог:

```bash
install -m 0755 target/release/racc ~/.local/bin/racc
```

Проверьте установку:

```bash
racc --help
racc --version
```

Если команда не находится — убедитесь, что каталог установки (`~/.local/bin`) добавлен в `PATH`.

## Версии интерфейсов

Raccpack поставляется с тремя интерфейсами. Сейчас доступен только CLI.

| Интерфейс | Бинарник | Статус |
|-----------|----------|--------|
| CLI | `racc` | Доступен (MVP) |
| TUI | `racc-tui` | Планируется (Beta, 0.5.x) |
| Desktop | `raccpack` (Tauri) | Планируется (Beta, 0.5.x) |

> **Note** TUI и Desktop находятся в разработке. Их установка будет описана здесь, когда появятся первые сборки. Целевое поведение интерфейсов — в разделах [TUI](tui-usage.md) и [Desktop](desktop-usage.md).

## Проверка окружения

Создайте минимальную конфигурацию и убедитесь, что `racc` видит ваши проекты:

```bash
mkdir -p ~/.config/raccpack
cat > ~/.config/raccpack/config.toml <<'EOF'
[paths]
scan_root = "~/DEV/PROJS"
den_dir = "~/.raccpack/den"
EOF

racc sniff
```

Подробнее о настройках — в разделе [Конфигурация](configuration.md).

## Дальнейшие шаги

- [Быстрый старт](quick-start.md) — первый прогон за пять минут.
