# Desktop (Tauri) — Beta

Графический клиент. Логика только в core через Tauri commands.

---

## Запуск (dev)

```bash
cd apps/desktop-ui && pnpm install && cd ../..
cargo tauri dev
```

Build:

```bash
cargo tauri build
```

---

## Сценарии

1. **Settings** — scan root + den  
2. **Scan** — проекты (sniff)  
3. **Secrets** — dig (masked, risk, git status)  
4. **Raid** — dry-run → confirm → passphrase → progress → result (atomic)  
5. **Reveal** — confirm → ephemeral modal (не Zustand)

---

## Безопасность

- React не получает raw в store  
- Passphrase только на время invoke  
- Reveal минует global store  

---

## Эквивалент CLI

```bash
racc sniff --root PATH --json
racc dig --project PATH --json
racc raid --project PATH --den PATH --yes --json
```
