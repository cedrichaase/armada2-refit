#!/usr/bin/env bash
# The nebulae layer's install: Nebulae.asi, which draws each nebula class that has a
# recipe as a volume of gas in place of its billboards (nebulae/nebulae.c),
# Nebulae.ini, and the recipes, nebulae/recipes/<odf>.ini, into the game's Nebulae\.
#
#   install.sh             build, then install
#   install.sh --timing    ... and log the gas's GPU time (Timing=1)
#   install.sh --remove    take it out again
#
# Needs nothing from A2_DATA. A class without a recipe, or a chain with no Direct3D 9
# device behind d3d8, keeps the stock billboards.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

timing=0
remove=0
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        --timing) timing=1 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

remove_recipes() {
    local f
    [ -d "$GAME/Nebulae" ] || return 0
    for f in "$here"/recipes/*.ini; do rm -f "$GAME/Nebulae/$(basename "$f")"; done
    rmdir "$GAME/Nebulae" 2>/dev/null || true
}

if [ "$remove" = 1 ]; then
    rm -f "$GAME/Nebulae.asi" "$GAME/Nebulae.ini" "$GAME/Nebulae.log"
    remove_recipes
    echo "removed Nebulae from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Nebulae.asi" "$GAME/Nebulae.asi"
cp "$here/Nebulae.ini"       "$GAME/Nebulae.ini"
[ "$timing" = 1 ] && sed -i 's/^Timing=.*/Timing=1/' "$GAME/Nebulae.ini"
mkdir -p "$GAME/Nebulae"
cp "$here"/recipes/*.ini "$GAME/Nebulae/"
rm -f "$GAME/Nebulae.log"
echo "installed Nebulae.asi into $GAME with recipes: $(cd "$here/recipes" && ls *.ini | sed 's/\.ini$//' | tr '\n' ' ' | sed 's/ $//')"
