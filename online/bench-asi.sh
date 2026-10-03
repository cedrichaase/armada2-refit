#!/usr/bin/env bash
# Build Online.asi and put it, with Online.ini, into a test-bench clone's game.
#
#   online/bench-asi.sh <session state.json> [--loss N] [--server] [--relay]
#
# Nothing else: the clone keeps Proton's own DirectPlay, which cannot host,
# so a game hosted through Internet - Online runs on our transport alone.  A
# scenario's `Setup:` line runs this on each player's clone
# (testbench/scenarios/multiplayer-online-match.md).
#
#   --loss N   Loss=N: N% of the datagrams each game sends are dropped.
#   --server   Server= a local a2online-server on 127.0.0.1:23990, started here
#              if none is listening yet.  It exits by itself after 90 s without
#              a datagram, and logs to OnlineServer.log in the clone that
#              started it.  Without it Server= is empty: a bench game never
#              talks to the public server.
#   --relay    Direct=0: the games never try each other directly, everything
#              goes through the server's relay (implies --server).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
state="${1:?usage: bench-asi.sh <session state.json> [--loss N] [--server] [--relay]}"
shift
loss="" server="" relay=""
while [ $# -gt 0 ]; do
    case "$1" in
        --loss)   loss="${2:?--loss needs a percentage}"; shift 2 ;;
        --server) server=1; shift ;;
        --relay)  server=1; relay=1; shift ;;
        *) echo "unknown option $1" >&2; exit 2 ;;
    esac
done
work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
[ -d "$work/game" ] || { echo "no clone at $work/game" >&2; exit 1; }

bash "$here/build.sh" >/dev/null
cp "$here/build/Online.asi" "$here/Online.ini" "$work/game/"
ini="$work/game/Online.ini"

# replaced, not appended: the profile reader takes a key's first value
set_key() {
    sed -i "s/^$1=.*/$1=$2/" "$ini"
    grep -q "^$1=$2" "$ini" || { echo "$1= not set" >&2; exit 1; }
}

port=23990
if [ -n "$server" ]; then
    # one server for every game on this machine: the first set-up starts it
    exec 9>"${XDG_RUNTIME_DIR:-/tmp}/a2online-bench-server.lock"
    flock 9
    if ! python3 -c 'import socket,sys; s=socket.socket(socket.AF_INET,socket.SOCK_DGRAM); s.bind(("127.0.0.1",int(sys.argv[1])))' "$port" 2>/dev/null; then
        echo "a server is already listening on 127.0.0.1:$port"
    else
        setsid python3 "$here/server/a2online-server.py" --bind 127.0.0.1 --port "$port" --idle-exit 90 \
            >>"$work/game/OnlineServer.log" 2>&1 9>&- </dev/null &
        sleep 0.5
        echo "started a2online-server on 127.0.0.1:$port (log: OnlineServer.log)"
    fi
    flock -u 9
    set_key Server "127.0.0.1:$port"
else
    # never the public server from the bench
    set_key Server ""
fi
[ -n "$loss" ] && set_key Loss "$loss"
[ -n "$relay" ] && set_key Direct 0
echo "Online.asi installed in $work/game (log: Online.log)${loss:+, Loss=$loss}${server:+, Server=127.0.0.1:$port}${relay:+, Direct=0}"
