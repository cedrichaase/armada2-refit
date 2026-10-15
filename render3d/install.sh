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
# Superseded: MSAA.asi always (our scene target has its own samples), and each plugin
# whose part a module of ours draws, while that module is on (its default below, or
# `[Modules] <name>=` in the game's Render3D.ini). A module stands down while its
# plugin is loaded, so ./install runs this last, after the plugins' own installers.
# To go back to a plugin: set its module to 0 in Render3D.ini and run the plugin's
# install.sh.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
GAME="$A2_GAME"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# module  default  layers whose plugin it replaces
MODULES="
effects   1  explosions shields repair
ordnance  1  specials
sky       1  sky
overlays  1  fog
"

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

# The plugins the modules that are on replace.
removed=""
while read -r module default layers; do
    [ -n "$module" ] || continue
    on=$(python3 - "$GAME/Render3D.ini" "$module" "$default" <<'EOF'
import re, sys
on, sec = sys.argv[3], None
for line in open(sys.argv[1]).read().splitlines():
    m = re.match(r'^\s*\[([^\]]+)\]', line)
    if m:
        sec = m.group(1).lower()
    elif sec == 'modules':
        m = re.match(r'^\s*(\w+)\s*=\s*(\d+)', line)
        if m and m.group(1).lower() == sys.argv[2]:
            on = m.group(2)
print(on)
EOF
)
    [ "$on" != 0 ] || continue
    for l in $layers; do
        "$here/../$l/install.sh" --remove >/dev/null
        removed="$removed $l"
    done
done <<< "$MODULES"

rm -f "$GAME/Render3D.log"
echo "installed into $GAME: Render3D.asi, Render3D.ini; superseded and removed: msaa$removed"
echo "after a launch:  cat \"$GAME/Render3D.log\""
