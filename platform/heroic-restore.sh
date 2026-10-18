#!/usr/bin/env bash
# Put back what heroic-clean.sh set aside.
#
#   platform/heroic-restore.sh            restore (Heroic must be closed)
#   platform/heroic-restore.sh --dry-run  say what would move, move nothing
#
# Reads <aside>/paths.sh (A2_ASIDE, else <game's parent>/.a2-clean-aside, as
# heroic-clean.sh chose). Deletes nothing: whatever the test left at each path (the
# fresh game, its prefix, Heroic's new settings, the installer's) is moved into
# <aside>/fresh-test-<time>/, and the originals go back. Heroic's install list gets the
# game's original entry back; any other game installed meanwhile stays listed. When
# everything is back, the aside folder holds only the test's leftovers, to delete.

set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
. "$here/a2env.sh"

APP=1174788223
dry=0
[ "${1:-}" = --dry-run ] && dry=1

die() { echo "heroic-restore: $*" >&2; exit 1; }

aside=${A2_ASIDE:-$(dirname "$A2_GAME")/.a2-clean-aside}
[ -f "$aside/paths.sh" ] || die "nothing set aside: no $aside/paths.sh (set A2_ASIDE?)"
. "$aside/paths.sh"
[ -d "$aside/game" ] || die "$aside/game is missing: restore by hand, nothing touched"

pgrep -x heroic >/dev/null && [ $dry = 0 ] && \
  die "Heroic is running; quit it first (it rewrites its files on exit)"

fresh=$aside/fresh-test-$(date +%Y%m%d-%H%M%S)
echo "Aside folder: $aside"
echo "Test leftovers go to: $fresh"
put_back() {  # NAME in the aside folder back to DEST, the test's DEST out of the way
  local name=$1 dest=$2
  if [ -e "$dest" ] || [ -L "$dest" ]; then
    echo "  park     $dest"
    if [ $dry = 0 ]; then mkdir -p "$(dirname "$fresh/$name")"; mv -- "$dest" "$fresh/$name"; fi
  fi
  if [ -e "$aside/$name" ]; then
    echo "  restore  $dest"
    if [ $dry = 0 ]; then mkdir -p "$(dirname "$dest")"; mv -- "$aside/$name" "$dest"; fi
  fi
}

put_back game            "$GAME"
put_back prefix          "$PREFIX"
# Heroic's settings: park every file the test made for the game, then restore ours.
for f in "$GAMESCFG" "$GAMESCFG".*; do
  [ -e "$f" ] || continue
  echo "  park     $f"
  if [ $dry = 0 ]; then mkdir -p "$fresh/heroic-config"; mv -- "$f" "$fresh/heroic-config/"; fi
done
for f in "$aside"/heroic-config/*; do
  [ -e "$f" ] || continue
  echo "  restore  $(dirname "$GAMESCFG")/$(basename "$f")"
  [ $dry = 1 ] || mv -- "$f" "$(dirname "$GAMESCFG")/"
done
put_back gogdl-manifest  "$MANIFEST"
put_back installer.json  "$INSTCFG"
put_back installer-cache "$INSTCACHE"
echo "  merge    $HEROIC/gog_store/installed.json  (app $APP's original entry)"

if [ $dry = 1 ]; then echo "Dry run: nothing moved."; exit 0; fi

mkdir -p "$fresh"
live=$HEROIC/gog_store/installed.json
[ -f "$live" ] && cp -p -- "$live" "$fresh/installed.json"
python3 - "$live" "$aside/installed.json" "$APP" <<'EOF'
import json, os, sys
live, saved, app = sys.argv[1:]
d = json.load(open(live)) if os.path.exists(live) else {'installed': []}
mine = [g for g in json.load(open(saved)).get('installed', []) if g.get('appName') == app]
d['installed'] = [g for g in d.get('installed', []) if g.get('appName') != app] + mine
json.dump(d, open(live, 'w'), indent=2)
EOF
mv -- "$aside/installed.json" "$aside/paths.sh" "$fresh/" 2>/dev/null || true
rmdir "$aside/heroic-config" 2>/dev/null || true

echo
echo "Restored. What is left in $aside is the test's:"
du -sh "$aside"/* 2>/dev/null | sed 's/^/  /'
echo "Delete it when you are done with it:  rm -rf '$aside'"
