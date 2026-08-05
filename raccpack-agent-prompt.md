# Промпт для агента: реализация raccpack-core с нуля

## Роль

Ты — инженерный агент, который **пишет библиотеку** `raccpack-core` (Rust) **с чистого листа** (весь предыдущий код удалён). Твоя задача — пройти фазы 0–11 как план построения кода, сразу соблюдая жёсткие правила ревью (строгие типы ошибок, безопасность секретов, тесты-инварианты для PATTERNS / CONTENT_MARKERS / heuristics).

## Исходные материалы (обязательно прочитать перед работой)

1. `raccpack-agent-workflow.md` — ОБЯЗАТЕЛБНЫЙ к выполнению порядок и алгоритм работ, «как правильно».
2. `raccpack-architecture-vision.md` — Оьщее видение архитектуры.
3. `raccpack-facade-and-den.md` — Дополнение к [видению архитектуры](raccpack-architecture-vision.md). 
4. `raccpack-roadmap-v1.md` — Дорожная карта. (ОБЯЗАТЕЛЬНО после каждого выполненного этапа отмечать, что было выполнено и паралллельно писать в CHANGELOG).

## Жёсткие правила

- Документы из папки `docs/` не читать, кроме одного конкретного ддокумента, на который, перед каждым этапом, буду давать тебе ссылку.
- Один **этап** = одна узкая задача. Не смешивай рефакторинг API, новые тесты и документацию в одном шаге.
- После каждого этапа: код компилируется в рамках этапа (или явно помечен `TODO` только если этап про промежуточный каркас).
- Не меняй семантику first-match таблиц sensitive без тестов-инвариантов.
- Не добавляй `unwrap()` в production-код.
- Не возвращай `anyhow::Error` / `Box<dyn std::error::Error>` из public library API.
- Секреты (raw values, passphrase) не должны попадать в `Display` ошибок и логи.
- Пиши на том же стиле, что и окружающий код (rustfmt-совместимо).
- Если этап затрагивает public API — обнови re-exports в `lib.rs` и кратко опиши breaking change в конце ответа этапа.

## Стартовые условия

- Код пишется **с нуля**: никакого наследуемого «уже сделано» нет, этапы 1–11 реализуются заново.
- Каждый этап обязан оставлять код компилируемым и покрытым тестами по своему критерию.
- Инварианты domain-логики (first-match порядок PATTERNS / CONTENT_MARKERS, heuristics) фиксируются тестами по мере появления кода.

---

# ФАЗА 0 — Подготовка и инвентаризация

## Этап 0.1 — Карта crate’а

**Цель:** зафиксировать текущее состояние, чтобы не дублировать работу.

**Сделай:**

1. Перечисли все `*.rs` модули верхнего уровня и подмодули (старт — с пустого дерева crate).
2. Для каждого отметь: «готово» / «частично» / «не начато».
3. Найди все вызовы:
   - `WalkDir::new` без `follow_links(false)`;
   - `Error::Walk` / `Error::Encrypt(` / `Error::Archive(` (старые конструкторы);
   - `RaccConfig::load_from` / `load_forgiving` / `den_dir()` без `?`;
   - `SKIP_DIRS` / дубли списков skip;
   - `DefaultHasher` в cache.
4. Сохрани отчёт в `WORKLOG.md` (раздел «0.1 Inventory»).

**Критерий готовности:** в WORKLOG есть таблица модулей и список «дыр».

## Этап 0.2 — Сборка и тесты baseline

**Цель:** знать, что зелёное до изменений.

**Сделай:**

1. Если есть `Cargo.toml` workspace/crate — `cargo test -p raccpack-core` (или эквивалент).
2. Если crate нет вообще (пишем с нуля) — зафиксируй это в WORKLOG как «пустой baseline» и отметь, что первой задачей становится развёртывание crate.
3. Запиши в WORKLOG: сколько тестов, какие падают **до** твоих правок.

**Критерий готовности:** baseline зафиксирован; падающие тесты не списываются на «и так было», если ты их не чинил.

---

# ФАЗА 1 — EnabledGroups как type-safe set (P1)

## Этап 1.1 — Enum Group

**Цель:** заменить строковые имена групп на enum.

**Сделай:**

1. В `sensitive` (или `config`) заведи:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensitiveGroup {
    Filename,
    Keys,
    Aws,
    // ... все из KNOWN_SENSITIVE_GROUPS
}
```

2. Реализуй `from_str` / `as_str` 1:1 с `KNOWN_SENSITIVE_GROUPS`.
3. Unit-тесты: roundtrip всех известных строк; unknown → error.

**Не делай:** ещё не подключай к `EnabledGroups` в finder (это 1.2).

**Критерий:** тесты roundtrip зелёные.

## Этап 1.2 — EnabledGroups на EnumSet / bitflags

**Цель:** убрать `HashSet<String>`.

**Сделай:**

1. Выбери один вариант:
   - `enumset` / `EnumSet<SensitiveGroup>`, или
   - `bitflags` с константами 1, 2, 4…
2. API:
   - `EnabledGroups::all()`
   - `EnabledGroups::from_config(&SensitiveConfig)`
   - `is_enabled(SensitiveGroup)` и/или `is_enabled_str(&str)` для совместимости
3. Замени внутренности в `sensitive/mod.rs`.
4. Обнови вызовы в patterns/content/heuristics/walk.

**Критерий:** поиск по crate не находит `HashSet<String>` внутри `EnabledGroups`; существующие тесты sensitive зелёные.

## Этап 1.3 — Конфиг: хранение groups

**Цель:** toml по-прежнему принимает строки `"aws"`, внутри — enum.

**Сделай:**

1. При parse `enabled_groups: Vec<String>` → конвертация в `EnabledGroups`.
2. Validation: unknown group → `ConfigError::Validation` (уже есть список unknown — убедись, что согласован с enum).
3. Тест: конфиг с `"aws"` и с `"not-a-group"`.

**Критерий:** validation ловит unknown; known группы работают.

---

# ФАЗА 2 — Fingerprint и masking секретов (P2 / security)

## Этап 2.1 — Стабильный fingerprint

**Цель:** не использовать слабый/нестабильный хэш как identity секрета.

**Сделай:**

1. В `sensitive/heuristics.rs` (или отдельный `fingerprint.rs`):
   - функция `fingerprint(value: &str) -> u64` (или `[u8; 16]`).
2. Предпочтительно: `blake3` (первые 8/16 байт) **или** siphash с **фиксированным** key, зашитым как константа crate (не `DefaultHasher`).
3. Замени `fnv1a_64` в `find_repeated`.
4. Тест: один и тот же input → один hash; разные input → с высокой вероятностью разные.

**Критерий:** `fnv1a_64` не используется для secret identity.

## Этап 2.2 — Безопасный masked_value

**Цель:** не светить длинный prefix секрета.

**Сделай:**

1. Новая политика, например:
   - длина `< 8` → `"***"`;
   - иначе → `"****" + last_4` **или** только `"len=N,hash=…"` без plaintext prefix.
2. Документируй выбор в комментарии модуля.
3. Тесты на короткие/длинные/UTF-8 строки.
4. Убедись, что при `reveal_secrets` (если используется выше) masking не обходится случайно в library path без явного флага.

**Критерий:** unit-тесты masking; в `RepeatedSecret.masked` нет 12-символьного prefix секрета.

---

# ФАЗА 3 — Pack и sensitive policy (security)

## Этап 3.1 — Общий deny/allow helper

**Цель:** pack и dig не расходятся по имени файла.

**Сделай:**

1. Вынеси правила имён секретов в один модуль (например `sensitive::names` или `policy`).
2. Pack `is_denied_secret` должен вызывать тот же helper, что и filename patterns (или явный subset «hard deny»).
3. Тест: файл, который dig помечает HIGH/CRITICAL по имени, pack **не** кладёт в архив.

**Критерий:** один source of truth для name-based deny.

## Этап 3.2 — Опциональная content-проверка при pack

**Цель:** не упаковать API key внутри `config.json`.

**Сделай:**

1. Параметр/флаг на `Packer` (default: warn или deny на CRITICAL content).
2. Для eligible файлов — лёгкий content scan (переиспользовать `scan_file_content` / finder), **без** записи raw secret в лог.
3. Поведение: skip file + счётчик `skipped_secrets` в `PackResult`.
4. Тест: json с `AKIA…` не попадает в archive (или попадает только если явно `allow_secrets`).

**Критерий:** тест на content-secret + поле статистики в результате.

---

# ФАЗА 4 — Git abstraction (P1 architecture)

## Этап 4.1 — Trait GitClient

**Цель:** убрать прямую зависимость app-кода от `Command::new("git")`.

**Сделай:**

```rust
pub trait GitClient: Send + Sync {
    fn available(&self) -> bool;
    fn find_repo_root(&self, path: &Path) -> Option<PathBuf>;
    fn classify_file(&self, repo: &Path, file: &Path) -> GitFileStatus;
    fn analyze(&self, path: &Path) -> Result<Option<GitState>>;
}
```

1. Реализация `ProcessGitClient` — перенеси текущую логику из `git.rs`.
2. Default в production — `ProcessGitClient`.
3. Не ломай публичные free-functions: они могут делегировать в default client (thin wrappers).

**Критерий:** `git.rs` содержит trait + impl; free API сохраняет поведение.

## Этап 4.2 — Mock GitClient для тестов

**Цель:** тесты classify без реального git.

**Сделай:**

1. `MockGitClient` с заранее заданной map path → status / state.
2. 2–3 unit-теста classify/analyze на mock.
3. Хотя бы один integration-тест с реальным git оставь `#[ignore]` или feature `git-integration`.

**Критерий:** mock-тесты зелёные без `git` binary.

---

# ФАЗА 5 — Parallelism из конфига (P2)

## Этап 5.1 — Thread pool для scanner

**Цель:** `advanced.parallel_jobs` реально ограничивает rayon.

**Сделай:**

1. В `Scanner::find_projects` (par_iter) оберни в pool:

```rust
rayon::ThreadPoolBuilder::new()
    .num_threads(jobs.max(1) as usize)
    .build()
    .map_err(...)?
    .install(|| { /* par_iter */ })
```

2. `jobs` бери из `Scanner`/переданного `AdvancedConfig` (сейчас в Scanner только `ScannerConfig` — добавь jobs аргументом или расширь config, **минимально**).
3. Тест: хотя бы что `parallel_jobs == 0` уже отсекается validation’ом; jobs=1 не падает.

**Критерий:** при `parallel_jobs = 1` код пути pool используется (можно unit-тест через cfg test helper).

---

# ФАЗА 6 — Config migration и validation (P1/P2)

## Этап 6.1 — Явная цепочка migrate

**Цель:** не «stamp version», а пошаговые функции.

**Сделай:**

```rust
fn migrate_v1_to_v2(&mut self) { ... }
fn migrate_v2_to_v3(&mut self) { ... }
```

1. `migrate()` вызывает шаги по порядку от `from_version` до `CURRENT`.
2. Unit-тесты: fixture toml v1 → после migrate version=CURRENT; v3 → `None`.

**Критерий:** тесты миграций зелёные.

## Этап 6.2 — Расширить validate()

**Цель:** больше semantic checks.

**Сделай (минимум):**

1. Пустые строки в `extra_skip_dirs` / `extra_marker_files` → problem.
2. Дубликаты в `enabled_strategies` / `enabled_groups` → warning или problem.
3. `zstd_level`, `parallel_jobs` — уже есть; не регрессируй.
4. Тесты на каждый новый problem.

**Критерий:** новые validation-тесты зелёные.

---

# ФАЗА 7 — Single-pass walk (крупный, дробить)

## Этап 7.1 — Дизайн WalkEvent

**Цель:** описать API без большой реализации.

**Сделай:**

1. Документ/модуль `walk_session.rs` (можно сначала только типы + комментарии):

```rust
enum WalkEvent<'a> {
    EnterDir { path: &'a Path, name: &'a OsStr },
    File { path: &'a Path, name: &'a OsStr, meta: Metadata },
}
```

2. Trait `WalkVisitor { fn on_event(&mut self, e: WalkEvent) -> WalkControl; }`
3. Описание в WORKLOG: кто будет visitor (scanner markers, sensitive, cache markers, trash).

**Не пиши** ещё полный scanner rewrite.

**Критерий:** типы компилируются; WORKLOG обновлён.

## Этап 7.2 — Минимальный WalkSession

**Цель:** один WalkDir → callbacks.

**Сделай:**

1. `WalkSession::run(root, policy, max_depth, visitor)`.
2. `follow_links(false)`, SkipPolicy.
3. Unit-тест на tempfile: N файлов → visitor видит N File events.

**Критерий:** тест session зелёный.

## Этап 7.3 — Подключить cache collect_markers к WalkSession

**Цель:** первый реальный consumer.

**Сделай:**

1. Перепиши `ScanCache::collect_markers` на WalkSession + visitor.
2. Поведение и тесты cache сохранить.

**Критерий:** cache-тесты зелёные.

## Этап 7.4 — Подключить sensitive walk

**Цель:** `scan_walk` / collect через session.

**Сделай:**

1. Адаптируй `sensitive/walk.rs` к WalkSession **или** общему helper.
2. Не меняй результаты `find` / `find_repeated` (golden тесты).

**Критерий:** sensitive-тесты зелёные.

## Этап 7.5 — Подключить scanner candidates (опционально, отдельно)

**Цель:** marker discovery через session.

**Сделай только если 7.3–7.4 стабильны.** Иначе отложи и зафиксируй в WORKLOG.

**Критерий:** scanner-тесты зелёные **или** явный skip с причиной.

---

# ФАЗА 8 — Инварианты PATTERNS / CONTENT_MARKERS (P2)

## Этап 8.1 — Тесты порядка whitelist

**Цель:** `.env.example` всегда LOW и раньше `.env`.

**Сделай:**

1. Таблица тестов: пары (whitelist_name, generic_name) → whitelist risk < severity generic **или** first match = whitelist.
2. Аналогично для `*.pub` vs private key names.
3. Не меняй таблицы, пока тест не докажет регрессию.

**Критерий:** тесты порядка зелёные.

## Этап 8.2 — Тесты content marker shadowing

**Цель:** `postgresql://` раньше `postgres://`; `sk_live_` раньше `sk-`.

**Сделай:**

1. Фикстуры строк content → ожидаемый label/risk.
2. При падении — либо fix порядка таблицы, либо явный priority field (отдельный этап, не смешивать).

**Критерий:** content order tests зелёные.

---

# ФАЗА 9 — Public API hygiene (P2)

## Этап 9.1 — Аудит pub use

**Цель:** library не экспортирует внутренности.

**Сделай:**

1. Пройди `lib.rs`. Всё, что нужно только CLI/internal, сделай `pub(crate)` или убери из prelude.
2. Оставь: config, report types, Scanner, SensitiveFinder, Packer, Raider, Error, SkipPolicy.
3. Список breaking exports — в WORKLOG.

**Критерий:** `lib.rs` короткий; internal matchers не в корневом prelude.

## Этап 9.2 — missing_docs на public

**Цель:** публичные типы документированы.

**Сделай:**

1. `#![warn(missing_docs)]` на crate или module level для public.
2. Добавь doc-комментарии на public struct/enum/fn без них.
3. Не обязательно на каждом private fn.

**Критерий:** `cargo doc` / clippy missing_docs без ошибок на public (или warn зафиксирован и закрыт).

---

# ФАЗА 10 — Секреты в памяти (stash) (P2 security)

## Этап 10.1 — zeroize passphrase

**Цель:** passphrase не торчит в String после использования.

**Сделай:**

1. Зависимость `zeroize` (и при необходимости `secrecy`).
2. Passphrase как `SecretString` / `Zeroizing<String>` на время encrypt/decrypt.
3. Ошибки encrypt **не** содержат passphrase.
4. Тест: message ошибки не contains passphrase fixture.

**Критерий:** тест на отсутствие passphrase в error string.

---

# ФАЗА 11 — Финальная стабилизация

## Этап 11.1 — Полный test suite

**Сделай:**

1. `cargo test` (все features, что есть).
2. Почини только регрессии, внесённые твоими фазами.
3. WORKLOG: итог green/red.

## Этап 11.2 — Clippy + fmt

**Сделай:**

1. `cargo fmt`
2. `cargo clippy -- -D warnings` (или максимально строго без ломки внешнего API).
3. Исключения только с комментарием why.

## Этап 11.3 — Сводка для человека

**Сделай:**

1. Обнови `CHANGES.md`: что добавлено в ходе фаз 0–11.
2. Краткий `MIGRATION.md` для CLI-авторов (breaking API).

**Критерий:** два документа актуальны.

---

# Порядок выполнения (рекомендуемый)

```
0.1 → 0.2
→ 1.1 → 1.2 → 1.3
→ 2.1 → 2.2
→ 3.1 → 3.2
→ 4.1 → 4.2
→ 5.1
→ 6.1 → 6.2
→ 7.1 → 7.2 → 7.3 → 7.4 → (7.5)
→ 8.1 → 8.2
→ 9.1 → 9.2
→ 10.1
→ 11.1 → 11.2 → 11.3
```

Не перескакивай через фазу 0. Не начинай фазу 7, пока 1–3 не стабильны (меньше конфликтов в walk).

---

# Формат отчёта после КАЖДОГО этапа

```markdown
## Этап X.Y — <название>

### Сделано
- ...

### Файлы
- path/to/file.rs (changed|created)

### Тесты
- command: ...
- result: pass/fail (details)

### Риски / follow-up
- ...

### Критерий готовности
- [x] / [ ] <текст из промпта>
```

Если критерий не выполнен — **не** переходи к следующему этапу; сначала закрой текущий.
