#!/usr/bin/env bash
# Write a scenario's variant with Display8 in another mode (stub by default) and print
# its path, for `./a2test run`:
#
#   ./a2test run "$(display/stub-scenario.sh ui-hud-map)" --install . -j 1
#   display/stub-scenario.sh ui-hud-bar stub Drop2D=1
#
# The variant's `Setup:` becomes `display/bench-mode.sh <mode> [keys] <the old setup>`;
# everything else is the scenario as it is. Written to a temporary file outside the
# checkout (a2test takes a scenario by path).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
name="${1:?usage: stub-scenario.sh <scenario> [mode] [Key=value ...]}"
mode="${2:-stub}"
shift $(( $# >= 2 ? 2 : 1 ))
src="$here/../testbench/scenarios/${name%.md}.md"
[ -f "$src" ] || { echo "no scenario $src" >&2; exit 1; }
out="$(mktemp -d "${TMPDIR:-/tmp}/display-scenario.XXXXXX")/${name%.md}-$mode.md"
python3 - "$src" "$out" "$mode" "$@" <<'EOF'
import re, sys
src, out, mode, keys = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
text = open(src).read()
pre = ' '.join(['display/bench-mode.sh', mode] + keys)
m = re.search(r'^Setup:\s*(.*)$', text, re.M)
if m:
    text = text[:m.start()] + 'Setup: ' + pre + ' ' + m.group(1).strip() + text[m.end():]
else:
    # After the title's header block: before the first blank line that follows a header.
    h = re.search(r'^(Resolutions|Mod|Launch|Timeout|Players|Reference):.*$', text, re.M)
    at = text.index('\n', h.end() if h else text.index('\n')) + 1
    text = text[:at] + 'Setup: ' + pre + '\n' + text[at:]
text = re.sub(r'^# (.*)$', lambda t: '# (Display8 ' + mode + ') ' + t.group(1), text, count=1, flags=re.M)
open(out, 'w').write(text)
EOF
# The fragments a scenario includes are looked up beside it.
cp "$here/../testbench/scenarios/"_*.md "$(dirname "$out")/"
echo "$out"
