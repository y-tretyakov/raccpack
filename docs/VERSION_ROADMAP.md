# Raccpack — версии по этапам

**Правило:** после закрытия **каждого** этапа bump `workspace.package.version` (и tag опционально `vX.Y.Z`).  
Сборка из исходников = по версии видно, какой последний закрытый этап.

**Формат:** `0.MINOR.PATCH` до 1.0.0 (ломающие изменения API допустимы).

---

## Якоря вех (совместимо с roadmap v1)

| Веха | Exit-версия | Смысл |
|------|-------------|--------|
| **MVP** | **0.1.0** | sniff → dig → pack → den |
| **Alpha** | **0.3.0** | atomic raid + stash + rinse + git/DX CLI |
| **Detect v2** | **0.4.0** | composite DAG + batch raid (`racc raid --root`) |
| **Beta** | **0.5.0** | TUI + Desktop + reveal + hardening |
| **RC** | **0.9.0** | freeze API/den |
| **Stable** | **1.0.0** | semver-стабильность |

Внутри вехи — **patch** (и промежуточный minor только на границе вехи).

---

## Текущая позиция

> **Detect v2 exit (0.4.0) done.** **Alpha exit: 0.3.0.** **B1.1 TUI skeleton done (0.4.1).**

| | |
|--|--|
| **Текущая версия workspace** | **`0.4.1`** |
| Последний этап | **B1.1** — TUI skeleton (`raccpack-tui`) |
| Следующий этап | **B1.2** — TUI sniff screen (Beta → `0.5.0`) |
| Detect v2 exit | **`0.4.0`** ✅ (batch raid CLI + wiki + E2E) |

```text
0.1.0  MVP
0.2.0 … 0.2.11  Alpha A1–A3
0.2.12 … 0.3.0  Alpha A4
0.3.1 … 0.4.0   Detect v2   ✅ D4.4 done
0.4.1           Beta B1.1   ← ВЫ ЗДЕСЬ (TUI skeleton)
0.4.2 …        Beta B1.2+
0.5.0 …        Beta exit
0.9.0 …        RC
1.0.0          Stable
```

---

## MVP → 0.1.0 (закрыто)

Все этапы M1–M4 закрыты одним exit-тегом **0.1.0** (исторически).  
Для ретроспективы поэтапных номеров не восстанавливаем.

| Этап | Версия (ретро) | Статус |
|------|----------------|--------|
| M1.1–M1.4 | → 0.1.0 | ✅ |
| M2.1–M2.4 | → 0.1.0 | ✅ |
| M3.1–M3.4 | → 0.1.0 | ✅ |
| M4.1–M4.4 | **0.1.0** exit | ✅ |

---

## Alpha → 0.3.0

Старт Alpha после MVP: **0.2.0**.

### A1 — Stash (age)

| Этап | Версия | Статус | Фича (кратко) |
|------|--------|--------|----------------|
| A1.1 | **0.2.0** | ✅ | age + zeroize |
| A1.2 | **0.2.1** | ✅ | stash manifest + remove + path containment |
| A1.3 | **0.2.2** | ✅ | facade stash + den/secrets |
| A1.4 | **0.2.3** | ✅ | CLI `racc stash` |

### A2 — Rinse

| Этап | Версия | Статус | Фича |
|------|--------|--------|------|
| A2.1 | **0.2.4** | ✅ | cleanup strategies + config |
| A2.2 | **0.2.5** | ✅ | facade rinse DryRun/Commit |
| A2.3 | **0.2.6** | ✅ | CLI `racc rinse` |

### A3 — Raid (atomic)

| Этап | Версия | Статус | Фича |
|------|--------|--------|------|
| A3.1 | **0.2.7** | ✅ | facade raid fail-fast (база) |
| A3.2 | **0.2.8** | ✅ | ProgressSink + thin CLI `racc raid` |
| A3.3 | **0.2.9** | ✅* | Atomic: staging + WAL + rollback |
| A3.4 | **0.2.10** | ✅* | Manifest только после success |
| A3.5 | **0.2.11** | ✅* | Full CLI, exit 1, E2E, wiki |

\* По вашей фиксации «сделано до A3 включительно». Если A3.3–A3.5 ещё в работе — текущая версия = последний **реально** закрытый (например 0.2.8), см. § «Как поправить».

### A4 — Git и DX

| Этап | Версия | Статус | Фича |
|------|--------|--------|------|
| A4.1 | **0.2.12** | ✅ | GitClient + git_status в dig |
| A4.2 | **0.2.13** | ✅ | config migrate + `racc init` |
| A4.3 | **0.2.14** | ✅ | tracing + `--verbose` |
| A4.4 | **0.3.0** | ✅ | integration + CI = **Alpha exit** |

---

## Detect v2 → 0.4.0

После Alpha. Старт **0.3.1** (или сразу 0.4.0-pre); exit **0.4.0**.

| Этап | Версия | Статус | Фича |
|------|--------|--------|------|
| D1.1 | **0.3.1** | ✅ | StackDetector trait + registry |
| D1.2 | **0.3.2** | ✅ | Detection / StackNode DTO |
| D1.3 | **0.3.3** | ✅ | `detect.mode` config + CLI |
| D2.1 | **0.3.4** | ✅ | WorkspaceDetector → tree |
| D2.2 | **0.3.5** | ✅ | conflict merge (`detect::merge`) |
| D2.3 | **0.3.6** | ✅ | flat stack + stack_tree compat + tree render |
| D3.1 | **0.3.7** | ✅ | rinse по DAG scopes |
| D3.2 | **0.3.6** | ✅ | sniff tree output (shipped in D2.3, closed as D3.2) |
| D3.3 | **0.3.8** | ✅ | fixtures монорепо (D3 phase done) |
| D4.1 | — | ✅ | batch raid design (`--root` vs `--project`; docs) — design-only (без bump) |
| D4.2 | **0.3.8** | ✅ | facade `raid_batch` (1 project = 1 raid, sequential, continue-on-error; без bump) |
| D4.3 | **0.3.9** | ✅ | CLI `racc raid --root` (+ `--only`/`--limit`/`--stop-on-error`) |
| D4.4 | **0.4.0** | ✅ | wiki + E2E = **Detect v2 exit** |

---

## Beta → 0.5.0

### B1 — TUI

| Этап | Версия | Статус |
|------|--------|--------|
| B1.1 | **0.4.1** | ✅ skeleton |
| B1.2 | **0.4.2** | ⬜ sniff screen |
| B1.3 | **0.4.3** | ⬜ dig screen |
| B1.4 | **0.4.4** | ⬜ raid + progress |
| B1.5 | **0.4.5** | ⬜ reveal modal |

### B2 — Desktop

| Этап | Версия | Статус |
|------|--------|--------|
| B2.1 | **0.4.6** | ⬜ Tauri + React skeleton |
| B2.2 | **0.4.7** | ⬜ BFF sniff/dig/raid |
| B2.3 | **0.4.8** | ⬜ UI tables |
| B2.4 | **0.4.9** | ⬜ raid + passphrase |
| B2.5 | **0.4.10** | ⬜ reveal IPC |

### B3 — Security + reveal

| Этап | Версия | Статус |
|------|--------|--------|
| B3.1 | **0.4.11** | ⬜ content-deny + default_pack |
| B3.2 | **0.4.12** | ⬜ EnabledGroups |
| B3.3 | **0.4.13** | ⬜ path containment + perms |
| B3.4 | **0.4.14** | ⬜ EphemeralSecret |
| B3.5 | **0.4.15** | ⬜ CLI reveal |
| B3.6 | **0.4.16** | ⬜ threat checklist |
| B3.7 | **0.4.17** | ⬜ reveal audit |

### B4 — Productization

| Этап | Версия | Статус |
|------|--------|--------|
| B4.1 | **0.4.18** | ⬜ `racc den` list/gc |
| B4.2 | **0.4.19** | ⬜ parallel_jobs |
| B4.3 | **0.4.20** | ⬜ user docs |
| B4.4 | **0.5.0** | ⬜ tag = **Beta exit** |

---

## RC → 0.9.0

| Этап | Версия | Статус |
|------|--------|--------|
| R1.1–R1.4 | **0.5.1 … 0.5.4** | ⬜ API freeze |
| R2.1–R2.4 | **0.5.5 … 0.5.8** | ⬜ quality |
| R3.1–R3.4 | **0.5.9 … 0.5.12** | ⬜ UX RC |
| R4.1–R4.4 | **0.9.0** | ⬜ validation exit (можно сжать patch→0.9.0 на R4.4) |

*Альтернатива RC:* сразу `0.9.0-rc.1` … `0.9.0` без длинной 0.5.x — зафиксировать в AGENTS при старте RC.

---

## Stable → 1.0.0

| Этап | Версия | Статус |
|------|--------|--------|
| S1.1–S1.4 | **1.0.0** | ⬜ release |

---

## Практика в репо

### Где менять

```toml
# Cargo.toml (workspace)
[workspace.package]
version = "0.4.0"
```

Все crates: `version.workspace = true`.

### Когда bump

1. Этап **CLOSED** (DoD green, PR в `dev`).
2. В том же PR (или follow-up): bump version + строка в `CHANGELOG.md` / WORKLOG.
3. Опционально: annotated tag `v0.2.11` на merge в dev (или только на exit-вехи 0.1.0 / 0.3.0 / 0.4.0 / 0.5.0 / 0.9.0 / 1.0.0).

### Как читать версию

```bash
cargo run -p raccpack-cli -- --version
# raccpack-cli 0.4.1   →  B1.1 (TUI skeleton, Beta)
```

| Версия | Значит «есть» |
|--------|----------------|
| ≥ 0.2.3 | stash CLI |
| ≥ 0.2.6 | rinse CLI |
| ≥ 0.2.8 | thin raid CLI |
| ≥ 0.2.9 | atomic raid |
| ≥ 0.2.11 | full raid CLI + wiki |
| ≥ 0.2.12 | git_status в dig (GitClient) |
| ≥ 0.2.13 | config migrate + `racc init` |
| ≥ 0.2.14 | tracing-логи без секретов + глобальный `--verbose` |
| ≥ 0.3.0 | **Alpha complete**: integration + CI, MSRV 1.85 |
| ≥ 0.3.0 | Alpha complete (git, init, -v, CI) |
| ≥ 0.3.1 | Detect v2 start: StackDetector trait + detector_registry (внутреннее, без изменений CLI) |
| ≥ 0.3.2 | Detection / StackNode DTO + Project.stack_tree (аддитивно, JSON back-compat) |
| ≥ 0.3.3 | `detect.mode` config + `racc sniff --detect-mode` (`composite_dag` = заглушка до D2.x) |
| ≥ 0.3.4 | Composite DAG pipeline: `sniff --detect-mode composite_dag` заполняет `stack_tree` (experimental) |
| ≥ 0.3.5 | Merge policy `detect::merge` (nesting, framework union, same-scope merge; внутреннее, без изменений CLI) |
| ≥ 0.3.6 | Compat flat `stack`/`stack_tree` + indent tree render для `composite_dag` в human sniff output |
| ≥ 0.3.7 | Rinse по DAG scopes (scoped trash discovery per ecosystem) |
| ≥ 0.3.8 | D3 phase done + D4.2 facade raid_batch (sniff tree output, fixtures, batch raid core) |
| ≥ 0.3.9 | CLI `racc raid --root` (D4.3) |
| ≥ 0.4.0 | **Detect v2 complete**: composite DAG + batch raid CLI + wiki + E2E (D4.4) |
| ≥ 0.5.0 | TUI + Desktop + reveal |

---

## Как поправить, если A3.3–A3.5 ещё не все в dev

| Реально закрыто до | Ставить version |
|--------------------|-----------------|
| A3.2 only | **0.2.8** |
| A3.3 | **0.2.9** |
| A3.4 | **0.2.10** |
| A3.5 | **0.2.11** |

Таблица этапов выше не меняется — двигается только «текущая» точка.

---

## Сводка «сейчас»

```text
Текущая версия:  0.4.1
Этап:            B1.1 (TUI skeleton) — CLOSED; Beta start
Следующий bump:  0.4.2 (B1.2 sniff screen)
```
