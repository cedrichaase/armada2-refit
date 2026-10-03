#!/usr/bin/env bash
# Build Online.asi and put it, with Online.ini, into a test-bench clone's game.
#
#   online/bench-asi.sh <session state.json> [--loss N]
#
# Nothing else: the clone keeps Proton's own DirectPlay, which cannot host,
# so a game hosted through Internet - Online runs on our transport alone.  A
# scenario's `Setup:` line runs this on each player's clone
# (testbench/scenarios/multiplayer-online-match.md).  --loss N sets Loss=N in
# the clone's Online.ini: N% of the datagrams each game sends are dropped.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-asi.sh <session state.json> [--loss N]}"
loss=""
if [ "${2:-}" = "--loss" ]; then loss="${3:?--loss needs a percentage}"; fi
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }

bash "$here/build.sh" >/dev/null
cp "$here/build/Online.asi" "$here/Online.ini" "$work/game/"
if [ -n "$loss" ]; then
    # replaced, not appended: the profile reader takes a key's first value
    sed -i "s/^Loss=.*/Loss=$loss/" "$work/game/Online.ini"
    grep -q "^Loss=$loss" "$work/game/Online.ini" || { echo "Loss= not set" >&2; exit 1; }
fi
echo "Online.asi installed in $work/game (log: Online.log)${loss:+, Loss=$loss}"
