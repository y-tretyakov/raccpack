# Beta → 0.5.0 — полный индекс спек (переписано под TUI v2/v4)

**Предусловия:** Alpha 0.3.x (atomic raid, CLI) + желательно Detect v2 0.4.x (stack_tree).  
**Источники TUI:** `raccpack-tui-spec-v2.md`, `raccpack-tui-spec-v4-verified.md` (проверено по core `dev`).

**Цель Beta:** CLI + **операционный TUI** + Desktop; den v1; no raw secrets by default; ephemeral reveal opt-in; hardening.

| Фаза | Смысл | Индекс |
|------|--------|--------|
| **B1** | TUI — thin client, Operations, Preview, Raid-as-op | [b1-index.md](b1-index.md) |
| **B2** | Desktop (Tauri) — тот же core, masked DTO | [b2-index.md](b2-index.md) |
| **B3** | Security + EphemeralSecret + reveal + threat | [b3-index.md](b3-index.md) |
| **B4** | den gc, parallel, docs, tag 0.5.0 | [b4-index.md](b4-index.md) |

```text
(Detect 0.4) → B1 TUI → B2 Desktop → B3 security/reveal → B4 → v0.5.0
```

**Порядок safety:** B3.4 `EphemeralSecret` желательно **до** TUI/Desktop reveal UI (B1.7 / B2.5).

## Beta exit criteria

- [ ] `racc tui` — Inspect → Preview → Execute → Verify; thin client over core only
- [ ] Raid = **Operation**, не отдельный «экран-приложение»; Atomic/FailFast честно
- [ ] CLI + TUI + Desktop: sniff / dig / atomic raid
- [ ] Reveal opt-in; raw не в store/logs/JSON/render buffer
- [ ] EnabledGroups; content-deny; path containment; den perms
- [ ] `racc den list|gc`; docs; tag **v0.5.0**

## Главные решения TUI (зафиксированы)

1. TUI ≠ второй CLI; только presentation над `raccpack-core`.
2. **Raid — Operation**, не View.
3. Preview обязателен перед WRITE/DESTRUCTIVE.
4. `ProgressEvent` / `ProgressSink` — единственный источник progress.
5. Core sync → `spawn_blocking`; UI loop non-blocking.
6. Cancellation: **нет** в core P0 — UX честный («leave view» / stop-after-phase), без fake cancel.
7. Config (`RaccConfig`) ≠ `TuiPreferences`.
8. Keyboard-first; 80×24; NO_COLOR; no plaintext secrets.

## Wiki

- [wiki-tui.md](wiki-tui.md) · [wiki-desktop.md](wiki-desktop.md) · [wiki-den.md](wiki-den.md) · [wiki-security.md](wiki-security.md)
