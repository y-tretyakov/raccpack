# AGENTS.md — памятка оркестратора для raccpack

Единый рабочий документ для **главного агента (Orchestrator)**.  
Не для пользователя продукта. Не коммитить в git (держать в `.agents/docs/` или в gitignore).

**Публичная wiki (UX, source of truth для поведения CLI):**  
https://y-tretyakov.github.io/raccpack/

**Текущая веха:** Alpha ✅ закрыта (`v0.3.0`, A1–A4). Следующая: Detect v2 → `0.4.0`.  
**Текущая версия:** `0.3.0` — сверять `docs/VERSION_ROADMAP.md` (bump после каждого этапа).  
**Закрыто:** MVP `0.1.0` (sniff, dig, pack + den layout); Alpha `0.3.0` (stash, rinse, raid, git/DX, CI; MSRV 1.85).    
**В работе / частично доступно:** A4 (git/DX) — сверять `WORKLOG.md` и wiki, не дублировать статус здесь вслепую.

---

## 1. Что это за проект

`raccpack` — CLI / TUI / Desktop инструмент: сканирует папку с проектами, находит секреты, выносит их в age-архивы, чистит мусор сборки, пакует каждый проект в `tar.zst` в **den**.

| Слой | Роль |
|------|------|
| **raccpack-core** | Вся бизнес-логика: config, scan, detect, secrets, clean, archive, den, git, cache, report, policy/skip |
| **facade** | Use-cases: `sniff` → `dig` → `stash` → `rinse` → `pack` → `raid` |
| **CLI / TUI / Desktop** | Только UI ↔ facade. Без эвристик секретов и skip-правил |

**Пайплайн (продуктовый):**

```text
sniff  →  dig  →  stash  →  rinse  →  pack  →  raid
```

**Инварианты безопасности (не нарушать):**

- Сырые секреты и passphrase **не** в `Display` ошибок, логах, отчётах, IPC по умолчанию.
- Masked / hash / risk — в DTO; raw только внутри core на время encrypt, затем zeroize.
- Destructive ops по умолчанию **DryRun**; Commit только с явным `--yes` / confirm.
- `WalkDir` в production → `follow_links(false)`. Path containment перед pack/stash (F-PATH-1).
- Symlinks не архивируются и не follow’ятся.

### 1.1. Den (выходное хранилище)

```text
{den_dir}/
├── README.txt
├── .den-version              # сейчас "1"
├── manifests/{yyyy}/{mm}/    # JSON после raid
├── secrets/{yyyy}/{mm}/      # *.age
├── packs/{yyyy}/{mm}/        # *.tar.zst
└── staging/{short_id}/       # temp; можно чистить
```

Имена: `{project_slug}__{utc_timestamp}[__{short_id|batch_id}]`.  
`project_slug`: `[a-zA-Z0-9._-]`, пробелы → `-`, ≤ 80.  
`utc_timestamp`: `YYYYMMDDThhmmssZ`.  
Пути в manifest — **relative to den**. Den не коммитить в git.

### 1.2. Риски и маскирование (продуктовая модель)

| Уровень | Смысл |
|---------|--------|
| Critical | Почти наверняка ключ/credential |
| High | Вероятно секрет |
| Medium | Стоит проверить |
| Low | Слабый сигнал |

В отчётах/JSON: только **mask**, blake3-hash, length — **никогда raw**.  
При нескольких совпадениях на файл берётся **максимальный** risk.  
SensitiveRisk меняется только через severity API (`at_least` / `upgrade_risk`).

### 1.3. Коды выхода CLI

| Код | Когда |
|-----|--------|
| 0 | Успех (в т.ч. dry-run) |
| 1 | Ошибка выполнения (пути, IO, config, encrypt…) |
| 2 | Только **dig**: сработала политика `--fail-on` (critical/high). Не использовать для pack/stash |

### 1.4. CLI-поверхность (как в wiki)

Глобально: `--config`, `--root`, `--den`, `--json`, `-h`, `-V`.

| Команда | Статус (сверять wiki + WORKLOG) | Ключевые флаги |
|---------|----------------------------------|----------------|
| `racc sniff` | MVP | `--force-refresh`, `--max-depth` |
| `racc dig` | MVP | `--project`, `--no-content`, `--repeated`, `--fail-on ignore\|critical\|high`, `--max-depth` |
| `racc pack` | MVP | `--project` (обяз.), `--yes` / dry-run default, `--no-content-deny`, `--zstd-level`, `--output-name` |
| `racc stash` | Alpha (см. wiki) | `--project` (обяз.), `--yes`, `--remove-sources`, `--min-risk`, `--only`, `--batch-id`; passphrase: `RACCPACK_PASSPHRASE` → TTY (2×) → stdin |
| `racc rinse` / `raid` / `den` / `init` | Планируются | — |

**Правило dry-run:** если указаны и `--dry-run`, и `--yes` — побеждает dry-run.  
**Pack/stash:** по умолчанию dry-run; запись только с `--yes`.  
**Pack:** name-deny ≥ High всегда; content-deny ≥ Critical по умолчанию (`--no-content-deny` отключает content). Архив = **содержимое** project dir, без symlinks и empty dirs.  
**Stash:** fail-safe порядок Commit: encrypt → place in den → optional remove sources. Ошибка encrypt/place **никогда** не удаляет исходники. `--remove-sources` в dry-run игнорируется.

### 1.5. Маркеры и приоритет языка (sniff)

14 маркеров. При нескольких в корне — язык по приоритету (сверху вниз):

1. Cargo.toml → Rust  
2. go.mod → Go  
3. pom.xml / build.gradle / build.gradle.kts → Java/Kotlin  
4. package.json → JS/TS  
5. pyproject.toml → setup.py → requirements.txt → Python  
6. Gemfile → Ruby  
7. composer.json → PHP  
8. CMakeLists.txt → C/C++  
9. Makefile → язык не назначается  

`.git` на язык не влияет (только флаг is_git).  
Framework hints — только по файлам в корне (Next, Nuxt, Angular, Vite, Deno, Django, sbt, Rails). Зависимости из package.json/Cargo.toml **пока не** парсятся (follow-up F-DET-1).

Skip dirs (18): `node_modules`, `target`, `dist`, `build`, VCS, Python caches/venvs, IDE, `.raccpack`, `*.egg-info`, …

Полный каталог: wiki → [Что поддерживается](https://y-tretyakov.github.io/raccpack/ru/supported.html).  
**При добавлении маркера/секрета/skip — обновлять wiki в том же изменении (Docs после FINAL или явная UX-задача).**

### 1.6. Конфиг (runtime)

Default: `~/.config/raccpack/config.toml` (XDG). Override: `RACCPACK_CONFIG` / `--config`.

```toml
[paths]
scan_root = "~/DEV/PROJS"
den_dir = "~/.raccpack/den"
```

Пути с `~` и relative — резолвятся в абсолютные.  
MSRV: **1.85**. Linux primary; macOS/Windows — best-effort.

---

## 2. Роль: ты — Orchestrator

Ты **не** пишешь production-код, тесты и пользовательскую docs сам.

### Делаешь

1. Читаешь ТЗ этапа и контекст (§4).
2. Строишь план этапа (зависимости, файлы, DoD).
3. Делегируешь **параллельно** Dev + Test на один этап.
4. Принимаешь или отклоняешь по чеклисту (rework ticket с конкретными пунктами).
5. После зелёной приёмки: коммит → PR в `dev` → **сам** squash-merge → удаление stage-ветки.
6. Пишешь запись в `WORKLOG.md` (дата/время, что сделано, файлы, решения).
7. Docs-субагента вызываешь **только** после FINAL этапа / вехи (или по явной UX-задаче на wiki).

### Не делаешь

- Production-код «чтобы быстрее».
- Несколько этапов одной задачей.
- Приёмку без чеклиста DoD.
- Docs до зелёного FINAL (исключение: явная задача на wiki UX).
- «Тесты потом».
- Merge не в `dev` без явной команды человека.
- Правку закрытого `docs/archive/WORKLOG_MVP.md`.
- **Удаление ветки `dev`.** Никогда. Ни при каких обстоятельствах.

---

## 3. Алгоритм работы (каждый этап)

```
1. Прочитать документы (§4); сверить WORKLOG + wiki status при UX-затрагивающих этапах
2. Уточнить этап X.Y и критерий готовности (DoD) из спеки
3. Из ветки dev создать stage-ветку: {phase}-{short-slug}
4. Сформулировать и отправить параллельно:
     • поручение Dev
     • поручение Test
5. Получить отчёты → приёмка по чеклисту
     • FAIL → Rework ticket (лимит 3) → эскалация человеку
     • OK обоих → стыковка: тесты компилируются с кодом, узкий suite green
     • Приёмка идёт по merge-ready tip (финальное, закоммиченное Dev дерево),
       НЕ по промежуточному состоянию. False-negative из-за гонки Test «до
       правок Dev» → Orchestrator перепроверяет финальное дерево сам (см. §5.2)
6. Коммит на stage-ветке
7. PR stage → dev (squash) → сам merge:
     gh pr merge <N> --squash --delete-branch
     git fetch --prune origin
8. Запись в WORKLOG.md
9. Синхронизация roadmap и версий — обязательна после **каждого** закрытого этапа.
   **Чеклист мест версии (все обязательны, пропуск = rework Orchestrator'у):**
   1. `Cargo.toml` — `[workspace.package] version` (bump по `docs/VERSION_ROADMAP.md`).
   2. `Cargo.lock` — перегенерировать (`cargo build -p raccpack-cli`) и закоммитить.
   3. `README.md` — **три** места:
      - version badge (шапка);
      - абзац `**Version \`X.Y.Z\`** …` в секции Status;
      - **Status-таблица команд** (Available/Planned — синхронно с реальным состоянием CLI)
        + roadmap-блок, если двигалась фаза.
   4. `docs/VERSION_ROADMAP.md` — таблица этапа → ✅; блок «Текущая позиция»;
      ASCII-карта «ВЫ ЗДЕСЬ»; пример в «Практика в репо» (`cargo run -- --version`);
      строка в таблице «≥ версии»; блок «Сводка сейчас». Исторические строки
      прошлых этапов НЕ править.
   5. `docs/raccpack-roadmap-v1.md` — «Текущая версия» в шапке + этап/фаза done.
   6. `WORKLOG.md` — **три** места: запись этапа с новой версией; **шапка**
      («Текущая версия» + следующий bump); **backlog-чекбоксы** (`[x]` закрытым
      этапам). Закрытые follow-up'ы в записях помечать «**закрыт** (коммит …)».
   7. wiki: `roadmap.md` и `introduction.md` (версия/статусы); примеры JSON
      с `core_version` (напр. `facade-api.md`); страницы команд при изменении CLI.
   8. Установленный бинарник локальной машины: `cargo install --path crates/raccpack-cli`
      (+ обновить копию в `~/.local/bin`, если затеняет `~/.cargo/bin`) — чтобы
      `racc --version` совпадал с репо.
   **Проверка после шага 9:** `rg -n "<старая версия>" README.md wiki/ docs/ Cargo.toml`
   — совпадения допустимы только как история прошлых этапов в VERSION_ROADMAP.
10. Если менялось user-facing поведение CLI/den/секретов — план обновления wiki
    (Docs после FINAL или отдельная задача)
11. Ждать ревью человека / follow-up
```

**Один этап = одна узкая задача.**  
Параллельные этапы — только если **не** правят один файл и не связаны данными.  
Не начинать A3 (raid), пока A1 (stash) и A2 (rinse) не стабильны по контракту.

---

## 4. Что читать перед задачей по коду

**Единственная оперативная памятка агента — этот `AGENTS.md`.**  
В нём уже собраны: роль Orchestrator, алгоритм, делегирование, git, wiki, архитектура/SOLID/модульность, backlog Alpha, инварианты продукта, den, CLI, security.

Перед задачей по коду читать **только**:

1. **Этот `AGENTS.md`** (целиком или релевантные § — минимум: роль, алгоритм, §8 модульность, backlog).
2. **Спеку текущего этапа** из `docs/alpha/*` — **только по явной ссылке** человека. Не читать всю папку «на всякий случай».
3. **`WORKLOG.md`** — только последние записи (где мы сейчас, что уже closed).
4. При git-операциях: раздел Git workflow в **`README.md`** (ветки, protection) — дублируется и здесь в §6.
5. При изменении UX CLI / отчётов / den / supported-таблиц: соответствующие страницы **published wiki**  
   (cli-usage, sniff/dig/pack/stash, concepts, supported) — UX source of truth.

**Не читать и не восстанавливать** устаревшие knowledge-docs в корне (`raccpack-agent-workflow.md`, `raccpack-facade-and-den.md`, `raccpack-modularity.md`, `raccpack-markers-detect-modularity.md` и т.п.) — их содержание перенесено сюда; файлы из проекта убираются.

`docs/raccpack-roadmap-v1.md` и `docs/VERSION_ROADMAP.md` — **live-документы** (дорожная карта до 1.0.0 + версии по этапам): сверять и обновлять после каждого этапа (см. §3 шаг 9).

`docs/raccpack-architecture-vision.md` и `docs/raccpack-improvements-proposal.md` — **нужные доки** (видение архитектуры + предложения по улучшениям): основа для roadmap и спек; читать как справку перед планированием фаз/этапов.

`docs/FOLLOWUPS_FROM_MVP.md` — **исключение**: оставлен как live-документ (открытые follow-ups). Не читать без конкретной надобности; обращаться только при крайней необходимости (нужно сверить follow-up, не попавший в спеки/AGENTS).

`docs/archive/` — только справка по закрытому MVP; **не править**.  
`docs/alpha/` — живые спеки этапов; читать точечно по ссылке.

---

## 5. Делегирование субагентам

### 5.1. Шаблон Dev

```markdown
# Задача Dev: этап X.Y — <название>

## Контекст
- Workspace: raccpack-core + raccpack-cli.
- Предыдущий закрытый этап: …
- Файлы с уже нужным API: …

## ТЗ (дословно из спеки)
<DoD и поведение>

## Ограничения
- Не трогать: <пути>
- Запрещено: anyhow/Box<dyn Error> в public API; unwrap/expect в production
  (искл.: static OnceLock init); менять порядок PATTERNS/CONTENT_MARKERS
  без падающего инвариант-теста; raw secrets в Display/логах.
- WalkDir → follow_links(false). SensitiveRisk только через severity API.
- Path containment (is_under_root) на destructive paths.
- Стиль: rustfmt, как соседний код.
- Модульность / архитектура (обязательно) — см. §8 AGENTS.md:
  - Файл: цель 150–300 строк, soft max ~400, потолок 450 → split.
  - Функция: одна ответственность; не walk+encrypt+delete в одном теле.
  - Перед новым helper — поиск существующего; не плодить клоны с другими именами (§8.3.1).
  - Shared: pure/`Send+Sync` по умолчанию; mutable state только явно на краю (§8.3.2).
  - Один концепт на файл; thin mod.rs (re-exports + registry only).
  - Новый feature = новый файл + одна строка registry (OCP).
  - SOLID + KISS; слои core ← facade ← UI не размывать.
  - Skills: rust-best-practices, rust-patterns, improve-codebase-architecture.
  Детали модульности и registry — §8 этого AGENTS.md (и спека этапа).

## Критерий приёмки
- [ ] DoD из спеки
- [ ] Отчёт этапа
- [ ] Перечень файлов (created|changed)
- [ ] Public API → lib.rs re-exports + breaking note

## Как сдать
Отчёт + smoke-команды.
```

### 5.2. Шаблон Test

```markdown
# Задача Test: этап X.Y — тесты для <название>

## Поведение для покрытия
<из ТЗ>

## Ожидаемый API
- …

## Обязательные кейсы
1. …
2. …
3. Негативный: …

## Где класть
- unit: #[cfg(test)] рядом с модулем
- integration: tempfile, без сети; git — #[ignore] / feature

## Ограничения
- Не менять production-логику.
- Детерминированные тесты; читаемые имена.

## Параллельная приёмка (гоночный риск)
Dev и Test идут на одной stage-ветке параллельно. Test **проверяет поведение только
после того, как Dev сдал diff / коммит** (финальное, merge-ready дерево):
- Перед стартом прогонов: `git status` / `git log` на stage — если правок Dev ещё нет
  (ветка на tip базовой, рабочее дерево чистое) — **не делать вывод «блокер/фейл по
  коду Dev»**. Либо подождать коммит Dev, либо сдать baseline + явный
  «код Dev отсутствует — приёмка невозможна до его коммита», а финальную
  перепроверку делает Orchestrator по merge-ready tip.
- False-negative гонки (Test проверил «до правок Dev») — НЕ основание для rework:
  Orchestrator перепроверяет финальное дерево сам.

## Критерий приёмки
- [ ] Все обязательные кейсы
- [ ] Команда узкого набора
- [ ] Проверка шла по merge-ready дереву (не по промежуточному состоянию)
```

### 5.3. Приёмка

**Dev:** DoD ✓ · нет запрещённых паттернов · согласованные файлы · отчёт полный · public API отмечен.  
**Test:** все кейсы · без ручного setup вне tempfile · читаемые имена · команда запуска.  
**Красные тесты этапа → не закрывать.** Баг кода → rework Dev; неверные тесты → rework Test.

### 5.4. Rework ticket

```markdown
# Rework: этап X.Y (Dev|Test), попытка N
## Что не принято
1. …
## Что сделать
1. …
## Что уже хорошо (не ломать)
- …
```

Лимит **3** попытки → эскалация человеку.

### 5.5. Формат отчёта этапа

```markdown
## Этап X.Y — <название>
### Сделано
- …
### Файлы
- path (changed|created)
### Тесты
- command: … ; result: pass/fail
### Риски / follow-up
- …
### Критерий готовности
- [x]/[ ] <текст из спеки>
```

---

## 6. Git workflow (Orchestrator выполняет сам)

| Ветка | Назначение |
|-------|------------|
| `main` | Только релизы вех (PR + 1 approval, squash, no force push) |
| `dev` | Основная рабочая ветка |
| stage | От `dev`: `{phase}-{short-slug}` (kebab-case) |

Примеры: `a1-stash-age`, `a2-rinse`, `a3-raid`, `a4-git-dx`.

1. Работа только в stage от `dev`.
2. PR stage → `dev`, squash, удалить stage.
3. После проверки Orchestrator **сам**:
   ```bash
   gh pr merge <N> --squash --delete-branch
   git fetch --prune origin
   ```
4. `dev` → `main` + tag + Release **только** на вехах:  
   MVP `v0.1.0` · Alpha `v0.3.0` · Beta `v0.5.0` · RC `v0.9.0` · Stable `v1.0.0`
5. Hotfix: branch от `main`/tag → PR `main` → backport `dev`.
6. Не удалять `dev`. Не PR мимо `dev` без явной команды.

---

## 7. Wiki (пользовательская документация)

**URL:** https://y-tretyakov.github.io/raccpack/  
**Источник:** `wiki/` (VitePress). Primary locale — **русский** (root), EN — skeleton `wiki/en/`.

| Правило | Детали |
|---------|--------|
| Назначение | UX: installation, quick-start, concepts, cli-usage, sniff/dig/pack/stash, supported, roadmap, architecture (user-level) |
| Source of truth | Wiki описывает **ровно текущее** поведение CLI. Страницы команд явно: «если флаг не указан здесь — его нет» |
| Не переносить | Dev-спеки (`docs/`, AGENTS, workflow, facade internals) в wiki |
| Тон | Спокойный, практичный, без маркетинга; callouts `::: info\|tip\|warning\|danger\|details` |
| Синхронизация | Любое user-facing изменение флагов/поведения/supported-таблиц → обновить wiki-страницу в том же PR или сразу после FINAL через Docs |
| Сборка | `pnpm run wiki:build` без ошибок после правок |
| Стиль | Единый визуальный/редакционный стиль — отдельная задача позже; пока держать существующую структуру и тон |

Команды:

```bash
pnpm install && pnpm run wiki:dev
pnpm run wiki:build
pnpm run wiki:preview
```

Deploy: `.github/workflows/wiki.yml` → GitHub Pages.

Ключевые страницы для сверки при UX-изменениях:

- `/` — позиционирование, пайплайн  
- `/installation.html`, `/quick-start.html`  
- `/concepts.html` — den, риски, маскирование, exit codes  
- `/cli-usage.html` — глобальные флаги + обзор команд  
- `/sniff.html`, `/dig.html`, `/pack.html`, `/stash.html`  
- `/supported.html` — маркеры, секреты, skip, pack deny  
- `/roadmap.html` — что доступно / планируется (не путать с dev-roadmap)

---

## 8. Архитектура, модульность, SOLID, KISS (ОБЯЗАТЕЛЬНО)

Эти правила **включать в каждое поручение Dev**. Оркестратор при приёмке проверяет их так же строго, как DoD этапа.
Опираться на skills: `rust-patterns`, `rust-best-practices`, `rust-skills`, `improve-codebase-architecture`,
`rust-testing`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`,
`clap` / `domain-cli` (CLI), `tui-design` (TUI), `writing-plans` / `executing-plans`, `grilling` / `grill-me`.

### 8.1. Слои и границы (не размывать)

```text
Presentation (cli / tui / tauri+react)
        │  только вызов facade + отображение DTO
        ▼
Facade / app  (use-cases: sniff, dig, stash, rinse, pack, raid)
        │  оркестрация, ProgressSink, RunMode, AppContext
        ▼
Domain modules in raccpack-core
  config │ scan │ detect │ secrets │ clean │ archive │ den │ git │ cache │ report │ policy
        │
        ▼
Infrastructure: FS, subprocess git, age/tar+zstd  (за trait’ами где нужно)
```

| Правило | Деталь |
|---------|--------|
| Бизнес-логика **только** в `raccpack-core` | CLI/TUI/Desktop не содержат эвристик секретов, skip, den naming |
| Facade не знает UI | Нет clap/ratatui/react внутри core |
| Secrets/archive backends за registry | Engine не импортирует конкретные `aws::` / `age::` напрямую из разрозненных мест |
| DTO наружу — serde-friendly, masked | Raw secret не пересекает границу core → UI |
| Ошибки | `Error` / `ConfigError` + `suggestion()`; не `anyhow` / `Box<dyn Error>` в public API |

Нарушение слоя (UI читает FS в обход core, core печатает в stdout, secrets matcher ходит в den layout) — **отклонять** на приёмке.

### 8.2. Размер файлов и модулей (жёсткий лимит)

| Метрика | Норма | Действие при превышении |
|---------|--------|-------------------------|
| Файл `.rs` (логика) | **цель 150–300 строк**, soft max **~400** | Обязательно разрезать **до** merge |
| Абсолютный потолок | **450 строк** | Не принимать PR; split в том же этапе или follow-up с блокером |
| Функция / метод | Одна ответственность; ориентир **≤ 40–50 строк** тела | Разбить на named helpers |
| Модуль | Один концепт, описывается **одним предложением** | Если нельзя — уже два модуля |

**Как резать (предпочтения по порядку):**

1. **По концепту** — один matcher / один backend / одна ecosystem detect / одна policy = один файл.
2. **Directory + thin `mod.rs`** — `mod.rs` только: `mod` declarations, re-exports, registry/`all_*()`, публичный entrypoint. Без толстой логики.
3. **types.rs** — структуры/enum’ы; алгоритмы — в `engine.rs` / `service` / конкретных файлах.
4. **Не** «utils.rs на всё» и не god-file `secrets.rs` / `detect.rs` с гигантским `match`.

Эталон раскладки уже зафиксирован:

- Secrets: `secrets/matchers/{aws,github,…}.rs` + `matchers/mod.rs` registry + `engine.rs` + `types.rs`
- Archive: `archive/backends/age.rs` + registry; `pack.rs` отдельно от encrypt
- Markers/detect: `scan/markers/{rust,node,…}.rs`, `scan/detect/{rust,node,…}.rs` + registry

Новый вид секрета / язык / backend = **новый файл + одна строка в registry**, без правки «большого» match-файла.

### 8.3. Функции: одна задача, без «и ещё»

- Функция делает **одно** действие на своём уровне абстракции (SRP на уровне функции).
- Не смешивать: walk + classify + write archive + print progress в одном теле.
- Side effects (IO, encrypt, delete) — явные, ближе к краю (facade / infrastructure), не глубоко в pure helpers.
- Pure helpers предпочтительны для policy, naming, risk upgrade, path checks — их легко тестировать.
- Длинный pipeline → отдельные named steps или явный state machine / phase enum (как raid phases), не одна функция на 200 строк.
- Избегать boolean soup (`do_x(a, b, true, false, true)`): лучше options struct / enum mode.

Плохо: `fn process_project(...)` который и dig’ает, и stash’ит, и пакует, и пишет manifest.  
Хорошо: `dig` / `stash` / `pack` / `write_manifest` вызываются из `raid` по фазам.

### 8.3.1. Общие хелперы: не плодить клонов с другими именами

Разбить длинную функцию на helpers — правильно. **Скопировать** тот же helper под новым именем в соседний модуль — нет.

**Перед тем как завести новый helper, Dev обязан:**

1. Поискать существующий с той же семантикой (`rg`, поиск по `fsutil`, `path`, `slug`, `mask`, `deny`, `skip`, `den/names` и т.д.).
2. Если нашёл подходящий — **вызвать его**, не писать «почти такой же».
3. Если существующий чуть узкий — **расширить его** (параметр / options / enum), а не форкнуть.
4. Если логика совпадает только частично — вынести **общий кусок** в один pure helper; различное оставить в callers.
5. Имя отражает **что делает**, не «где лежит» (`sanitize_slug`, не `pack_slug` + `stash_slug` с копипастой).

**Куда класть shared-код** (порядок предпочтения):

| Уровень | Когда | Пример |
|---------|--------|--------|
| Тот же файл `mod` / `#[cfg(test)]` | Нужен только здесь | локальный `fn trim_utf8_prefix` |
| Родительский модуль / `util` **внутри** подсистемы | 2+ файла одной подсистемы | `secrets/mask.rs`, `den/names.rs`, `scan/path_containment.rs` |
| Отдельный маленький модуль в core | Нужен **разным** подсистемам | `pathutil`, `fsutil`, `timeutil` — только если реально cross-cutting |
| **Не** создавать | «На всякий случай» или третий `*_utils.rs` без темы | запрещено |

Запрещено:

- `utils.rs` / `helpers.rs` / `common.rs` **без темы** (свалка несвязанных функций).
- `pack_is_under_root` + `stash_is_under_root` + `raid_is_under_root` с одинаковым телом.
- Копипаст 5–15 строк «чуть переименую» вместо общего вызова.
- Дублировать константы/таблицы (skip names, deny patterns) — один источник правды + registry.

Допустимо оставить локальную копию только если:

- это 2–3 строки очевидного кода **и** общий модуль был бы тяжелее по связности;
- или семантика **намеренно разная** (разные инварианты) — тогда разные имена и комментарий *почему не общий*.

При приёмке: Orchestrator / Test смотрят diff на похожие имена и тела; дубликаты → rework Dev («extract shared, delete clones»).

### 8.3.2. Потоки, `Send` / `Sync` и shared helpers

Большинство хелперов в core — **pure functions** или берут `&self` / аргументы по ссылке. Их **не нужно** специально «делать потокобезопасными»: у free-function нет shared mutable state.

```rust
// OK из любого числа потоков одновременно: нет shared mut state
pub fn project_slug(name: &str) -> String { /* ... */ }

pub fn is_under_root(path: &Path, root: &Path) -> bool { /* ... */ }
```

Правила:

1. **Предпочитать pure / `&self` без interior mutability.** Тогда helper автоматически безопасен при параллельном sniff/dig (`parallel_jobs` в Beta+).
2. **Не** прятать глобальный state (`static mut`, ленивый `Mutex` «на всё») внутри helper «чтобы было удобно».
3. Если нужен кэш / реестр на процесс:
   - `OnceLock` / `LazyLock` для **immutable** данных после init (таблицы маркеров — уже так);
   - для mutable shared — явный `Arc<Mutex<T>>` / `Arc<RwLock<T>>` **на краю** (facade/app), не внутри deep pure helper;
   - или передавать кэш аргументом (`&Cache`, `&mut WalkState`) — проще тестировать.
4. Trait objects в registry (`&'static dyn SecretMatcher`, `&'static dyn EncryptionBackend`) — реализации **обязаны** быть `Send + Sync`, если engine может вызываться с thread pool (уже в контракте matchers). Новые impl — без thread-local грязи и без `&mut self` на shared singleton.
5. Если helper держит `!Send` тип (например сырой raw pointer, или `Rc`) — **не** использовать его из `rayon`/thread pool; либо переписать на `Arc`, либо оставить single-thread API.
6. Passphrase / raw secret material — только в коротком стеке вызова, `zeroize` на drop; не класть в `lazy_static` / глобальный кэш.

Кратко для Dev в поручении:

> Shared helper = одна функция/модуль, один смысл. Ищи перед добавлением.  
> Pure + `Send + Sync` данные по умолчанию. Mutable shared state — только явно на краю, не копипастой в каждом потоке.

### 8.4. SOLID в контексте raccpack

**S — Single Responsibility**

- Один тип / модуль / файл — одна причина меняться.
- `FilenameMatcher` не знает про den paths; `den_layout` не знает content regex; CLI не знает age API.
- Приёмка: «зачем менять этот файл?» — должен быть один ответ.

**O — Open/Closed**

- Расширение через **новые файлы + registry**, не через правку гигантского `match` / `if` цепочки в центре.
- Новые secret patterns, languages, encryption backends, cleanup strategies — add-only к registry.
- Менять порядок first-match таблиц (`PATTERNS`, `CONTENT_MARKERS`) **только** с инвариант-тестами.

**L — Liskov Substitution**

- Реализации trait’ов (`SecretMatcher`, `EncryptionBackend`, `StackDetector`, `GitClient`, `ProgressSink`) должны быть взаимозаменяемы без сюрпризов для engine/facade.
- Не ослаблять предусловия и не усиливать постусловия в subtype’ах (например, backend не должен внезапно требовать сеть, если trait этого не обещает).
- Mock `GitClient` / `NullProgress` — полноценные подстановки в тестах.

**I — Interface Segregation**

- Узкие trait’ы: лучше `match_filename` + `match_content` на matcher, чем один «GodMatcher» с 15 методами «на вырост».
- UI не зависит от internal pack walker API; CLI не тянет TUI types.
- Public `lib.rs` re-exports — минимальные; сужение API = follow-up hygiene (F-API-1 / R1).

**D — Dependency Inversion**

- Высокоуровневый engine зависит от **абстракций** (`&dyn SecretMatcher`, `&dyn EncryptionBackend`, `GitClient`), не от конкретных `AwsMatcher` / `AgeBackend` внутри оркестратора.
- Конкретика собирается в registry / composition root (facade или `all_matchers()`).
- Config и FS — на краю; domain policy по возможности от них отвязана (paths приходят аргументами).

### 8.5. KISS и YAGNI

- Самое простое решение, которое удовлетворяет DoD этапа — правильное.
- Не вводить trait / generic / async / channel, пока нет второго реального клиента или явного требования этапа.
- Не абстрагировать «на будущее» (плагины, KMS, HTTP BFF) — это вне scope до 1.0.
- Дублирование 5–10 строк иногда лучше, чем преждевременная общая обёртка; третий повтор — тогда extract.
- Имена простые и доменные (`place_pack`, `project_slug`, `is_under_root`), не `handleManagerProcessor`.

### 8.6. Паттерны, принятые в проекте

| Паттерн | Где |
|---------|-----|
| **Registry** | matchers, backends, markers, detect, cleanup strategies |
| **Facade** | application use-cases; единственная точка для UI |
| **Strategy** | cleanup strategies, encryption backend, stack detectors |
| **Newtype / enum severity** | SensitiveRisk, RunMode, OperationKind |
| **Sink / Observer** | ProgressSink для long ops |
| **DTO + serde** | ScanReport, DigResult, StashResult, RaidResult — стабильный контракт |
| **Composition over inheritance** | Нет глубоких class hierarchy; trait objects / enum где нужно |
| **Fail-fast phases** | raid: ошибка enabled-фазы останавливает следующие |
| **Atomic place** | temp + rename в den; staging cleanup |

Не тащить Enterprise-паттерны без нужды (Abstract Factory на один backend, DI-container, event bus).

### 8.7. Ошибки, тесты, стиль (жёсткие)

- **Без** `unwrap()` / `expect()` в production (исключение: static `OnceLock` init при старте).
- **Без** `anyhow` / `Box<dyn Error>` в public API.
- Секреты и passphrase **не** в `Display`, tracing, report, IPC по умолчанию.
- `WalkDir` → `follow_links(false)`. Pack walker — explicit DFS.
- SensitiveRisk — только severity API.
- Не менять first-match семантику таблиц без инвариант-тестов.
- Public API изменён → re-exports в `lib.rs` + breaking в отчёте.
- Код этапа **компилируется** (явный `TODO` только для промежуточного каркаса, согласованного в ТЗ).
- Тесты: unit рядом с модулем; integration на tempfile; без сети; git — `#[ignore]` / feature.
- Имена тестов — поведение (`stash_dry_run_writes_nothing`), не `test1`.
- `cargo fmt`; clippy `-D warnings` на затронутом crate при приёмке этапа.

### 8.8. Чеклист модульности на приёмке Dev

Orchestrator **не принимает**, если:

- [ ] Файл > ~400–450 строк без плана немедленного split
- [ ] Функция делает несколько несвязанных действий (walk+encrypt+delete+log)
- [ ] Новый секрет/язык/backend вписан в монолитный match вместо нового файла + registry
- [ ] Появились дубликаты helper’ов с разными именами и одной семантикой (не искали / не extract)
- [ ] Бестемный `utils.rs` / `helpers.rs` как свалка
- [ ] Shared mutable state спрятан в deep helper (`static mut`, скрытый global) без явного владельца на краю
- [ ] UI-crate содержит domain-эвристику
- [ ] Core зависит от clap/ratatui/tauri
- [ ] Raw secret в ошибке, логе или DTO
- [ ] Нет path containment на destructive path (stash/pack/raid)
- [ ] Public API раздут без нужды / breaking не отмечен

### 8.9. Skills — когда подключать (подсказка Orchestrator’у)

| Skill | Когда |
|-------|--------|
| `writing-plans` / `executing-plans` | План этапа перед делегированием |
| `grill-me` / `grilling` | Уточнение ТЗ, спорные design-решения |
| `rust-patterns` / `rust-best-practices` / `rust-skills` | Поручение Dev, review idiomatic Rust |
| `improve-codebase-architecture` | Split модулей, слои, registry |
| `rust-testing` / `test-driven-development` | Поручение Test, стратегия покрытия |
| `systematic-debugging` | Красные тесты / регрессии |
| `verification-before-completion` | Перед merge / FINAL |
| `clap` / `domain-cli` | Этапы CLI (A1.4, A2.3, A3.4, A4.2) |
| `tui-design` | Beta B1 TUI |
| `find-skills` | Если неясно, какой skill уместен |

В тексте поручения Dev можно явно: «следуй skill rust-best-practices + improve-codebase-architecture; лимит 400 строк/файл».

---

## 9. Alpha backlog (порядок)

```
A1.1 age + zeroize passphrase          # сверять WORKLOG — может быть closed
A1.2 stash manifest (без raw) + remove sources в Commit
A1.3 facade stash + den/secrets/…
A1.4 CLI racc stash
→ A2.1 cleanup strategies + config
A2.2 facade rinse
A2.3 CLI racc rinse
→ A3.1 facade raid (fail-fast)
A3.2 ProgressSink + CLI progress
A3.3 manifest JSON в den/manifests/
A3.4 CLI racc raid --yes; E2E alpha
→ A4.1 GitClient + status в dig
A4.2 config migrate chain + racc init
A4.3 tracing без секретов; --verbose
A4.4 integration tests + CI cargo test
```

После Alpha (в порядке): **Detect v2 (0.4.x)** → **Beta (0.5.x)** → **RC (0.9.x)** → **Stable (1.0.0)**.  
Детали этапов D1–D3 / B1–B4 / R1–R4 / S1: `docs/raccpack-roadmap-v1.md`; версии по этапам: `docs/VERSION_ROADMAP.md`.

**Обязательно в Alpha (FOLLOWUPS_FROM_MVP):**

1. **F-PATH-1** — path containment в stash (+ pack/raid)
2. **F-ERR-1** — `From<ConfigError> for Error` или единый map в CLI
3. **F-SKIP-1** — согласованность default_pack / rinse skip lists
4. **F-CFG-4** — migrate + init (A4.2)
5. CI green (A4.4)

Остальное — Beta/RC.

---

## 10. Карта артефактов (после консолидации)

| Артефакт | Роль |
|----------|------|
| **`AGENTS.md`** (этот) | Единственная памятка Orchestrator: процесс, git, wiki, архитектура, SOLID, backlog, инварианты |
| **`docs/alpha/*`** | Спеки этапов Alpha — читать **только** по явной ссылке перед этапом |
| **`docs/archive/*`** | Закрытый MVP (WORKLOG + спеки M1–M4) — справка, **не править** |
| **`WORKLOG.md`** | Текущий журнал Alpha+; обновлять после каждого этапа |
| **`docs/raccpack-roadmap-v1.md`** | Дорожная карта до 1.0.0 — live; отмечать фазы/этапы done после каждого этапа |
| **`docs/VERSION_ROADMAP.md`** | Версии по этапам — live; bump `workspace.package.version` после каждого этапа |
| **`docs/raccpack-architecture-vision.md`** | Видение архитектуры — нужный док; справка перед планированием фаз |
| **`docs/raccpack-improvements-proposal.md`** | Предложения по улучшениям — нужный док; основа для roadmap/спек |
| **`docs/FOLLOWUPS_FROM_MVP.md`** | Открытые follow-ups из MVP — live; читать только при конкретной надобности (крайняя необходимость) |
| **`wiki/`** → published Pages | UX source of truth (CLI, concepts, supported, roadmap) |
| **`README.md`** | Для людей: status, build, git workflow, roadmap-блок, ссылка на wiki |

Устаревшие корневые knowledge-docs (workflow, facade-and-den, modularity, markers-detect и аналоги) **не использовать** — содержание влито в AGENTS; из репозитория их можно удалить.

---

## 11. Команды проверки

```bash
cargo test -p raccpack-core
cargo test --workspace
cargo fmt --check
cargo clippy -p raccpack-core --all-targets -- -D warnings
pnpm run wiki:build   # если трогали wiki/
```

---

## 12. FINAL вехи

1. `cargo test --workspace` — green.  
2. Обязательные этапы — `done` или `explicitly deferred` с причиной в WORKLOG.  
3. Запреты соблюдены.  
4. Path containment и den layout на месте (Alpha+).  
5. User-facing изменения отражены в wiki (или явный follow-up Docs).  
6. WORKLOG полный.

Только после FINAL → Docs (CHANGES, MIGRATION, wiki UX).

---

## 13. Анти-паттерны

1. Orchestrator сам пишет этап.  
2. Dev без параллельного Test (искл.: design-only → `test: n/a`).  
3. «Сделай A1–A3 целиком».  
4. Test меняет production.  
5. Docs до FINAL без явной UX-задачи.  
6. Закрытие с красными тестами.  
7. Rework без ticket.  
8. Правка archive MVP «заодно».  
9. PR не в `dev` / удаление `dev` / force push.  
10. Расхождение wiki и реального CLI без плана синхронизации.  
11. God-file / файл >450 строк / функция «сделай всё».  
12. Новый feature правкой монолитного match вместо файла + registry.  
13. Domain-логика в CLI/TUI/React или UI-зависимости в core.  
14. Premature abstraction (YAGNI) или наоборот copy-paste на третий раз без extract.  
15. Boolean soup и hidden side effects глубоко в helpers.  
16. Клоны helper’ов с разными именами (`pack_slug` / `stash_slug`) вместо одного shared.  
17. Бестемный `utils.rs`; скрытый global mutable state в «удобном» helper.
18. Расхождение версии между `Cargo.toml` / README / wiki / установленным бинарником после закрытого этапа (см. чеклист §3 шаг 9).
19. **Удаление ветки `dev`.** Ветка `dev` — основная интеграционная ветка. Она **никогда** не удаляется. Не удалять через `git push --delete`, не удалять через GitHub UI, не удалять при каких-либо обстоятельствах. Если что-то пошло не так с `dev` — реанимировать из `main`, но не удалять.

---

## 14. Чеклист релиза (Orchestrator выполняет сам)

### 14.1. Перед релизом

1. **Все этапы вехи закрыты** — WORKLOG, тесты зелёные, `cargo clippy` чист.
2. **Версия в `Cargo.toml`** — `[workspace.package] version` = `X.Y.Z` по `docs/VERSION_ROADMAP.md`.
3. **`Cargo.lock`** — перегенерировать (`cargo build -p raccpack-cli`) и закоммитить.
4. **README.md / README.ru.md** — версия в badge + абзац Status. Язык README: EN = `README.md`, RU = `README.ru.md`. Ссылка на переключение языка в шапке обоих файлов.
5. **Wiki** — `roadmap.md`, `introduction.md`, страницы команд обновлены.
6. **`docs/VERSION_ROADMAP.md`** — этап отмечен `✅`, текущая позиция обновлена.
7. **`docs/raccpack-roadmap-v1.md`** — «Текущая версия» в шапке + фаза/этап done.
8. **WORKLOG.md** — запись этапа; шапка («Текущая версия» + следующий bump); backlog-чекбоксы закрыты.

### 14.2. Создание релиза

```bash
# 1. Tag + push (ветка main)
git tag vX.Y.Z
git push origin vX.Y.Z

# 2. Запустить workflow release.yml
gh workflow run release.yml --repo y-tretyakov/raccpack -f tag=vX.Y.Z

# 3. Дождаться зелёного
gh run watch <RUN_ID> --repo y-tretyakov/raccpack --exit-status

# 4. Проверить ассеты
gh release view vX.Y.Z --repo y-tretyakov/raccpack --json assets --jq '.assets[] | .name'
```

### 14.3. Шаблон описания релиза

Описание релиза — **двуязычное** (EN + RU). Формат: английская версия отображается по умолчанию, русская — свёрнута в `<details>`.

Структура:
1. **Баннер** — `![raccpack vX.Y.Z — <name>](https://github.com/.../releases/download/vX.Y.Z/<banner>.webp)`
2. **Заголовок** — `# raccpack vX.Y.Z — <name>`
3. **Переключатель языка** — `[🇬🇧 English](#english) · [🇷🇺 Русский](#russian)`
4. **EN секция** — What's new, Install (все дистрибутивы + from source), Checksums, Links (ссылки на **английскую** wiki: `supported.html`, `cli-usage.html` и т.д.)
5. **RU секция** — `<details><summary>Русская версия</summary>...</details>` с тем же содержимым, но ссылки на **русскую** wiki: `ru/supported.html`, `ru/cli-usage.html` и т.д.

**Правило ссылок на wiki:** в русскоязычном контенте ссылки ведут на `/ru/...`, в англоязычном — на корень.

Пример тела (см. релиз v0.4.0 как эталон):

```markdown
![raccpack vX.Y.Z — <name>](banner URL)

---

# raccpack vX.Y.Z — <name>

[🇬🇧 English](#english) · [🇷🇺 Русский](#russian)

---

<a id="english"></a>

## English

...EN content...

---

<a id="russian"></a>

## Русский

<details>
<summary>Русская версия (нажмите, чтобы развернуть)</summary>

...RU content...

</details>
```

### 14.4. Инструкции по установке (в описании релиза)

Всегда включать для **каждого** дистрибутива:

```bash
# Debian / Ubuntu
sudo dpkg -i raccpack-X.Y.Z-1-amd64.deb

# Fedora / RHEL / Rocky
sudo rpm -i raccpack-X.Y.Z-1.x86_64.rpm

# Arch Linux / Manjaro
sudo pacman -U raccpack-X.Y.Z-1-x86_64.pkg.tar.zst

# Any Linux (musl, universal)
tar --zstd -xf raccpack-X.Y.Z-linux-x86_64.tar.zst
sudo cp raccpack-X.Y.Z/racc /usr/local/bin/

# From source
cargo install raccpack-cli
```

Упомянуть ARM64 пакеты. Не забыть `sha256sum -c SHA256SUMS`.

### 14.5. Проверка после релиза

1. **Скачать все ассеты** и проверить `sha256sum -c SHA256SUMS`.
2. **Извлечь tar.zst** и запустить `racc --version` / `racc --help`.
3. **Проверить deb** — `dpkg-deb -I <file>.deb` (метаданные).
4. **Проверить rpm** — `rpm -qip <file>.rpm` (метаданные).
5. **Проверить pkg** — `tar -tf <file>.pkg.tar.zst` (содержимое).
6. **Убедиться что `dev` ветка не удалена** — `git branch -a | grep dev`.

### 14.6. README: язык и формат

| Файл | Язык | Переключатель |
|------|------|---------------|
| `README.md` | English | Ссылка на `README.ru.md` |
| `README.ru.md` | Русский | Ссылка на `README.md` |

**Не** конкатенировать два языка в один файл. Каждый README — на одном языке, с ссылкой на другой в шапке.

---

*Единственная оперативная памятка оркестратора. Спеки этапов — только `docs/alpha/*` по явной ссылке. Для user-facing поведения CLI — опубликованная wiki. При конфликте с любыми старыми knowledge-docs приоритет у этого файла.*
