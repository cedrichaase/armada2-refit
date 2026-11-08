#!/usr/bin/env bash
# Armada II Refit for Fleet Operations: Roots -- install into Roots under Wine or Proton.
#
#   ./install.sh [<Roots folder>]               install
#   ./install.sh --uninstall [<Roots folder>]   take it all out again
#
# The Roots folder is the one holding Data/ArmadaL.exe (its Data/ folder works too): the
# argument, else the folder this package was unzipped into (Roots or its Data/), else Heroic's
# default ~/Games/Heroic/Star Trek Armada II Fleet Operations Roots.
# Everything goes into Data/. A d3d8.dll (DxWrapper's) or winmm.dll there that is not ours
# is kept as d3d8.dll.a2display-backup / winmm.dll.a2asi-backup and put back by --uninstall.
# An .ini already there is kept (the player's settings); delete it to get the shipped one.
set -euo pipefail

pkg="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
uninstall=0
dir=""
for a in "$@"; do
    case "$a" in
        --uninstall|-u) uninstall=1 ;;
        -h|--help) sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) dir="$a" ;;
    esac
done

roots() { [ -f "$1/Data/ArmadaL.exe" ]; }
if [ -z "$dir" ]; then
    for d in "$pkg/.." "$pkg/../.." "$HOME/Games/Heroic/Star Trek Armada II Fleet Operations Roots"; do
        if roots "$d"; then dir="$d"; break; fi
    done
fi
[ -n "$dir" ] || { echo "Roots not found: pass its folder (the one with Data/ArmadaL.exe)" >&2; exit 1; }
[ -f "$dir/ArmadaL.exe" ] && dir="$dir/.."   # its Data/ folder was given
roots "$dir" || { echo "no Data/ArmadaL.exe in $dir" >&2; exit 1; }
data="$(cd "$dir/Data" && pwd)"
if pgrep -ix 'ArmadaL.exe' >/dev/null 2>&1; then
    echo "ArmadaL.exe is running -- quit the game first" >&2; exit 1
fi

LOADER=baba99929487b005bb9b168acfd852550055f22e5f1059c9032765209bb185e5
display8() { [ -f "$1" ] && grep -qa "Display8 stopped the game" "$1"; }
loader() { [ -f "$1" ] && [ "$(sha256sum "$1" | cut -d' ' -f1)" = "$LOADER" ]; }
# The package's files other than the two DLLs, relative to Data/.
files() { (cd "$pkg/Data" && find . -type f ! -name d3d8.dll ! -name winmm.dll -printf '%P\n'); }

if [ "$uninstall" = 1 ]; then
    if display8 "$data/d3d8.dll"; then
        rm -f "$data/d3d8.dll"
        [ ! -f "$data/d3d8.dll.a2display-backup" ] || mv "$data/d3d8.dll.a2display-backup" "$data/d3d8.dll"
    fi
    if loader "$data/winmm.dll"; then
        rm -f "$data/winmm.dll"
        [ ! -f "$data/winmm.dll.a2asi-backup" ] || mv "$data/winmm.dll.a2asi-backup" "$data/winmm.dll"
    fi
    while IFS= read -r f; do rm -f "$data/$f"; done < <(files)
    rm -f "$data"/{Display8,Render3D,UI,QOL}.log
    rmdir "$data/Surfaces" "$data/Nebulae" 2>/dev/null || true
    echo "Refit taken out of $data; DxWrapper's d3d8.dll is back."
    exit 0
fi

(cd "$pkg" && sha256sum --quiet -c SHA256SUMS) || { echo "the package is damaged (SHA256SUMS)" >&2; exit 1; }
exe_sha="$(sha256sum "$data/ArmadaL.exe" | cut -d' ' -f1)"
if [ "$exe_sha" != 4300ab71731e323f5767d8ee05457e6444401b823253aebee86c5ac23c2b00de ]; then
    echo "WARNING: this ArmadaL.exe is not Roots 5.0Z11's. The plugins check the build and"
    echo "  stand down where they cannot verify it, so parts (or all) of Refit may not run."
fi

# put SRC DEST BACKUP OURS -- a DLL into its slot, keeping a foreign one as BACKUP.
put() {
    if [ -f "$2" ] && ! "$4" "$2"; then
        if [ -f "$3" ]; then
            echo "$2 is not ours and $3 already holds another one; move one of them away first" >&2
            exit 1
        fi
        cp -p "$2" "$3"
    fi
    cp "$1" "$2.new" && mv -f "$2.new" "$2"
}
put "$pkg/Data/d3d8.dll" "$data/d3d8.dll" "$data/d3d8.dll.a2display-backup" display8
put "$pkg/Data/winmm.dll" "$data/winmm.dll" "$data/winmm.dll.a2asi-backup" loader

kept=()
while IFS= read -r f; do
    mkdir -p "$(dirname "$data/$f")"
    case "$f" in
        *.ini) if [ -f "$data/$f" ] && [ "${f%%/*}" = "$f" ]; then kept+=("$f"); continue; fi ;;
    esac
    cp "$pkg/Data/$f" "$data/$f"
done < <(files)
rm -f "$data"/{Display8,Render3D,UI,QOL}.log

echo "Refit installed into $data ($(tr -d '\r' < "$pkg/version.txt"))."
[ ${#kept[@]} = 0 ] || echo "Kept your ${kept[*]} (delete one and re-run to get the shipped file)."
echo "Wine needs the native d3d8 and winmm: WINEDLLOVERRIDES=\"winmm=n,b;d3d8=n,b\""
echo "  (Heroic: Settings > Advanced > Environment variables). Logs: drive_c/users/<user>/AppData/Local/armada2-refit/logs/ in the prefix."
