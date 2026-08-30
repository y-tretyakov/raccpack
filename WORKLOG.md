# WORKLOG — raccpack

Журнал **текущей** вехи. Orchestrator: y-tretyakov.

| Архив | Путь |
|-------|------|
| MVP | [`docs/archive/WORKLOG_MVP.md`](docs/archive/WORKLOG_MVP.md) |
| Alpha (A1–A4) | [`docs/archive/WORKLOG_ALPHA.md`](docs/archive/WORKLOG_ALPHA.md) |
| **Detect v2 (D1–D4)** | [`docs/archive/WORKLOG_DETECT.md`](docs/archive/WORKLOG_DETECT.md) |
| Версии / roadmap | [`docs/VERSION_ROADMAP.md`](docs/VERSION_ROADMAP.md) |

---

## Текущий статус

| | |
|--|--|
| **Версия** | **`0.4.5`** |
| **Веха** | Detect v2 ✅ CLOSED · **Beta B1.4 done (0.4.4)** · **B1-V2 Visual System 2.0 done (0.4.5)** · Beta → 0.5.0 |
| **Этап** | **B1.5** — TUI reveal modal (→ 0.4.6) |
| **Предыдущее** | B1-V2 (V2-A…F) TUI Visual System 2.0 closed (0.4.5) |

```text
MVP 0.1.0 ✅ → Alpha 0.3.0 ✅ → Detect v2 0.4.0 ✅ → Beta 0.5.0 → RC 0.9.0 → 1.0.0
```

---

## Backlog (Beta → 0.5.0)

Кратко (детали — `docs/VERSION_ROADMAP.md` / roadmap-v1):

```
[x] B1.1 TUI skeleton (0.4.1)
[x] B1.2  TUI sniff screen (0.4.2)
[x] B1.2.3 Design tokens source of truth (DTCG), нет bump
[x] B1.2.4 Sidebar-space token в token-const, нет bump
[x] B1.2.5 Detail strip (detail-height 7, git-маркеры, empty placeholder `·`), нет bump
[x] B1.3  TUI dig screen (0.4.3)
[x] B1.4  TUI raid + progress (0.4.4)
[x] B1-V2 TUI Visual System 2.0 (0.4.5): theme tokens, shell+badges, overview, projects Cards, activity, polish+split
[ ] B1.5  TUI reveal modal (→ 0.4.6)
[ ] B2  Desktop (Tauri + React) + BFF + ephemeral reveal
[ ] B3  Security hardening + Safe Reveal contract
[ ] B4  Productization (den gc, parallel sniff, docs) → Beta exit 0.5.0
```

Спеки TUI: `docs/raccpack-tui-spec-*.md` (уточнять по мере B1).

---

## Открытые follow-ups (не блокеры B1)

Перенесены с Alpha/Detect; полный список и история — в архивах WORKLOG.

| ID | Суть | Горизонт |
|----|------|----------|
| F-SKIP-1 | единый skip ↔ cleanup | B3 |
| F-PACK-SIZE / F-ATOMIC-SIZE / F-TEST-SIZE / F-CLI-SIZE | файлы ≳400 строк | next touch |
| P2-7 | сужение public API | R1 |
| OS-WIN | Windows paths best-effort | R2 |

---

## Решения (живые)

| Дата | Решение |
|------|---------|
| 2026-08-19/20 | Raid default **Atomic**; FailFast = debug |
| 2026-08-20 | Manifest только после successful Atomic commit |
| 2026-08-20 | CLI raid: exit **1** при `!success` |
| 2026-08-21 | MSRV **1.85**; логи → stderr; never log passphrase/raw |
| 2026-08-22 | Detect v2 = отдельная веха **0.4.0** (закрыта) |
| 2026-08-26 | **Один продукт на PR.** Crate только под roadmap raccpack. |
| 2026-08-30 | Raid = **modal-overlay workflow** (не отдельный ViewId); passphrase — native-модалка 2 ввода (zeroize, redacted Debug, env-шорткат `RACCPACK_PASSPHRASE`); **Esc в Running не отменяет** (core без cancel) — блокирует до результата |

---

## Инцидент (кратко)

**2026-08-26 — synthrodex-tui contamination**

В PR #103 вместе с `raccpack-tui` попал чужой crate `synthrodex-tui` (X11/Rofi/NowBar) — не продукт репозитория.  
**Fix:** crate удалён, workspace очищен (`rg synthrodex` → 0).  
**Правило:** не смешивать посторонние продукты в workspace raccpack.

---

## Этапы (Beta)

### 2026-08-30 — B1-V2 (V2-A…F) — TUI Visual System 2.0 ✅ CLOSED (0.4.5)

- **Ветки:** `v2-a-theme` … `v2-f-polish` (PR #114…#119 → `dev`, squash). **Версия:** 0.4.5 (единственный bump всей фазы в V2-F; V2-A…E — «нет bump»).
- **DoD по V2-A (theme):** graphite+orange палитра (DTCG v0.2.0, 14 primitive + semantic surface-raised/info/analysis + git алиасы); teal удалён → `FOCUS`/`BRAND_PRIMARY` = `#FF8A3D`, `SELECTION→surface_raised`, `ACCENT_DIM` удалён; `src/theme/{mod,primitive,semantic,intent}.rs`, `ui/theme.rs` удалён; ban-тест на teal в primitive.rs.
- **DoD по V2-B (shell):** `ui/widgets/sidebar.rs` — brand `◈ RACCPACK` + workspace, nav с live-badges (Projects = count, Findings = WARNING если >0, `·` до dig), версия снизу; header = brand + root + версия; activity-слот хук `main_split` (`ACTIVITY_WIDTH=0` до V2-E); footer без изменений.
- **DoD по V2-C (overview):** `ui/screens/overview.rs` — KPI strip (projects/Rust/JS-TS/size/git из реальных counts), recent cards (PRIMARY→SECONDARY→STATE→METADATA, `·` для пустого), health (`✓ detection READY`, `(cache)`, подсказки до скана — не blank stub); **`format_bytes` извлечён в `ui/widgets/mod.rs`** (единый источник для sniff + kpi + cards).
- **DoD по V2-D (projects):** `ProjectsMode::{Cards,Table,Tree}`, **Cards default**, `v` циклит; общая селекция j/k/g/G/Enter/R инвариантна между режимами; `projects_cards.rs` (grid, brand-border) + `projects_tree.rs` (stub hint); content-флаг `c` сохранён; help документирует `v`.
- **DoD по V2-E (activity):** `app/activity.rs` (ActivityLog cap 32, newest-first, `(n)`) + `app/activity_feed.rs` + `ui/widgets/activity.rs` (глифы `✔/!/✖/·`, NO_COLOR-safe); источники: Progress (троттл ≥10 п.п./сменa текста), SniffDone/DigDone (ok/err, findings>0 → warn), старты r/dig; панель только ≥120 колонок content (28 ширина), 80×24 скрыта; raid-фазы не дублируются; секреты/пароли не попадают. Rework: event.rs 459→423 (split activity_feed, попытка 1).
- **DoD по V2-F (polish + hygiene):** app.rs 1267→**171** (split: `app/sniff.rs`, `app/keys.rs`, тесты в `app/tests/*`), `resize_smoke_test.rs` (все view × modes × `[80×24,120×30,160×40,40×12]` без паники), TerminalGuard нетронут + test `drop_never_panics`, help c `v`; DAG-panel — follow-up (данных per-repo нет).
- **Сквозная верификация:** `cargo test --workspace` green (прогоны по merge-ready tip каждого этапа; у V2-E rework 459→423), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` — все зелёные; CI зелёный по каждому PR.
- **Файлы (суммарно):** `src/theme/{mod,primitive,semantic,intent}.rs`, `ui/widgets/{sidebar,kpi_strip,project_card,activity}.rs`, `ui/screens/{overview,projects_cards,projects_tree}.rs`, `app.rs` + `app/{sniff,keys,tests/*,activity,activity_feed}.rs`, `event.rs`, `ui/layout.rs`, `ui/screens/{mod,sniff,help}.rs`, `ui/widgets/mod.rs`, `docs/design-tokens/raccpack.tokens.json` (v0.2.0), Cargo.toml/lock, README.md/ru, `docs/VERSION_ROADMAP.md`, `docs/raccpack-roadmap-v1.md`, wiki (tui-usage/roadmap/introduction).
- **Решения:** вся фаза A…F по одному плану (пользовательское да); единственный bump 0.4.5 в V2-F (прецедент B1.2.3–5); B1.5 reveal — **после** визуальной фазы (→ 0.4.6); палитра графит+оранж, teal запрещён; sidebar-ширина и glyph `●/·/·` не менялись; activity — только широкие терминалы (80×24 не деградирует); modules ≤450 строк (event.rs/app.rs после сплитов).
- **Follow-up:** DAG-panel (когда появятся per-repo stack/graph данные); findings-badge на project-card (нужен роутинг dig_state в render — отложено); Tree-view полный (не stub); `--no-activity-close` (широкий терминал); `spawn_bridged_worker` ×2 в integration — вынести `tests/common/mod.rs` при 3-й копии; wiki-раздел про клавиши визуальной фазы (частично в этой записи — tui-usage обновлён).

### 2026-08-30 — B1.4 — TUI raid (Atomic) + progress ✅ CLOSED

- **Ветка:** `b1.4-tui-raid` (PR #113 → `dev`, squash)
- **Версия:** 0.4.4
- **DoD:**
  - [x] `raid()` вызывается только из worker (`worker/raid.rs`); TUI не дублирует логику фаз
  - [x] Preview = `RunMode::DryRun` (+ placeholder identity) — **ничего не пишет** в den (интеграция: temp den пуст после preview, и ден-дир вообще не создаётся)
  - [x] Проверить Passphrase: **Zeroizing** везде (`PassphraseInput`, `RaidFlow.confirmed_passphrase`, `WorkerPassphrase`), **redacted Debug** в App/RaidFlow/PassphraseInput/RaidCommand/WorkerMsg; 2 ввода + mismatch reset; env-шорткат `RACCPACK_PASSPHRASE` (без модалки); Drop после use
  - [x] Running: пайплайн STASH/RINSE/PACK/MOVE из реальных `ProgressEvent` (OperationKind::Raid через `RaidProgressSink`), ✓/→/○, `overall_percent` bar — без выдуманных процентов
  - [x] Result honesty: Success / Rolled back (+count warnings) / Failed; FailFast → "артефакты могут остаться"; артефакты den-relative (≤5 + n more)
  - [x] Модалка: `R` на проекте (Projects) → Preparing → Preview (проект · бейдж ATOMIC/FAIL-FAST · фазы · keep/skip · dry-run) → `y` confirm / `n`+Esc cancel / `K`/`S`/`m` toggle; Esc в Running **не** отменяет; `q` заблокирован в flow; Enter/Esc закрывает Done/Failed
  - [x] Модульность: `app/raid/{mod,passphrase,tests}.rs` + `worker/{mod,raid,tests}.rs` (worker.rs → директория), `ui/screens/raid.rs` (render-only), `centered_rect` вынесен в `ui/widgets/mod.rs` (help переиспользует), footer raid-status, help документирует R/K/S/m
  - [x] Тесты: unit (state machine, passphrase 2-ввод/mismatch/backspace/redacted, on_progress pipeline/unknown-phase/skip-stash), worker (preview no-write, commit missing→Err, Debug-redacted, RaidProgressSink filter), integration `tests/raid_flow_test.rs` (preview no-write, commit → ровно 1 `.age` + 1 pack + 1 manifest, no plaintext/raw в den, no passphrase в событиях, keep_sources/skip_stash, progress op=Raid)
  - [x] `cargo test --workspace` green, `cargo fmt --check` ok, `cargo clippy --workspace --all-targets -- -D warnings` ok
- **Файлы:** Cargo.toml, Cargo.lock, app.rs (Command Raid*, raid-keys, guard help_visible), app/raid/{mod,passphrase,tests}.rs (created), event.rs (start_raid_preview/send_raid_run/resolve_raid_passphrase, RaidProgressDone routing), worker.rs → worker/{mod,raid,tests}.rs (created), ui/layout.rs, ui/screens/{mod,raid,help}.rs (raid created), ui/widgets/mod.rs (+`centered_rect`), tests/raid_flow_test.rs (created)
- **Решения:** raid = modal overlay поверх текущего view (не ViewId — совпадает со спекой new/b1.6 "workflow, не секция"); passphrase из env → native-модалка 2 ввода (mismatch → сброс/error), подтверждённая passphrase живёт в flow, не в Command (Command остаётся Copy); skip_stash → placeholder identity; min_risk=High, content-deny on (как CLI); preview переиспользует тот же worker-путь, что и commit
- **Follow-up:** `app.rs` линейно растёт (state + routing + tests) → кандидат на split (routing/keys в отдельный модуль); reveal `v` (B1.5) + food-цепочка stash/rinse/pack прочих экранов; тест-дублирование `spawn_bridged_worker` в integration-тестах (2 копии) — вынести `tests/common/mod.rs` при появлении третьей

### 2026-08-29 — B1.3 — TUI dig screen (Findings) ✅ CLOSED

- **Ветка:** `b1.3-tui-dig-screen` (PR #112 → `dev`, squash)
- **Версия:** 0.4.3
- **DoD:**
  - [x] Worker `Dig` (пакет в одном треде с Sniff): `WorkerMsg::Dig { project, den_dir, scan_content }` → `WorkerEvent::DigDone(Result<DigResult, Error>)`; реальный core `dig` (read-only, DryRun)
  - [x] Enter на выбранном проекте (Projects, Focus::Main) → dig; sidebar Enter на Projects по-прежнему фокусирует Main
  - [x] Экран Findings: loading (progress %) / error / no-scope / empty (0 findings или фильтр вырезал всё) / таблица Risk·Path·Kind·Git
  - [x] **Masked-only (DoD security):** `FindingRow` не хранит `content_match`/masked (module-doc контракт) — raw и masked не пересекают core→UI; интеграционный тест пинает это напрямую (fixture `.env` + AWS content)
  - [x] Risk-цвета (Critical=DANGER, High=WARNING, Medium=FG, Low=MUTED), git-глифы `●`/`·`, empty `·`; shared detail strip (compact: Risk/Path/Kind/Git/Project) через `widgets/detail.rs`
  - [x] `f` — cycle min-risk (all → critical → high+ → medium+), `c` — toggle content-scan + re-dig, `r` — re-dig, `Esc` — обратно в Projects (освобождает scope); j/k/g/G — навигация по строкам Findings
  - [x] Progress events роутится по `OperationKind::{Sniff,Dig}` (другие операции — заглушка до B1.4)
  - [x] footer dig-status (findings · filter · content on/off · project); help обновлён
  - [x] Тесты: unit app/dig.rs (map без content_match, sort risk+path, filter/clamp/leave), app.rs key-flow (Enter/Esc/f/c/r, sidebar-anchored guards), worker DigDone fixture, `tests/dig_screen_test.rs` (bridge + no-leak + no-content-scan + missing-root)
  - [x] `cargo test --workspace` green, `cargo fmt --check` ok, `cargo clippy -p raccpack-tui --all-targets -- -D warnings` ok
- **Файлы:** worker.rs, app.rs, app/dig.rs (created), event.rs, ui/widgets/{mod,detail}.rs (created), ui/mod.rs, ui/theme.rs, ui/layout.rs, ui/screens/{mod,dig,help,sniff}.rs, tests/dig_screen_test.rs (created)
- **Решения:** единственный scope dig = выбранный проект (dig всего scan_root — не в B1.3); `leave()` близкий к краю сбрасывает project/selection, но сохраняет результаты для возврата Tab; filter применяется к видимой копии (all_findings хранит канон); `v` reveal — B1.5, здесь явно не вводится
- **Follow-up:** B1.4 raid+progress приконнектит `OperationKind::Stash//Rinse//Pack//Raid` к роутингу progress; reveal `v` (B1.5) получит доступ к маскированным preview уже без правки контракта строк.

### 2026-08-29 — B1.2.4 / B1.2.5 — sidebar-space token + detail strip ✅ CLOSED (no bump)

- **Ветка:** `b1.3-tui-dig-screen` (закрыт вместе с B1.3 тем же PR)
- **Версия:** без bump (полировка B1.2, закрыто вместе с B1.3 в PR #112)
- **DoD:**
  - [x] `theme.rs`: space-токены `SPACE_SIDEBAR_WIDTH/HEADER_HEIGHT/FOOTER_HEIGHT/DETAIL_HEIGHT/…_ACCENT_BAR` 1:1 с `space.semantic.*` токен-JSON + glyph/placeholder const (`●`, `·`, `·`)
  - [x] `widgets/detail.rs` — общий bordered detail strip (token-высота, `·` для пустого, muted-path) для sniff и dig
  - [x] layout.rs на токенах; sniff: git-глифы, empty `·`, selected-радар `▎`, detail strip под таблицей
  - [x] unit-тесты space/glyph контрактов (сверка с design tokens JSON)
  - [x] fmt + clippy `-D warnings`, `cargo test -p raccpack-tui` green
- **Файлы:** ui/theme.rs, ui/widgets/{mod,detail}.rs (created), ui/mod.rs, ui/layout.rs, ui/screens/sniff.rs

### 2026-08-29 — B1.2.3 — Design tokens source of truth (DTCG) ✅ CLOSED

- **Ветка:** `b1.2-tokens-adopt` (PR #111 → `dev`, squash)
- **Версия:** без bump (полировка B1.2, остаётся 0.4.2)
- **DoD:**
  - [x] `docs/design-tokens/raccpack.tokens.json` — DTCG 2025.10 source of truth: primitive → semantic → component (цвет, space, typography)
  - [x] `docs/design-tokens/README.md` — гайд: слои, таблица theme.rs↔token, правило «space в клетках», «что не делаем пока» (Style Dictionary/light-theme/CI-gen до Desktop)
  - [x] `theme.rs` — 13 semantic const, имена 1:1 с `color.semantic.*`: добавлены `ACCENT_DIM`, `GIT_CLEAN`, `GIT_DIRTY_OR_ABSENT`; существующие 10 const не тронуты
  - [x] unit-тесты theme.rs: новые const, `GIT_CLEAN==SUCCESS`, `GIT_DIRTY_OR_ABSENT==MUTED`, `ACCENT_DIM!=ACCENT`
  - [x] Нет ad-hoc hex в layout-коде (только через `theme::` const)
  - [x] `cargo test --workspace` green (49 suites, 0 failed), `cargo fmt --check` ok, `cargo clippy -p raccpack-tui/core --all-targets -- -D warnings` ok
- **Файлы:** `docs/design-tokens/raccpack.tokens.json` (created), `docs/design-tokens/README.md` (created), `crates/raccpack-tui/src/ui/theme.rs` (changed)
- **Решения:** токены = контракт между TUI и Desktop (не npm-зависимость); числовые space-значения шерим только именами (cell → px/rem); Detail-strip / sidebar numerics в токены-const — отдельный подэтап (B1.2.4), Style Dictionary — только при появлении Desktop (B2).
- **Follow-up:** B1.2.4 — перенести sidebar (23) и пространственные numerics в token-const; B1.2.5 — detail strip (detail-height 7, git-маркеры, empty placeholder `·`).

### 2026-08-29 — B1.2 — TUI sniff screen ✅ CLOSED

- **Ветка:** `b1.2-sniff-screen-fix` (PR #108 → `dev`, squash); исходный PR #107 (B1.2 sniff screen) доделан и закрыт этим фиксом
- **Версия:** 0.4.2
- **DoD:**
  - [x] Отдельный worker-поток + `WorkerMsg`/`WorkerEvent` + `TuiProgressSink` через core `ProgressSink`
  - [x] Экран проекта (loading / error / empty / table): name, language, frameworks, size, git
  - [x] Неблокирующий sniff; j/k навигация; progress %; cache indicator
  - [x] **Fix bridge:** worker `WorkerEvent` → `AppEvent::Worker` (ранее `worker_receiver` отбрасывался, события не доходили до UI)
  - [x] **Fix loading:** `set_loading(true)` до отправки `WorkerMsg::Sniff`
  - [x] Panic hook через `OnceLock`; фильтр `KeyEventKind::Press` (фиксы B1.1)
  - [x] Тесты: worker (cancel/sniff done) + event bridge + state/format_bytes + integration fixture
  - [x] `cargo test -p raccpack-tui` green (64), fmt + clippy `-D warnings` чистые
- **Файлы:** `crates/raccpack-tui/src/worker.rs`, `event.rs`, `app.rs`, `src/ui/screens/sniff.rs`, `tests/worker_bridge_test.rs`
- **Решения:** wiring через dedicated bridge thread (по образцу `event_reader`), а не select!/mux — проще при стандартном `std::sync::mpsc`.

### 2026-08-29 — B1.2.1 — TUI chrome + navigation polish (B1.2 follow-up) ✅ CLOSED

- **Ветка:** `b1.2.1-tui-chrome-nav` → `dev`
- **Версия:** без bump (полировка B1.2, остаётся 0.4.2)
- **DoD:**
  - [x] Keyboard contract §3: `Tab`/`Shift+Tab` cycle views; `j`/`k`/arrows — focus-aware (Sidebar = views, Main+Projects = rows); `h`/`l`/arrows и `Esc` переключают фокус; `1`–`4` jump; `r`/`o` (view-scoped) не зависят от фокуса
  - [x] Focus model `Focus::{Sidebar, Main}` + `ViewId::prev()` + `ALL_VIEWS` registry
  - [x] Chrome: single-line dense header (title/root/hotkeys с truncation по ширине), sidebar 23 колонки (accent bar + key hints, SELECTION при фокусе), footer left=status / right=focus·view без hardcoded spacer spaces
  - [x] Stub-экраны Overview/Findings/Operations — подсказка «press 2 or Tab for Projects»
  - [x] Help обновлён под реальную keymap; `g`/`G` first/last row
  - [x] Worker bridge, loading/error/empty состояния и sniff table сохранены
  - [x] Тесты: Tab/BackTab, sidebar j/k/arrows, focus h/l/arrows/Esc, rows только при Focus::Main, help блокирует навигацию, prev/next round-trip
  - [x] `cargo test -p raccpack-tui` green (88), `cargo test --workspace` green (1015), fmt + clippy `-D warnings` чистые
- **Файлы:** `crates/raccpack-tui/src/app.rs`, `src/ui/layout.rs`, `src/ui/screens/{mod,sniff,help}.rs`, `tests/{app_test,sniff_integration_test}.rs`
- **Решения:** Tab = next view (не focus cycle); простая модель фокуса — sidebar cursor всегда = `current_view`, `Enter` на Sidebar активирует Main; заголовок/футер рендерятся без спейсов-хакеров (два перекрывающихся Paragraph на футере).

### 2026-08-29 — B1.2.2 — TUI launch args contract: clap + pre-TTY version/help ✅ CLOSED

- **Ветка:** `b1.2.2-tui-clap-launch` (PR #110 → `dev`, squash)
- **Версия:** без bump (полировка B1.2, остаётся 0.4.2)
- **DoD:**
  - [x] clap (derive) интегрирован в `raccpack-tui`; `pub mod cli` с `Cli` (name `racc-tui`, version) — argv парсится **до** любого terminal init
  - [x] `--version`/`-V` и `--help`/`-h` работают **без** TerminalGuard (exit 0)
  - [x] Non-TTY / pipe-safe: `racc-tui requires an interactive terminal` (stderr) + exit 1 **до** raw mode (`std::io::IsTerminal` на stdin/stdout)
  - [x] Launch-флаги: `--root`, `--den`, `--view`, `--refresh` реализованы; `--config`/`-c`, `-v/--verbose` — parse+store (stub). Имена совпадают со словарём CLI (`raccpack-cli`)
  - [x] `--den` > `RACCPACK_DEN` > `~/.raccpack/den`; den передаётся в worker из `app.den_dir` (форма `WorkerMsg::Sniff` не менялась)
  - [x] `ViewArg` ↔ `ViewId` биекция; default view = Overview (существующие onvg-тесты не тронуты)
  - [x] `--refresh` → `Command::SniffRefresh` на старте loop, когда view == Projects
  - [x] `docs/launch-contract.md` — общий контракт семантики флагов TUI + future Desktop (B2)
  - [x] Тесты: cli parse unit (default/root/den/config/view/invalid/refresh/verbose + биекция) + binary `launch_args_test` (version без TTY, help, badflag, non-TTY refusal)
  - [x] `cargo test -p raccpack-tui` green (74 lib + 19 app + 5 launch + 3 sniff + 5 worker), `cargo test --workspace` green (49 suites), fmt + clippy `-D warnings` чистые; `racc-tui --version` → `racc-tui 0.4.2`, `echo test | racc-tui` → clean refusal exit 1
- **Файлы:** `crates/raccpack-tui/{Cargo.toml,src/{main,cli,lib,event,app}.rs}`, `tests/launch_args_test.rs` (created), `docs/launch-contract.md` (created), `Cargo.lock`
- **Решения:** version/help — встроенные clap short-circuit’ы (DisplayVersion/DisplayHelp), не вызываем TerminalGuard ни в одном meta-пути; den-резолвер живёт в binary main.rs (`apply_launch_args`/`resolve_den_dir`), а не в lib — чистая функция без unit-теста (покрыт flag-capture). Worker bridge (`WorkerMsg`/`WorkerEvent`) не тронут.
- **Follow-up:** вынести den-резолвер в `lib`/core как чистую `resolve_den(flag, env, default)` и покрыть unit-тестом (сейчас в binary); wiki `tui-usage` синхронизировать при B1 Docs-проходе.

### 2026-08-29 — B1.1 — TUI skeleton (`raccpack-tui`) ✅ CLOSED

- **Ветка:** `b1-tui-skeleton` (PR → `dev`, squash)
- **Версия:** 0.4.1
- **DoD:**
  - [x] Crate `raccpack-tui` в workspace
  - [x] Ratatui + crossterm: event loop, screens enum, theme
  - [x] Навигация: `1`–`4`, `?`, `q`; restore terminal on exit
  - [x] Stubs only (без реального sniff)
  - [x] Builds in workspace
  - [x] `cargo test -p raccpack-tui` (49 tests green)
- **Файлы:** `crates/raccpack-tui/` (new crate)

### 2026-08-26 — B1.x — TUI skeleton (in progress)

- **Ветка:** `b1-tui-skeleton` (или superseding clean branch)
- **Статус:** cleanup после инцидента; только `crates/raccpack-tui`
- **Не merge в `dev` без явного approval**

Когда этап закроется — одна короткая запись сюда (PR, DoD, version bump если нужен).
