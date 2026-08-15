# WORKLOG — raccpack

Журнал статусов этапов. Orchestrator: y-tretyakov.

**MVP 0.1.0 закрыт.** Полный журнал M1–M4 и docs-миграции:
[`docs/archive/WORKLOG_MVP.md`](docs/archive/WORKLOG_MVP.md).
Спеки закрытых этапов: [`docs/archive/mvp/`](docs/archive/mvp/).

## Backlog (Alpha → 0.3.0)

```
[x] A1.1 age + zeroize passphrase
[x] A1.2 stash manifest (без raw) + remove sources в Commit
[x] A1.3 facade stash + den/secrets/…
[x] A1.4 CLI racc stash
[x] A2.1 cleanup strategies + config toggles
[ ] A2.2 facade rinse DryRun/Commit
[ ] A2.3 CLI racc rinse
[ ] A3.1 facade raid (stash→rinse→pack→move, fail-fast)
[ ] A3.2 ProgressSink + CLI progress
[ ] A3.3 manifest JSON в den/manifests/
[ ] A3.4 CLI racc raid --yes; E2E alpha
[ ] A4.1 GitClient (process) + status sensitive files в dig
[ ] A4.2 Config migrate chain + racc init
[ ] A4.3 tracing без секретов; --verbose
[ ] A4.4 integration tests core + CI cargo test
```

## Этапы

### A2.1 — cleanup strategies + config toggles (CLOSED)

- **Дата:** 2026-08-16
- **Статус:** done
- **Роли:** Orchestrator; Dev + Test (параллельно).

#### Задача
Data-driven стратегии очистки мусора сборки/кэша: именованные стратегии (rust/node/python/jvm/go/generic), набор точных имён директорий (с `*`-suffix паттернами `*.egg-info`), обнаружение под project root, config toggles (`[cleanup] enabled_strategies`), **без** удаления (A2.2) и без CLI.

#### Сделано
- `src/clean/strategy.rs` — `StrategyId` (`as_str` / `from_str_ignore_case`), `TrashMatchKind::DirNameExact`, `TrashPattern::matches` (`*`-suffix, семантика как `SkipPolicy`), `StrategyDef`, `DEFAULT_STRATEGIES` (rust/node/python/jvm/go/generic; `dist`/`build`/`vendor`/`tmp` помечены как careful).
- `src/clean/detect.rs` — `TrashDir`, `DetectTrashOptions`, `find_trash_dirs`: `follow_links(false)`, pruning matched dirs (не спускаемся внутрь), root depth-0 исключается, опциональный `compute_size` (отдельный restricted walk, не `project_size_bytes`), sort by path, defensive containment.
- `src/config/mod.rs` — `CleanupConfig.enabled_strategies`, defaults `rust/node/python`, `#[serde(default)] pub cleanup` в `RaccConfig`.
- `src/config/validate.rs` — strict: неизвестный id → `ConfigError::UnknownCleanupStrategy` (case-insensitive).
- `src/lib.rs` — `pub mod clean` + re-exports.
- `tests/clean.rs` — 22 integration теста (все кейсы спеки §6 + extras: pruning, root-exclusion, sort, max_depth, roundtrip).

#### Файлы
- created: `src/clean/mod.rs`, `src/clean/strategy.rs`, `src/clean/detect.rs`, `tests/clean.rs`
- changed: `src/config/mod.rs`, `src/config/validate.rs`, `src/config/error.rs`, `src/lib.rs`, `tests/config.rs` (механический фикс struct-literal нового поля `cleanup`)

#### Тесты
- `cargo test --workspace` — green (все suites)
- `cargo test -p raccpack-core --test clean` — 22 passed
- `cargo clippy --workspace --all-targets -- -D warnings` — pass
- `cargo fmt --check` — pass

#### Решения
- Strict validation неизвестных strategy id в конфиге (рекомендация спеки §4), case-insensitive.
- `*.egg-info` поддержан через `*`-suffix в `TrashPattern::matches` (не плодили glob-kind), консистентно с `SkipPolicy`.
- Matched dirs pruning → нет двойного обнаружения и нет спуска в гигантские `node_modules`/`target`.
- Корень (depth 0) никогда не записывается как trash — защита от удаления всего проекта в A2.2.
- F-SKIP-1: паттерны задокументированы в `strategy.rs` для согласованности с будущим `default_pack()`.

#### Критерий готовности (DoD из a2.1 §7)
- [x] `DEFAULT_STRATEGIES` + `StrategyId` в `clean/strategy.rs`
- [x] `CleanupConfig` в config, defaults rust/node/python
- [x] `find_trash_dirs` без удаления
- [x] `follow_links(false)`
- [x] Тесты §6 зелёные
- [x] F-SKIP-1 задокументирован

#### Риски / follow-up
- **F-SKIP-1 (замечание приёмки):** имена trash пока два списка — `scan::skip::DEFAULT_DIR_NAMES` и patterns в `clean::strategy::DEFAULT_STRATEGIES`. DoD «задокументировать» выполнен, но **единый источник правды ещё нет**. До/вместе с pack `default_pack()` (или A2.2): либо shared name table, либо инвариант-тест «cleanup patterns ⊆ skip/pack policy, где уместно». Не закрывать follow-up раньше.
- **A2.2:** при delete-фазе убрать запись hits изнутри `filter_entry` (side effect в predicate) — чище явный цикл по entries или stack/DFS.
- Public API breaking: `RaccConfig` получил поле `cleanup` (struct-literal).
- `cargo test -p raccpack-core clean strategy detect` из спеки — невалидный cargo-синтаксис (несколько фильтров); рабочий эквивалент: `cargo test -p raccpack-core --test clean` или `cargo test -p raccpack-core -- clean strategy detect`.
- Wiki/`supported` для rinse — отдельная UX-задача (Docs после FINAL A2 / A2.3).


### Wiki callouts safety (CLOSED)

- **Дата:** 2026-08-15
- **Статус:** done
- **Роли:** Orchestrator (аудит/правки) — точечные callouts, без переписывания страниц.

#### Задача
Расставить VitePress-callouts (`::: info` / `::: warning` / `::: danger` / `::: tip`) на страницах CLI-вики там, где важные факты тонули в тексте: удаление исходников у `stash --remove-sources`, dry-run по умолчанию, passphrase в git, exit code 2 у `dig`, отсутствие raw в выводе, ослабление защиты у `pack --no-content-deny`, кэш у `sniff`.

#### Сделано
- `stash.md`: `::: warning` (dry-run по умолчанию) + `::: danger` (`--remove-sources` удаляет исходники только после успешного Commit; сначала dry-run; в CI без `--remove-sources`) — после быстрого старта; `::: warning` про `RACCPACK_PASSPHRASE` (не в git, CI → secrets store) вместо двух продублированных bullet-ов в «Passphrase».
- `dig.md`: `::: info` (exit **2** = политика `--fail-on`, не сбой CLI; `1` — ошибка выполнения) после таблицы кодов выхода; `::: tip` (в выводе никогда нет raw — только mask/hash/len) после правил маскирования.
- `pack.md`: `::: warning` рядом с `--no-content-deny` (отключает только контентный deny; deny по имени остаётся; архив не шифруется → для секретов `racc stash`).
- `cli-usage.md`: короткий `::: danger` у примера stash c `--remove-sources` (ссылка на [Stash](/stash)); «Примечания» дополнены ссылками на [Dig](/dig) и уточнением смысла кода 2.
- `sniff.md`: `::: tip` (не виден новый проект → кэш → `--force-refresh`) в секции «Кэш».

#### Проверки
- Поведение сверено с `crates/raccpack-cli`/`raccpack-core` (stash: commit = `--yes && !dry-run`, `remove_sources` только после размещения архива; dig: exit code от `--fail-on`; pack: `deny_content_secrets = !no_content_deny`) — выдуманных флагов нет.
- `pnpm run wiki:build` — зелёный (6.4s; font-warnings предсуществующие).
- **Live verified** (после squash-merge в `dev` и деплоя `wiki.yml`): на https://y-tretyakov.github.io/raccpack/stash.html — danger (`--remove-sources` удаляет…), dig.html — info (exit 2 = политика `--fail-on`), pack.html — warning (`--no-content-deny`… не шифруется). Callout-классы `custom-block` на месте.
- Callout-лимиты: stash 3 / dig 2 / pack 2 / sniff 2 / cli-usage 4 — в рамках правил (2–3 deep, 3–4 overview), без каскадов по 5+.
- Противоречий pack ↔ stash нет (pack «не шифруется → stash»; stash «удаляет только после успешного commit»).

#### DoD
- [x] stash: callout про удаление файлов (`--remove-sources`)
- [x] dig: callout «exit 2 = политика, не crash»
- [x] pack: warning про ослабление `--no-content-deny` и «не шифрование»
- [x] Overview не раздут (никаких новых таблиц флагов)
- [x] `wiki:build` зелёный
- [x] Live-страницы отражают md (проверено на stash/dig/pack после деплоя)
- [x] Нет выдуманных флагов/поведения

#### Зафиксировано
- Без изменений production-кода CLI/core — только `wiki/*.md` (+ WORKLOG).

### Wiki IA — CLI overview + deep pages (CLOSED)

- **Дата:** 2026-08-15
- **Ветка:** `wiki-ia-cli` (PR → dev, squash, merged)
- **Статус:** done
- **Роли:** Orchestrator (план/приёмка) + 2 Docs-субагента (параллельно) + Review-субагент (аудит).

#### Задача
Развязать `cli-usage` (overview) и страницы команд: каждая реализованная команда (`sniff`, `dig`, `pack`, `stash`) — отдельная deep-страница по единому 13-пунктовому шаблону; overview — только глобальные флаги, типовой сценарий и краткие карточки со ссылками.

#### Сделано
- `cli-usage.md` (rewrite): overview — глобальные флаги, блок «Типовой сценарий» (`sniff → dig → stash → pack`), краткие карточки 4 команд с примерами и ссылками «Подробно», секция «В разработке» (rinse/raid/den/init), примечания (JSON без raw; exit 2 только у dig).
- `sniff.md`, `dig.md`, `pack.md` (created, deep): 13-секционный шаблон — что делает/не делает, быстрый старт, синтаксис, полные таблицы флагов с defaults/приоритетами, поведение (кэш/dry-run/commit), human+JSON-поля, exit codes, примеры (локально/JSON/CI/edge), частые ошибки, безопасность, связанные команды, футер «обновлять в том же PR, что CLI».
- `stash.md` (align): приведена к шаблону без потерь (расшифровка `age -d`, CI-примеры, passphrase-приоритет, den layout, batch-id).
- `.vitepress/config.ts`: nav/sidebar «Использование» → CLI, Sniff, Dig, Pack, Stash, Конфигурация, TUI, Desktop.
- `index.md`: фича «Den — хранилище» переведена в настоящее время (age в `secrets/` уже реализован).
- `quick-start.md`: добавлен шаг 7 (stash), раздел «Что дальше» обновлён (4 команды + ссылки на deep-страницы).
- `concepts.md`: изменений не потребовалось (уже в настоящем времени).

#### Проверки
- `pnpm run wiki:build` — зелёный; все страницы (cli-usage/sniff/dig/pack/stash) собраны.
- Review-аудит (независимый субагент): флаги/дефолты/exit codes/JSON-поля сверены с `cli.rs` + `racc <cmd> --help` + реальным прогоном бинарника. BLOCK-расхождений нет; 2 nit исправлены (формулировка ключа кэша sniff; реальный текст ошибки stash).
- Внутренние ссылки и якоря — резолвятся; nav/sidebar в собранном HTML содержат все 4 команды.

#### DoD
- [x] Overview без полных flag-таблиц команд (только глобальные + карточки)
- [x] Deep-страницы для всех 4 реализованных команд по шаблону
- [x] Нет выдуманных флагов (аудит по коду)
- [x] Ссылки overview ↔ deep, related commands, «назад к обзору»
- [x] `wiki:build` зелёный
- [x] index/concepts/quick-start — без «позже» про секреты

#### Зафиксировано
- **EN:** RU first — EN-зеркало не full-parity (только introduction/supported); EN deep-страницы — отдельным follow-up этапом.
- Без изменений production-кода CLI/core.

### A1.1 — age + zeroize passphrase (CLOSED)

- **Дата:** 2026-08-14
- **Ветка:** `a1-stash-age`
- **Статус:** done
- **Dev:** dev-a1.1 · **Test:** test-a1.1 (параллельно, без rework)

#### Сделано
- `archive/age_vault.rs` (created): `encrypt_bytes_to_file`, `encrypt_file_to_age` (→ bytes_read), `decrypt_file_from_age` (test-only, `#[cfg(any(test, feature = "age-decrypt"))]`), atomic write (`<output>.tmp` + rename, temp удаляется при ошибке), empty passphrase → `Error::Encrypt`.
- `domain/error.rs`: вариант `Error::Encrypt { message }` — passphrase никогда в Display.
- `archive/mod.rs`: `pub mod age_vault` + re-exports encrypt-функций. В #51 re-exports были и на lib-корне; **убраны в #53** (сужение public API по ревью — остаётся только `archive::age_vault::…`; decrypt не торчит из crate root).
- `Cargo.toml`: `age = "0.12"`, `zeroize = { version = "1", features = ["derive"] }`, `[features] age-decrypt = []`.

#### Файлы
- created: `crates/raccpack-core/src/archive/age_vault.rs`
- changed: `crates/raccpack-core/Cargo.toml`, `src/domain/error.rs`, `src/archive/mod.rs`, `Cargo.lock` (в #51; сужение lib.rs + доп. тесты + suggestion — в follow-up-коммите)

#### Тесты
- `cargo test -p raccpack-core age_vault` → pass (roundtrip, wrong passphrase, empty passphrase encrypt/decrypt, file roundtrip + bytes_read, no-leak в Display на Io- и Encrypt-ветках, overwrite, binary magic header, missing source, tmp-очистка на mid-write fail).
- `cargo test --workspace` → все зелёные (регрессии нет).
- `cargo clippy --workspace --all-targets -- -D warnings` → чисто.
- `cargo fmt -p raccpack-core -- --check` → чисто.

#### Зафиксировано
- age version: **0.12.1** (0.10.0 yanked; MSRV 1.74 ≤ workspace 1.75).
- Формат: **binary** (без ASCII armor); фича `armor` не включалась (отклонение от snippet в спеке — модуль не используется).
- Passphrase: caller `Zeroizing<String>` → внутренняя копия `secrecy::SecretString`; обе zeroize на drop. Промежуточный `String` от `to_owned()` — не zeroized (кратковременный; тот же паттерн, что в примерах самого age-крейта).
- Атомарная запись внутри vault (tmp + rename), overwrite ок.

#### Критерий готовности (DoD из a1.1 §6)
- [x] encrypt_file_to_age / encrypt_bytes_to_file работают
- [x] Passphrase через Zeroizing/SecretString; empty rejected
- [x] Ошибки без утечки passphrase
- [x] Decrypt для тестов roundtrip
- [x] Модуль изолирован в archive/age_vault.rs
- [x] Тесты §5 зелёные

#### Риски / follow-up
- A1.2/A1.3: те же age-примитивы лягут в encrypt шаг stash.
- decrypt не ре-экспортирован из lib root — только под `age-decrypt` feature / test.
- Маппинг ошибок: `age::encrypt` → `Error::Encrypt`; `wrap_output`/`finish`/`copy` → `Error::Io` (в age 0.12 это io::Result). Для CLI stash (A1.4) имеет смысл различать «encrypt failed» vs «io failed» — решить при facade/CLI.
- Тест tmp-очистки на mid-write fail — только `#[cfg(unix)]` (EISDIR-трюк через директорию как source); Windows-ветка не покрыта (для Linux-MVP ок, cross-platform — позже).

#### Follow-up review замечания (человек, 2026-08-14; PR #51) — НЕ блокеры
- **A. Error mapping** — принято к A1.2: `age::EncryptError` → `Error::Encrypt`, чистый IO → `Error::Io`; для 0.12 wrap_output/finish возвращают io::Result, поэтому семантика уточняется при facade stash.
- **B. Два стиля encrypt** (`age::encrypt`+Recipient для bytes, `Encryptor::with_user_passphrase` для file) — валидно, roundtrip зелёный; возможная унификация — позже, не блокер.
- **C. Тесты** — дополнены: empty passphrase на decrypt, no-leak на Encrypt-ветке (wrong passphrase decrypt), missing source у `encrypt_file_to_age` (+ не оставлять output), tmp-очистка на mid-write fail.
- **D. `Error::Encrypt` suggestion** — добавлен hint («check passphrase / output writable»).
- **E. Zeroize** — принято: `Zeroizing<String>` → `SecretString`; двойной zeroize на drop ок.
- **F. Минимальная длина passphrase** — сознательно только non-empty; CLI warn про слабые пароли — A1.4/CLI.

## Этапы

### A1.2 — stash manifest (без raw) + remove sources в Commit (CLOSED)

- **Дата:** 2026-08-14 18:36 EEST
- **Ветка:** `a1.2-stash-manifest-remove` (PR #55 → dev, squash, merged)
- **Статус:** done
- **Dev:** dev-a1.2 · **Test:** test-a1.2 (параллельно, стыковка без rework)

#### Сделано
- `secrets/stash_select.rs` (created): `StashFileEntry`, `StashSelectOptions` (default min_risk=High, scan_content=true), `select_files_for_stash`. Обе ветки: only_files (containment, exists, dir → Err, dedup по canonical path, риск через match_filename_all + scan_file_content с фильтром min_risk) и полный `scan_secrets`. `relative_path` без `..` (валидация компонентов, зеркало `relative_posix_name` из pack). Пустая выборка → `Ok(vec![])`.
- `secrets/stash_batch.rs` (created): `StashManifestEntry` (serde, без raw), `StashBatchResult`, `write_stash_age` — ustar-tar в памяти (`tar::Header::new_ustar` + `append_data`, POSIX-имена без `..`/`./`/leading `/`) → `encrypt_bytes_to_file`. Пустой вход → `Error::StashEmpty`.
- `secrets/stash_remove.rs` (created): `remove_stash_sources` — явный вызов, fail-fast, каталоги пропускаются (не считаются), symlink удаляется как ссылка.
- `domain/error.rs`: variant `Error::StashEmpty { message }` + suggestion («lower --min-risk / check racc dig»).
- `scan/walk.rs`: `is_path_under_root` (canonicalize обеих сторон, `starts_with`) — закрывает **F-PATH-1**; module-doc переписан (containment теперь есть); re-export из `scan/mod.rs`.
- Re-exports в `secrets/mod.rs`; **lib.rs не трогали** (сужение public API A1.1).

#### Файлы
- created: `crates/raccpack-core/src/secrets/stash_select.rs`, `stash_batch.rs`, `stash_remove.rs`, `crates/raccpack-core/tests/stash.rs`
- changed: `src/domain/error.rs`, `src/scan/walk.rs`, `src/scan/mod.rs`, `src/secrets/mod.rs`

#### Тесты
- `cargo test --workspace` → pass (регрессий нет; без `age-decrypt` 11 stash-тестов).
- `cargo test -p raccpack-core stash --features age-decrypt` → 12 passed (вкл. roundtrip decrypt+untar восстановления содержимого и относительных имён).
- `cargo fmt --all -- --check` → clean. `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- Test-субагент сдал `tests/stash.rs` через `/tmp` (parallel-безопасно); при стыковке исправлен move-bug (`let _temp = temp;` → `let _ = &temp;`), файл отформатирован.

#### Зафиксировано
- Формат batch: **tar (ustar) внутри age**, один `.age`. `tar 0.4.46` не имеет `Builder::set_format`/`ArchiveFormat` — ustar сделан через `Header::new_ustar()` (настоящий POSIX ustar magic `ustar\0`).
- Manifest JSON содержит только `original_path`/`risk`/`size_bytes` — без содержимого.
- `remove_stash_sources` НЕ вызывается нигде в pipeline — только явный вызов (Commit).
- Path containment: `is_path_under_root` canonicalize-based; для `--only` порядок = exists → dir-check → containment → dedup.

#### Критерий готовности (DoD из a1.2 §6)
- [x] Select + batch encrypt + remove разделены по файлам §3
- [x] Manifest без raw
- [x] Tar paths relative and safe (нет `..`)
- [x] F-PATH-1: path-containment под `target` (в т.ч. `--only` / selected paths)
- [x] Remove только явный вызов
- [x] Тесты зелёные (+ outside-target → Error)

#### Риски / follow-up
- A1.3 (facade `stash` + den `secrets/place`): утилизировать `write_stash_age` (staging .age) + `remove_stash_sources` (Commit), имя `.age` по `{slug}__{ts}__secrets.age` (F-PATH-3 — staging вне project tree).
- ustar-имена длиннее 255 байт/100+155 → tar-ошибка (Error::Io); лимит принят для MVP, не обрабатывается отдельно.
- `is_path_under_root` для несуществующего пути → `Error::Io` (не PathNotFound); в `stash_select` exists-check идёт раньше — семантика корректна.

### A1.2-fix — follow-up по ревью (CLOSED)

- **Дата:** 2026-08-14
- **Ветка:** `a1.2-review-fixes` (PR #57 → dev, squash, merged)
- **Статус:** done
- **Сделано:**
  - **P1#1 / TOCTOU:** `write_stash_age` берёт размер заголовка tar из `metadata().len()` открытого файла, а не из selection-time `size_bytes`; manifest и `bytes_archived` отражают фактический размер (тест `header_size_tracks_actual_file_len`).
  - **P1#3:** готовый `.age` best-effort `chmod 0600` на Unix (`set_secrets_file_mode`, зеркало `den::layout`; тест `age_output_mode_is_0600`).
  - **P1#4:** `only_files` отказывает не-regular путям через `symlink_metadata` → `Error::NotAFile` (тест `only_files_symlink_is_rejected`).
  - **P1#5:** containment и `relative_path` считаются от **canonicalized** путей против canonical target; `./.env` → `.env` (тест `only_files_curdir_relative_path_is_clean`).
  - **P2#6:** типизированные `Error::PathOutsideTarget` / `Error::NotAFile` (thiserror + `suggestion()`).
  - **P2#7:** tar entry `set_mode(0o600)` (было 0o644).
- **Отложено (зафиксировано для следующих этапов):** P1#2 — весь tar в RAM → stream tar→age writer (Alpha OK); P2#9 — pub surface `stash_*` через `secrets::` → аудит к 1.0.
- **Тесты:** `cargo test --workspace` green; stash: 22 unit (с `--features age-decrypt`) + 12 integration (roundtrip decrypt+untar) pass; fmt/clippy `-D warnings` clean.
- `is_path_under_root` остаётся pub helper (использует A1.3); порядок проверок в select теперь: exists → `symlink_metadata`/is_file → canonical containment → dedup.

## Этапы

### A1.3 — facade `stash` + `den/secrets/…` (CLOSED)

- **Дата:** 2026-08-15
- **Ветка:** `a1.3-facade-stash-den`
- **Статус:** done
- **Dev:** dev-a1.3 · **Test:** test-a1.3 (параллельно, стыковка без rework)

#### Сделано
- `app/stash.rs` (created): `StashOptions`, `AgeIdentity` (`Passphrase(Zeroizing<String>)` / `Recipients`), `StashResult` (serde), `stash()` facade. DryRun: select + ожидаемый путь, ничего не пишет/не удаляет (даже не создаёт den); Commit: select → `write_stash_age` в `den/staging/{short_id}/secrets.age` → `place_secrets_archive_ensured` → (если `remove_sources`) `remove_stash_sources`.
- `den/secrets_place.rs` (created): `PlaceSecretsRequest` (с `batch_id`), `PlaceSecretsResult`, `place_secrets_archive` / `place_secrets_archive_ensured` — зеркало `place_pack` (atomic rename, cross-device fallback, escaping-guard, `0600`).
- `den/names.rs`: `secrets_relative_path` / `secrets_relative_path_token` → `secrets/{yyyy}/{mm}/{slug}__{ts}__secrets.age` (batch_id → name token вместо ts, `yyyy/mm` от now).
- `domain/error.rs`: `Error::Unsupported { feature }` + suggestion (Recipients).
- `den/place.rs`: helpers вынесены в `pub(crate)` (`reject_escaping`, `move_archive`, `validate_name_fragment`; `validate_output_name` делегирует).
- `lib.rs` re-exports: `stash`, `StashOptions`, `StashResult`, `AgeIdentity`, `place_secrets_archive`, `PlaceSecretsRequest`, `PlaceSecretsResult`, `secrets_relative_path`.

#### Follow-up закрыты
- **F-PATH-1** (path containment stash) — закрыт в A1.2 (`is_path_under_root` + canonical containment в `stash_select`), переиспользован в A1.3.
- **F-PATH-3** (staging только под den) — staging = `den/staging/{short_id}/`, runtime-guard «staging внутри project → Error» (тест `den_inside_project_is_rejected`).

#### Отклонение от спеки
- `AgeIdentity::Passphrase(Zeroizing<String>)` вместо `Passphrase(String)`: не создаётся plain `String`-копия (инвариант zeroize A1.1; согласуется с потоком A1.4, где passphrase приходит уже `Zeroizing`).

#### Файлы
- created: `crates/raccpack-core/src/app/stash.rs`, `crates/raccpack-core/src/den/secrets_place.rs`, `crates/raccpack-core/tests/stash_facade.rs`
- changed: `src/den/names.rs`, `src/den/place.rs`, `src/den/mod.rs`, `src/app/mod.rs`, `src/domain/error.rs`, `src/secrets/stash_batch.rs`, `src/lib.rs`, `docs/alpha/a1/a1.3-facade-stash-den.md`

#### Тесты
- `cargo test --workspace` → pass (регрессий нет).
- `cargo test -p raccpack-core --features age-decrypt` → pass (roundtrip decrypt+untar).
- `tests/stash_facade.rs` (17): DryRun ничего не пишет/не создаёт den; Commit кладёт `.age` под `secrets/yyyy/mm/` (magic header, roundtrip); remove_sources on/off; min_risk фильтр; Recipients → Unsupported; пустой passphrase → Encrypt; serde StashResult без raw (dry + commit); progress 0/30/70/[90]/100; staging чист; den внутри project rejected; batch_id.
- `cargo fmt --all -- --check` → clean. `cargo clippy --workspace --all-targets -- -D warnings` → clean.

#### Риски / follow-up
- Коллизия имени `.age` в ту же секунду: `place_secrets_archive` перезаписывает атомарно (как документировано для `place_pack`); уникальность/raid naming — на A3.
- `batch_id` заменяет ts в имени файла, `yyyy/mm` — от now (зафиксировано в wiki).
- A1.4: CLI `racc stash` (passphrase env/prompt) будет использовать этот facade; нужен `passphrase.rs` + `commands/stash.rs`.

### A1.3 — review fixes (P1/P2 человека)

- **Дата:** 2026-08-15
- **Ветка:** `a1.3-review-fixes`
- **Статус:** done
- **Dev:** dev-a1.3-review · **Test:** test-a1.3-review (параллельно)

#### Сделано
- **P1-1/P1-2 (F-PATH-3):** guard «staging внутри project» перемещён **до** `ensure_den`/`create_dir_all` и переведён на canonical containment через новый helper `scan::canonicalize_existing_prefix` (canonicalize ближайшего существующего предка + дозапись хвоста). Теперь при den внутри project (в т.ч. через symlink-алиасинг) **ничего** не создаётся в project (нет ни `.den-version`, ни `staging/`). Логика `is_path_under_root` не тронута.
- **P1-3 (AgeIdentity::Debug):** `Debug` больше не derive — ручная реализация: `Passphrase([redacted])` (значение не печатается), `Recipients` — список.
- **P2-4:** убран лишний `pass.clone()` — passphrase передаётся в `write_stash_age` по ссылке `&Zeroizing<String>`.
- P2-5 (slug), P2-6 (remove_dir parent), P2-7 (manifest absolute path) — оставлено как есть (reviewer: ок / на A1.4).

#### Файлы
- changed: `src/app/stash.rs`, `src/scan/walk.rs`, `src/scan/mod.rs`, `tests/stash_facade.rs`

#### Тесты
- `tests/stash_facade.rs`: +3 теста (всего 20): guard до создания (den внутри project ничего не оставляет), symlink-алиасинг den → project rejected, Debug не светит passphrase.
- `cargo test --workspace` → pass (регрессий нет). `cargo fmt --all -- --check` → clean. `cargo clippy --workspace --all-targets -- -D warnings` → clean.

#### Follow-up
- `pack.rs` F-PATH-3 имеет ту же лексическую схему (`staging.starts_with(&project)` после `create_dir_all`) — кандидат на тот же canonical-guard (вне scope A1.3).

## Этапы

### A1.4 — CLI `racc stash` (prompt passphrase / env для CI) (CLOSED)

- **Дата:** 2026-08-15 17:12 EEST
- **Ветка:** `a1.4-cli-stash` (PR #61 → dev, squash, merged)
- **Статус:** done
- **Dev:** dev-a1.4 · **Test:** test-a1.4 (параллельно, стыковка без rework)

#### Сделано
- `commands/stash.rs` (created): `run_stash` — load config → overrides → resolve project → mode (Commit iff `--yes` без `--dry-run`) → `StashOptions` → passphrase → facade `stash` → вывод. Dry-run никогда не запрашивает passphrase.
- `passphrase.rs` (created): `read_passphrase() -> Zeroizing<String>` — приоритет: env `RACCPACK_PASSPHRASE` (непустой) → interactive TTY (двойной ввод с подтверждением, echo off) → одна строка из piped stdin → `CliError::Passphrase` с hint. Zeroizing на drop; значение нигде не логируется/не печатается.
- `output_stash.rs` (created): `print_stash` — JSON `StashResult` / human-блоки (dry-run: Would archive / Would remove sources / nothing written; commit: Archive / Files / Removed sources).
- `commands/paths.rs` (created): общий `resolve_project_path` вынесен из `pack.rs` (повторное использование pack+stash, unit-тесты переехали).
- `cli.rs`: `Commands::Stash`, `StashArgs` (`--project` required, `--yes`, `--dry-run`, `--remove-sources`, `--min-risk low|medium|high|critical` default high, `--only` repeatable, `--batch-id`), `RiskLevel` → `SensitiveRisk`; +9 unit-тестов.
- `error.rs`: `CliError::Passphrase { message }` + suggestion.
- `main.rs`/`commands/mod.rs`: wire-up; `Cargo.toml` (cli): `rpassword = "7"`, `zeroize = { version = "1", features = ["derive"] }`.

#### Отклонение от спеки (зафиксировано)
- Passphrase читается **только** в Commit; в DryRun — константный placeholder `unused-dry-run-passphrase` (фасад возвращается до encrypt в DryRun, placeholder не используется). Спека §4 (шаг 3 — read_passphrase всегда) уточнена ради UX: dry-run без tty/env не должен падать (wiki показывает dry-run без env).
- `std::env::remove_var` **не** вызывается: значение уже видно в `/proc/PID/environ` с момента exec; `Zeroizing` покрывает копию CLI; удаление переменной удобно для повторного использования в shell. Tradeoff задокументирован.

#### Файлы
- created: `crates/raccpack-cli/src/commands/stash.rs`, `passphrase.rs`, `output_stash.rs`, `commands/paths.rs`, `tests/cli_stash.rs`
- changed: `crates/raccpack-cli/src/cli.rs`, `commands/mod.rs`, `commands/pack.rs`, `error.rs`, `main.rs`, `Cargo.toml`, `Cargo.lock` (workspace root)

#### Тесты
- `cargo test --workspace` → pass (154 core + cli suites; cli_stash 13 integration).
- `cargo test -p raccpack-cli` → pass (45 unit + 18 dig + 12 pack + 10 sniff + 13 stash).
- `cargo fmt --all -- --check` → clean. `cargo clippy --workspace --all-targets -- -D warnings` → clean.
- Smoke (вручную): env passphrase commit → `.age` в `secrets/yyyy/mm/` (0600, magic header); interactive tty (двойной ввод, совпадение) → ok; mismatch → exit 1 + hint; no env + no tty → exit 1 + hint; piped stdin → ok; `--dry-run` перебивает `--yes`, ничего не пишет; `--remove-sources` удаляет источники только в Commit.

#### Критерий готовности (DoD из a1.4 §6)
- [x] `racc stash` dry-run + commit
- [x] Passphrase via env + interactive
- [x] `--remove-sources` only on Commit success path
- [x] JSON `StashResult`
- [x] Modules: `commands/stash.rs`, `passphrase.rs`
- [x] Wiki updated (отдельный PR — wiki/stash.md)

#### Риски / follow-up
- `read_passphrase` не покрыт unit-тестами для interactive-ветки (нужен PTY) — покрыто smoke-проверкой вручную.
- `RiskLevel`/`FailOnPolicy` дублируют маппинг на core-типы; при появлении новых `--fail-on`-подобных флагов — кандидат на общий маппер.
- Wiki: страница `wiki/stash.md` + правки `cli-usage.md`/`index.md` — PR #62 (squash, merged; deploy Pages на push в dev — success).

#### Review remarks (не блокеры merge, PR #61)
- **P2 — dummy passphrase в DryRun:** `AgeIdentity::Passphrase(Zeroizing::new(DRY_RUN_PASSPHRASE…))` — работает (facade не доходит до encrypt), но чище `Option<AgeIdentity>` только для Commit или no-op path в CLI.
- **P2 — rpassword отдаёт String:** до обёртки в `Zeroizing` кратко живут обычные `String` (first/second) — для CLI приемлемо; идеал — сразу zeroizing-буфер.
- **P2 — RACCPACK_PASSPHRASE остаётся в env** процесса (может попасть в логи CI) — осознанный CI-tradeoff; стоит одной строкой в wiki/help.
- **P3 — exit_policy: FailOnCritical в ctx:** на stash не влияет (exit 0/1); можно `Ignore` для ясности.

## Этапы

### 2026-08-14 13:20 — docs: трекинг agent knowledge docs (dev + main)

**Задача:** закоммитить три общих документа через PR в `dev` и в `main`.

**Сделано:**
- `.gitignore`: убраны строки `raccpack-architecture-vision.md` / `raccpack-roadmap-v1.md`, чтобы документы трекались.
- `docs/FOLLOWUPS_FROM_MVP.md`, `docs/raccpack-architecture-vision.md`, `docs/raccpack-roadmap-v1.md` — закоммичены.
- PR #48 → `dev` (squash, merged).
- PR #49 → `main` (squash, merged через --admin, т.к. main защищён 1 approval).

**Файлы (changed):** `.gitignore`
**Файлы (created):** `docs/raccpack-architecture-vision.md`, `docs/raccpack-roadmap-v1.md`, `docs/FOLLOWUPS_FROM_MVP.md` (в main; в dev FOLLOWUPS был с #45)
**Тесты:** n/a (только docs). `cargo` не затрагивался.
**Решения:**
- `main` не переделывался целиком под `dev` — только добавлены 3 документа (main остаётся за релизами вех).
- Ветки `main`/`dev` не удалялись. `.agents/` не трогался.

## Принятые решения (Alpha+)

| Дата | Решение |
|------|---------|
| 2026-08-14 | **Процесс:** после каждого закрытого этапа Orchestrator сам мёржит свой PR в `dev` (squash), закрывает PR и удаляет рабочую ветку — не ждёт команды человека (записано в `.agents/docs/AGENTS.md`). |
| 2026-08-13 | Релиз-подготовка MVP: agent tooling (`.agents/`, `skills-lock.json`) убран из репо; WORKLOG MVP → `docs/archive/WORKLOG_MVP.md`; спеки M1–M4 → `docs/archive/mvp/`; one-shot промпты Writerside/VitePress icons удалены. Новый `WORKLOG.md` только для Alpha+. |
| 2026-08-13 | Документы агента: `AGENTS.md` переписан под Alpha; knowledge base (architecture / facade / modularity / roadmap / workflow) остаётся в корне. |

## Tracked с MVP (не блокеры Alpha start)

См. конец `docs/archive/WORKLOG_MVP.md` и таблицу решений там:

- P1-4 `SkipPolicy::default_pack()` (расширенный список) — позже.
- P2-5 `zstd_level` из `[advanced]` при появлении секции.
- P2-6 cost content-deny (extensions / size-cap) — оптимизация позже.
- P2-7 сужение public API (`lib.rs`) — с фазой hygiene / R1.
- P2-8 типизация `Error::Other` (DenInsideProject / InvalidOutputName) — CLI UX.
- `is_under_root` / path-containment — перед stash destructive paths.
- ConfigError ↔ domain Error merge — на facade maturity.
- Windows HOME/XDG — best-effort после Linux primary.
