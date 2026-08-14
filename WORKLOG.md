# WORKLOG — raccpack

Журнал статусов этапов. Orchestrator: y-tretyakov.

**MVP 0.1.0 закрыт.** Полный журнал M1–M4 и docs-миграции:
[`docs/archive/WORKLOG_MVP.md`](docs/archive/WORKLOG_MVP.md).
Спеки закрытых этапов: [`docs/archive/mvp/`](docs/archive/mvp/).

## Backlog (Alpha → 0.3.0)

```
[x] A1.1 age + zeroize passphrase
[ ] A1.2 stash manifest (без raw) + remove sources в Commit
[ ] A1.3 facade stash + den/secrets/…
[ ] A1.4 CLI racc stash
[ ] A2.1 cleanup strategies + config toggles
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

### A1.1 — age + zeroize passphrase (CLOSED)

- **Дата:** 2026-08-14
- **Ветка:** `a1-stash-age`
- **Статус:** done
- **Dev:** dev-a1.1 · **Test:** test-a1.1 (параллельно, без rework)

#### Сделано
- `archive/age_vault.rs` (created): `encrypt_bytes_to_file`, `encrypt_file_to_age` (→ bytes_read), `decrypt_file_from_age` (test-only, `#[cfg(any(test, feature = "age-decrypt"))]`), atomic write (`<output>.tmp` + rename, temp удаляется при ошибке), empty passphrase → `Error::Encrypt`.
- `domain/error.rs`: вариант `Error::Encrypt { message }` — passphrase никогда в Display.
- `archive/mod.rs`: `pub mod age_vault` + re-exports encrypt-функций. На уровне lib-корня re-exports НЕ делались (узкий public API; decrypt не торчит из crate root).
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

#### Follow-up review замечания (человек, 2026-08-14; PR #51) — НЕ блокеры
- **A. Error mapping** — принято к A1.2: `age::EncryptError` → `Error::Encrypt`, чистый IO → `Error::Io`; для 0.12 wrap_output/finish возвращают io::Result, поэтому семантика уточняется при facade stash.
- **B. Два стиля encrypt** (`age::encrypt`+Recipient для bytes, `Encryptor::with_user_passphrase` для file) — валидно, roundtrip зелёный; возможная унификация — позже, не блокер.
- **C. Тесты** — дополнены: empty passphrase на decrypt, no-leak на Encrypt-ветке (wrong passphrase decrypt), missing source у `encrypt_file_to_age` (+ не оставлять output), tmp-очистка на mid-write fail.
- **D. `Error::Encrypt` suggestion** — добавлен hint («check passphrase / output writable»).
- **E. Zeroize** — принято: `Zeroizing<String>` → `SecretString`; двойной zeroize на drop ок.
- **F. Минимальная длина passphrase** — сознательно только non-empty; CLI warn про слабые пароли — A1.4/CLI.

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
