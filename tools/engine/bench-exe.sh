#!/usr/bin/env bash
# Change the executable of a test-bench clone, from a scenario's `Setup:` line, and then
# run the setup the scenario would have run, if any (the way display/bench-mode.sh does):
#
#   Setup: tools/engine/bench-exe.sh stamp display/bench-mode.sh stub
#
#   stamp    the PE header's TimeDateStamp of the clone's Armada2.exe changed by one bit:
#            an unknown build, to every profile (testbench/scenarios/engine-unknown-build.md)
#
# The bench passes the session's state file first. The clone's exe is a reflink or a copy
# of the install's, so it is never written in place: the changed copy is made beside it
# and moved over its name, and the user's install is untouched.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
usage="usage: bench-exe.sh <session state.json> stamp [script.sh ...]"
state="${1:?$usage}"
what="${2:?$usage}"
shift 2
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
exe="$work/game/Armada2.exe"
[ -f "$exe" ] || { echo "no Armada2.exe in $work/game" >&2; exit 1; }
case "$what" in
    stamp)
        python3 - "$exe" <<'PY'
import os, struct, sys
p = sys.argv[1]
d = bytearray(open(p, 'rb').read())
nt = struct.unpack_from('<I', d, 0x3c)[0]
assert d[nt:nt + 4] == b'PE\0\0', 'not a PE file'
old = struct.unpack_from('<I', d, nt + 8)[0]
struct.pack_into('<I', d, nt + 8, old ^ 1)
tmp = p + '.stamp'
with open(tmp, 'wb') as f:
    f.write(d)
os.replace(tmp, p)
print(f'bench-exe: TimeDateStamp 0x{old:08x} -> 0x{old ^ 1:08x} in {p}')
PY
        ;;
    *) echo "$usage (got '$what')" >&2; exit 2 ;;
esac
if [ $# -gt 0 ]; then
    script="$1"
    shift
    exec bash "$repo/$script" "$state" "$@"
fi
