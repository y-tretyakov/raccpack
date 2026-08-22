# Security — masked by default, reveal opt-in

**Статус:** Beta

---

## Инварианты

1. Сырые секреты **не** в JSON-отчётах, логах, Zustand.  
2. Шифрование — age (passphrase); zeroize.  
3. Destructive ops — dry-run / confirm.  
4. Raid — **Atomic** (rollback, нет orphan).  
5. Reveal — только explicit opt-in, ephemeral.

---

## Reveal

### CLI

```bash
racc reveal --project ~/DEV/PROJS/my-api --file .env
# Confirm: REVEAL
```

### TUI

Клавиша `v` на finding → confirm → modal → Esc wipe.

### Desktop

Кнопка Reveal → confirm → isolated modal (не store).

---

## Проверка no-leak

```bash
SECRET_VAL='unique-leak-token'
echo "API_KEY=$SECRET_VAL" > /tmp/proj/.env
RACCPACK_PASSPHRASE='unique-pass' \
  racc stash --project /tmp/proj --den /tmp/den --yes -vv 2>&1 | tee /tmp/out.log
grep -F "$SECRET_VAL" /tmp/out.log && echo FAIL || echo OK
grep -F 'unique-pass' /tmp/out.log && echo FAIL || echo OK
racc dig --project /tmp/proj --json | grep -F "$SECRET_VAL" && echo FAIL || echo OK
```

---

## Groups (config)

```toml
[sensitive]
groups = ["env", "keys", "cloud"]
```

```bash
racc dig --project PATH --groups env,keys
```

---

## Audit (optional)

```toml
[security]
reveal_audit = true
reveal_audit_path = "~/.local/state/raccpack/reveal-audit.log"
```

Лог факта запроса **без** значения секрета.
