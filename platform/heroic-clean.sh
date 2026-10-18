#!/usr/bin/env bash
# Set the Heroic install of Armada II aside, so the game can be installed from GOG and
# the mod from a release as a new player would, then put back with heroic-restore.sh.
#
#   platform/heroic-clean.sh            move everything aside (Heroic must be closed)
#   platform/heroic-clean.sh --dry-run  say what would move, move nothing
#
# Moves, never copies or deletes, into one folder beside the game (A2_ASIDE overrides;
# default <game's parent>/.a2-clean-aside, the same filesystem, so every move is a
# rename): the game directory (HD textures, saves and every plugin go with it), its
# Wine prefix, Heroic's settings for the game (GamesConfig/<app>.json and the backups
# beside it), gogdl's manifest for it, the refit installer's settings and download
# cache. Heroic's install list (gog_store/installed.json) is copied, and the game's
# entry is taken out of the live one, so Heroic shows it as not installed. The rest of
# Heroic (login, Proton builds, other games) is left alone, and so is A2_DATA.
#
# The paths it moved are written to <aside>/paths.sh; heroic-restore.sh reads them.

set -euo pipefail
here=$(cd "$(dirname "$0")/.." && pwd)
. "$here/a2env.sh"

APP=1174788223          # Armada II's GOG app id
dry=0
[ "${1:-}" = --dry-run ] && dry=1

die() { echo "heroic-clean: $*" >&2; exit 1; }

# Heroic's config: native first, then Flatpak; the one that lists the game wins.
heroic=${HEROIC_CONFIG:-}
if [ -z "$heroic" ]; then
  for c in "${XDG_CONFIG_HOME:-$HOME/.config}/heroic" \
           "$HOME/.var/app/com.heroicgameslauncher.hgl/config/heroic"; do
    if grep -qs "\"$APP\"" "$c/gog_store/installed.json"; then heroic=$c; break; fi
  done
fi
[ -n "$heroic" ] || die "no Heroic config lists app $APP as installed (set HEROIC_CONFIG)"
installed=$heroic/gog_store/installed.json
gamescfg=$heroic/GamesConfig/$APP.json

pgrep -x heroic >/dev/null && [ $dry = 0 ] && \
  die "Heroic is running; quit it first (it rewrites its files on exit)"

# The game and prefix as Heroic knows them, else as a2env does.
read_json() { python3 - "$@" <<'EOF'
import json, sys
path, what, app = sys.argv[1:4]
try: d = json.load(open(path))
except Exception: sys.exit(0)
if what == 'game':
    for g in d.get('installed', []):
        if g.get('appName') == app: print(g.get('install_path', '')); break
else:
    print(d.get(app, {}).get('winePrefix', ''))
EOF
}
game=$(read_json "$installed" game "$APP"); game=${game:-$A2_GAME}
prefix=$(read_json "$gamescfg" prefix "$APP"); prefix=${prefix:-$A2_PREFIX}
manifest=$heroic/gogdlConfig/heroic_gogdl/manifests/$APP
instcfg=${XDG_CONFIG_HOME:-$HOME/.config}/armada2-refit/installer.json
instcache=${XDG_CACHE_HOME:-$HOME/.cache}/armada2-refit/installer

aside=${A2_ASIDE:-$(dirname "$game")/.a2-clean-aside}
[ -e "$aside" ] && die "$aside already exists: run heroic-restore.sh first, or set A2_ASIDE"
[ -f "$game/Armada2.exe" ] || die "no Armada2.exe in $game"

echo "Heroic config: $heroic"
echo "Aside folder:  $aside"
move() {  # move SRC to <aside>/NAME, if SRC exists
  local src=$1 name=$2
  if [ ! -e "$src" ]; then echo "  (none)   $src"; return; fi
  echo "  move     $src"
  [ $dry = 1 ] || mv -- "$src" "$aside/$name"
}

[ $dry = 1 ] || mkdir -p "$aside/heroic-config"
move "$game"      game
move "$prefix"    prefix
for f in "$gamescfg" "$gamescfg".*; do
  [ -e "$f" ] && move "$f" "heroic-config/$(basename "$f")"
done
move "$manifest"  gogdl-manifest
move "$instcfg"   installer.json
move "$instcache" installer-cache
echo "  copy     $installed  (and take app $APP out of it)"

if [ $dry = 1 ]; then echo "Dry run: nothing moved."; exit 0; fi

cp -p -- "$installed" "$aside/installed.json"
python3 - "$installed" "$APP" <<'EOF'
import json, sys
path, app = sys.argv[1:]
d = json.load(open(path))
d['installed'] = [g for g in d.get('installed', []) if g.get('appName') != app]
json.dump(d, open(path, 'w'), indent=2)
EOF

{
  printf 'HEROIC=%q\nGAME=%q\nPREFIX=%q\nGAMESCFG=%q\n' "$heroic" "$game" "$prefix" "$gamescfg"
  printf 'MANIFEST=%q\nINSTCFG=%q\nINSTCACHE=%q\n' "$manifest" "$instcfg" "$instcache"
} > "$aside/paths.sh"

cat <<EOF

Set aside. Now, as a new player:
  1. Open Heroic, install Star Trek: Armada II (default folder and Proton).
  2. Launch it once, so Heroic writes its settings file for the game.
  3. Run the refit installer (the AppImage from the GitHub release) and install.
While this lasts, a2env's A2_GAME is the fresh install: don't run ./install, ./a2tex,
./a2mod or new a2test sessions. To come back: quit Heroic, then
  platform/heroic-restore.sh${A2_ASIDE:+   (with A2_ASIDE=$A2_ASIDE)}
EOF
