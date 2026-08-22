# Detect v2 → 0.4.x — Composite DAG

**Место в roadmap:** после Alpha 0.3.0 (atomic raid), **до** Beta UI — чтобы TUI/Desktop сразу получили корректное дерево стека.

**Проблема PriorityTable:** «один язык ≈ один файл + статический приоритет» слепнет на monorepo (Rust backend + React frontend): rinse может снести не то или оставить гигабайты мусора.

**Цель:** модульные детекторы + **WorkspaceDetector** строят **DAG/дерево** технологий; `rinse`/`pack`/`sniff` используют scope.

| Фаза | Документы |
|------|-----------|
| **D1** | [d1-index.md](d1-index.md) — trait, DTO, config mode |
| **D2** | [d2-index.md](d2-index.md) — Composite / DAG / compat |
| **D3** | [d3-index.md](d3-index.md) — rinse / sniff impact + fixtures |
| **D4** | [d4-index.md](d4-index.md) — batch raid по scan root (спеки появятся позже — ссылка допустима) |

```text
D1.1 → D1.2 → D1.3 → D2.1 → D2.2 → D2.3 → D3.1 → D3.2 → D3.3 → D4.1 → D4.2 → D4.3 → D4.4
```

## Exit criteria Detect v2

- [ ] На monorepo `sniff` показывает корректное дерево (`stack_tree`)
- [ ] `rinse` удаляет только релевантный мусор по scope
- [ ] `detect.mode = priority_table` — **без регрессий** (default)
- [ ] `composite_dag` включается конфигом / CLI
- [ ] Плоский `stack: String` остаётся в JSON
- [ ] `racc raid --root` прогоняет все проекты scan root батчем (каждый проект — отдельный raid; planned, D4.3)

## Зависимости

- **Требует:** M2 detect modules (существующие экосистемы)
- **Не требует:** TUI/Desktop
- **После:** Beta B1.2 может показать дерево

## Wiki / CLI

Примеры в [d1.3](d1.3-detect-mode-config.md), [d3.2](d3.2-sniff-tree-output.md); user-facing — обновить `sniff` / `configuration` wiki при D3.2.
