# Raccpack — модульность secrets и archive backends

**Статус:** accepted decision  
**Дата:** 2026-08-05  
**Контекст:** дополнение к [видению архитектуры](raccpack-architecture-vision.md), [facade/den](raccpack-facade-and-den.md) и [roadmap](raccpack-roadmap-v1.md).  
**Цель:** зафиксировать файловую раскладку и контракты расширяемости **до** реализации dig/stash, чтобы не рефакторить ядро позже.

---

## 1. Принцип

- Каждый **вид секрета** (токен, connection string, private key, …) — отдельный `*.rs`.
- Каждый **encryption / pack backend** (age, опционально 7z, …) — отдельный `*.rs`.
- Агрегация только в registry (`mod.rs`): engine и facade не знают конкретных реализаций.
- Новый секрет / backend = новый файл + одна строка регистрации. Без правок «большого» матчера.

Это согласуется с архитектурным видением:

> Новые secret patterns — groups + tables в secrets; toggle в config.  
> Другой encrypt backend — trait `SecretVault` / `EncryptionBackend` в archive.

---

## 2. Секреты

### 2.1. Дерево модулей

```text
crates/raccpack-core/src/
  secrets/
    mod.rs              # публичный API: dig/scan entrypoints, re-exports
    engine.rs           # оркестратор: walk + apply matchers + dedup + risk
    types.rs            # SensitiveRisk, MatchKind, ContentHit (masked), …
    groups.rs           # EnabledGroups / mapping из config.sensitive
    matchers/
      mod.rs            # registry всех SecretMatcher
      filename.rs       # общие filename-паттерны (.env, id_rsa, credentials, …)
      aws.rs
      github.rs
      database.rs       # connection strings (postgres, mysql, redis, …)
      private_key.rs    # PEM / OpenSSH
      generic_token.rs  # entropy / high-entropy fallback (heuristics)
      # … новые — по одному файлу
```

### 2.2. Контракт матчера

```rust
use std::path::Path;
use crate::secrets::types::{SensitiveRisk, ContentHit};

/// Один вид / семейство секретов.
/// Реализации живут в `matchers/*.rs` и регистрируются в `matchers/mod.rs`.
pub trait SecretMatcher: Send + Sync {
    /// Стабильный id: "aws", "github_pat", "database", …
    fn id(&self) -> &'static str;

    /// Группа для config toggle (`config.sensitive.groups`).
    fn group(&self) -> &'static str;

    /// Filename / path heuristics. `None` — не матч.
    fn match_filename(&self, path: &Path) -> Option<SensitiveRisk>;

    /// Content scan. Возвращает только masked/prefix/hash — **без raw**.
    /// Пустой Vec — нет content-совпадений.
    fn match_content(&self, path: &Path, bytes: &[u8]) -> Vec<ContentHit>;
}
```

`ContentHit` (ориентир):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentHit {
    pub matcher_id: String,
    pub risk: SensitiveRisk,
    /// Masked preview (например `AKIA…XXXX`), никогда raw.
    pub masked: String,
    /// Стабильный fingerprint для repeated-secrets (hash).
    pub value_hash: String,
    pub byte_offset: Option<u64>,
}
```

### 2.3. Registry

```rust
// secrets/matchers/mod.rs
mod filename;
mod aws;
mod github;
mod database;
mod private_key;
mod generic_token;

use super::SecretMatcher;

pub fn all_matchers() -> Vec<&'static dyn SecretMatcher> {
    vec![
        &filename::FilenameMatcher,
        &aws::AwsMatcher,
        &github::GithubMatcher,
        &database::DatabaseMatcher,
        &private_key::PrivateKeyMatcher,
        &generic_token::GenericTokenMatcher,
    ]
}
```

Engine (`engine.rs`) только:

1. Берёт `all_matchers()`.
2. Фильтрует по `EnabledGroups` из конфига / override.
3. Применяет filename → content (с лимитом размера файла).
4. Собирает `SensitiveFile` + repeated, **без raw** в отчёте.

### 2.4. Правила

| Правило | Зачем |
|---------|--------|
| Один matcher ≈ один файл | Изоляция, review, тесты рядом |
| Registry — единственное место перечисления | Нет скрытых глобальных списков |
| Config groups включают/выключают целые matcher’ы | Не отдельные regex внутри файла |
| Raw secret только внутри engine на время encrypt | Инвариант безопасности |
| Unit-тесты рядом с matcher’ом | `aws.rs` + `#[cfg(test)]` или `tests/secrets_aws.rs` |

---

## 3. Archive / encryption backends

### 3.1. Дерево модулей

```text
crates/raccpack-core/src/
  archive/
    mod.rs              # публичный API pack/stash helpers
    pack.rs             # tar+zstd project pack (основной)
    den_layout.rs       # manifests/secrets/packs/staging пути и имена
    backends/
      mod.rs            # registry EncryptionBackend
      age.rs            # primary для секретов
      # 7z.rs           # опционально, feature-flag
      # zip.rs          # если понадобится
```

Pack (чистый проект) и encryption (секреты) **не смешиваются**:

- `pack` → `tar+zstd` (или другой pack-формат позже).
- `stash` → `EncryptionBackend` (age по умолчанию).

### 3.2. Контракт backend’а

```rust
use std::path::Path;
use crate::facade::AgeIdentity; // или более общий KeyMaterial

/// Backend шифрования секретов.
pub trait EncryptionBackend: Send + Sync {
    fn name(&self) -> &'static str;

    /// Шифрует plaintext и атомарно пишет в `dest` (temp + rename где возможно).
    /// `identity` / ключ **не** логировать; zeroize после use.
    fn encrypt(
        &self,
        plaintext: &[u8],
        identity: &AgeIdentity,
        dest: &Path,
    ) -> Result<(), crate::Error>;

    // decrypt — не требуется в v1 (опционально позже)
}
```

### 3.3. Registry

```rust
// archive/backends/mod.rs
mod age;
// #[cfg(feature = "backend-7z")]
// mod seven_z;

use super::EncryptionBackend;

pub fn default_backend() -> &'static dyn EncryptionBackend {
    &age::AgeBackend
}

pub fn backend_by_name(name: &str) -> Option<&'static dyn EncryptionBackend> {
    match name {
        "age" => Some(&age::AgeBackend),
        // "7z" => Some(&seven_z::SevenZBackend),
        _ => None,
    }
}
```

`stash` / `raid` зависят только от trait’а. Выбор backend’а — из config (default `age`).

### 3.4. Правила

| Правило | Зачем |
|---------|--------|
| Один backend = один файл | Та же изоляция, что у matcher’ов |
| age — primary, остальное optional | Соответствует security-инвариантам vision |
| Feature-flag для тяжёлых/редко нужных backend’ов | Не раздувать default build |
| Zeroize ключа внутри backend | Инвариант core |
| Pack и encrypt — разные подсистемы | Разная семантика и пути в den |

---

## 4. Как это ложится на roadmap

| Фаза | Действие по модульности |
|------|-------------------------|
| **M1.3–M1.4** | Можно создать пустые `secrets/` и `archive/` (mod.rs + заготовки). Не блокирует config/walk. |
| **M3 dig** | Сразу `matchers/` + trait + 2–3 реальных matcher’а (filename + content). Не один монолитный файл. |
| **M4 pack** | `archive/pack.rs` + `den_layout.rs`. Encryption backend ещё не нужен. |
| **A1 stash** | `backends/age.rs` + trait + registry. |
| **Позже** | Новый секрет / backend = новый `*.rs` + регистрация в `mod.rs`. |

Жёсткая зависимость: TUI/Desktop только после стабильного facade; модульность secrets/archive живёт **внутри** core и не затрагивает UI-контракты.

---

## 5. Инварианты (кратко)

1. Facade **не** возвращает raw secret material.
2. Matcher’ы отдают только masked / hash / risk.
3. Raw bytes секрета существуют в памяти только внутри engine → backend encrypt, затем zeroize.
4. Registry — единственная точка агрегации; engine не импортирует конкретные `aws::` / `github::`.
5. Config groups управляют matcher’ами целиком.
6. Default encryption backend = age; другие — явно и опционально.

---

## 6. Следующий шаг

1. При старте M1.3 / подготовке к M3 — создать скелет директорий и пустые trait’ы + registry.
2. Не переносить существующую логику «в один файл», а сразу раскладывать по matcher’ам.
3. Решение считать принятым; при отклонении — обновить этот документ и WORKLOG.

---

*Документ можно уточнять (имена trait’ов, точный shape `ContentHit`), но правило «один вид = один файл + registry» менять не стоит: иначе эвристики снова свалятся в монолит между dig и будущими UI.*
