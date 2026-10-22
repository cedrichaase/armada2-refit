#!/usr/bin/env bash
# Armada II Refit -- install the release package into the game, on Linux (Wine/Proton)
# and on macOS (a Wine prefix: CrossOver, Whisky, a wrapper, plain Wine).
#
#   ./install.sh [<game dir>]               install
#   ./install.sh --uninstall [<game dir>]   take all of it out again
#
# No choices to make: what can work here is installed, the rest is skipped and says why.
# First the prerequisites (prereqs.txt: the ASI loader with STA2WidescreenPatch), bundled
# in vendor/, each file only if missing. Then the Direct3D chain the repository's ./install
# sets up (chain/): crosire's d3d8to9 as d3d8.dll on DXVK's d3d9.dll, so dxvk.conf has
# DXVK under it. Render3D.asi (the 3D view) and UI.asi (the HUD, the menus and the
# loading screen) draw on the Direct3D 9 device d3d8to9 gives them; where the chain is
# left as it is, the UI's place is taken by the plugins it supersedes (HUD.asi, Menus.asi,
# GridLayout.asi) and the 3D view stays the engine's. The models are rewritten from the
# player's own SOD files with python3; bloom is set up when a vkBasalt layer is installed. Textures,
# the menu backdrops and the replacement movies are built from the game's own art, so a
# release cannot carry them.
#
# On macOS Direct3D 9 can only be Wine's own (wined3d, on OpenGL): DXVK's d3d9.dll cannot
# run there, so d3d8to9 goes in on Wine's d3d9, and dxvk.conf and bloom (no
# vkBasalt) are skipped. The script keeps to the /bin/bash 3.2 and BSD tools a Mac ships.
#
# The game directory is the one holding Armada2.exe: the argument, else this folder's
# parent if the package was unzipped into the game, else $A2_GAME, else (macOS) the GOG
# game in a CrossOver bottle or in ~/.wine, else Heroic's default for the GOG release.
# The model rewrites run with $A2_PYTHON if set (the GUI installer's own program), else
# python3. This is the release zip's installer, and it
# stands alone: it needs nothing from the repository. (The repository's own ./install
# builds from source and installs the asset layers too.)
#
# With A2_PROGRESS=1 it also prints "::step <id> <what>" as each stage starts, for the
# GUI installer (gui-installer/README.md, "Progress"). The package's manifest.json lists
# the ids in order; keep the two in step (publish/package.sh writes it). An error the GUI
# can explain is first named as "::error <code> <path>", one line per file, before the
# sentence for a person on stderr (codes: av-locked, av-missing, damaged, game-running,
# no-game, write-denied; gui-installer/README.md, "Errors").
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
action=install game=""
mac=0; [ "$(uname -s)" = Darwin ] && mac=1

# Portable to a Mac: no sha256sum there (shasum -a 256), no tac, a BSD sed.
if command -v sha256sum >/dev/null 2>&1; then SHA256="sha256sum"
elif command -v shasum >/dev/null 2>&1; then SHA256="shasum -a 256"
else echo "neither sha256sum nor shasum found" >&2; exit 1; fi
sha256() { $SHA256 "$1" | cut -d' ' -f1; }
# An error the GUI installer can explain (code, the files it is about, the sentence).
stop_with() {
    local code="$1" msg="$2" p; shift 2
    if [ "${A2_PROGRESS:-}" = 1 ]; then for p in "$@"; do echo "::error $code $p"; done; fi
    echo "$msg" >&2; exit 1
}
AV_HELP="https://github.com/cedrichaase/armada2-refit#antivirus-false-positives"
av_advice() {
    echo "Some antivirus programs flag Armada II Refit's plugins as a false positive." \
         "Restore the files from the antivirus's quarantine, or add exclusions for $here" \
         "and $game, then try again ($AV_HELP)."
}
reverse_lines() { awk '{ l[NR] = $0 } END { for (i = NR; i > 0; i--) print l[i] }' "$1"; }
while [ $# -gt 0 ]; do
    case "$1" in
        --uninstall) action=uninstall ;;
        -h|--help)   sed -n '2,/^set -e/p' "$0" | grep '^#' | sed 's/^# \{0,1\}//'; exit 0 ;;
        -*)          echo "unknown option: $1" >&2; exit 2 ;;
        *)           game="$1" ;;
    esac
    shift
done
# Unzipped into the game directory itself: that is the one.
if [ -z "$game" ] && [ -f "$here/../Armada2.exe" ]; then game="$here/.."; fi
game="${game:-${A2_GAME:-}}"
# On a Mac the game sits in a Wine prefix: a CrossOver bottle first, then ~/.wine.
if [ -z "$game" ] && [ "$mac" = 1 ]; then
    for g in "$HOME/Library/Application Support/CrossOver/Bottles/"*"/drive_c/GOG Games/"*Armada* \
             "$HOME/.wine/drive_c/GOG Games/"*Armada*; do
        if [ -d "$g" ]; then game="$g"; break; fi
    done
fi
game="${game:-$HOME/Games/Heroic/Star Trek Armada II}"
exe=$(ls "$game/Armada2.exe" "$game/armada2.exe" "$game/ARMADA2.EXE" 2>/dev/null | head -1 || true)
[ -n "$exe" ] || stop_with no-game "no Armada2.exe in $game -- pass the game directory as an argument" "$game"
game="$(cd "$game" && pwd)"

if pgrep -if 'armada2\.exe' >/dev/null 2>&1; then
    stop_with game-running "Armada2.exe is running -- quit the game first" Armada2.exe
fi

# Where bloom's files go. vkBasalt's config parser cannot take a space or a '#' in a
# path, which rules out the game directory ("Star Trek Armada II").
BLOOM="${XDG_DATA_HOME:-$HOME/.local/share}/armada2-refit-bloom"
DXVK_MARKER="# generated by tools/renderer-config.sh"   # dxvk.conf files we wrote
BLOOM_MARKER="# written by armada2-refit install.sh"

MANIFEST="$game/armada2-refit-prereqs.txt"   # what the prerequisites step added

step() { [ "${A2_PROGRESS:-}" != 1 ] || echo "::step $*"; }
# The per-part plugins earlier packages and installs put in, replaced by Render3D.asi:
# taken out, in every case (a module of Render3D.asi stands down beside its old plugin).
OLD_PLUGINS="MSAA Planets Lighting Sky Nebulae Surfaces Explosions Shields Repair Venting Specials Fog"
old_plugins_out() {
    local n
    for n in $OLD_PLUGINS; do rm -f "$game/$n.asi" "$game/$n.ini" "$game/$n.log"; done
    # Sky.asi's recipes (one per sky name, mbg*.ini); anything else there is the player's.
    if [ -d "$game/Sky" ]; then
        rm -f "$game/Sky"/mbg*.ini
        rmdir "$game/Sky" 2>/dev/null || true
    fi
}

# Render3D.asi's recipes (render3d/surfaces, render3d/nebulae): Surfaces/ and Nebulae/
# in the game directory, only the files of these names; anything else there is the
# player's.
RECIPES="Surfaces Nebulae"
recipes_out() {
    local d f
    for d in $RECIPES; do
        [ -d "$game/$d" ] || continue
        for f in "$here/game/$d"/*.ini; do
            [ -e "$f" ] && rm -f "$game/$d/$(basename "$f")"
        done
        rmdir "$game/$d" 2>/dev/null || true
    done
}
# The plugins UI.asi takes the place of (ui/supersede.sh), and Menus.asi's old name.
LEGACY_UI="HUD Menus MenuScale GridLayout"
legacy_ui_out() {
    local n
    for n in $LEGACY_UI; do rm -f "$game/$n.asi" "$game/$n.ini" "$game/$n.log"; done
}

is_proxy() { grep -q 'BinkProxy' "$1" 2>/dev/null; }
# A2_SKIP="cutscenes ..." leaves those parts out -- the GUI installer does not install the
# cutscene player yet. Only cutscenes can be skipped so far.
skipped() { case " ${A2_SKIP:-} " in *" $1 "*) return 0 ;; esac; return 1; }
# The proxy out again, the stock binkw32.dll back.
unproxy() {
    if is_proxy "$game/binkw32.dll"; then
        if [ -f "$game/binkw32.dll.a2neb-backup" ]; then
            mv -f "$game/binkw32.dll.a2neb-backup" "$game/binkw32.dll"
            rm -f "$game/binkw32_orig.dll"
        elif [ -f "$game/binkw32_orig.dll" ]; then
            mv -f "$game/binkw32_orig.dll" "$game/binkw32.dll"
        else
            echo "binkw32.dll is the proxy and there is no stock copy -- restore it by hand" >&2
        fi
    fi
    rm -f "$game/BinkProxy.ini" "$game/BinkProxy.log"
}
# DXVK by content, never by name: GOG puts a d3d8.dll there too.
is_dxvk() { grep -a -q -i 'dxvk' "$1" 2>/dev/null; }
# crosire's d3d8to9 1.16.0, as the repository's ./install places it (platform/vendor/), by hash.
D3D8TO9_SHA=122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8
is_d3d8to9() { [ -f "$1" ] && [ "$(sha256 "$1")" = "$D3D8TO9_SHA" ]; }
# DXVK in the chain: its own d3d8, or d3d8to9 in front of its d3d9.
# GOG's own d3d8.dll (its d3d8to9 build), as the release ships it. Kept as
# d3d8.dll.gog-backup, the name platform/d3d8-chain.py gives it, when replaced.
GOG_D3D8_SHA=735dbb81a5fa0368c6436349fc5bf97a9ae15daeb6829dfe95a17704294e6719
is_gog_d3d8() { [ -f "$1" ] && [ "$(sha256 "$1")" = "$GOG_D3D8_SHA" ]; }
CHAIN_BACKUP=.a2chain-backup   # any other d3d8.dll / d3d9.dll the chain step replaced
# The SOD rewrites (models/): Python, with the game they work on. $A2_PYTHON (a path,
# perhaps with spaces: the GUI installer's frozen program runs "<it> script.py args" as
# python would), else python3. On a Mac without the Command Line Tools, /usr/bin/python3
# is only a stub that asks to install them, so it does not count.
PY="${A2_PYTHON:-}"
if [ -z "$PY" ] && command -v python3 >/dev/null 2>&1; then
    PY="$(command -v python3)"
    if [ "$mac" = 1 ] && [ "$PY" = /usr/bin/python3 ] && ! xcode-select -p >/dev/null 2>&1; then
        PY=""
    fi
fi
sods() {   # --install | --revert
    local s
    for s in moon-sod select-sod hull-sod asteroid-sod; do
        A2_GAME="$game" "$PY" "$here/models/$s.py" "$1" 2>&1 | sed 's/^/    /' \
            || echo "  $s.py $1 failed -- the rest goes on"
    done
}

# ------------------------------------------------------------------ uninstall

if [ "$action" = uninstall ]; then
    step uninstall "taking the mod out"
    for n in Render3D UI QOL QOLRules Online; do
        rm -f "$game/$n.asi" "$game/$n.ini" "$game/$n.log"
    done
    legacy_ui_out
    recipes_out
    old_plugins_out
    if [ -n "$PY" ] && [ -d "$here/models" ]; then
        echo "  the models back to stock:"; sods --revert
    fi
    unproxy
    if [ -f "$game/dxvk.conf" ] && grep -qF "$DXVK_MARKER" "$game/dxvk.conf"; then
        rm -f "$game/dxvk.conf"
    fi
    # The prerequisites, newest record first; a file changed since is left, and said so.
    if [ -f "$MANIFEST" ]; then
        reverse_lines "$MANIFEST" | while read -r f sum; do
            [ -n "$f" ] && [ -e "$game/$f" ] || continue
            if [ "$(sha256 "$game/$f")" != "$sum" ]; then
                echo "  left $f: it changed since it was installed"; continue
            fi
            rm -f "$game/$f"
        done
        rm -f "$MANIFEST"
    fi
    # What the chain step replaced, back where it took it out.
    for f in d3d8.dll d3d9.dll; do
        [ -e "$game/$f" ] && continue
        if [ "$f" = d3d8.dll ] && [ -f "$game/d3d8.dll.gog-backup" ]; then
            mv -f "$game/d3d8.dll.gog-backup" "$game/d3d8.dll"
        elif [ -f "$game/$f$CHAIN_BACKUP" ]; then
            mv -f "$game/$f$CHAIN_BACKUP" "$game/$f"
        fi
    done
    if [ -f "$BLOOM/vkBasalt.conf" ] && grep -qF "$BLOOM_MARKER" "$BLOOM/vkBasalt.conf"; then
        rm -rf "$BLOOM"
        echo "removed $BLOOM -- also take ENABLE_VKBASALT and VKBASALT_CONFIG_FILE out of the launcher"
    fi
    echo "uninstalled from $game"
    exit 0
fi

# Every file checked before anything is said: an antivirus seldom takes only one.
step verify "checking the package"
missing=() unreadable=() damaged=()
while read -r sum file || [ -n "$sum" ]; do
    [ -n "$sum" ] || continue
    file="${file#\*}"
    if [ ! -f "$here/$file" ]; then missing+=("$file")
    elif [ ! -r "$here/$file" ] || ! got="$(sha256 "$here/$file" 2>/dev/null)"; then unreadable+=("$file")
    elif [ "$got" != "$sum" ]; then damaged+=("$file")
    fi
done < "$here/SHA256SUMS"
list() { local IFS=,; echo "$*" | sed 's/,/, /g'; }
if [ ${#missing[@]} -gt 0 ] || [ ${#unreadable[@]} -gt 0 ]; then
    if [ "${A2_PROGRESS:-}" = 1 ]; then
        for p in ${damaged[@]+"${damaged[@]}"}; do echo "::error damaged $p"; done
        for p in ${missing[@]+"${missing[@]}"}; do echo "::error av-missing $p"; done
    fi
    what="could not be read" them=them
    if [ ${#missing[@]} -gt 0 ]; then
        what="are missing"; [ ${#unreadable[@]} -eq 0 ] || what="are missing or could not be read"
    fi
    if [ $(( ${#missing[@]} + ${#unreadable[@]} )) -eq 1 ]; then what="${what/#are/is}"; them=it; fi
    stop_with av-locked "$(list ${missing[@]+"${missing[@]}"} ${unreadable[@]+"${unreadable[@]}"}) from the package \
$what. An antivirus has most likely quarantined or locked $them. $(av_advice)" \
        ${unreadable[@]+"${unreadable[@]}"}
fi
if [ ${#damaged[@]} -gt 0 ]; then
    stop_with damaged "the package is damaged: $(list "${damaged[@]}") does not match SHA256SUMS. \
Download it again. An antivirus that cleans or cuts files in place can do this too: look in its \
quarantine and history." "${damaged[@]}"
fi
echo "installing into $game"

# ------------------------------------------------------------------ prerequisites

# The ASI loader and STA2WidescreenPatch (prereqs.txt), in vendor/<name>/ because their
# licence allows it. A file already in the game directory is never overwritten. What this
# adds is recorded in $MANIFEST with its hash, so --uninstall takes out only that, and
# only unchanged.
record() { echo "$1 $(sha256 "$game/$1")" >> "$MANIFEST"; }
# A file of the package into the game. What stops it is named: the package's copy gone or
# unreadable (an antivirus, after the check above), else the game folder refusing it.
copy_into() {   # src dst
    local err rel="${1#"$here"/}"
    err="$(cp "$1" "$2" 2>&1)" && return 0
    [ -e "$1" ] || stop_with av-missing "$rel has gone from the package. An antivirus has most likely quarantined it. $(av_advice)" "$rel"
    [ -r "$1" ] || stop_with av-locked "$rel from the package could not be read. An antivirus has most likely locked it. $(av_advice)" "$rel"
    stop_with write-denied "$(basename "$2") could not be written into $game ($err). An antivirus \
may be blocking it, or the folder is not yours to write: add an exclusion for $game, or fix \
its permissions, then try again ($AV_HELP)." "$2"
}

prereq() {   # name|version|author|licence|page|files
    local name ver author lic page files f
    IFS='|' read -r name ver author lic page files <<< "$1"
    f="${files%% *}"
    if [ -e "$game/$f" ]; then echo "  $name: already installed"; return 0; fi
    for f in $files; do
        [ -e "$game/$f" ] || { copy_into "$here/vendor/$name/$f" "$game/$f"; record "$f"; }
    done
    echo "  $name $ver -- $author; $lic"
    echo "      from $page"
}

step prereqs "prerequisites"
echo "prerequisites:"
while IFS= read -r line || [ -n "$line" ]; do
    if [ -n "$line" ]; then prereq "$line"; fi
done < "$here/prereqs.txt"

# ------------------------------------------------------------------ the Direct3D chain

# As the repository's ./install has it (platform/README.md): crosire's d3d8to9 as d3d8.dll,
# on DXVK's d3d9.dll. A
# d3d9.dll that is DXVK already (Proton's, say) stays. GOG's d3d8.dll, or DXVK's, is
# replaced and kept; a d3d8.dll or d3d9.dll that is anything else is someone's own
# choice, and the chain is then left as it is. What goes in is recorded in $MANIFEST like
# the prerequisites, so --uninstall takes it out, unchanged only, and puts the old back.
chain_put() {   # d3d8.dll | d3d9.dll
    local f="$game/$1"
    if [ -e "$f" ]; then
        if [ "$1" = d3d8.dll ] && is_gog_d3d8 "$f"; then
            [ -e "$f.gog-backup" ] || cp -p "$f" "$f.gog-backup"
        else
            [ -e "$f$CHAIN_BACKUP" ] || cp -p "$f" "$f$CHAIN_BACKUP"
        fi
        rm -f "$f"
    fi
    copy_into "$here/chain/$1" "$f"; record "$1"
}

step chain "Direct3D chain"
echo "Direct3D chain:"
d8="$game/d3d8.dll" d9="$game/d3d9.dll"
# On a Mac, never DXVK's d3d9.dll: Wine's own is behind d3d8to9. One an earlier install
# put in (recorded with chain/d3d9.dll's hash) comes out again, and out of $MANIFEST.
if [ "$mac" = 1 ] && [ -f "$d9" ] && [ -f "$MANIFEST" ] \
   && grep -qxF "d3d9.dll $(sha256 "$here/chain/d3d9.dll")" "$MANIFEST" \
   && [ "$(sha256 "$d9")" = "$(sha256 "$here/chain/d3d9.dll")" ]; then
    rm -f "$d9"
    grep -v '^d3d9\.dll ' "$MANIFEST" > "$MANIFEST.tmp" || true
    mv -f "$MANIFEST.tmp" "$MANIFEST"
    echo "  d3d9.dll: took out the DXVK one an earlier install put in"
fi
if [ -e "$d8" ] && ! is_d3d8to9 "$d8" && ! is_gog_d3d8 "$d8" && ! is_dxvk "$d8"; then
    echo "  left as it is: d3d8.dll is none of GOG's, DXVK's or crosire's d3d8to9"
elif [ -e "$d9" ] && ! is_dxvk "$d9"; then
    echo "  left as it is: d3d9.dll is not DXVK's"
else
    if is_d3d8to9 "$d8"; then echo "  d3d8.dll: crosire's d3d8to9 already"
    else chain_put d3d8.dll; echo "  d3d8.dll: crosire's d3d8to9 1.16.0 -- BSD-2-Clause"; fi
    if [ -e "$d9" ] && [ "$mac" = 1 ]; then
        echo "  d3d9.dll: DXVK's, not ours, left as it is -- it cannot run Direct3D 9 on a Mac;"
        echo "  take it out to have Wine's own"
    elif [ -e "$d9" ]; then echo "  d3d9.dll: DXVK already"
    elif [ "$mac" = 1 ]; then echo "  d3d9.dll: Wine's own (wined3d) -- DXVK cannot run Direct3D 9 on a Mac"
    else chain_put d3d9.dll; echo "  d3d9.dll: DXVK 3.1.1 -- zlib/libpng"; fi
fi
echo "the mod:"

# ------------------------------------------------------------------ plugins

# A changed .ini is kept beside the new one, as <name>.ini.bak.
put() {   # file -- copy from the package into the game directory
    local f="$1" d="$game/$1"
    if [ -f "$d" ] && [ "${f##*.}" = ini ] && ! cmp -s "$here/game/$f" "$d"; then
        cp -p "$d" "$d.bak"; echo "  kept your old $f as $f.bak"
    fi
    copy_into "$here/game/$f" "$d"
    echo "  $f"
}

# Render3D.asi and UI.asi need a Direct3D 9 device behind the game's d3d8: crosire's
# d3d8to9, as the chain step puts in. Without it each would stand down to stock.
d3d9=0; is_d3d8to9 "$game/d3d8.dll" && d3d9=1

old_plugins_out   # earlier packages' per-part plugins: Render3D.asi's modules replace them
step render3d "the 3D view"
if [ "$d3d9" = 1 ]; then
    put Render3D.asi; put Render3D.ini
    recipes_out
    for d in $RECIPES; do
        mkdir -p "$game/$d"
        for f in "$here/game/$d"/*.ini; do copy_into "$f" "$game/$d/$(basename "$f")"; done
        echo "  $d/ ($(ls "$here/game/$d" | wc -l | tr -d ' ') recipes)"
    done
else
    rm -f "$game/Render3D.asi" "$game/Render3D.ini" "$game/Render3D.log"; recipes_out
    echo "  Render3D.asi skipped: it needs crosire's d3d8to9 as d3d8.dll -- the 3D view stays the engine's"
fi
step ui "the HUD and the menus"
if [ "$d3d9" = 1 ]; then
    legacy_ui_out   # UI.asi takes their place; each would stand down against the other
    put UI.asi; put UI.ini
else
    # The UI cannot draw: the plugins it supersedes keep the HUD and menus widescreen.
    rm -f "$game/UI.asi" "$game/UI.ini" "$game/UI.log" "$game/MenuScale.asi" "$game/MenuScale.ini"
    echo "  UI.asi skipped: it needs crosire's d3d8to9 as d3d8.dll -- the older plugins instead:"
    put HUD.asi; put HUD.ini
    put Menus.asi; put Menus.ini
    put GridLayout.asi
    # The player's [Cells] choices survive a reinstall; only a missing file is replaced.
    if [ -e "$game/GridLayout.ini" ]; then echo "  GridLayout.ini (yours, kept)"; else put GridLayout.ini; fi
fi
step qol "quality of life"
put QOL.asi; put QOL.ini
# Pay on queue; against the computer only, it stands down in a network game.
put QOLRules.asi; put QOLRules.ini
step models "models"
if [ -n "$PY" ]; then
    echo "  the moons, the selection bubble, the hulls and the asteroids, from your own SOD files:"
    sods --install
else
    echo "  moons, selection bubble, hulls and asteroids skipped: needs python3"
fi
# Internet - Online, on our own transport: Wine's dpnet.dll cannot host.
step online "online multiplayer"
put Online.asi; put Online.ini

step cutscenes "cutscene player"
if skipped cutscenes; then
    unproxy   # one an earlier install put in goes too
    echo "  cutscene player skipped (A2_SKIP)"
else
    # The proxy forwards every call to the stock DLL as binkw32_orig.dll.
    if ! is_proxy "$game/binkw32.dll"; then
        [ -f "$game/binkw32.dll" ] || { echo "no binkw32.dll in $game" >&2; exit 1; }
        cp -p "$game/binkw32.dll" "$game/binkw32.dll.a2neb-backup"
        cp -p "$game/binkw32.dll" "$game/binkw32_orig.dll"
    fi
    [ -f "$game/binkw32_orig.dll" ] || { echo "binkw32.dll is the proxy but binkw32_orig.dll is missing" >&2; exit 1; }
    put binkw32.dll; put BinkProxy.ini
fi

step renderer "renderer settings"
if [ "$mac" = 1 ]; then
    # Only DXVK reads it; one of ours from an earlier install goes.
    if [ -f "$game/dxvk.conf" ] && grep -qF "$DXVK_MARKER" "$game/dxvk.conf"; then
        rm -f "$game/dxvk.conf"
    fi
    echo "  dxvk.conf skipped: only DXVK reads it, and a Mac has no DXVK for Direct3D 9"
elif [ -f "$game/dxvk.conf" ] && ! grep -qF "$DXVK_MARKER" "$game/dxvk.conf"; then
    echo "  dxvk.conf skipped: there is one already that this package did not write"
else
    put dxvk.conf
fi

# ------------------------------------------------------------------ bloom

# Needs a 32-bit vkBasalt layer (lib32-vkbasalt on Arch; or postfx/vkbasalt/build.sh in
# the repository) -- the game is a 32-bit process. Without one, bloom is skipped.
step bloom "bloom"
bloom=0
[ "$mac" = 1 ] || for d in "${XDG_DATA_HOME:-$HOME/.local/share}/vulkan/implicit_layer.d" \
         /etc/vulkan/implicit_layer.d /usr/share/vulkan/implicit_layer.d; do
    ls "$d"/*[Bb]asalt*.json >/dev/null 2>&1 && bloom=1
done

# The shaders are not ours to redistribute, so they come from upstream, pinned by commit
# and checked by hash (<sha256> <commit> <file>). Bloom is optional: a failure skips it.
fetch_shaders() {
    local sum commit file dst url
    while read -r sum commit file || [ -n "$sum" ]; do
        [ -n "$sum" ] || continue
        dst="$BLOOM/Shaders/$file"
        if [ -f "$dst" ] && [ "$(sha256 "$dst")" = "$sum" ]; then continue; fi
        url="https://raw.githubusercontent.com/crosire/reshade-shaders/$commit/Shaders/$file"
        if command -v curl >/dev/null; then curl -fsSL -o "$dst" "$url" || return 1
        else wget -qO "$dst" "$url" || return 1; fi
        [ "$(sha256 "$dst")" = "$sum" ] || { rm -f "$dst"; return 1; }
    done < "$here/bloom/shaders.txt"
}

if [ "$mac" = 1 ]; then
    echo "  bloom skipped: not available on macOS yet"
elif [ "$bloom" = 0 ]; then
    echo "  bloom skipped: no vkBasalt layer installed (optional -- README.txt)"
    # How to get one on this distribution: the GUI installer in the package holds the
    # instructions, so there is one copy of them.
    { [ -n "$PY" ] && "$PY" "$here/armada2-refit-installer.py" --vkbasalt-howto 2>/dev/null; } \
        | sed 's/^/    /' \
        || echo "    install your distribution's 32-bit vkBasalt, then run this again"
else
    case "$BLOOM" in *' '*|*'#'*) bloom=0; echo "  bloom skipped: vkBasalt cannot read the path $BLOOM" ;; esac
fi
if [ "$bloom" = 1 ]; then
    mkdir -p "$BLOOM/Shaders" "$BLOOM/Textures"
    if fetch_shaders; then
        cp "$here/bloom/A2Bloom.fx" "$BLOOM/A2Bloom.fx"
        { echo "$BLOOM_MARKER"; sed -e "s|@BLOOM@|$BLOOM|g" "$here/bloom/vkBasalt.conf.in"; } \
            > "$BLOOM/vkBasalt.conf"
        echo "  bloom, in $BLOOM"
    else
        bloom=0; echo "  bloom skipped: could not download its shaders from GitHub"
    fi
fi

# ------------------------------------------------------------------ what is left

step done "done"
# Wine prefers its own builtin DLLs; these make it load the game directory's.
overrides="winmm=n,b;d3d8=n,b"
if [ -e "$game/d3d9.dll" ]; then overrides="$overrides;d3d9=n,b"; fi
# The GUI installer reads every line of the form "    NAME=value" as a launcher variable;
# print no other line in that shape.
echo
if [ "$mac" = 1 ]; then
    echo "In CrossOver: the bottle's Wine configuration -> Libraries, or set these for the game:"
else
    echo "In the launcher (Heroic: Game settings -> Advanced -> Environment variables; Steam:"
    echo "launch options, then %command%), set:"
fi
echo "    WINEDLLOVERRIDES=$overrides"
if [ "$bloom" = 1 ]; then
    echo "and for bloom:"
    echo "    ENABLE_VKBASALT=1"
    echo "    VKBASALT_CONFIG_FILE=$BLOOM/vkBasalt.conf"
fi
echo "Uninstall: $0 --uninstall \"$game\""
