#!/usr/bin/env bash
# flag.sh SESSION_ID install|spawn ARGS|stall [MS]|kill|hide|show|logs DEST
# Talks to M1Spike.asi in a bench clone through its flag files. Throwaway.
set -euo pipefail
sid=$1; what=$2; shift 2
game="$HOME/.cache/a2test/sessions/$sid/game"
[ -d "$game" ] || { echo "no clone at $game" >&2; exit 1; }
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
case $what in
    install) cp "$here/build/M1Spike.asi" "$here/build/M1Host.exe" "$game/" ; echo installed ;;
    spawn) printf '%s' "$*" > "$game/M1Spawn.flag" ;;
    stall) printf '%s' "${1:-3000}" > "$game/M1Stall.flag" ;;
    kill) : > "$game/M1Kill.flag" ;;
    hide) : > "$game/M1Hide.flag" ;;
    show) : > "$game/M1Show.flag" ;;
    logs) mkdir -p "$1"; cp "$game"/M1Spike.log "$game"/M1Host-*.log "$1/" 2>/dev/null || true; ls -la "$1" ;;
    *) echo "unknown $what" >&2; exit 2 ;;
esac
