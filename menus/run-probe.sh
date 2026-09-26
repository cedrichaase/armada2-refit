#!/usr/bin/env bash
# Run probe.exe inside the running game's Wine session (same Wine, same
# prefix, so the same wineserver and the same window tree).
#   run-probe.sh list | click X Y | key VK
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
. "$here/../a2env.sh"
PREFIX="$A2_PREFIX" PROTON="$A2_PROTON"
[ -f "$here/build/probe.exe" ] || bash "$here/build.sh" >/dev/null
WINEPREFIX="$PREFIX/pfx" WINEDEBUG=-all "$PROTON/files/bin/wine" "$here/build/probe.exe" "$@"
