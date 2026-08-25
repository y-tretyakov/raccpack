# WORKLOG_ALPHA — архив журнала Alpha (закрыт)

**Архивировано:** 2026-08-22  
**Бывший путь:** `WORKLOG.md` (корень)  
**Рекомендуемый путь в репо:** `docs/archive/WORKLOG_ALPHA.md`  
**Связанный MVP-архив:** `docs/archive/WORKLOG_MVP.md`  
**Спеки Alpha:** `docs/archive/alpha/`

---

## Итог вехи

| | |
|--|--|
| **Версия exit** | **0.3.0** |
| **Статус** | **ALPHA EXIT** |
| **Закрыто** | A1.1–A1.4, A2.1–A2.3, A3.1–A3.5, A4.1–A4.4 |
| **Следующая веха** | Detect v2 → 0.4.0 (D1.1 → bump **0.3.1**) |
| **Версии по этапам** | см. `docs/VERSION_ROADMAP.md` |

### Backlog Alpha (всё закрыто)

```
[x] A1.1–A1.4  age / stash / CLI
[x] A2.1–A2.3  rinse strategies / facade / CLI
[x] A3.1       facade raid fail-fast
[x] A3.2       ProgressSink + thin CLI raid
[x] A3.3       Atomic (staging + WAL + rollback, ORPHAN-1..4)  PR #78–#80
[x] A3.4       manifest JSON после success  PR #81
[x] A3.5       full CLI raid, exit 1, E2E, wiki  PR #82
[x] A4.1       GitClient + dig git_status  PR #85 → 0.2.12
[x] A4.2       config migrate + racc init  PR #86 → 0.2.13
[x] A4.3       tracing + -v  PR #87 → 0.2.14
[x] A4.4       integration + CI  PR #89 → **0.3.0**
```

---

## Ключевые решения Alpha (перенос в новый WORKLOG)

| Дата | Решение |
|------|---------|
| 2026-08-14 | После закрытого этапа Orchestrator сам squash-merge в `dev`, закрывает PR, удаляет ветку |
| 2026-08-19/20 | Default raid **Atomic**; FailFast ≡ A3.1; DryRun без WAL; remove_sources/rinse delete в **commit** |
| 2026-08-20 | Manifest только после successful Atomic commit; сбой записи manifest → success:false, артефакты остаются (staging уже снят) |
| 2026-08-20 | A3.5: exit **1** при `!success` (смена контракта A3.2) |
| 2026-08-21 | MSRV **1.85** (блокер blake3/sha2/age edition2024); не даунгрейдить age |
| 2026-08-21 | F-ERR-1: `From<ConfigError> for Error` |
| 2026-08-21 | Логи только stderr; `RUST_LOG` побеждает `-v`; passphrase/raw не в логах |
| 2026-08-22 | docs: `docs/alpha/` → `docs/archive/alpha/`; каркасы `docs/detect/`, `docs/beta/` |

---

## Что реализовано (кратко по фазам)

### A1 Stash
age + zeroize; manifest без raw; path containment; den/secrets; CLI env/TTY/stdin passphrase.

### A2 Rinse
Strategies rust/node/python (+jvm/go/generic); DryRun/Commit; CLI.

### A3 Raid
Fail-fast база → **Atomic** (staging `{raid_id}`, WAL JSONL, rollback, ORPHAN tests); manifest schema v1; CLI toggles + `--fail-fast`; wiki raid.

### A4 DX
GitClient process + soft dig enrichment; init + config_version migrate; tracing `-v`; GitHub CI.

---

## Открытые follow-up на момент Alpha exit

*(перенесены в новый `WORKLOG.md` — не потерять)*

### Блокеры Detect v2
- Нет (можно стартовать D1.1).

### Техдолг / hygiene (не блокируют D1)
| ID | Тема | Когда |
|----|------|--------|
| F-SKIP-1 | Два списка имён skip vs cleanup strategies; нужен единый источник / `default_pack()` | B3.1 / pack |
| F-PACK-SIZE | `archive/pack.rs` ~436 строк | next pack touch |
| F-ATOMIC-SIZE | `app/raid/atomic.rs` ~410 строк | next atomic touch |
| F-TEST-SIZE | `raid_atomic.rs` ~1036, `cli_raid.rs` ~714 | next test touch |
| F-CLI-SIZE | `cli.rs` ~941 (тест-тяжёлый) | hygiene |
| F-TRACE-RAID | info-события raid/rinse/pack | optional |
| F-DOC-LINKS | doc-comments → `docs/alpha/…` устарели после archive | next file touch |
| F-MSRV | MSRV 1.85 зафиксирован | — |
| P2-5 | `zstd_level` из `[advanced]` | later |
| P2-6 | cost content-deny (size-cap) | later |
| P2-7 | сужение public API | R1 |
| P2-8 | типизация `Error::Other` | CLI UX / R3 |
| OS | Windows HOME/XDG best-effort | R2.2 |

### Закрытые в Alpha (не тащить снова)
- F-PATH-1 containment stash — A1.2/A1.3  
- F-PATH-3 staging under den — A1.3  
- F-ERR-1 ConfigError→Error — A4.2  
- Atomic orphan ORPHAN-1..4 — A3.3  
- Wiki raid / init / verbose — A3.5 / A4.x  

---

## PR map (Alpha, выборочно)

| PR | Тема |
|----|------|
| #75–#77 | A3.1 / A3.2 |
| #78–#80 | A3.3 PR1–PR3 |
| #81 | A3.4 manifest |
| #82 | A3.5 CLI E2E wiki |
| #85–#89 | A4.1–A4.4 |
| #91 | docs archive alpha + detect/beta scaffolds |

---

## Полный построчный журнал

Исходный развёрнутый текст этапов (A3.3 PR details, A4.x, docs) сохраняйте из git history коммита **до** замены `WORKLOG.md`, либо дополните этот файл содержимым бывшего корневого `WORKLOG.md` при переносе в репозиторий:

```bash
# в репо, до перезаписи WORKLOG.md:
git show HEAD:WORKLOG.md > docs/archive/WORKLOG_ALPHA_FULL.md
# или:
mv WORKLOG.md docs/archive/WORKLOG_ALPHA.md
```

Этот файл — **аудиторская выжимка + статус**. Для юридической полноты истории агентов предпочтителен полный dump из git.
