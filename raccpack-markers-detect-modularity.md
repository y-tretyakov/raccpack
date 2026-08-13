# Raccpack — модульность markers / detect по экосистемам

**Статус:** accepted decision (follow-up к M2.1, обязателен с M2.2)
**Дата:** 2026-08-05
**Контекст:** дополнение к [raccpack-modularity.md](raccpack-modularity.md) (secrets / archive backends) и к agent-prompt modular Rust.
**Связь с roadmap:** M2.1 candidates — закрыт с единой таблицей маркеров; **M2.2 detect → Stack** — резать по экосистемам сразу, не после god-file.

---

## 1. Проблема

После M2.1 все маркеры живут в одной таблице:

```rust
pub static DEFAULT_MARKERS: &[MarkerDef] = &[
    // Cargo.toml, package.json, go.mod, pyproject… Makefile, .git
];
```

Для **M2.1 (только candidates)** этого достаточно: «добавить маркер = одна строка в таблице».

Для **M2.2 detect → `Stack`** и дальше — **модульность по экосистемам нужна**. Иначе `detect` раздуется в один god-file с гигантским `match` по языкам, framework-эвристиками и особыми случаями (Cargo workspace, monorepo, Gradle Kotlin DSL и т.д.).

Правило то же, что у secrets и archive backends:

> **Один язык / экосистема ≈ один файл; агрегация только в registry (`mod.rs`).**

---

## 2. Рекомендуемая раскладка

Не ломает публичный API M2.1 (`find_candidates`, `MarkerDef`, `ProjectCandidate`): меняется только внутренняя организация модулей.

```text
crates/raccpack-core/src/scan/
  markers/
    mod.rs              # registry: DEFAULT_MARKERS = concat всех групп
    types.rs            # MarkerKind, MarkerDef, MarkerHit
    rust.rs             # Cargo.toml (+ позже workspace heuristics)
    node.rs             # package.json, pnpm-workspace, …
    python.rs           # pyproject.toml, setup.py, requirements.txt
    go.rs               # go.mod
    jvm.rs              # pom.xml, build.gradle, build.gradle.kts
    ruby.rs             # Gemfile
    php.rs              # composer.json
    cpp.rs              # CMakeLists.txt
    make.rs             # Makefile (language_hint: None)
    git.rs              # .git (DirName)
  candidates.rs         # find_candidates — без знания языков
  detect/               # M2.2: MarkerHit[] → Stack
    mod.rs              # registry детекторов + entrypoint
    types.rs            # при необходимости промежуточные типы
    rust.rs
    node.rs
    python.rs
    go.rs
    jvm.rs
    …
  skip.rs
  walk.rs
  mod.rs                # тонкий re-export (как сейчас)
```

### 2.1. Границы ответственности

| Модуль | Знает | Не знает |
|--------|--------|----------|
| `markers/*` | имена файлов/dir, `MarkerKind`, optional `language_hint` | frameworks, stack, size, git status |
| `candidates.rs` | walk + match имён → `ProjectCandidate` | какой язык «главный», как выбрать Stack |
| `detect/*` | `MarkerHit[]` (+ позже содержимое файлов) → `Stack` | FS-walk, skip policy |

`find_candidates` остаётся **агностичным**: только матчит имена по registry.
Языковая и framework-семантика — **только** в `detect`.

---

## 3. Контракты (ориентир)

### 3.1. Types (`markers/types.rs`)

Без изменения смысла относительно M2.1:

```rust
pub enum MarkerKind {
    FileName,
    DirName,
}

pub struct MarkerDef {
    pub kind: MarkerKind,
    pub name: &'static str,
    pub language_hint: Option<&'static str>,
}

pub struct MarkerHit {
    pub name: String,
    pub kind: MarkerKind,
    pub language_hint: Option<String>,
}
```

### 3.2. Registry маркеров (`markers/mod.rs`)

```rust
mod types;
mod rust;
mod node;
mod python;
mod go;
mod jvm;
mod ruby;
mod php;
mod cpp;
mod make;
mod git;

pub use types::{MarkerDef, MarkerHit, MarkerKind};

/// Единая таблица по умолчанию: конкатенация групп в стабильном порядке.
pub fn default_markers() -> Vec<&'static MarkerDef> {
    // или static slice, собранный из group slices
    let mut out = Vec::new();
    out.extend(rust::MARKERS);
    out.extend(node::MARKERS);
    out.extend(python::MARKERS);
    out.extend(go::MARKERS);
    out.extend(jvm::MARKERS);
    out.extend(ruby::MARKERS);
    out.extend(php::MARKERS);
    out.extend(cpp::MARKERS);
    out.extend(make::MARKERS);
    out.extend(git::MARKERS);
    out
}

// Совместимость с M2.1: при желании оставить
// pub static DEFAULT_MARKERS: &[MarkerDef] = ... через once_cell / Lazy,
// либо миграция callers на default_markers().
```

Пример группы:

```rust
// markers/rust.rs
use super::types::{MarkerDef, MarkerKind};

pub static MARKERS: &[MarkerDef] = &[MarkerDef {
    kind: MarkerKind::FileName,
    name: "Cargo.toml",
    language_hint: Some("Rust"),
}];
```

Новый язык / маркер = **новый файл** (или одна строка в существующей группе) + регистрация в `mod.rs`. Без правок `candidates.rs`.

### 3.3. Detect (`detect/`, с M2.2)

```rust
pub trait StackDetector: Send + Sync {
    fn id(&self) -> &'static str;
    /// Может ли этот детектор что-то сказать по уже найденным hits.
    fn matches(&self, hits: &[MarkerHit]) -> bool;
    /// Заполнить / уточнить Stack (language, frameworks, markers).
    fn detect(&self, hits: &[MarkerHit], project_dir: &Path) -> Stack;
}
```

Registry в `detect/mod.rs` перечисляет `&'static dyn StackDetector`.
Оркестратор: по hits выбрать применимые детекторы (порядок стабильный), смержить в один `Stack` (политика merge — отдельное решение M2.2: first-wins / priority / union frameworks).

`candidates` **не** вызывает detect: facade `sniff` делает
`find_candidates` → для каждого candidate `detect` → `Project`.

---

## 4. Когда резать

| Момент | Действие |
|--------|----------|
| **M2.1 (сейчас)** | Закрыт. Единая таблица допустима. Рефакторинг **не** обязателен в том же PR. |
| **Перед / вместе с M2.2** | Разрезать `markers.rs` → `markers/{types,rust,node,…,mod}.rs`. Сразу писать `detect/` по файлам экосистем, **не** одним `match language`. |
| **Позже** | Новые маркеры и framework-эвристики только в своём `*.rs` + registry. |

Имеет смысл сделать split markers **первым коммитом ветки M2.2** (механический move + registry), затем добавлять detect-логику.

---

## 5. Инварианты

1. **Один язык / экосистема ≈ один файл** (markers group + detect implementation).
2. **Registry — единственное место перечисления** групп/детекторов.
3. **`candidates` не знает языков** — только имена и `MarkerKind`.
4. **`detect` не ходит в walk/skip** — работает с уже найденным candidate + hits (и при необходимости читает файлы проекта точечно).
5. **Публичный API M2.1** (`find_candidates`, типы hit/def) сохраняется или меняется только additive.
6. То же правило модульности, что в [raccpack-modularity.md](raccpack-modularity.md) для secrets/archive и в agent-prompt modular Rust.

---

## 6. Связь с follow-up M2.1 review

При приёмке M2.1 отдельно отмечены:

- матчить `MarkerKind` по real `file_type`, не только по имени;
- тест nested parent+child projects;
- **не складывать Stack-эвристики в один файл на M2.2.**

Этот документ фиксирует третий пункт как структурное решение.

---

## 7. Следующий шаг

1. При старте M2.2 — PR «split `markers.rs` → `markers/` registry» (без смены поведения).
2. Затем `detect/` с trait + по одному файлу на экосистему (хотя бы rust + node + fallback).
3. Обновить WORKLOG / AGENTS: ссылка на этот документ рядом с modularity secrets/archive.

*Правило «один язык = один файл + registry» после принятия не ослаблять: иначе detect и marker-таблицы снова свалятся в монолит между CLI, TUI и core.*
