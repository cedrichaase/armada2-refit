#!/usr/bin/env bash
# Install / remove Online.asi in the game directory.
#
#   install.sh            build, then install
#   install.sh --remove   take it out again
#
# Adds Online.asi, Online.ini and (at run time) Online.log, and nothing else;
# NetworkManager.dll is patched in memory only.  Removing the three files is a
# complete uninstall, so there is no backup to keep.
#
# Not run by ./install: this version logs the game's DirectPlay traffic and
# adds the Internet - Online entry, which connects as Manual IP does
# (README.md).  On a stock Proton prefix that traffic is Wine's builtin
# dpnet.dll, which cannot host; online/reference-dplay.sh sets up a bench clone
# with Microsoft's instead.
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
    rm -f "$GAME/Online.asi" "$GAME/Online.ini" "$GAME/Online.log"
    echo "removed Online from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Online.asi" "$GAME/Online.asi"
cp "$here/Online.ini"       "$GAME/Online.ini"
rm -f "$GAME/Online.log"
echo "installed into $GAME:"
ls -l "$GAME/Online.asi" "$GAME/Online.ini"
echo "after a launch:  cat \"$GAME/Online.log\""
