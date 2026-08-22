#!/usr/bin/env fish
# raid-all.fish — полный raid по всем проектам из sniff (scan_root/den из config)

set -q RACC; or set RACC racc
set -q DRY_RUN; or set DRY_RUN 0
set -q CONTINUE_ON_ERROR; or set CONTINUE_ON_ERROR 1
set -q EXTRA_RAID_ARGS; or set EXTRA_RAID_ARGS ""

if not command -q $RACC
    echo "need: $RACC" >&2
    exit 1
end
if not command -q jq
    echo "need: jq" >&2
    exit 1
end

set -q RACCPACK_PASSPHRASE; or set RACCPACK_PASSPHRASE ""
if test -z "$RACCPACK_PASSPHRASE"; and not string match -q '*--no-stash*' -- $EXTRA_RAID_ARGS
    echo "ERROR: set RACCPACK_PASSPHRASE (or EXTRA_RAID_ARGS=--no-stash)" >&2
    exit 1
end
set -gx RACCPACK_PASSPHRASE $RACCPACK_PASSPHRASE

# ── sniff (root/den из config.toml) ────────────────────────
echo "==> sniff --json --force-refresh"
set SNIFF_JSON ($RACC sniff --json --force-refresh)
or begin
    echo "sniff failed" >&2
    exit 1
end

set PROJECTS (echo $SNIFF_JSON | jq -r '.report.projects[].path // empty')

if test (count $PROJECTS) -eq 0
    echo "No projects found."
    exit 0
end

echo "Found "(count $PROJECTS)" project(s):"
for p in $PROJECTS
    echo "  - $p"
end
echo

# ── raid loop ──────────────────────────────────────────────
set ok 0
set fail 0
set failed_list

for proj in $PROJECTS
    echo "────────────────────────────────────────"
    echo "==> raid: $proj"

    set RAID_ARGS raid --project $proj
    if test "$DRY_RUN" = "1"
        set -a RAID_ARGS --dry-run
    else
        set -a RAID_ARGS --yes
    end
    if test -n "$EXTRA_RAID_ARGS"
        set -a RAID_ARGS (string split ' ' -- $EXTRA_RAID_ARGS)
    end

    if $RACC $RAID_ARGS
        echo "OK: $proj"
        set ok (math $ok + 1)
    else
        echo "FAIL: $proj" >&2
        set fail (math $fail + 1)
        set -a failed_list $proj
        if test "$CONTINUE_ON_ERROR" != "1"
            echo "Stopping on first error." >&2
            exit 1
        end
    end
    echo
end

echo "════════════════════════════════════════"
echo "Done. ok=$ok  fail=$fail  total="(count $PROJECTS)
if test $fail -gt 0
    echo "Failed projects:"
    for p in $failed_list
        echo "  - $p"
    end
    exit 1
end