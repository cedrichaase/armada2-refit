#!/usr/bin/env bash
# Install / remove Render3D.asi in the game directory.
#
#   install.sh            build, then install; takes out the plugins it supersedes
#   install.sh --remove   take it out again (the superseded plugins come back with
#                         their own installers, or ./install from a checkout without
#                         render3d)
#
# Adds Render3D.asi, Render3D.ini and (at run time) Render3D.log. The exe is patched in
# memory only, so removing the three files is a complete uninstall.
#
# Superseded now: MSAA.asi (our scene target has its own samples). The other plugins
# in lib.rs's SUPERSEDED keep drawing their parts inside the engine's steps until a
# module of ours takes each part over; this script removes each one as that happens.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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
    echo "removed Render3D from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

"$here/../msaa/install.sh" --remove >/dev/null
cp "$here/build/Render3D.asi" "$GAME/Render3D.asi"
# The player's settings stay: a key the shipped file adds is taken from it.
if [ -f "$GAME/Render3D.ini" ]; then
    python3 - "$here/Render3D.ini" "$GAME/Render3D.ini" <<'EOF'
import sys, re
new, cur = open(sys.argv[1]).read(), open(sys.argv[2]).read()
have = set(re.findall(r'^(\w+)=', cur, re.M))
add = [l for l in new.splitlines() if re.match(r'^(\w+)=', l) and l.split('=')[0] not in have]
if add:
    open(sys.argv[2], 'a').write(''.join(l + '\n' for l in add))
EOF
else
    cp "$here/Render3D.ini" "$GAME/Render3D.ini"
fi
rm -f "$GAME/Render3D.log"
echo "installed into $GAME: Render3D.asi, Render3D.ini (MSAA.asi removed: superseded)"
echo "after a launch:  cat \"$GAME/Render3D.log\""
