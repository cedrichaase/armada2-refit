#!/usr/bin/env bash
# Write render3d-build-subset's variant for another module left out and print its path,
# for `./a2test run`:
#
#   ./a2test run "$(render3d/subset-scenario.sh world)" --install . -j 1
#
# The variant builds without <module> (render3d/bench-asi.sh --without), on the showcase
# scene (on the firing scene for ordnance, which the showcase has none of), and compares
# the frame with that scene's `roundD-base-port-<scene>-<module>0` baseline, the full
# build with `[Modules] <module>=0` at load; the row counts it expects are worked out
# from the settings tables (tools/settings/gen.py --rows). Written to a temporary file
# outside the checkout (a2test takes a scenario by path).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
m="${1:?usage: subset-scenario.sh <module>}"
all="world particles hulls effects ordnance specials sky overlays"
[[ " $all " == *" $m "* ]] || { echo "subset-scenario.sh: no module $m ($all)" >&2; exit 2; }
src="$repo/testbench/scenarios/render3d-build-subset.md"
out="$(mktemp -d "${TMPDIR:-/tmp}/render3d-subset.XXXXXX")/render3d-build-subset-$m.md"
python3 - "$src" "$out" "$m" "$repo" $all <<'EOF'
import re, subprocess, sys
src, out, m, repo, mods = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:]
text = open(src).read()
rest = [x for x in mods if x != m]
rows = subprocess.run(['python3', repo + '/tools/settings/gen.py', '--rows', repo + '/render3d/settings.toml'] + rest,
                      check=True, capture_output=True, text=True).stdout.split('\n')
rows = [r for r in rows if r.strip()]
total = len(rows) - 1   # the module's own [Modules] row is left out too
import tomllib
core_n = len(tomllib.load(open(repo + '/render3d/settings.toml', 'rb'))['row']) - 1
text = text.replace('--without hulls', '--without ' + m)
if m == 'ordnance':
    text = text.replace('showcase Seed=7 Step=16 Pause=200 Race=federation Hud=1', 'firing Seed=7 Step=16 Pause=200')
    text = text.replace('"showcase"', '"firing"').replace('port-showcase-', 'port-firing-')
text = text.replace('hulls0', m + '0')
text = text.replace('"modules not built in: hulls"', f'"modules not built in: {m}"')
text = text.replace('"module hulls:"', f'"module {m}:"')
text = re.sub(r'"settings: \d+ rows exported \(\d+ the core\'s\)"', f'"settings: {total} rows exported ({core_n} the core\'s)"', text)
text = re.sub(r'^# (.*)$', lambda t: f'# (without {m}) ' + t.group(1), text, count=1, flags=re.M)
open(out, 'w').write(text)
EOF
cp "$repo/testbench/scenarios/"_*.md "$(dirname "$out")/" 2>/dev/null || true
echo "$out"
