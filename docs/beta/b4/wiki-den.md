# Den — хранилище артефактов

```text
{den}/
├── .den-version
├── README.txt
├── secrets/yyyy/mm/*.age
├── packs/yyyy/mm/*.tar.zst
├── manifests/yyyy/mm/*.json
└── staging/              # можно чистить
```

---

## Примеры CLI

```bash
racc den list --den ~/.raccpack/den
racc den list --den ~/.raccpack/den --json

# dry-run gc
racc den gc --den ~/.raccpack/den --older-than 7d
# commit gc
racc den gc --den ~/.raccpack/den --older-than 7d --yes

racc init --den ~/.raccpack/den --ensure-den
```

**gc не удаляет** secrets/packs/manifests — только staging.

Права: den `0700`, `.age` `0600` (best-effort). Не коммитьте den в git.
