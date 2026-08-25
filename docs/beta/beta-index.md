# Beta → 0.5.0 — полный индекс спек

**Предусловия:** Alpha 0.3.0 (atomic raid + CLI). Detect v2 (0.4.x) желателен до B1.2 DAG UI.

**Цель:** CLI + TUI + Desktop; den v1; no raw secrets by default; ephemeral reveal opt-in; hardening.

| Фаза | Смысл | Индекс |
|------|--------|--------|
| **B1** | TUI + reveal | [b1-index.md](b1-index.md) |
| **B2** | Desktop + reveal | [b2-index.md](b2-index.md) |
| **B3** | Security + EphemeralSecret | [b3-index.md](b3-index.md) |
| **B4** | den gc, parallel, docs, tag | [b4-index.md](b4-index.md) |

```text
B1 → B2 → B3 → B4 → v0.5.0
```

B3.4 (`EphemeralSecret`) желательно до B1.5 / B2.5.

## Beta exit criteria

- [ ] CLI + TUI + Desktop: sniff / dig / atomic raid
- [ ] Reveal opt-in; raw not in store/logs/JSON
- [ ] EnabledGroups; content-deny; path containment; den perms
- [ ] `racc den list|gc`; parallel_jobs; docs
- [ ] Tag v0.5.x

## Wiki

- [wiki-tui.md](wiki-tui.md) · [wiki-desktop.md](wiki-desktop.md) · [wiki-den.md](wiki-den.md) · [wiki-security.md](wiki-security.md)
