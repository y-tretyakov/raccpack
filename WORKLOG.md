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

### 2026-08-14 13:37 — A1.1 age + zeroize passphrase

**Задача:** crypto-примитив stash: age (scrypt passphrase) encrypt/decrypt, zeroize материала ключа. Без facade / den / CLI.

**Сделано (Orchestrator: Dev + Test субагенты параллельно, приёмка по §6 спеки):**
- `archive/age_vault.rs` (created): `encrypt_bytes_to_file`, `encrypt_file_to_age` (→ bytes_read), `decrypt_file_from_age` (test-only, `#[cfg(any(test, feature = "age-decrypt"))]`), atomic write (`<output>.tmp` + rename, temp удаляется при ошибке), empty passphrase → `Error::Encrypt`.
- `domain/error.rs`: новый вариант `Error::Encrypt { message }` — passphrase никогда в Display.
- `archive/mod.rs` + `lib.rs`: `pub mod age_vault` + re-exports encrypt-функций (decrypt на уровне lib НЕ ре-экспортирован).
- `Cargo.toml`: `age = "0.12"`, `zeroize = { version = "1", features = ["derive"] }`, `[features] age-decrypt = []`.

**Файлы:**
- `crates/raccpack-core/src/archive/age_vault.rs` (created)
- `crates/raccpack-core/Cargo.toml`, `src/domain/error.rs`, `src/archive/mod.rs`, `src/lib.rs`, `Cargo.lock` (changed)

**Тесты:**
- `cargo test -p raccpack-core age_vault` → 7/7 pass (roundtrip, wrong passphrase, empty passphrase, file roundtrip + bytes_read, no-leak в Display, overwrite, binary magic header).
- `cargo test --workspace` → все зелёные (включая регрессию CLI/core).
- `cargo clippy --workspace --all-targets -- -D warnings` → чисто.
- `cargo fmt -p raccpack-core -- --check` → чисто.

**Зафиксировано (решения):**
- age version: **0.12.1** (0.10.0 yanked; MSRV 1.74 ≤ workspace 1.75).
- Формат: **binary** (без ASCII armor); фича `armor` не включалась (отклонение от snippet в спеке — модуль не используется).
- Passphrase: caller `Zeroizing<String>` → внутренняя копия `secrecy::SecretString`; обе zeroize на drop. Промежуточный `String` от `to_owned()` — не zeroized (кратковременный; тот же паттерн, что в примерах самого age-крейта).
- Атомарная запись внутри vault (tmp + rename), overwrite ок.

**Риски / follow-up:**
- A1.2/A1.3: те же age-примитивы лягут в encrypt шаг stash.
- decrypt не ре-экспортирован из lib root — только под `age-decrypt` feature / test.

**Критерий готовности §6:**
- [x] encrypt_file_to_age / encrypt_bytes_to_file работают
- [x] Passphrase через Zeroizing/SecretString; empty rejected
- [x] Ошибки без утечки passphrase
- [x] Decrypt для тестов roundtrip
- [x] Модуль изолирован в archive/age_vault.rs
- [x] Тесты §5 зелёные

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
