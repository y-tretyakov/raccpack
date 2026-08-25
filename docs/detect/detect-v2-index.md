# Detect v2 → 0.4.0 — Composite DAG + batch raid

**Место:** после Alpha 0.3.0, **до** Beta.

| Фаза | Документы |
|------|-----------|
| **D1** | [d1-index.md](d1-index.md) — trait, DTO, detect.mode |
| **D2** | [d2-index.md](d2-index.md) — WorkspaceDetector / DAG / compat |
| **D3** | [d3-index.md](d3-index.md) — rinse scopes, sniff tree, fixtures |
| **D4** | [d4-index.md](d4-index.md) — **batch `raid --root`** (конец вехи) |

```text
D1 → D2 → D3 → D4 → tag 0.4.0
```

## Exit criteria

- [ ] Monorepo: `stack_tree` в composite_dag; rinse по scope
- [ ] `priority_table` default без регрессий
- [ ] **`racc raid --root`**: каждый проект — отдельный raid (секреты + pack)
- [ ] Wiki без обязательного shell-цикла для multi-project

## Версии

D1.1 → 0.3.1 … D3.3 → … D4.4 → **0.4.0**
