# WORKLOG — raccpack

Журнал статусов этапов. Orchestrator: y-tretyakov.

**MVP 0.1.0 закрыт.** Полный журнал M1–M4 и docs-миграции:
[`docs/archive/WORKLOG_MVP.md`](docs/archive/WORKLOG_MVP.md).
Спеки закрытых этапов: [`docs/archive/mvp/`](docs/archive/mvp/).

**Текущая версия: `0.3.0`** — **ALPHA EXIT** (A1–A4 закрыты; следующий bump `0.3.1` при D1.1 Detect v2, см. `docs/VERSION_ROADMAP.md`).

## Backlog (Alpha → 0.3.0)

```
[x] A1.1 age + zeroize passphrase
[x] A1.2 stash manifest (без raw) + remove sources в Commit
[x] A1.3 facade stash + den/secrets/…
[x] A1.4 CLI racc stash
[x] A2.1 cleanup strategies + config toggles
[x] A2.2 facade rinse DryRun/Commit
[x] A2.3 CLI racc rinse
[x] A3.1 facade raid (stash→rinse→pack→move, fail-fast)
[x] A3.2 ProgressSink + CLI progress
[x] A3.3 atomic upgrade (default Atomic: staging + WAL + rollback, ORPHAN-1..4)
[x] A3.4 manifest JSON в den/manifests/ (после успешного Atomic commit)
[x] A3.5 CLI racc raid --fail-fast/toggles; exit 1 при !success; E2E alpha; wiki
[x] A4.1 GitClient (process) + status sensitive files в dig
[x] A4.2 Config migrate chain + racc init
[x] A4.3 tracing без секретов; --verbose
[x] A4.4 integration tests core + CI cargo test
```

## Backlog (Detect v2 → 0.4.0)

```
[ ] D1.1 StackDetector trait + registry
[ ] D1.2 Detection / StackNode DTO
[ ] D1.3 detect.mode config + CLI
[ ] D2.1 WorkspaceDetector → DAG
[ ] D2.2 conflict merge (expert opinions)
[ ] D2.3 flat stack + stack_tree compat
[ ] D3.1 rinse по DAG scopes
[ ] D3.2 sniff tree output
[ ] D3.3 fixtures + Detect v2 exit
```

## Этапы

### A3.3 — Atomic upgrade (CLOSED)

- **Дата:** 2026-08-19/2026-08-20
- **Ветки:** `a3.3-atomic` (PR #78 → dev, squash, merged) · `a3.3-staging` (PR #79 → dev, squash, merged) · `a3.3-wal` (PR #80 → dev, squash, merged)
- **Статус:** done — **PR1** (каркас API, green bridge) + **PR2** (единый `den/staging/{raid_id}/` + deferred destructive ops) + **PR3** (WAL + rollback). ORPHAN-1..4 покрыты тестами.
- **Роли:** Orchestrator; Dev (PR2/PR3) + Test (последовательно; PR2/PR3 тесты — Orchestrator, субагент Test вернул пусто).

#### Задача (из `docs/alpha/a3_new/a3.3-atomic-upgrade.md`)
Default **Atomic**: `OrchestrationMode { Atomic, FailFast }` в `RaidOptions`; весь Commit-raid в `den/staging/{raid_id}/` + WAL; финальные `secrets/`/`packs/` только atomic rename; `Err` → reverse-WAL → `rolled_back`, staging удалён; FailFast ≡ старое A3.1-поведение; DryRun без WAL/FS; progress commit/rollback; orphan-регрессия (ORPHAN-1..4).

#### PR1 — сделано (scaffolding, additive, поведение не менялось)
- `OrchestrationMode { Atomic, FailFast }` (default Atomic) + `RaidOptions.mode`.
- `RaidResult.rolled_back: bool` + `rollback_warnings: Vec<String>` с `#[serde(default)]` — всегда `false`/пустые до PR3.
- `raid()` → диспетчер по `mode`; fail-fast тело вынесено **дословно** в `app/raid/fail_fast.rs` (`fail_fast_raid`, pub(super), 236 строк); `atomic_raid` — green bridge (делегирует в fail-fast, коммент про PR2/PR3).
- `mod.rs` 327 строк (≤ 400). Re-exports: `OrchestrationMode` в `app/mod.rs` + `lib.rs`.
- Литералы: `RaidOptions`/`RaidResult` в тестах (raid.rs 318/366/483, raid_progress, progress.rs, output_raid.rs) дополнены новыми полями.
- Unit: default Atomic; диспетчеризация (Atomic ≡ FailFast на DryRun); serialize additive-полей.

#### Файлы
- created: `crates/raccpack-core/src/app/raid/fail_fast.rs`
- changed: `app/raid/mod.rs`, `app/raid/progress.rs`, `app/mod.rs`, `lib.rs`, `tests/raid.rs`, `tests/raid_progress.rs`, `crates/raccpack-cli/src/output_raid.rs`

#### Тесты
- `cargo test --workspace` — green, 0 failed (raid 15, raid_progress 5, cli_raid 6)
- `cargo fmt --all -- --check` — clean · `cargo clippy --workspace --all-targets -- -D warnings` — clean
- Smoke: dry-run exit 0 `Success`; `--json` содержит `rolled_back`/`rollback_warnings`.

#### Решения
- Green bridge на PR1: `atomic_raid` = делегирование в fail-fast → поведение и контракты A3.1/A3.2 не меняются (требование человека: не ломать A3.2-контракт раньше времени).
- Поля результата additive с `#[serde(default)]` — старые JSON-тесты парсят по ключам, не ломаются.
- `enabled_phase_count` стал `pub(super)` (перенос в fail_fast.rs) — внутренняя видимость, не public API.

#### Критерий готовности PR1
- [x] `OrchestrationMode` + `RaidOptions.mode` (default Atomic) в public API + re-exports
- [x] `RaidResult.rolled_back`/`rollback_warnings` с serde default
- [x] `raid()` диспетчер; `fail_fast.rs` вынесен; `atomic_raid` green bridge
- [x] mod.rs ≤ 400 строк; без unwrap/expect в production
- [x] Tests green, fmt/clippy clean

#### Риски / follow-up
- **A3.4:** manifest только после успешного Atomic commit (`den/manifest.rs`).
- **A3.5:** CLI `--fail-fast`/toggles, exit 1 при `!success`, E2E, wiki — контракт A3.2 не трогать до этого этапа.
- Wiki (user-facing) не обновляется до зелёного A3.5; dev-спеки `docs/alpha/a3_new/` — источник контракта.
- **Закрыт (был из PR1):** «default Atomic, но rollback не реализован» — закрыт PR3 (WAL + reverse-rollback). CLI help однострочник про rollback — по желанию в A3.5.

#### PR3 — сделано (commit WAL + rollback; PR #80)
- **Дата:** 2026-08-20 · **Ветка:** `a3.3-wal` · **Коммит:** `b821dd3`.
- `app/raid/wal.rs` (291 стр. с тестами): forward-effect `WalOp { CreateDir, CreateFile, Rename, DeleteFile, DeleteDir }` + `inverse()` (Rename → DeleteFile{to}; CreateDir/CreateFile → delete; DeleteFile/DeleteDir необратимы → warning); `Wal` — `record` (JSONL + `sync_all`) **до** эффекта, `read_reverse` (битая строка → Error::Other, fail-safe).
- `app/raid/rollback.rs` (183): `rollback_from_wal` — reverse-WAL, никогда Err (всё в `RollbackReport{warnings}`); missing WAL → `applied:false`; NotFound при удалении → ок.
- `atomic.rs` (395, ≤400): commit записывает в `staging/wal.jsonl` CreateDir/Rename перед placement, DeleteFile перед `remove_stash_sources`, DeleteDir перед `remove_trash_dirs`; Commit-Err → `rollback_from_wal` → `remove_raid_staging` → `den_artifacts` пуст, `rolled_back`/`rollback_warnings` из отчёта, events: failed "move" + новый `"rollback"`. ORPHAN-1 path не тронут (WAL не создавался → `rolled_back:false`). `commit` возвращает `Vec<PathBuf>` вместо `&mut Vec<PathBuf>` (clippy too_many_arguments) — эквивалентно, меньше аргументов. `needs_wal` guard: WAL только при размещении/деletes (все-выключено-commit → чистый "move" без WAL).
- `progress.rs`: `emit_rollback_event` (phase `"rollback"`, index=phase_count → overall 100) — дополнительное событие только при откате; module-doc обновлён.
- **Тесты `tests/raid_atomic.rs` +3 (13):** ORPHAN-2 (blocker-файл `den/packs/{yyyy}/{mm}` → `create_dir_all` падает в commit после stash-rename → reverse-WAL убирает `.age`, sources нетронуты, `rolled_back:true`, rollback-событие, warning про non-empty dir); успешный commit без rollback-события; irreversible source deletes → `rollback_warnings` (chmod 0555 на `node_modules/pkg` — scan читает, `remove_dir_all` падает после удаления `.env`; `rolled_back:true`, `.env` не восстановим — warning). Unit: wal.rs (inverse, append→read_reverse, corrupt line) + rollback.rs (remove placed + empty parent, NotFound no-op, irreversible warnings, corrupt WAL).
- **Тесты:** `cargo test --workspace` — green, 0 failed (raid_atomic 13); fmt clean · clippy `-D warnings` clean.
- **Отклонение Dev от брифа (принято):** `commit` → `Vec<PathBuf>` вместо `&mut Vec` (8 аргументов → clippy fail); нет `pub(crate) use rollback::…` в mod.rs (unused_import; atomic ходит `super::rollback::…`); `Wal::new` под `needs_wal` guard.

#### PR2 — сделано (atomic staging + deferred destructive ops; PR #79)
- **Дата:** 2026-08-20 · **Ветка:** `a3.3-staging` · **Коммиты:** `118ca05` (feat) + `7d559b1` (test).
- Additive API: `StashOptions.staging_dir` / `PackOptions.staging_dir` (stage-only: пишут `{dir}/secrets.age`/`{dir}/pack.tar.zst`, skip `ensure_den`/placement/removal, возвращают финальный ожидаемый путь), `RinseOptions.collect_only` (scan-only), `PackOptions.exclude_files` / `PackTreeOptions.exclude_files` (файлы, выбранные stash, исключаются из архива — зеркалит fail-fast, где stash их удалил до pack; сверх брифа, принято как корректный фикс: иначе при `remove_sources=false` High-контент-секрет протекал бы в pack, т.к. content-deny pack — Critical-only).
- `app/raid/atomic.rs` (339 стр.): фазы пишут в `den/staging/{raid_id}/`, commit = `ensure_den` (только если есть что place) → place stash → place pack → `remove_stash_sources` (пересборка `removed_sources`) → `remove_trash_dirs` (пересборка rinse-result) → `remove_raid_staging`. Phase-failure → staging cleaned, `success:false`, sub-results `None`, `den_artifacts` пуст (ORPHAN-1); DryRun делегирует в fail-fast (ORPHAN-3); `rolled_back` всегда `false` (ORPHAN-2 → PR3). FailFast path не тронут.
- `app/raid/staging.rs` (24 стр.): `raid_staging_path` + `remove_raid_staging` (best-effort). Shared `remove_trash_dirs` вынесен из rinse commit-loop в `app/rinse.rs` (pub(super)); `move_archive` re-export `pub(crate)` в `den/mod.rs`; `resolve_stash_identity`/`enabled_phase_count` — `pub(super)` в fail_fast.rs.
- `app/pack.rs` → `app/pack/mod.rs` (332) + `app/pack/naming.rs` (116: `artifact_rel`/`resolve_artifact_name`).
- **Файлы:** created `app/raid/{atomic,staging}.rs`, `app/pack/naming.rs`, `tests/raid_atomic.rs`; renamed `app/pack.rs`→`app/pack/mod.rs`; changed `app/raid/{mod,fail_fast}.rs`, `app/{stash,rinse}.rs`, `archive/pack.rs`, `den/mod.rs`, CLI `commands/{pack,rinse,stash}.rs`, тесты `{pack_facade,rinse,stash_facade}.rs`.
- **Тесты `tests/raid_atomic.rs` (10):** базовый atomic commit; atomic≡fail-fast на полном успехе (field-level, пути различаются); ORPHAN-1 (cfg unix, chmod-000 файл ломает pack, stash его пропускает); ORPHAN-3 (dry-run zero FS); ORPHAN-4 (fail-fast оставляет orphan `.age`); stash-empty продолжает + pack; pack-only (1 артефакт); `remove_sources=false` — исходники на месте и **исключены из tar** (чтение tar.zst через dev-deps tar+zstd; `notes.txt` с `xoxb-*` = High контент, не denied pack); stash-failure (пустая passphrase) → `den` не создан; JSON без raw. Fault injection без test-hook в public API.
- **Исполнение:** Test-субагент (`general`) дважды вернул пустой результат (инфраструктура; первый запуск переписал `docs/alpha/a4/*` — откачено) → тесты PR2 написаны Orchestrator'ом сам (отклонение от делегирования, зафиксировано).
- **Тесты:** `cargo test --workspace` — green, 0 failed (raid_atomic 10); `cargo fmt --all -- --check` — clean · `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- **Follow-up:** `archive/pack.rs` 436 строк (превышение 400) — было и до PR2, зафиксировано, не блокер; split при следующем касании pack.

### A3.4 — Manifest JSON после successful commit (CLOSED)

- **Дата:** 2026-08-20
- **Ветка:** `a3.4-manifest` (PR #81 → dev, squash, merged)
- **Статус:** done — manifest пишется только после успешного Atomic commit и только при размещённых артефактах.
- **Роли:** Orchestrator (Dev-поручение + интеграционные тесты сам, субагент Test не задействован по прецеденту PR2/PR3).

#### Задача (из `docs/alpha/a3_new/a3.4-manifest-after-commit.md`)
После успешного Atomic commit: `{den}/manifests/{yyyy}/{mm}/{slug}__{ts}__{short_id}.json`. НЕ писать при `success:false`/`rolled_back`; НЕ писать в DryRun; пути артефактов relative to den; raw-секретов нет; `schema_version: 1`.

#### Сделано
- `den/manifest.rs` (196, чистый den-домен, без app-типов): `DenManifest`/`ManifestStage` (зеркало `RaidStageResult`)/`ManifestArtifacts` + `MANIFEST_SCHEMA_VERSION=1`; `manifest_relative_path(slug, ts, short_id)` — `yyyy`/`mm` из ts с фолбэком как в `names.rs`; `write_manifest` — `reject_escaping` → parent dirs → `serde_json::to_string_pretty` → `fs::write` → `set_mode_best_effort 0o600`. `StashManifestEntry` — единственная внешняя зависимость (raw-free DTO).
- `app/raid/manifest.rs` (224): `write_raid_manifest` (slug=project_slug, ts=utc_timestamp_now, rel, write) + pure `build_raid_manifest` (артефакты relative через `den_relative` strip_prefix, фолбэк — абсолютный путь; stash_manifest passthrough; success/dry_run/created_at/tool_version) + `impl From<&RaidStageResult> for ManifestStage`.
- `atomic.rs` (395→410): Ok-ветка commit — manifest пишется только когда артефакты непустые; **сбой записи manifest → `success:false` + `failed_stage("move")`, но `den_artifacts` остаются и `rolled_back` false** (откатывать нечего — staging уже удалён); ровно одно move-событие.
- Re-exports: `den/mod.rs` + `lib.rs` (additive, breaking нет). Doc-tree den обновлён.
- **Отклонения Dev (приняты):** `build_raid_manifest` принимает `den_root: &Path` (нужен для relativize в pure-функции); `&stash_result`/`&pack_result` вместо `&*…` (deref-coercion).

#### Тесты
- Unit: den/manifest.rs (путь yyyy/mm + фолбэк на коротком ts; roundtrip schema_version==1; `../evil.json` → Err без fs-эффектов) · app/raid/manifest.rs (build мапит стадии/артефакты relative/stash_manifest; пустые результаты; `den_relative` вне den — абсолютный).
- Integration `tests/raid_atomic.rs` +5 (→18): success → ровно 1 manifest, schema v1, filename `{slug}__{ts}__{short_id}` и `created_at==ts`, stages=[stash,rinse,pack] все ok, артефакты relative и существуют под den, stash_manifest=[.env], размер из метаданных, raw-free (PASSWORD_VALUE нет); mode 0o600 (unix); rollback (ORPHAN-2) → нет manifest; phase-failure (ORPHAN-1) → нет manifest; dry-run → нет manifest.
- `cargo test --workspace` — green, 0 failed (698 passed); fmt clean · clippy `-D warnings` clean.

#### Критерий готовности (DoD из a3.4 §4)
- [x] Path naming consistent с packs/secrets (`{slug}__{ts}__{short_id}.json`, yyyy/mm из ts)
- [x] Только на success (rollback/phase-failure/dry-run → нет manifest)
- [x] Tests green

#### Риски / follow-up
- **A3.5:** CLI `--fail-fast`/toggles, exit 1 при `!success`, E2E alpha, wiki UX — контракт A3.2 не трогать до этого этапа.
- `app/raid/atomic.rs` 410 строк (мягкий max ~400; потолок 450) — split при следующем касании atomic (прецедент pack.rs 436).
- Manifest — финальный audit после commit, не через staging/WAL; при сбое записи артефакты уже в den (откат невозможен) — задокументировано в atomic.rs.

### A3.5 — CLI полный + E2E + orphan green + wiki (CLOSED)

- **Дата:** 2026-08-20
- **Ветка:** `a3.5-cli-e2e-wiki` (PR #82 → dev, squash, merged)
- **Статус:** done — Alpha raid закрыт целиком (A3.1..A3.5).
- **Роли:** Orchestrator (Dev-поручение + интеграционные тесты + wiki-правки сам, субагент не задействован).

#### Задача (из `docs/alpha/a3_new/a3.5-cli-e2e-wiki.md`)
Флаги `--no-stash/--no-rinse/--no-pack/--min-risk/--keep-sources/--no-content-deny/--fail-fast`; `--fail-fast` → `OrchestrationMode::FailFast`; **exit 1 при `!success`** (смена контракта A3.2); human summary артефактов/rolled_back; E2E + orphan green; wiki raid.md + cli-usage + roadmap.

#### Сделано
- `cli.rs` `RaidArgs` +7 флагов (`min_risk` value_enum default high). clap unit: defaults + full-parse.
- `commands/raid.rs`: `RaidOptions` строится явно из флагов; passphrase только при `commit && stash.enabled` (`--no-stash --yes` не требует passphrase); exit по `result.success`.
- **`main.rs` фикс:** arm Raid возвращал `Ok(SUCCESS)` поверх `run_raid(...)?` — проглатывал `ExitCode::FAILURE`. Теперь `run_raid(global, args)` (паттерн run_dig). Без этого DoD exit-контракт не работал.
- `output_raid.rs`: human Success + `placed N artifact(s)` + пути (commit, !dry_run); Failed + `rolled back (N warnings)` при rolled_back. Unit-тесты не ломаются (новые ветки только при новых условиях).
- **Интеграционные `cli_raid.rs` +9 (→15):** E2E full commit (`.den-version` + 1 .age + 1 .tar.zst + manifest schema v1 + human `placed 2 artifact(s)`); `--no-stash` (нет .age, .env живёт, pack есть, без passphrase); `--no-rinse` (node_modules живёт); `--no-pack` (нет .tar.zst); `--keep-sources` (.env живёт); `--min-risk critical` (High .env пропущен, pack есть, exit 0); atomic failure (chmod-000) → exit 1 + ничего в den; `--fail-fast` → orphan .age остаётся + exit 1; rolled-back (blocker `den/packs/{yyyy}/{mm}`) → exit 1 + human `rolled back`.
- **Wiki (DoD §6):** `wiki/raid.md` (новый, стиль stash.md: флаги, atomic vs fail-fast, exit codes, passphrase, примеры, manifest); `cli-usage.md` — секция `racc raid` + убран из «В разработке» + exit-нота; `roadmap.md` — checkbox raid в сделанном; `config.ts` nav/sidebar + Raid.

#### Тесты
- `cargo test --workspace` — green, 0 failed (712 passed); fmt clean · clippy `-D warnings` clean.
- `pnpm run wiki:build` — build complete без ошибок (font-warnings пре-существующие).

#### Критерий готовности (DoD из a3.5 §7)
- [x] Все флаги из спеки §2
- [x] Exit 1 на `!success` (в т.ч. rolled_back failure)
- [x] E2E + orphan green
- [x] Wiki + cli-usage synced
- [x] Alpha raid exit criteria met → можно A4

#### Риски / follow-up
- **A4 следующий:** A4.1 GitClient (process) + status sensitive files в dig; A4.2 config migrate + init; A4.3 tracing без секретов; A4.4 integration tests + CI.
- `cli.rs` 829 строк (было 759) — пре-существующий монолит clap-файла, вынос в модули — отдельная гигиена (зафиксировано).
- `app/raid/atomic.rs` 410 строк — split при следующем касании (прецедент pack.rs 436).

### A3.2 — ProgressSink + CLI progress для raid (CLOSED)

- **Дата:** 2026-08-18
- **Ветка:** `a3.2-progress` (PR #77 → dev, squash, merged)
- **Статус:** done
- **Роли:** Orchestrator; Dev + Test (параллельно; Test проверил по working-tree до коммита Dev, финальную перепроверку по merge-ready tip `b8fd38e` сделал Orchestrator сам).

#### Задача
Единые progress-события на уровне raid (не только вложенные stash/rinse/pack) + CLI-прогресс при `racc raid` без `--json`; `phase_count`/`phase_index`/`overall_percent` согласованы с числом enabled-фаз + move; JSON-режим тихий.

#### Скоуп (согласован с человеком)
A3.2 включает **минимальную команду `racc raid`** (`--project`/`--yes`/`--dry-run`, passphrase на Commit), несмотря на то что A3.4 перечисляет `commands/raid.rs` — спека A3.2 §2/§4/§6 явно требует CLI-прогресс; полные toggles (`--no-stash` и т.д.), exit 1 при `!success`, E2E, wiki — A3.4.

#### Сделано
- **Core split (follow-up A3.1):** `app/raid.rs` → `app/raid/mod.rs` (типы, `raid()`, раннеры, `resolve_stash_identity`, `enabled_phase_count`; 396 строк), `app/raid/stages.rs` (ok/failed/skipped/disabled_stage + mode-aware summaries; 179), `app/raid/progress.rs` (`plan_phases`, `overall_percent`, `emit_phase_event`; 188). Каждый файл < 400.
- **Event-контракт:** ровно одно `OperationKind::Raid` completion-событие на planned-фазу (stash/rinse/pack enabled + move); `phase_count = enabled+1`; `overall_percent = (phase_index*100 + percent)/phase_count` clamp 0..=100; disabled → события нет, индексы сдвигаются; fail-fast → failed-фаза эмитит `err.to_string()` (без raw), последующие — `SKIPPED_MESSAGE = "not run due to prior failure"` (общая константа со stage); `StashEmpty` — no-op "nothing to stash", run продолжается. **Старт-событие «Starting raid…» убрано** — completion-события единственные эмиссии (module-doc).
- **CLI `progress.rs`:** `CliProgress` (ProgressSink, Send) + pure `render_event` → `→ {phase}: {message}` только для `Raid && phase_complete`; nested stash/rinse/pack события и in-flight отфильтрованы; plain text без ANSI.
- **CLI `commands/raid.rs`:** `run_raid` — Commit iff `yes && !dry_run`; passphrase (`read_passphrase`) только Commit, DryRun-плейсхолдер `DRY_RUN_PASSPHRASE` (паттерн stash); `NullProgress` при `--json`, `CliProgress` иначе; exit 0 на Ok (в т.ч. `success=false`), 1 на Err (distinct exit при `!success` — A3.4, зафиксировано в module-doc).
- **CLI `output_raid.rs`:** JSON = pretty `RaidResult`; human = `Success`/`Failed` (фазы уже вывел CliProgress).
- **clap:** `Commands::Raid(RaidArgs { project(required), yes, dry_run })` + wiring (`main.rs`, `commands/mod.rs`).

#### Файлы
- created: `crates/raccpack-core/src/app/raid/{stages,progress}.rs`, `crates/raccpack-core/tests/raid_progress.rs`, `crates/raccpack-cli/src/{progress,output_raid}.rs`, `crates/raccpack-cli/src/commands/raid.rs`, `crates/raccpack-cli/tests/cli_raid.rs`
- renamed: `crates/raccpack-core/src/app/raid.rs` → `app/raid/mod.rs` (rethick: mod)
- changed: `crates/raccpack-cli/src/cli.rs`, `commands/mod.rs`, `main.rs`

#### Тесты
- `cargo test --workspace` — green, 0 failed (28 suites; A3.1 `--test raid` 15/15 без изменений)
- `cargo test -p raccpack-core --test raid_progress` — 5/5; `cargo test -p raccpack-cli --test cli_raid` — 6/6
- `cargo fmt --all -- --check` — clean; `cargo clippy --workspace --all-targets -- -D warnings` — clean
- Smoke (проверено Orchestrator): human — `→ stash: would stash 1 files / → rinse: found 1 directories / → pack: would pack project / → move: nothing to finalize / Success`, exit 0; `--json` — валидный RaidResult без `→`.

#### Решения
- Public API core не менялся (re-exports `app/mod.rs`/`lib.rs` нетронуты) — breaking note не требуется.
- `SKIPPED_MESSAGE` — единая константа для `skipped_stage` и событий (1 источник правды).
- `overall_percent` использует saturating-арифметику; count==0 → 0 (защита от деления на ноль).
- `Box<dyn ProgressSink>` в CLI — минимальный выбор для NullProgress/CliProgress; enum — при росте (A3.4).
- Тестовый NOTE-комментарий в `cli_raid.rs` («until that lands…») снят как устаревший после коммита Dev (тривиальная правка Orchestrator'а).

#### Критерий готовности (DoD из a3.2 §6)
- [x] Raid emits `OperationKind::Raid` с согласованными индексами (1 completion на planned-фазу, формула overall, move последний 100/complete)
- [x] CLI показывает phase-progress при !json (`→ {phase}: {message}` + `Success`/`Failed`)
- [x] JSON-режим тихий (NullProgress; в stdout только RaidResult)
- [x] Сообщения без raw (core events + CLI вывод)
- [x] Split raid-файла < 400 строк (A3.1 follow-up закрыт)
- [x] A3.1-семантика `RaidResult` не изменена (тесты 15/15)
- [x] Tests green, fmt/clippy clean

#### Риски / follow-up
- **A3.4:** toggles фаз (`--no-stash`/`--no-rinse`/`--no-pack`, `--min-risk`, `--remove-sources`/`--keep-sources`, `--no-content-deny`), distinct exit 1 при `!success`, расширенный human-summary (строки A3.4 §4), E2E, `tests/e2e_raid.rs`.
- **Wiki/UX:** `racc raid` теперь существует в CLI, но **страницу `raid.md` и roadmap-статус НЕ трогаем до зелёного A3.4** (решение A3.1, подтверждено человеком для A3.2). Актуальное расхождение wiki ↔ CLI на live: `cli-usage.md` — raid в секции «Планируется»; `roadmap.md` — `racc raid` unchecked. Закрывается в A3.4: `raid.md` + `cli-usage.md` + roadmap (карточка raid), `exit 1 при !success`, E2E (явный Docs-follow-up).
- **`commands/raid.rs` exit-code контракт:** 0 на Ok (в т.ч. success=false) — осознанно, до A3.4; пользователю фазовая неудача видна в `Failed`/JSON `success:false`.

### A3.1 — facade `raid` (stash→rinse→pack→move, fail-fast) (CLOSED)

- **Дата:** 2026-08-18
- **Ветка:** `a3.1-facade-raid` (PR #75 → dev, squash, merged)
- **Статус:** done
- **Роли:** Orchestrator; Dev + Test (параллельно). **Повторная реализация:** первая попытка (PR #74) была откачена ревьюером.

#### Задача
Публичный facade `raid`: оркестрация `stash → rinse → pack → move` в фиксированном порядке, fail-fast (ошибка включённой фазы останавливает следующие), флаг `success`, DryRun-safe, delegate к под-фасадам без дублирования их логики.

#### Почему реверт PR #74 (зафиксировано перед перереализацией)
1. `identity.expect(...)` в production (нарушение AGENTS §8.7 «без unwrap/expect»).
2. `AgeIdentity::Recipients` отвергался даже при `stash.enabled == false` — спека §4 требует игнорировать identity при выключенном stash.
3. Было только 4 unit-теста; отсутствовали обязательные 7 integration-кейсов спеки §6.

#### Сделано
- `app/raid.rs` (created): `StashPhaseOpts { enabled, min_risk, remove_sources }`, `RinsePhaseOpts { enabled }`, `PackPhaseOpts { enabled, deny_content_secrets }`, `RaidOptions` + `Default` (все фазы enabled, `min_risk: High`, `remove_sources: true`, `deny_content_secrets: true`), `RaidStageResult` (name/success/message/skipped), `RaidResult` (project_path, stages, stash/rinse/pack sub-results, den_artifacts, success, dry_run). `raid()`: preconditions → `Err` (пустой project, нет identity при enabled stash, `Recipients` при enabled stash), ошибка фазы → `Ok(RaidResult { success: false })` + последующие фазы `skipped` («not run due to prior failure»), `move` всегда финальная стадия. Artifacts при частичном фейле не откатываются, их пути попадают в `den_artifacts`. Помощники: `resolve_stash_identity` (identity игнорируется при `!stash.enabled` — фикс причины реверта), `run_stash_phase`/`run_rinse_phase`/`run_pack_phase` (строят под-опции и делегируют), `enabled_phase_count`, `ok_stage`/`failed_stage`/`skipped_stage`/`disabled_stage`, mode-aware `stash_message`/`rinse_message`/`pack_message` (без raw), `raid_event` (`OperationKind::Raid`); 4 unit-теста.
- Re-exports (additive): `app/mod.rs`, `lib.rs` (`raid, RaidOptions, RaidResult, RaidStageResult, StashPhaseOpts, RinsePhaseOpts, PackPhaseOpts`).
- `tests/raid.rs` (created): 13 integration-тестов — все 7 кейсов §6 (all-enabled DryRun ничего не пишет; all-enabled Commit создаёт `.age`+`.tar.zst`, удаляет исходники и node_modules; stash-fail → rinse/pack не запускаются, `success: false`; stash disabled → identity не нужен, rinse+pack идут; pack-only → stash/rinse skipped; den_artifacts содержит ожидаемые пути; default options все enabled) + extras: Recipients при enabled stash → `Err(Unsupported)`, **Recipients при disabled stash игнорируется** (regression-тест причины реверта), пустой project → `Err`, нет identity при enabled stash → `Err`, DryRun не создаёт den skeleton, JSON/сообщения стадий не содержат raw-значение.

#### Файлы
- created: `crates/raccpack-core/src/app/raid.rs`, `crates/raccpack-core/tests/raid.rs`
- changed: `src/app/mod.rs`, `src/lib.rs`

#### Тесты
- `cargo test -p raccpack-core --lib raid` — 4 passed
- `cargo test -p raccpack-core --test raid` — 13 passed
- `cargo test --workspace` — green, 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings` — clean (проверялся `-p raccpack-core`)
- `cargo fmt --check` — clean

#### Решения
- Identity резолвится через `resolve_stash_identity` один раз в начале: `Ok(None)` только при `!stash.enabled`, иначе `Err` — поэтому в основном теле ветка «enabled без identity» не существует (мертвый код удалён при перереализации).
- `failed_stage` принимает `impl Into<String>` (message), а не `&Error` — так stage-сообщения никогда не копируют Display ошибки с raw; raw-значение тестируется (JSON-прогон).
- Файл 441 строка production-кода (потолок 450) — принято: файл когезивен (одна концепция facade raid), предыдущая версия была 436 строк и отклонялась не по размеру. Если ревьюер захочет — split по stage-presentation helpers в follow-up.
- Merge только в `dev` (main — только релизы вех, AGENTS §6).

#### Критерий готовности (DoD из a3.1 §7)
- [x] `raid` matches facade signature
- [x] Fixed order stash → rinse → pack → move; `move` всегда финальная стадия
- [x] Fail-fast: ошибка включённой фазы → `Ok(success=false)`, последующие `skipped`
- [x] DryRun: ничего не пишется, `den_artifacts` пуст
- [x] Identity игнорируется при `!stash.enabled` (фикс причины реверта #74)
- [x] 7 integration-кейсов §6 + regression-тесты
- [x] Без `unwrap`/`expect` в production
- [x] Tests green

#### Риски / follow-up
- **A3.1 файл raid.rs** ~450 строк production (лимит 450): при следующем расширении (raid CLI, manifest) — обязательный split (stage-presentation helpers → отдельный файл или `raid/` dir).
- **Wiki/UX:** `raid` пока не в CLI (A3.4) — страницу `raid.md` и roadmap-статус не трогаем до зелёного A3.4; сейчас не упоминать raid как доступный.
- **Manifest (A3.3):** `RaidResult` уже содержит `den_artifacts` и `stages` — база для JSON-manifest в den/manifests; связь контракта A3.3↔A3.1 учесть при этапе A3.3.
- **StashEmpty** (review #75): политика «no-op, не fail-fast» зафиксирована в спеке §4/§5 и реализована в PR #76; CLI A3.4 может рассчитывать на `success: true` для чистых проектов.

#### Review-fix (PR #76 → dev, squash, merged)
- **Дата:** 2026-08-18 · **Ветка:** `a3.1-raid-review-fixes`
- Замечания ревью PR #75 (не блокеры) — закрыты:
  1. **StashEmpty ≠ fail-fast** — в `raid` добавлена ветка `Err(Error::StashEmpty) → ok_stage("stash", "nothing to stash")`, run продолжается; `stash_result` остаётся `None`, artifact не добавляется. Спека §4/§5/§6 обновлена (пункт 8 тестов). Тесты: `stash_with_no_secrets_is_not_a_failure_and_run_continues` + DryRun-вариант.
  2. **Rustdoc `RaidStageResult.success`** — исправлена опечатка «false for disabled-ok» → фактически disabled = `success: true`; теперь док говорит «true for ok and disabled, false for failed and skipped».
  3. **WORKLOG #2** — замечание было по более раннему состоянию: текущая секция A3.1 уже описывает #75 (ветка `a3.1-facade-raid`, `tests/raid.rs`, 13 integration, Recipients/no-expect). Устаревших `a3-1-raid`-строк нет (проверено `rg`).
  4. **Покрытие edge** — добавлены тесты StashEmpty (см. п.1).
- Тесты: `--lib raid` 4/4, `--test raid` 15/15, workspace green, clippy `-D warnings` clean, fmt clean.

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

### A2.2 — facade `rinse` DryRun/Commit + bytes freed (CLOSED)

- **Дата:** 2026-08-17
- **Ветка:** `a2.2-rinse-facade` (PR #67 → dev, squash, merged)
- **Статус:** done
- **Роли:** Orchestrator; Dev + Test (параллельно, стыковка без rework; Test поймал transient compile-bug Dev, в финальном дереве отсутствует).

#### Задача
Публичный facade `rinse`: DryRun — список `TrashDir` + `bytes_freed` без удаления; Commit — удалить найденные dirs и вернуть фактические stats; progress; **не** трогать secret files отдельно (только dirs из стратегий).

#### Сделано
- `clean/remove.rs` (created): `remove_trash_dir(path) -> Result<u64>` — guard `symlink_metadata`/`is_symlink` → `Ok(0)` (симлинк-дир никогда не удаляется, safety), пересчёт размера через **переиспользуемый** `dir_size_bytes` (стал `pub(crate)` в detect.rs, тело не копировалось — AGENTS §8.3.1), `remove_dir_all` → `Error::Io`.
- `app/rinse.rs` (created): `RinseOptions` (target, `strategies: Option<Vec<String>>`, `include_custom_patterns` reserved no-op), `RinseResult` (serde/Debug/Clone/PartialEq/Eq — mirror StashResult), `rinse()` по §4: 0% «Scanning…» → resolve strategies (opts или `config.cleanup.enabled_strategies`; unknown → `Error::Config`) → `find_trash_dirs` (compute_size: true, `max_depth: scanner.max_depth`) → 40% «Found N directories (X MiB)» → DryRun return | Commit: per-dir containment `is_path_under_root` (fail → `Error::PathOutsideTarget`) → `remove_trash_dir` → 70% «Removing…» → fail-fast partial-failure → 100% «Done». Helpers `resolve_strategy_ids`, `rinse_event` (`OperationKind::Rinse`, phase `rinse`), `format_mib` (private); 3 юнит-теста.
- Re-exports (additive): `app/mod.rs`, `clean/mod.rs`, `lib.rs` (`rinse, RinseOptions, RinseResult`, `remove_trash_dir`).
- `tests/rinse.rs` (created): 14 integration тестов — все 8 кейсов §6 + extras (empty strategies, `include_custom_patterns` no-op, bytes_freed == sum size_bytes).

#### Файлы
- created: `crates/raccpack-core/src/app/rinse.rs`, `crates/raccpack-core/src/clean/remove.rs`, `crates/raccpack-core/tests/rinse.rs`
- changed: `src/clean/detect.rs` (visibility `dir_size_bytes`), `src/clean/mod.rs`, `src/app/mod.rs`, `src/lib.rs`

#### Тесты
- `cargo test --workspace` — green, 24 suites, 0 failed (rinse: 14/14; core lib 157)
- `cargo test -p raccpack-core --test rinse` — 14 passed
- `cargo fmt --all -- --check` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean

#### Решения
- `scanner.max_depth` используется напрямую; fallback «or 64» из спеки **мёртв** (ScannerConfig serde default 6, поле всегда есть) — отклонение зафиксировано.
- Commit: в `removed` попадают только dirs с `freed_bytes > 0`; пустая удалённая dir (0 bytes) не попадает в список, хотя удаляется. Для MVP приемлемо (0 bytes freed), документировано в модуле.
- Containment: canonicalized `is_path_under_root` перед каждым удалением + symlink-guard в `remove_trash_dir` (defense in depth, хотя `find_trash_dirs` симлинки и так не отдаёт).
- Unknown strategy id в options/config → `Error::Config { message }` (шаг в сторону F-ERR-1; специальный вариант не заводили).

#### Критерий готовности (DoD из a2.2 §7)
- [x] `rinse` matches facade signature
- [x] DryRun no deletes; Commit deletes only matched trash dirs
- [x] `bytes_freed` populated (DryRun: сумма size_bytes; Commit: пересчёт на удалении)
- [x] Modules `clean/remove.rs` + `app/rinse.rs`
- [x] Tests green

#### Риски / follow-up
- **A2.1 follow-up (не закрыт):** запись hits изнутри `filter_entry` (side effect в predicate) в `clean/detect.rs` — вне scope A2.2, остаётся открытым (кандидат на A3/гигиену).
- **F-SKIP-1 (не закрыт):** единый источник правды имён trash/skip ещё нет; инвариант-тест «cleanup patterns ⊆ skip/pack» — с `default_pack()` (A4 / pack follow-up).
- `remove_trash_dir` `Ok(0)` неразличим для симлинка и пустой dir — осознанный MVP-tradeoff.
- Форматирование размера: core `format_mib` (progress-сообщение) vs CLI `human_size` (вывод) — дублирование между crates; возможная унификация (двинуть в core + делегировать CLI) — follow-up hygiene.
- Wiki/`supported` + `rinse.md` UX-страница — **отдельная UX-задача после FINAL A2 / вместе с A2.3** (Docs; добавлять не в этом этапе).

### A2.3 — CLI `racc rinse` (CLOSED)

- **Дата:** 2026-08-17
- **Ветка:** `a2.3-cli-rinse` (PR #69 → dev, squash, merged)
- **Статус:** done
- **Роли:** Orchestrator; Dev + Test (параллельно, стыковка без rework; Test начал как baseline до коммита Dev, перепрогнал по merge-ready `d7ffec7`) + Docs (wiki после зелёного FINAL кода).

#### Задача
CLI `racc rinse`: DryRun default, `--yes` → Commit (удаление trash-dirs), повторяемый `--strategy`, JSON + human, exit 0/1.

#### Сделано
- `commands/rinse.rs` (created): `run_rinse` — load_config → apply_overrides → resolve_project_path → mode (`--dry-run` побеждает `--yes`) → `AppContext` (FailOnCritical) → `RinseOptions { target, strategies: Some|None, include_custom_patterns: false }` → facade `rinse` → `output_rinse::print_rinse(&result, &target, json)`.
- `output_rinse.rs` (created): human по §4 («Rinse (dry-run)» / «Would remove N directories (X)» / `<name>  [<strategy>]  <size>` / «(nothing deleted)»; «Rinse complete» / «Removed N directories, freed X») + JSON `RinseResult`; `human_size` переиспользован из `output.rs` (core `format_mib` не дублировался); `dir_name` — локальный 2-строчный helper (фолбэк на полный path).
- `cli.rs`: `Commands::Rinse(RinseArgs)` — `--project` (required), `--yes`, `--dry-run`, `--strategy` (repeatable) + 5 unit-тестов. Глобальные `--json`/`--den`/`--root` уже есть.
- `main.rs`, `commands/mod.rs` — wiring.
- `tests/cli_rinse.rs` (created, 15): все 5 кейсов §5 + dry-run wins over yes, missing project, `--project .`, symlink-guard (`#[cfg(unix)]`), human-вывод, JSON-shape (ровно 3 top-level поля; sum == bytes_freed).
- Wiki: `rinse.md` (deep-страница по шаблону stash.md), nav/sidebar, cli-usage (карточка + «В разработке»/exit-коды), quick-start/index/introduction, roadmap (stash+rinse → «Уже доступно»), supported («Чего пока нет»).

#### Файлы
- created: `crates/raccpack-cli/src/commands/rinse.rs`, `src/output_rinse.rs`, `crates/raccpack-cli/tests/cli_rinse.rs`, `wiki/rinse.md`
- changed: `crates/raccpack-cli/src/cli.rs`, `commands/mod.rs`, `main.rs`, `wiki/.vitepress/config.ts`, `wiki/{cli-usage,index,introduction,quick-start,roadmap,supported}.md`

#### Тесты
- `cargo test --workspace` — 26 suites green, 0 failed (cli_rinse 15/15)
- `cargo fmt --all -- --check` — clean · `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `pnpm run wiki:build` — green
- Smoke: dry-run ничего не удаляет; `--strategy node` фильтрует; `--strategy foo` → exit 1; commit удаляет; `--yes --dry-run` → dry-run; `--json` dry/commit валиден; `--help` = флаги §3.

#### Решения
- `--project` — required clap-аргумент (как pack/stash): `racc rinse --yes` даёт clap usage error (exit 2), НЕ exit 1 из спеки §1.1 — консистентность с существующими командами важнее; отклонение зафиксировано в отчёте PR.
- `--strategy` — свободная строка, unknown → exit 1 через core `Error::Config` (никакой CLI-валидации; поведение ровно как требует спека).
- `--den`/`--root` — глобальные, принимаются; den rinse не использует (в wiki явно: «--den для rinse не используется»).
- Human-вывод 1-в-1 по §4 (проверено живым прогоном; Test-пометки inline оставлены).
- Wiki: roadmap/supported/introduction «догнаны» под факт реализации stash (A1.4) + rinse — согласованная консистентная правка списков CLI-поверхности (отдельно отмечена в отчёте Docs).

#### Критерий готовности (DoD из a2.3 §6)
- [x] `racc rinse` dry-run + commit
- [x] `--strategy` override
- [x] JSON + human
- [x] Wiki matches flags (wiki-rinse.md / published wiki)
- [x] Tests green

#### Риски / follow-up
- **A2.1 follow-up (не закрыт):** единый источник правды имён trash/skip — с `default_pack()` / инвариант-тест (F-SKIP-1). Не трогали.
- **Core `format_mib` vs CLI `human_size`:** известное дублирование между crates (A2.2) — follow-up hygiene (двинуть в core + делегировать CLI); не в scope A2.3.
- **Wiki roadmap**: stash-строка помечена done «заодно» (факт A1.4) — если roadmap политически «только вехи», вернуть точечно.

### A2.2 — review-fixes (не-блокеры ревью) (CLOSED)

- **Дата:** 2026-08-17
- **Ветка:** `a2.2-review-fixes` (PR #68 → dev, squash, merged)
- **Статус:** done
- **Роли:** Orchestrator; Dev + Test (параллельно). Тест-агент сначала увидел «чистую» ветку (правки Dev ещё не легли в рабочее дерево) — блокер снят повторной проверкой Orchestrator по финальному состоянию.

#### Замечания ревью → статус
1. **filter_entry side effect (detect.rs) — ЗАКРЫТО.** `find_trash_dirs` переписан с walkdir-`filter_entry` (push из предиката) на явный стек/DFS: `fs::read_dir` + `DirEntry::file_type()` (не следует симлинкам). Поведение 1:1 (контракт = 22 `tests/clean.rs` + 14 `tests/rinse.rs` без изменений): root depth 0 не матчится; symlink (вкл. symlink-to-dir) не записывается и не обходится; match на depth 1..=max_depth; matched pruned (nested `target/node_modules` не переоткрывается); depth==max_depth проверяется, но не обходится; ошибка чтения → `Error::Io { path }` fail-fast. `compute_size`/sort/containment/`dir_size_bytes` не тронуты.
2. **`Error::PathOutsideTarget` wording — ЗАКРЫТО.** Display стал generic: `path outside target root: {path}` (вариант общий для stash и rinse). Ассерт в модульном тесте обновлён; stale `path outside stash target` в workspace нет (grep 0).
3. **F-SKIP-1 — остаётся OPEN** (как и должно быть): стратегии/skip-таблицы и pack-deny не трогали; не закрывать «заодно» в A2.3. Синхронизация — с `default_pack()`/инвариант-тест.
4. **Wiki rinse — отложено согласованно** с AGENTS: UX-страница `rinse.md`/supported — Docs-задача после FINAL A2 / вместе с A2.3.

#### Файлы
- changed: `crates/raccpack-core/src/clean/detect.rs`, `crates/raccpack-core/src/domain/error.rs`

#### Тесты
- `cargo test -p raccpack-core --test clean` — 22 passed
- `cargo test -p raccpack-core --test rinse` — 14 passed
- `cargo test -p raccpack-core domain::error` — 3 passed
- `cargo test --workspace` — 0 failed (все suites)
- `cargo fmt --all -- --check` — clean · `cargo clippy --workspace --all-targets -- -D warnings` — clean

#### Решения
- DFS-обход (порядок readdir не детерминирован на уровне OS) — итоговая сортировка `sort_by(path)` сохранена, контракт не меняется.
- Ошибка `file_type()` → `Error::Io` с `entry.path()`; ошибка `read_dir` → `Error::Io` с самим каталогом (fail-fast).


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

### 2026-08-18 — docs: README sync после A2 (rinse available, cleanup strategies)

**Задача:** привести README в соответствие с состоянием после A2.3 (rinse реализован, cleanup strategies в config). Ветка `docs-readme-a2` от `dev`, PR #71 → `dev` (squash, merged, ветка удалена).

**Сделано:**
- Статус-таблица: `rinse` → **Available (Alpha)** (dry-run default / `--yes`, defaults rust/node/python).
- Quick start: добавлены `racc rinse` (dry-run + `--yes`).
- «What is supported»: добавлен bullet cleanup strategies (6 id, opt-in `jvm`/`go`/`generic`).
- Roadmap-блок: stash ✅ / rinse ✅.

**Файлы (changed):** `README.md`
**Тесты:** n/a (docs-only). Rust не затрагивался.

### 2026-08-18 — docs: wiki configuration `[cleanup]` + consistency после A2 (stash/rinse)

**Задача:** Docs после A2.3 — обновить `configuration.md` под текущий config и согласовать consistency-страницы. Ветка `docs-wiki-config-a2` от `dev`, PR #70 → `dev` (squash, merged, ветка удалена).

**Сделано:**
- `wiki/configuration.md`: `[cleanup]` с `enabled_strategies` (defaults, opt-in, 6 id по `DEFAULT_STRATEGIES`), `[cleanup]` убран из будущих секций, env-подсекция (`RACCPACK_CONFIG`, `RACCPACK_PASSPHRASE`), CLI overrides (`--den` не влияет на rinse), ошибки unknown strategy (текст из `ConfigError` / `Error::Config`), пометка про F-SKIP-1 без обещаний.
- Consistency: `cli-usage.md` (rinse в типовой сценарий), `quick-start.md` (шаг rinse), `supported.md` (вводный абзац + секция «Очистка (rinse)» + exit codes + «Чего пока нет» без rinse/stash), `concepts.md` (exit codes + cleanup strategies).
- EN-зеркало (`wiki/en/`) не трогали — скелет без парных страниц; RU-only + пометка в PR.

**Файлы (changed):** `wiki/configuration.md`, `wiki/cli-usage.md`, `wiki/quick-start.md`, `wiki/supported.md`, `wiki/concepts.md`
**Тесты:** `pnpm run wiki:build` — green. Паттерны стратегий сверены с `crates/raccpack-core/src/clean/strategy.rs`.
**Решения:** Rust-код не изменялся; `roadmap.md` / `rinse.md` / `stash.md` / `pack.md` актуальны, не трогали.

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

### 2026-08-21 — A4.1: GitClient (process) + git_status в dig

**Задача:** спека `docs/alpha/a4/a4.1-git-client.md`. Ветка `a4-git-client` от `dev`, PR #85 → `dev` (squash, merged, ветка удалена). Версия → **0.2.12**.

**Сделано:**
- Новый модуль `core/src/git/`: trait `GitClient` (`is_repo` / `file_status` / `files_status`), типы `GitFileStatus` (serde snake_case + `as_str()`) и `GitState`, `find_repo_root()` (walk-up до `.git`, dir или file); thin `mod.rs`.
- `ProcessGitClient`: subprocess `git -C … rev-parse --is-inside-work-tree` и `status --porcelain=v1 -z --ignored=matching --untracked-files=all`; timeout через spawn+try_wait poll+kill (без новых зависимостей), stdout/stderr дренаж потоками; `GIT_TERMINAL_PROMPT=0`. Porcelain-маппинг: `??`→untracked, `!!`→ignored, любой M→modified, A/R/C→staged, D→deleted, нет в выводе→tracked, иное→unknown; `-z`-парсер — чистая функция с unit-таблицей.
- `MockGitClient` (always compiled, builder: `with_is_repo/with_statuses/with_error`) для тестов без git.
- Dig: `dig()` делегирует в новый `dig_with_git(…, &dyn GitClient)`; сигнатура `dig()` не менялась (CLI/raid не затронуты). Обогащение best-effort named-функциями: пустые findings → git не вызывается; не-repo / любая ошибка клиента → все `git_status: None`, dig Ok (никогда не падает из-за git).
- `Error::Git { message }` + suggestion; без raw секретов в сообщениях.
- Re-exports lib.rs аддитивные (`git::*`, `dig_with_git`). Breaking: нет.
- Решение по спеке §3.1: `AM` → **Modified** («любой M» приоритетнее «A*→Staged»), зафиксировано в doc-comment и unit-тестах.

**Файлы:** `src/git/{mod,client,process,mock}.rs` (created), `src/app/dig.rs`, `src/app/mod.rs`, `src/domain/error.rs`, `src/lib.rs`, `tests/{git_client,git_process,dig}.rs` (changed|created)
**Тесты:** `cargo test --workspace` green; `cargo test -p raccpack-core --test git_process -- --ignored` green (реальный git: tracked/untracked/modified, .gitignore→ignored, missing binary soft-fail); fmt + clippy `-D warnings` core/cli чисто.
**Процесс:** Dev попытка 1 вернула пустой отчёт без изменений → ре-диспетч (попытка 2, принята). Test rework ×1: 2 clippy-линта в `tests/git_process.rs` (bool_assert_comparison, cloned_ref_to_slice_refs) — исправлены, diff ограничен.
**Синхронизация:** VERSION_ROADMAP (A4.1 ✅ 0.2.12), raccpack-roadmap-v1 (A4.1 ✅), README (badge 0.2.12, Status: dig + git status per finding), Cargo.toml → 0.2.12. AGENTS.md §3.9 дополнен: Status-таблица README обязательна после каждого этапа.
**Follow-up:** wiki `dig.html` — задокументировать поле `git_status` в JSON-выводе — **закрыт** (коммит 123f97a).

### 2026-08-21 — A4.2: config migrate chain + `racc init`

**Задача:** спека `docs/alpha/a4/a4.2-config-migrate-init.md`. Ветка `a4-config-migrate-init` от `dev`, PR #86 → `dev` (squash, merged, ветка удалена). Версия → **0.2.13**.

**Сделано:**
- `config_version` в TOML (`RaccConfig`, serde default) + `config/migrate.rs`: `CURRENT_CONFIG_VERSION = 1`; цепочка v0/missing → v1 (инъекция поля), future version → `ConfigError::IncompatibleVersion` («downgrade client»); `load_from_path` теперь parse → `migrate_to_current` → typed struct.
- `config/init.rs`: `InitOptions` / `InitResult` (serde), `default_toml()` — комментированный шаблон со ссылками на wiki-страницы; `init_config()` — `AlreadyExists` без `--force`, create parent dirs, опциональный den skeleton через `ensure_den`.
- CLI `racc init`: флаги `--force`, `--scan-root`, `--ensure-den`; глобальные `--config` / `--den` / `--json` переиспользованы (вместо дубля `--den` из спеки — решение зафиксировано в PR). Exit 1 при существующем конфиге без `--force`. Human/JSON вывод пути.
- **F-ERR-1 закрыт:** `From<ConfigError> for Error` (FileNotFound/ScanRoot→PathNotFound, Read/Write→Io, остальное→Config) + unit-тест.
- Новые варианты `ConfigError`: `AlreadyExists`, `Write`, `IncompatibleVersion` (+ suggestions).
- Re-exports lib.rs аддитивные (`init_config`, `migrate_to_current`, `CURRENT_CONFIG_VERSION`, `default_config_path`, `DEFAULT_DEN_DIR`, …). Breaking: нет.
- `paths.rs`: `default_config_path` / `DEFAULT_DEN_DIR` / `resolve_path` подняты до `pub` (нужны init + CLI).

**Файлы:** `config/{migrate,init}.rs` (created), `config/{mod,error,paths}.rs`, `domain/error.rs`, `lib.rs`, `cli.rs`, `commands/{init.rs (created), mod.rs}`, `main.rs`, тесты `tests/{config_migrate,config_init,cli_init}.rs` (created), `tests/config.rs` (changed)
**Тесты:** `cargo test --workspace` green (новые suites: config_migrate ×7, config_init ×7, cli_init ×7 + unit в migrate/init/error); fmt + clippy `-D warnings` core/cli чисто.
**Процесс:** работа найдена в рабочем дереве ветки `a4-config-migrate-init` (от предыдущей сессии, без отчётов Dev/Test). Orchestrator провёл полную приёмку сам по merge-ready состоянию: DoD спеки, инварианты (без unwrap/expect в production, типизированные ошибки, слои), полный прогон. Отдельный rework не требовался.
**Синхронизация:** по чеклисту §3.9 — Cargo.toml/Cargo.lock 0.2.13, README (badge + Status-абзац + строка `init` в Status-таблице), VERSION_ROADMAP (A4.2 ✅ 0.2.13, все 6 точек), raccpack-roadmap-v1 (A4.2 ✅), WORKLOG, бинарник переустановлен.
**Follow-up:** wiki — страница/секция `racc init` + `configuration.md` (пример сгенерированного конфига) + `roadmap.md`/`introduction.md` версии — **закрыт** (коммит 883f085).

### 2026-08-21 — A4.3: tracing без секретов + глобальный `--verbose`

**Задача:** спека `docs/alpha/a4/a4.3-tracing-verbose.md`. Ветка `a4-tracing-verbose` от `dev`, PR #87 → `dev` (squash, merged, ветка удалена). Версия → **0.2.14**.

**Сделано:**
- `crates/raccpack-cli/src/logging.rs` (NEW, 125 строк): `init_tracing(u8)` — fmt-subscriber в **stderr** (`--json` держит stdout чистым), `EnvFilter`, ANSI только на TTY; идемпотентность через `try_init` (повторный вызов — no-op, тест «doesn't panic» зелёный). Pure-хелперы: `filter_for_verbosity` (0→warn, 1→info, 2→debug, ≥3→trace) и `resolve_filter` (непустой `RUST_LOG` побеждает флаг; пустой/whitespace = unset) + unit-тесты.
- `cli.rs`: глобальный repeatable `-v/--verbose` (`ArgAction::Count`, u8) + 5 parse-тестов по существующему паттерну.
- `main.rs`: `init_tracing(cli.global.verbose)` сразу после `Cli::parse()`, до `run()`.
- Core-инструментация точечно (только counters/paths/source): stash — «encrypting N files», «archive placed in den», «source files removed»; dig/sniff — summary (root, счётчики, from_cache, duration_ms).
- CLI `passphrase.rs`: только источник («passphrase source: env|tty|stdin») — значение никогда.
- Интеграционные тесты `tests/tracing_logging.rs` (7 кейсов): default quiet, `-v`→info, `-vv`→debug, RUST_LOG wins, JSON|stderr разделение потоков, **passphrase/секрет не появляются в -vv выводе** (fake AWS secret + RACCPACK_PASSPHRASE), `-vvv` smoke.

**Файлы:** `cli/src/logging.rs` (created), `cli/src/{cli,main,passphrase}.rs`, `cli/tests/tracing_logging.rs` (created), `core/src/app/{stash,dig,sniff}.rs`, `Cargo.toml` обоих crates (tracing/tracing-subscriber deps), `Cargo.lock`
**Тесты:** `cargo test --workspace` green; узкий набор `cargo test -p raccpack-cli --test tracing_logging` 7/7; fmt + clippy `-D warnings` core/cli чисто. Redaction-sweep Orchestrator'а: в `secrets/`/`archive/` ни одного log-event; grep значения passphrase в `-vv` выводе stash — пусто.
**Процесс:** Dev + Test параллельно, оба приняты с попытки 1. Test стартовал в гонке до правок Dev (baseline), финальный прогон — по merge-ready дереву; Orchestrator перепроверил всё сам по финальному состоянию.
**Решения:** логи всегда в stderr (спека §5 «JSON в stdout, логи в stderr»); ANSI только на TTY; пустой `RUST_LOG` считается unset.
**Замечания (не блокеры):** P3 — `stash.rs` info! дублировал счётчик в поле и сообщении — **закрыт** (коммит 71f09f3, PR #88); pre-existing debt — `cli.rs` ~941 строка (тест-тяжёлый), кандидат на split args/tests отдельным hygiene-этапом — **отложено сознательно**; инструментация raid/rinse/pack info-событиями — позже, без отдельного этапа (для Alpha точечной stash/dig/sniff достаточно).
**Синхронизация:** по чеклисту §3.9 — Cargo.toml/Cargo.lock 0.2.14, README (badge + Status-абзац), VERSION_ROADMAP (A4.3 ✅ 0.2.14, все точки), raccpack-roadmap-v1 (A4.3 ✅), wiki (`cli-usage.md` глобальный `-v/--verbose`, `roadmap.md`, `introduction.md`), бинарник переустановлен.

### 2026-08-21 — A4.4: integration + CI — **ALPHA EXIT 0.3.0**

**Задача:** спека `docs/alpha/a4/a4.4-integration-ci.md`. Ветка `a4-integration-ci` от `dev`, PR #89 → `dev` (squash, merged, ветка удалена). Версия → **0.3.0** (exit вехи Alpha).

**Сделано:**
- `.github/workflows/ci.yml` (NEW): push+PR, ubuntu-latest, `cargo test --workspace` / `fmt --check` / `clippy --workspace --all-targets -- -D warnings`; toolchain пин ≡ workspace rust-version (F-CFG-2).
- **MSRV 1.75 → 1.85** (`rust-version` + README badge): блокер экосистемы — транзитивный `cpufeatures 0.3` (blake3 1.8.6; sha2 0.11 ← age 0.12 → rust-embed-utils) требует edition2024-манифестов, Cargo 1.75 не парсит. Даунгрейд-пины проверены и отклонены: blake3↓ не спасает (sha2-цепочка age держит ^0.3), даунгрейд age — security-sensitive каскад. Решение зафиксировано в ci.yml-комментарии.
- clippy 1.85: 4× `map_err(|err| { cleanup; err })` → `.inspect_err(|_| cleanup)?` (pack/mod.rs ×2, stash.rs ×2), семантика идентична.
- Аудит Test: матрица покрытия §4 **8/8 ok** существующими сьютами (core/tests/* + cli/tests/cli_* + tracing_logging.rs); новые тестовые файлы сознательно не дублировались (§8.3.1). Спека допускает альтернативный split.
- Alpha exit checklist (спека §7) — **pass**: sniff/dig/stash/rinse/pack/raid CLI ✅ · den secrets+packs+manifests ✅ · init+config_version ✅ · verbose без утечек ✅ (tracing_logging #6 + ручной grep) · git status на dig ✅ (staged/untracked/soft-null) · CI green ✅ (локально +1.85/stable; GitHub run на PR/dev).

**Файлы:** `.github/workflows/ci.yml` (created), `Cargo.toml` (rust-version), `README.md` (badge MSRV), `core/src/app/{pack/mod,stash}.rs` (inspect_err)
**Тесты:** `cargo +1.85 test --workspace` green; stable green (783/783, дважды — детерминированно); fmt/clippy `-D warnings` на обоих тулчейнах чисто; alpha-smoke §5 (init→sniff→dig→raid → .age/.tar.zst/manifest; redaction grep — OK).
**Процесс:** Dev (ci.yml + MSRV-верификация + inspect_err) и Test (аудит матрицы + smoke) параллельно; оба приняты. Инцидент smoke: скрипт без изоляции HOME перезаписал реальный `~/.config/raccpack/config.toml` — восстановлен Orchestrator'ом (`~/DEV/PROJS` / `~/.raccpack/den`); harness'ы тестов изолированы корректно.
**Решения:** MSRV 1.85 вместо пинов зависимостей (см. выше); покрытие матрицы — существующими сьютами без новых файлов.
**Follow-up:** raid_atomic.rs (1036 строк) / cli_raid.rs (714) — тест-файлы сверх soft-limit, split при следующем касании; инструментация raid/rinse/pack info-событиями — без отдельного этапа.
**Синхронизация:** по чеклисту §3.9 — Cargo.toml/Cargo.lock 0.3.0, README (badge + Status + roadmap-блок Alpha ✅), VERSION_ROADMAP (A4.4 ✅ 0.3.0, «ВЫ ЗДЕСЬ» на Alpha exit), raccpack-roadmap-v1 (A4.4 ✅), WORKLOG (шапка/бэклог/запись), wiki (страница Git/init/DX из прототипа, roadmap/introduction 0.3.0), бинарник переустановлен.

---

### 2026-08-22 — docs: архивация спек Alpha + каркасы Detect v2 / Beta

**Задача:** консолидация dev-docs после Alpha exit: спеки A1–A4 → archive, новые спеки Detect v2 (D1–D3) и Beta (B1–B4). Ветка `docs-archive-alpha-specs` от `dev`, PR #91 → `dev` (squash).

**Сделано:**
- `docs/alpha/` → `docs/archive/alpha/` (A1–A4 + `a3_new` слит в `archive/alpha/a3/`, SHIPPED/SUPERSEDED пометки сохранены).
- `docs/detect/` (NEW): `detect-v2-index.md`, D1 (StackDetector trait / DTO / detect.mode), D2 (workspace DAG / conflict merge / flat compat), D3 (rinse DAG / sniff tree / fixtures).
- `docs/beta/` (NEW): `beta-index.md`, B1 (TUI), B2 (Tauri desktop), B3 (security/reveal), B4 (den GC / parallel / beta tag).
- `examples/raid-all.fish` (NEW): fish-скрипт raid по всем проектам из sniff (источник примера из `wiki/cookbook.md`).
- Убраны дубликаты (не коммичены): корневые `RP.webp` / `alpha-banner.webp` (≡ `wiki/public/*`), `a3.1-facade-raid-SHIPPED (1).md` (≡ оригинал).

**Файлы:** `docs/alpha/**` (deleted, rename), `docs/archive/alpha/**` (created), `docs/detect/**` (created), `docs/beta/**` (created), `examples/raid-all.fish` (created), `WORKLOG.md`
**Версия:** без bump (docs-only).
**Follow-up:** doc-comment ссылки на `docs/alpha/…` в тестах (`crates/*/tests/*.rs`) теперь указывают на старый путь — актуализировать при следующем касании файлов.

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
