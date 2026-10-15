#!/usr/bin/env bash
# Install / remove Render3D.asi in the game directory.
#
#   install.sh              build, then install
#   install.sh --remove     take it out again
#
# Adds Render3D.asi, Render3D.ini, the world module's recipes in Surfaces\ and
# Nebulae\, and (at run time) Render3D.log. The exe is patched in memory only, so
# removing them is a complete uninstall.
#
# Render3D replaced the plugins that drew parts of the 3D view one at a time (MSAA,
# Lighting, Sky, Nebulae, Surfaces, Explosions, Shields, Repair, Venting, Specials, Fog,
# and the models layer's Planets.asi); they are no longer in this repository. A module
# stands down while its old plugin is loaded, so this takes out any an earlier install
# left.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# The plugins render3d replaced, as an earlier install left them.
OLD="MSAA Lighting Sky Nebulae Surfaces Explosions Shields Repair Venting Specials Fog Planets"

remove_recipes() {
    local f
    for f in "$here"/surfaces/*.ini; do rm -f "$GAME/Surfaces/$(basename "$f")"; done
    for f in "$here"/nebulae/*.ini; do rm -f "$GAME/Nebulae/$(basename "$f")"; done
    # Sky.asi's copies of the sky recipes, which would shadow the built-in ones.
    for f in "$here"/skies/*.ini; do rm -f "$GAME/Sky/$(basename "$f")"; done
    rmdir "$GAME/Surfaces" "$GAME/Nebulae" "$GAME/Sky" 2>/dev/null || true
}

remove=0
while [ $# -gt 0 ]; do
    case "$1" in
        --remove) remove=1 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

[ -d "$GAME" ] || { echo "game directory not found: $GAME" >&2; exit 1; }

if [ "$remove" = 1 ]; then
    rm -f "$GAME/Render3D.asi" "$GAME/Render3D.ini" "$GAME/Render3D.log"
    remove_recipes
    echo "removed Render3D from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Render3D.asi" "$GAME/Render3D.asi"
# The player's settings stay: a key the shipped file has and theirs lacks is added in
# its own section.
if [ -f "$GAME/Render3D.ini" ]; then
    python3 - "$here/Render3D.ini" "$GAME/Render3D.ini" <<'EOF'
import re, sys

def parse(text):
    secs, cur = {}, None
    for line in text.splitlines():
        m = re.match(r'^\s*\[([^\]]+)\]', line)
        if m:
            cur = m.group(1).lower()
            secs.setdefault(cur, [])
        elif cur is not None:
            secs[cur].append(line)
    return secs

new = parse(open(sys.argv[1]).read())
lines = open(sys.argv[2]).read().splitlines()
have = parse('\n'.join(lines))
for sec, body in new.items():
    keys = {k.lower() for k in (re.match(r'^(\w+)=', l).group(1) for l in have.get(sec, []) if re.match(r'^(\w+)=', l))}
    missing = [l for l in body if re.match(r'^(\w+)=', l) and l.split('=')[0].lower() not in keys]
    if not missing:
        continue
    if sec in have:
        # after the last line of that section
        at, cur = len(lines), None
        for i, l in enumerate(lines):
            m = re.match(r'^\s*\[([^\]]+)\]', l)
            if m:
                if cur == sec:
                    at = i
                    break
                cur = m.group(1).lower()
        while at > 0 and not lines[at - 1].strip():
            at -= 1
        lines[at:at] = missing
    else:
        name = next(l for l in open(sys.argv[1]).read().splitlines() if l.strip().lower() == '[' + sec + ']')
        lines += ['', name] + missing
open(sys.argv[2], 'w').write('\n'.join(lines) + '\n')
EOF
else
    cp "$here/Render3D.ini" "$GAME/Render3D.ini"
fi

removed=""
for n in $OLD; do
    if [ -e "$GAME/$n.asi" ] || [ -e "$GAME/$n.ini" ]; then
        rm -f "$GAME/$n.asi" "$GAME/$n.ini" "$GAME/$n.log"
        removed="$removed $n.asi"
    fi
done

remove_recipes
mkdir -p "$GAME/Surfaces" "$GAME/Nebulae"
cp "$here"/surfaces/*.ini "$GAME/Surfaces/"
cp "$here"/nebulae/*.ini "$GAME/Nebulae/"

rm -f "$GAME/Render3D.log"
echo "installed into $GAME: Render3D.asi, Render3D.ini and the recipes${removed:+; took out the old$removed}"
echo "after a launch:  cat \"$GAME/Render3D.log\""
