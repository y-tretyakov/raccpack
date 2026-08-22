# D4 — Batch raid по scan root (конец Detect v2)

**Место в вехе:** после D3.3 (exit gate Detect), **перед** tag 0.4.0 — или сразу после D3.2, если list проектов уже стабилен.

| Этап | Файл |
|------|------|
| D4.1 | [d4.1-batch-raid-design.md](d4.1-batch-raid-design.md) |
| D4.2 | [d4.2-facade-raid-batch.md](d4.2-facade-raid-batch.md) |
| D4.3 | [d4.3-cli-raid-root.md](d4.3-cli-raid-root.md) |
| D4.4 | [d4.4-wiki-e2e-batch.md](d4.4-wiki-e2e-batch.md) |

```text
D3.3 → D4.1 → D4.2 → D4.3 → D4.4 → Detect v2 0.4.0
```

**Зачем в Detect v2:** веха как раз про «что считать проектом». Batch raid = применить уже найденный список проектов без shell-цикла. DAG (D2–D3) улучшает границы проекта; batch их обходит по одному.
