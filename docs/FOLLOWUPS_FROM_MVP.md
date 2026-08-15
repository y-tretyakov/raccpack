# Open follow-ups из MVP 0.1.0

Источник: архивный `WORKLOG_MVP.md` (MVP closed).  
Здесь **только открытые** пункты — уже закрытые в MVP не повторяются.

Статусы: `open` | `partial` | `accepted-as-is` (осознанно не делать до вехи).

Вставлять в работу **целевой фазы**, не возвращаться править закрытый MVP WORKLOG.

---

## Сводка по фазам

| Цель | Кол-во open (примерно) | Главное |
|------|------------------------|---------|
| **A1 Stash** | 2–3 | `is_under_root` / path containment; den/staging safety |
| **A2 Rinse** | 1–2 | `default_pack`-подобные lists пересекаются с rinse strategies (уже в A2.1) |
| **A3 Raid** | 1 | progress multi-phase уже в A3.2 |
| **A4 DX** | 3–4 | ConfigError↔Error, CI, MSRV |
| **Beta / later** | много | EnabledGroups, Windows paths, manifest-deps parse, pub-use audit, EN wiki |

---

## 1. Безопасность путей (обязательно до/в stash & pack production paths)

| ID | Суть | Источник | Куда |
|----|------|----------|------|
| **F-PATH-1** | `is_under_root` / path-containment перед pack/stash (symlink, `..`, escape из root) | M1.4 follow-up; M4.1 «контракт output вне source» | **A1.2/A1.3** (stash select + place), **уже частично pack** — добить единый helper в `scan/` или `fsutil` |
| **F-PATH-2** | Root/cache path comparison без canonicalize → разные ключи для `/a/b` vs `/a/../a/b` | M2.3 PR#15 C | **A4** или Beta: опциональный canonicalize policy |
| **F-PATH-3** | Staging path не должен лежать внутри `project` (runtime guard уже в M4.3) — держать инвариант в raid/stash | M4.3 | **A1.3 / A3.1** проверить при stash staging |

**Вставка:** `a1.2`, `a1.3`, `a3.1` — явный DoD: path containment.

---

## 2. Ошибки и public API

| ID | Суть | Источник | Куда |
|----|------|----------|------|
| **F-ERR-1** | Merge `ConfigError` ↔ `domain::Error` (`From` или единый enum), чтобы CLI/UI не ветвились | M1.2, M1.3, M1.4 | **A4.2** (init/migrate) или раньше при следующем CLI touch — **не позже Beta B3** |
| **F-ERR-2** | `Error::Io` не `PartialEq` — сравнения через `matches!` | M1.2 | accepted-as-is (документировать в test style) |
| **F-ERR-3** | Типизация `Error::Other` → `DenInsideProject` / `InvalidOutputName` | M4.3 P2-8 | CLI UX / **A4** или Beta R3 |
| **F-API-1** | Сужение public `lib.rs` re-exports | M4.3 P2-7 | roadmap **9.1 pub use audit** (RC) |

---

## 3. Skip / pack / clean policies

| ID | Суть | Источник | Куда |
|----|------|----------|------|
| **F-SKIP-1** | `SkipPolicy::default_pack()` с расширенным списком (`.next`, `coverage`, `.turbo`, …) | M4.3 P1-4 | **A2.1** (rinse already has some) + **pack path** при следующем pack touch / A3 |
| **F-SKIP-2** | Отдельная **file-policy** (не dir): `.DS_Store` и т.п. | M1.4 | dig/pack later / Beta |
| **F-SKIP-3** | `POLICY_FINGERPRINT = "default_scan_v1"` — bump при смене default_scan | M2.3 | любой PR, меняющий SkipPolicy defaults |
| **F-SKIP-4** | Hidden-root + `skip_hidden_dirs` → пустой walk; UX warning | M1.4 | CLI/TUI Beta |

---

## 4. Detect / markers / secrets

| ID | Суть | Источник | Куда |
|----|------|----------|------|
| **F-DET-1** | Парсинг `package.json` / `Cargo.toml` deps для frameworks (next/react/vue/axum) | M2.2 | **Alpha optional** или Beta; не блокер A1–A4 |
| **F-DET-2** | Framework modules nested: `detect/node/next.rs` … при 4–5+ rules | M2.2 PR#13 | при росте detect |
| **F-DET-3** | Extension markers `*.csproj` / `*.sln` | M2.1 | optional later |
| **F-DET-4** | `extra_markers` owned `String` для config/CLI | M2.1 | когда config groups появятся (**B3 EnabledGroups**) |
| **F-DET-5** | Case-insensitive FS policy (macOS/Windows) | M2.1 | cross-platform smoke RC |
| **F-SEC-1** | Tune noisy `generic_*` content markers; min/max length on markers; вернуть `telegram_bot` | M3.2 PR#19 | Beta B3 security |
| **F-SEC-2** | `EnabledGroups` + `secret_groups_override` в AppContext | M2.3, M3 | **B3** (roadmap) |
| **F-SEC-3** | Content-deny cost: extension allow-list / size-cap сверх текущих limits | M4.3 P2-6 | optional optimize |

---

## 5. Sniff / cache / size

| ID | Суть | Источник | Куда |
|----|------|----------|------|
| **F-SNIFF-1** | Cache mtime-инвалидация дерева | M2.3 | optional later |
| **F-SNIFF-2** | `project_size_bytes` fail-fast vs skip+continue | M2.2 | UX policy later |
| **F-SNIFF-3** | size error → 0 in sniff (`unwrap_or_default`) — fail-fast option | M2.3 PR#15 A | later |

---

## 6. Config / platform / DX

| ID | Суть | Источник | Куда |
|----|------|----------|------|
| **F-CFG-1** | Windows HOME/XDG → USERPROFILE / `directories` crate | M1.3, M1.4 | RC cross-platform |
| **F-CFG-2** | MSRV 1.75 — пересмотреть при росте deps | M1.1 | ongoing |
| **F-CFG-3** | `zstd_level` из config `[advanced]` | M4.3 P2-5 | when advanced section lands |
| **F-CFG-4** | Config migrate chain (готово к v2+) | — | **A4.2** (уже в спеке) |

---

## 7. Pack / archive

| ID | Суть | Источник | Куда |
|----|------|----------|------|
| **F-PACK-1** | Empty dirs not in archive (files-only M4.1) — если нужно сохранить empty structure | M4.1 | optional |
| **F-PACK-2** | Parallel walk для pack — blocker снят DFS rewrite M4.3-followup | M4.3 | **B4 parallel_jobs** later |

---

## 8. Docs / wiki

| ID | Суть | Источник | Куда |
|----|------|----------|------|
| **F-DOC-1** | EN wiki полный перевод | docs-этап | Beta docs |
| **F-DOC-2** | Redirect map старых Writerside URLs | docs | optional |
| **F-DOC-3** | VitePress assets path `wiki/public/` — проверить при апгрейде | docs | maintenance |

---

## Приоритет для Alpha (A1–A4)

Сделать **обязательно** в Alpha:

1. **F-PATH-1** — path containment в stash (и проверка pack/raid)  
2. **F-ERR-1** — хотя бы `From<ConfigError> for Error` или единый map в CLI  
3. **F-SKIP-1** — `default_pack` или согласованность rinse/pack skip lists  
4. **F-CFG-4** — migrate + init (A4.2)  
5. CI green (A4.4)

Остальное — Beta/RC, не блокирует Alpha exit.

---

## Куда уже вставлено в спеки

| Файл | Что добавлено |
|------|----------------|
| `a1-index.md` / `a1.2` / `a1.3` | F-PATH-1 DoD |
| `a2.1` | связь F-SKIP-1 с strategies |
| `a3.1` | staging/path notes |
| `a4.2` | F-ERR-1, F-CFG-4 |
| `a4.4` | F-CFG-2 CI/MSRV note |

(см. правки в тех же артефактах, если секции «Follow-ups from MVP» присутствуют)
