#!/usr/bin/env bash
# Install / remove Render3D.asi in the game directory.
#
#   install.sh              build, then install
#   install.sh --remove     take it out again
#
# Adds Render3D.asi, Render3D.ini, the world module's recipes in Surfaces\ and
# Nebulae\, and (at run time) Render3D.log. The exe is patched in memory only, so
# removing them is a complete uninstall.
#
# Render3D replaced the plugins that drew parts of the 3D view one at a time (MSAA,
# Lighting, Sky, Nebulae, Surfaces, Explosions, Shields, Repair, Venting, Specials, Fog,
# and the models layer's Planets.asi); they are no longer in this repository. A module
# stands down while its old plugin is loaded, so on Armada II this runs retire/retire.sh,
# whose table (retire/known.txt) lists them, to take out any an earlier install left.
#
# It installs into the executable's folder (dirname $A2_EXE): the game directory on Armada
# II, Data/ on Fleet Operations: Roots, where the asiloader layer puts the ASI loader.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh"
# The executable's folder: the install folder itself, or its Data/ on Fleet Operations:
# Roots (games/<id>.toml, exe.data_dir), where the .asi loader looks.
GAME="$(dirname "$A2_EXE")"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

remove_recipes() {
    local f
    for f in "$here"/content/surfaces/*.ini; do rm -f "$GAME/Surfaces/$(basename "$f")"; done
    for f in "$here"/content/nebulae/*.ini; do rm -f "$GAME/Nebulae/$(basename "$f")"; done
    # Sky.asi's copies of the sky recipes are retire/retire.sh's, by hash: a Sky\<name>.ini
    # of the player's own, even one named like a shipped recipe, is theirs and stays.
    rmdir "$GAME/Surfaces" "$GAME/Nebulae" 2>/dev/null || true
}

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
    rm -f "$GAME/Render3D.asi" "$GAME/Render3D.ini" "$GAME/Render3D.log"
    remove_recipes
    echo "removed Render3D from $GAME"
    exit 0
fi

bash "$here/build.sh" >/dev/null

cp "$here/build/Render3D.asi" "$GAME/Render3D.asi"
# The player's settings stay: a key the shipped file has and theirs lacks is added in
# its own section.
if [ -f "$GAME/Render3D.ini" ]; then
    python3 "$here/../tools/ini-merge.py" --before-comments "$here/Render3D.ini" "$GAME/Render3D.ini"
else
    cp "$here/Render3D.ini" "$GAME/Render3D.ini"
fi

# retire/ is Armada II's: the plugins and files it takes out never existed in another game.
if [ "$A2_PROFILE" = armada2 ]; then
    bash "$here/../retire/retire.sh" "$GAME" | sed "s/^/  /"
fi

remove_recipes
mkdir -p "$GAME/Surfaces" "$GAME/Nebulae"
cp "$here"/content/surfaces/*.ini "$GAME/Surfaces/"
cp "$here"/content/nebulae/*.ini "$GAME/Nebulae/"

rm -f "$GAME/Render3D.log"
echo "installed into $GAME: Render3D.asi, Render3D.ini and the recipes"
echo "after a launch:  %LOCALAPPDATA%\\armada2-refit\\logs\\Render3D.log  (in the Wine prefix: drive_c/users/<user>/AppData/Local/armada2-refit/logs/)"
