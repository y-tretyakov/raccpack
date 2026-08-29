# Design tokens raccpack

Единый источник визуальных решений для **raccpack-tui** (Ratatui) и будущего **Desktop** (CSS/TS).

Файл токенов: [`raccpack.tokens.json`](./raccpack.tokens.json) — формат **DTCG** (schema `design-tokens.org/schema/2025.10/basic.json`).

## Назначение

Один файл описывает цвет, интервалы (space) и типографику приложения. Строка «текущее цветовое решение» живёт в JSON, а слой TUI (`theme.rs`) ссылается на semantic-имена из него. Так TUI и Desktop не разъезжаются: любое изменение визуала начинается с токен-файла.

## Три слоя

Токены строятся по цепочке **primitive → semantic → component**:

| Слой | Что | Пример |
|------|-----|--------|
| **primitive** | сырые палитры, без смысла | `color.primitive.teal-400` = `#56b6c2` |
| **semantic** | осмысленное назначение | `color.semantic.accent` = `{color.primitive.teal-400}` |
| **component** | привязка к конкретному виджету | `component.sidebar.item-active-bg` = `{color.semantic.selection}` |

Пример раскрытия ссылки: `component.table.row-selected-bg` → `color.semantic.selection` → `color.primitive.gray-700` → `#3b4048`.

## theme.rs ↔ token

Semantic-цвета в `crates/raccpack-tui/src/ui/theme.rs` совпадают 1:1 с `color.semantic.*`. Значение «вручную» в layout не hardcode — только через эти const.

| const в `theme.rs` | token |
|--------------------|-------|
| `BG` | `color.semantic.bg` |
| `FG` | `color.semantic.fg` |
| `MUTED` | `color.semantic.muted` |
| `ACCENT` | `color.semantic.accent` |
| `ACCENT_DIM` | `color.semantic.accent-dim` |
| `DANGER` | `color.semantic.danger` |
| `WARNING` | `color.semantic.warning` |
| `SUCCESS` | `color.semantic.success` |
| `BORDER` | `color.semantic.border` |
| `SURFACE` | `color.semantic.surface` |
| `SELECTION` | `color.semantic.selection` |
| `GIT_CLEAN` | `color.semantic.git-clean` |
| `GIT_DIRTY_OR_ABSENT` | `color.semantic.git-dirty-or-absent` |

Производные токены в JSON ссылаются на другие semantic-токены: `git-clean` = `success`, `git-dirty-or-absent` = `muted`. В Rust это выражено равенством const (`GIT_CLEAN == SUCCESS`) — единый смысл, одно место правки.

## Правило про space

`space.*` в **TUI** — это **клетки** (columns/rows): `sidebar-width` = `23` колонки. На **Desktop** те же имена мапятся в **px/rem** при трансформации, например `sidebar-width` 23 → `240px` или `15rem`.

Числовое значение **не** шерится слепо между платформами: единица измерения отличается (cell vs px). Имя токена переносится, значение пересчитывается под платформу.

## Правило для разработчика

- В layout/виджетах **не** добавляй новую hex-константу — только ссылайся на const из `theme.rs`.
- Semantic-имена в `theme.rs` и `color.semantic.*` в JSON должны совпадать **1:1**.
- Новый цвет → сначала запись в JSON (primitive + semantic), потом при необходимости const в `theme.rs`.

## Как расширять

1. Новый primitive/semantic: добавь запись в `raccpack.tokens.json` (`color.primitive.*` + `color.semantic.*`).
2. Если цвет нужен в TUI — добавь const в `theme.rs` с doc-комментарием и ссылкой на token, плюс unit-тест.
3. Для Desktop (этап B2): сгенерировать CSS-переменные `--rp-color-accent` и т.п. из этого же JSON (Style Dictionary или ручной экспорт).

## Что не делаем пока

- **Style Dictionary / npm / build-пайплайн** — не вводим до Desktop.
- **Light theme (modes DTCG)** — только один тёмный режим «Nocturnal».
- **CI-генерация Ratatui из JSON** — ручной sync `theme.rs` достаточен, пока нет Desktop.

*Значения токенов — контракт: не менять без синхронного обновления `theme.rs`, unit-тестов и (в будущем) CSS-выхода Desktop.*
