#!/usr/bin/env bash
# Offline check of tools/ini-merge.py: on Render3D.ini fixtures its output must be byte-identical
# to the merge render3d/install.sh used to carry inline (kept as selftest/render3d-inline-merge.py).
# Needs nothing from the game. Usage: tools/ini-merge-selftest.sh
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ship="$here/../render3d/Render3D.ini"
t=$(mktemp -d); trap 'rm -rf "$t"' EXIT
n=0; bad=0
check() {  # name, installed fixture file
    mkdir -p "$t/old" "$t/new"
    cp "$2" "$t/old/Render3D.ini"; cp "$2" "$t/new/Render3D.ini"
    python3 "$here/selftest/render3d-inline-merge.py" "$ship" "$t/old/Render3D.ini"
    python3 "$here/ini-merge.py" --before-comments "$ship" "$t/new/Render3D.ini" >/dev/null
    n=$((n+1))
    if cmp -s "$t/old/Render3D.ini" "$t/new/Render3D.ini"; then echo "ok   $1"
    else echo "FAIL $1"; diff "$t/old/Render3D.ini" "$t/new/Render3D.ini" | head || true; bad=$((bad+1)); fi
}
cp "$ship" "$t/f1"; check "identical to shipped" "$t/f1"
for k in 2 3 7; do
    awk -v k=$k '/^[A-Za-z0-9_]+=/ { c++; if (c % k == 0) next } { print }' "$ship" > "$t/f2$k"
    check "every ${k}th key missing" "$t/f2$k"
done
sed -e 's/^BeamLight=.*/BeamLight=1.0/' -e 's/^BeamReach=.*/BeamReach=40/' \
    -e 's/^LitExplosion=.*/LitExplosion=0.75/' -e 's/^Samples=.*/Samples=2/' "$ship" > "$t/f3"
check "retired defaults and a hand edit" "$t/f3"
awk '/^\[/ { sec=$0 } sec=="[Nebulae]" { next } { print }' "$ship" | sed 's/^BeamLight=.*/BeamLight=1.0/' > "$t/f4"
check "a section missing" "$t/f4"
printf '[Lights]\nBeamLight=1.0\n\n; a note the player wrote\n[Other]\nX=1\n' > "$t/f5"
check "short hand-written file" "$t/f5"
echo "$n cases, $bad failed"; [ "$bad" = 0 ]
