#!/usr/bin/env bash
# Take out of a game directory everything an earlier version of this project installed and
# nothing installs any more, by the table in known.txt.
#
#   retire.sh [--check] [--chain] [<game dir>]
#
#   --check   list what would be done and touch nothing
#   --chain   also take out the Direct3D chain's files (known.txt, chain-*): the vendored
#             d3d8to9 as d3d8_real.dll, DXVK's d3d9.dll and a marked dxvk.conf, which
#             nothing reads since display 1.0.0; the dxvk.conf's sampler values are carried
#             into Display8.ini [Samplers] first
#
# Called by ./install (before the layers' installers) and mirrored by the release
# installers; `a2mod refit` points at it when it restores a retired layer's files. Each
# entry is recognised by what it is -- a plugin by name as always, a binary by its hash,
# dxvk.conf by our marker -- so the player's own files, a dxvk.conf of their own, a
# d3d9.dll that is not ours, stay. The file-based fixes of hud 2.x come back to stock
# from their backups (hud-files.py). Without <game dir> it acts on $A2_GAME (a2env.sh).
set -euo pipefail
set -f   # known.txt holds patterns; word splitting must not expand them
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

check=0 chain=0 game=""
while [ $# -gt 0 ]; do
    case "$1" in
        --check) check=1 ;;
        --chain) chain=1 ;;
        -h|--help) sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        -*) echo "unknown argument: $1" >&2; exit 2 ;;
        *)  game="$1" ;;
    esac
    shift
done
if [ -z "$game" ]; then
    . "$here/../a2env.sh"
    game="$A2_GAME"
fi
[ -d "$game" ] || { echo "retire: no game directory at $game" >&2; exit 1; }

sha() { sha256sum "$1" | cut -d' ' -f1; }
out() {   # path -- remove a file, or say so under --check
    local p="$1" rel="${1#"$game"/}"
    [ -e "$p" ] || return 0
    if [ "$check" = 1 ]; then echo "retire: would remove $rel"; else rm -rf "$p"; echo "retire: removed $rel"; fi
}
# A marked dxvk.conf's sampler values into Display8.ini [Samplers], before it goes. Without a
# Display8.ini a stub at Defaults=1 is merged and taken away again when the merge changed
# nothing (display/install writes the file later, from the shipped one).
carry_samplers() {
    local ini="$game/Display8.ini" shipped="$here/../display/Display8.ini" made=0 rc=0
    [ -f "$shipped" ] || return 1
    if [ ! -f "$ini" ]; then printf "[Display8]\nDefaults=1\n" > "$ini"; made=1; fi
    # pipefail: the merge's status, not sed's. When it failed the values are not carried over.
    python3 "$here/../tools/ini-merge.py" "$shipped" "$ini" | sed 's/^/retire: /' || rc=1
    if [ "$made" = 1 ] && { [ "$rc" = 1 ] || cmp -s "$shipped" "$ini"; }; then rm -f "$ini"; fi
    return $rc
}
rmdir_if_empty() { if [ "$check" = 0 ]; then rmdir "$1" 2>/dev/null || true; fi; }

while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in ''|'#'*) continue ;; esac
    set -- $line
    kind="$1"; shift
    case "$kind" in
        plugin)
            for n in "$@"; do for e in asi ini log; do out "$game/$n.$e"; done; done ;;
        plugin-dir)
            for d in "$@"; do
                [ -d "$game/$d" ] || continue
                set +f
                for f in "$game/$d"/*.bmp; do [ -e "$f" ] && out "$f"; done
                set -f
                rmdir_if_empty "$game/$d"
            done ;;
        glob)
            [ -d "$game/$1" ] || continue
            set +f
            for f in "$game/$1"/$2; do [ -e "$f" ] && out "$f"; done
            set -f
            rmdir_if_empty "$game/$1" ;;
        glob-hash)
            [ -d "$game/$1" ] || continue
            gdir="$1"; gpat="$2"; shift 2
            set +f
            for f in "$game/$gdir"/$gpat; do
                [ -f "$f" ] || continue
                h="$(sha "$f")"
                for k in "$@"; do if [ "$h" = "$k" ]; then out "$f"; break; fi; done
            done
            set -f
            rmdir_if_empty "$game/$gdir" ;;
        file)
            out "$game/$1" ;;
        hud-files)
            if [ "$check" = 1 ]; then python3 "$here/hud-files.py" "$game" --check
            else python3 "$here/hud-files.py" "$game"; fi ;;
        bloom)
            # Outside the game directory: the per-user vkBasalt bloom before 13.0.0, under the
            # a2-vkbasalt name. The library and the build cache only with the layer's
            # manifest (a bench run has its own XDG_DATA_HOME and must not reach into the
            # real ~/.local/lib).
            share="${XDG_DATA_HOME:-$HOME/.local/share}"
            manifest="$share/vulkan/implicit_layer.d/vkBasalt.a2.x86.json"
            olds=("$share/a2-vkbasalt")
            if [ -e "$manifest" ]; then
                olds+=("$manifest" "$HOME/.local/lib/a2-vkbasalt" "${XDG_CACHE_HOME:-$HOME/.cache}/a2-vkbasalt")
            fi
            for old in "${olds[@]}"; do
                [ -e "$old" ] || continue
                if [ "$check" = 1 ]; then echo "retire: would remove the old vkBasalt bloom: $old"
                else rm -rf "$old"; echo "retire: removed the old vkBasalt bloom: $old"; fi
            done ;;
        chain-hash)
            if [ "$chain" = 1 ] && [ -f "$game/$1" ] && [ "$(sha "$game/$1")" = "$2" ]; then
                out "$game/$1"
            fi ;;
        chain-marker)
            f="$1"; shift
            if [ "$chain" = 1 ] && [ -f "$game/$f" ] && grep -qF "$*" "$game/$f"; then
                # dxvk.conf: its sampler values go into Display8.ini [Samplers] first
                # (tools/ini-merge.py, Defaults 3, which reads the dxvk.conf beside it).
                # A carry that failed keeps the dxvk.conf: its values are not lost with it.
                if [ "$f" = dxvk.conf ] && [ "$check" = 0 ] && ! carry_samplers; then
                    echo "retire: dxvk.conf stays: its sampler values could not be carried into Display8.ini" >&2
                    continue
                fi
                out "$game/$f"
            fi ;;
        *) echo "retire: known.txt: unknown entry kind $kind" >&2; exit 1 ;;
    esac
done < "$here/known.txt"
