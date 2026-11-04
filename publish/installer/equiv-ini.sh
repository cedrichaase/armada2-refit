#!/usr/bin/env bash
# The three ini mergers against one set of expected outputs.   equiv-ini.sh [--show]
#
# The release scripts merge a shipped .ini into the player's without Python: install.sh in
# awk (ini_merge, INI_UPGRADES, ini_retired), install.ps1 in PowerShell (Merge-Ini,
# $IniUpgrades, Get-Retired). The repository's ./install uses tools/ini-merge.py. This runs
# all three on the same cases and wants the same bytes from each:
#
#   tools/ini-merge-fixtures/<case>/   ship.ini (the shipped file), game/<Name>.ini (the
#       player's, named as installed: Render3D.ini's retired defaults go by the name),
#       game/dxvk.conf when the case needs one, want.ini (what every merger must write).
#       A merger known to write something else has want.<impl>.ini (impl: python, awk,
#       pwsh) and a line in the case's KNOWN file saying why; the harness prints it.
#   the shipped inis themselves (display, ui, qol, rules, online, render3d, cutscenes):
#       unchanged, every third key missing, a section missing -- no want.ini, the three
#       must agree with each other (and the unchanged one with its input).
#
# The awk and PowerShell are cut out of the installers as they ship (the function bodies
# and the tables, by their first and last lines), so a change to either is what is tested.
# pwsh missing: its column is skipped and said so, unless A2_EQUIV_REQUIRE_PWSH=1 (CI).
# --show prints each merger's output of every case. Exit 1 on any difference that is not
# known. publish/installer/README.md, "Equivalence".
set -euo pipefail
src="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$src/../.." && pwd)"
fix="$root/tools/ini-merge-fixtures"
show=0; [ "${1:-}" = --show ] && show=1
T="$(mktemp -d "${TMPDIR:-/tmp}/a2equiv-ini.XXXXXX")"; trap 'rm -rf "$T"' EXIT
PWSH="${PWSH:-pwsh}"
if ! command -v "$PWSH" >/dev/null 2>&1; then
    if [ "${A2_EQUIV_REQUIRE_PWSH:-}" = 1 ]; then echo "no pwsh, and A2_EQUIV_REQUIRE_PWSH=1" >&2; exit 1; fi
    echo "SKIP  pwsh not found: install.ps1's Merge-Ini is not checked here (CI checks it)"
    PWSH=""
fi

# cut FILE START-REGEX END-REGEX -- the lines from the first START to the next END, both
# included; fails when either is not there, so a renamed function stops the harness.
cut() {
    awk -v s="$2" -v e="$3" '!p && $0 ~ s { p = 1 } p { print } p && $0 ~ e { done = 1; exit } END { exit !done }' "$1"
}
{
    echo 'set -euo pipefail'
    cut "$root/publish/installer/install.sh" '^INI_UPGRADES=' '^done$'
    cut "$root/publish/installer/install.sh" '^ini_retired\(\) ' '^}'
    cut "$root/publish/installer/install.sh" '^ini_merge\(\) ' '^}'
    echo 'ini_merge "$1" "$2" "$INI_UPGRADES" "$(ini_retired "$(basename "$2")")"'
} > "$T/awk-merge.sh" || { echo "could not cut ini_merge out of install.sh" >&2; exit 1; }
if [ -n "$PWSH" ]; then
    {
        echo 'param([string]$Shipped, [string]$Installed)'
        echo '$ErrorActionPreference = "Stop"'
        echo 'function Say([string]$s) { [Console]::Out.WriteLine($s) }'
        cut "$root/publish/installer/install.ps1" '^\$IniUpgrades = @\(' '^}'
        cut "$root/publish/installer/install.ps1" '^function Get-Retired\(' '^}'
        cut "$root/publish/installer/install.ps1" '^function Merge-Ini\(' '^}'
        echo 'Merge-Ini $Shipped $Installed $IniUpgrades (Get-Retired (Split-Path -Leaf $Installed))'
    } > "$T/ps-merge.ps1" || { echo "could not cut Merge-Ini out of install.ps1" >&2; exit 1; }
fi
impls="python awk"; [ -z "$PWSH" ] || impls="$impls pwsh"

# merge IMPL SHIP GAMEDIR NAME -- into $T/out/IMPL/NAME, from a copy of GAMEDIR.
merge() {
    local impl="$1" ship="$2" game="$3" name="$4" d="$T/out/$1"
    rm -rf "$d"; mkdir -p "$d"; cp -R "$game"/. "$d"/
    case "$impl" in
        python) python3 -I "$root/tools/ini-merge.py" "$ship" "$d/$name" ;;
        awk)    bash "$T/awk-merge.sh" "$ship" "$d/$name" ;;
        pwsh)   "$PWSH" -NoProfile -NonInteractive -File "$T/ps-merge.ps1" "$ship" "$d/$name" ;;
    esac > "$T/log.$impl" 2>&1 || { echo "FAIL  $impl exited non-zero:"; sed 's/^/      /' "$T/log.$impl"; return 1; }
}
show_out() { [ "$show" = 1 ] || return 0; echo "      --- $1"; sed 's/^/      | /' "$2" | cat -A | sed 's/\$$//'; }

n=0 bad=0 known=0
for c in "$fix"/*/; do
    c="${c%/}"; case="$(basename "$c")"
    name="$(cd "$c/game" && ls *.ini)"
    for impl in $impls; do
        n=$((n + 1))
        want="$c/want.ini"; note=""
        if [ -f "$c/want.$impl.ini" ]; then
            want="$c/want.$impl.ini"
            note="$(awk -v i="$impl" '$1 == i { $1 = ""; sub(/^ /, ""); print }' "$c/KNOWN" 2>/dev/null)"
            [ -n "$note" ] || { echo "FAIL  $case: want.$impl.ini without a reason in KNOWN"; bad=$((bad + 1)); continue; }
        fi
        [ -f "$want" ] || { echo "FAIL  $case: no want.ini"; bad=$((bad + 1)); continue; }
        if ! merge "$impl" "$c/ship.ini" "$c/game" "$name"; then bad=$((bad + 1)); continue; fi
        show_out "$case $impl" "$T/out/$impl/$name"
        if cmp -s "$want" "$T/out/$impl/$name"; then
            if [ -n "$note" ]; then echo "known $case ($impl): $note"; known=$((known + 1))
            else echo "ok    $case ($impl)"; fi
        else
            echo "FAIL  $case ($impl): differs from $(basename "$want")"
            diff "$want" "$T/out/$impl/$name" | sed 's/^/      /' | cat -A | sed 's/\$$//' || true
            bad=$((bad + 1))
        fi
    done
done

# The shipped inis: the three must agree.
agree() {   # label ship gamedir name
    local first="" impl
    for impl in $impls; do
        merge "$impl" "$2" "$3" "$4" || { bad=$((bad + 1)); return; }
        cp "$T/out/$impl/$4" "$T/res.$impl"
        show_out "$1 $impl" "$T/res.$impl"
    done
    n=$((n + 1))
    for impl in $impls; do
        if [ -z "$first" ]; then first="$impl"; continue; fi
        if ! cmp -s "$T/res.$first" "$T/res.$impl"; then
            echo "FAIL  $1: $first and $impl differ"
            diff "$T/res.$first" "$T/res.$impl" | sed 's/^/      /' || true
            bad=$((bad + 1)); return
        fi
    done
    echo "ok    $1 (${impls// /, } agree)"
}
for s in display/Display8.ini ui/UI.ini qol/QOL.ini rules/QOLRules.ini online/Online.ini \
         render3d/Render3D.ini cutscenes/binkproxy/BinkProxy.ini; do
    f="$root/$s"; name="$(basename "$s")"
    rm -rf "$T/g"; mkdir -p "$T/g"; cp "$f" "$T/g/$name"
    agree "$name unchanged" "$f" "$T/g" "$name"
    n=$((n + 1))
    if cmp -s "$f" "$T/res.python"; then echo "ok    $name unchanged is left as it was"
    else echo "FAIL  $name unchanged: the merge changed it"; bad=$((bad + 1)); fi
    awk '/^[A-Za-z0-9_]+=/ { c++; if (c % 3 == 0) next } { print }' "$f" > "$T/g/$name"
    agree "$name every third key missing" "$f" "$T/g" "$name"
    awk '/^\[/ { s++ } s == 2 { next } { print }' "$f" > "$T/g/$name"
    agree "$name its second section missing" "$f" "$T/g" "$name"
done

# Known: ./install merges Render3D.ini with --before-comments (render3d/install.sh), the
# release scripts without it. Same keys and values; an added key lands before the comment
# lines that open the next section there, after them here.
f="$root/render3d/Render3D.ini"
awk '/^[A-Za-z0-9_]+=/ { c++; if (c % 3 == 0) next } { print }' "$f" > "$T/g/Render3D.ini"
rm -rf "$T/g2"; mkdir -p "$T/g2"; cp "$T/g/Render3D.ini" "$T/g2/"
python3 -I "$root/tools/ini-merge.py" --before-comments "$f" "$T/g2/Render3D.ini" >/dev/null
bash "$T/awk-merge.sh" "$f" "$T/g/Render3D.ini" >/dev/null
n=$((n + 1))
if cmp -s "$T/g/Render3D.ini" "$T/g2/Render3D.ini"; then
    echo "ok    Render3D.ini: ./install's --before-comments and the release scripts agree"
elif [ "$(python3 -I "$src/equiv-diff.py" --ini-keys "$T/g/Render3D.ini" "$T/g2/Render3D.ini")" = same ]; then
    echo "known Render3D.ini (python --before-comments, ./install): an added key goes before the comments that open the next section; the release scripts put it after them. Same keys and values."
    known=$((known + 1))
else
    echo "FAIL  Render3D.ini: --before-comments changes keys or values, not only where they go"; bad=$((bad + 1))
fi

echo "ini merge: $n checks, $known known, $bad failed (mergers: ${impls// /, })"
[ "$bad" = 0 ]
