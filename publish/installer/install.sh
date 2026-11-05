#!/usr/bin/env bash
# Armada II Refit -- install the release package into the game, on Linux (Wine/Proton)
# and on macOS (a Wine prefix: CrossOver, Whisky, a wrapper, plain Wine).
#
#   ./install.sh [<game dir>]               install
#   ./install.sh --uninstall [<game dir>]   take all of it out again
#   ./install.sh --without qol,rules [<game dir>]
#                                           leave layers out (also A2_SKIP="qol rules")
#   ./install.sh --all [<game dir>]         every layer again, forgetting an earlier choice
#
# What can work here is installed, the rest is skipped and says why. The layers that are a
# choice are layers.txt's (select on or off); --without / A2_SKIP names those to leave out
# (and takes them out when an earlier install left them), and the choice is remembered in
# armada2-refit-layers.txt in the game directory: a plain re-run, an upgrade, keeps it.
# Shipped .ini files are merged into the player's (their values stay), never replaced;
# RefitPlayer.ini, the Refit screen's overlay, is never touched.
# First the prerequisites (prereqs.txt: the ASI loader with STA2WidescreenPatch), bundled
# in vendor/, each file only if missing. Then Display8 (display/), the game's d3d8.dll of
# ours: it draws on Vulkan or Direct3D 12 itself, and when neither can start it hands the
# game the system's own d3d8.dll. Render3D.asi (the 3D view) and UI.asi (the HUD, the menus
# and the loading screen) draw the whole frame on Display8's device. Nothing stands behind
# it: no d3d8to9, no DXVK, no dxvk.conf (since 15.6.0; the sampler settings are Display8.ini
# [Samplers]). Where d3d8.dll is a file of the player's own, Display8 stays out, and so do
# Render3D.asi and UI.asi: the game keeps its own look. Then the retire step takes out
# what earlier versions installed and nothing installs now (retire/known.txt, baked as
# retire.txt): the old chain's d3d8to9 and DXVK, a marked dxvk.conf, HUD.asi, Menus.asi,
# GridLayout.asi and the per-part plugins. The models are rewritten from the
# player's own SOD files with python3. Textures,
# the menu backdrops and the replacement movies are built from the game's own art, so a
# release cannot carry them.
#
# On macOS Display8 goes in on Backend=auto as everywhere: Vulkan on MoltenVK first, which
# works under a 32-bit process there (display/README.md, "Will Vulkan work on a Mac?"), and
# the system's own Direct3D 8 if it cannot start. The script keeps to the /bin/bash 3.2 and
# BSD tools a Mac ships.
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
action=install game="" without="" all=0
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
        --all)       all=1 ;;
        --without)   shift; [ $# -gt 0 ] || { echo "--without needs a list of layers" >&2; exit 2; }
                     without="$without,$1" ;;
        --without=*) without="$without,${1#--without=}" ;;
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

# Where packages before 13.0.0 put a vkBasalt bloom; Render3D.asi blooms the 3D view
# itself now, so it is only ever taken out.
BLOOM="${XDG_DATA_HOME:-$HOME/.local/share}/armada2-refit-bloom"
BLOOM_MARKER="# written by armada2-refit install.sh"

MANIFEST="$game/armada2-refit-prereqs.txt"   # what the prerequisites step added

step() { [ "${A2_PROGRESS:-}" != 1 ] || echo "::step $*"; }
old_bloom_out() {
    if [ -f "$BLOOM/vkBasalt.conf" ] && grep -qF "$BLOOM_MARKER" "$BLOOM/vkBasalt.conf"; then
        rm -rf "$BLOOM"
        echo "  removed the old bloom in $BLOOM -- take ENABLE_VKBASALT and VKBASALT_CONFIG_FILE out of the launcher too"
    fi
}

# Render3D.asi's recipes (render3d/content/surfaces, render3d/content/nebulae): Surfaces/ and Nebulae/
# in the game directory, only the files of these names; anything else there is the
# player's.
RECIPES="Surfaces Nebulae"
recipes_out() {
    local d f
    for d in $RECIPES; do
        [ -d "$game/$d" ] || continue
        for f in "$here/layers/render3d/$d"/*.ini; do
            [ -e "$f" ] && rm -f "$game/$d/$(basename "$f")"
        done
        rmdir "$game/$d" 2>/dev/null || true
    done
}
# What earlier versions installed and nothing installs now: retire.txt, retire/known.txt
# without its comments, one entry a line (retire/README.md has the kinds). Each entry is
# recognised by what it is -- a plugin by name, the chain's binaries by hash, dxvk.conf by
# our marker -- so a file of the player's own stays. The vkBasalt bloom is the bloom step's.
RETIRED=0
retire_rm() {   # a file in the game directory
    if [ -e "$1" ]; then
        rm -f "${1:?}"
        echo "  retired ${1#"$game"/}"; RETIRED=1
    fi
}
manifest_drop() {   # a file name recorded for --uninstall that is gone now
    if [ -f "$MANIFEST" ]; then
        grep -v "^$1 " "$MANIFEST" > "$MANIFEST.tmp" || true
        mv -f "$MANIFEST.tmp" "$MANIFEST"
    fi
}
retire_table() {
    local line kind n e d f marker
    [ -f "$here/retire.txt" ] || return 0
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in ''|'#'*) continue ;; esac
        set -f; set -- $line; set +f
        kind="$1"; shift
        case "$kind" in
            plugin)     for n in "$@"; do for e in asi ini log; do retire_rm "$game/$n.$e"; done; done ;;
            plugin-dir) for d in "$@"; do
                            [ -d "$game/$d" ] || continue
                            for f in "$game/$d"/*.bmp; do [ -e "$f" ] && retire_rm "$f"; done
                            rmdir "${game:?}/${d:?}" 2>/dev/null || true
                        done ;;
            glob)       d="$1"; [ -d "$game/$d" ] || continue
                        for f in "$game/$d"/$2; do [ -e "$f" ] && retire_rm "$f"; done
                        rmdir "${game:?}/${d:?}" 2>/dev/null || true ;;
            glob-hash)  d="$1"; [ -d "$game/$d" ] || continue
                        local gp="$2" h k; shift 2
                        for f in "$game/$d"/$gp; do
                            [ -f "$f" ] || continue
                            h="$(sha256 "$f")"
                            for k in "$@"; do if [ "$h" = "$k" ]; then retire_rm "$f"; break; fi; done
                        done
                        rmdir "${game:?}/${d:?}" 2>/dev/null || true ;;
            file)       retire_rm "$game/$1" ;;
            hud-files)  # The file-based HUD fixes of hud 2.x, put back from their backups.
                        if [ -n "${PY:-}" ] && [ -f "$here/retire/hud-files.py" ]; then
                            "$PY" "$here/retire/hud-files.py" "$game" 2>&1 | sed 's/^/  /' || true
                        fi ;;
            chain-hash) f="$game/$1"
                        if [ -f "$f" ] && [ "$(sha256 "$f")" = "$2" ]; then
                            retire_rm "$f"; manifest_drop "$1"
                        fi ;;
            chain-marker)
                        f="$game/$1"; shift; marker="$*"
                        if [ -f "$f" ] && grep -qF "$marker" "$f"; then retire_rm "$f"; fi ;;
            *)          ;;   # bloom (the bloom step's), and kinds a newer table adds
        esac
    done < "$here/retire.txt"
}

is_proxy() { grep -q 'BinkProxy' "$1" 2>/dev/null; }
# The layers left out (the selection below sets SKIP): " qol rules ".
SKIP=" "
skipped() { case "$SKIP" in *" $1 "*) return 0 ;; esac; return 1; }
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
# Display8 (display/), by content: the panic message every build carries. Its hash
# changes with every build. Display8 holds the word "dxvk" too, so ask this first.
is_display8() { [ -f "$1" ] && grep -qa "Display8 stopped the game" "$1"; }
# What Display8 writes beside the exe (display/install --remove takes the same out).
# Its log, like every plugin's since 13.50.0, is in the Wine prefix's
# %LOCALAPPDATA%\armada2-refit\logs: an uninstall leaves that folder (the bug report
# reads it); the names here are what older builds wrote beside the exe.
DISPLAY_FILES="Display8.ini Display8.log Display8.inventory.tsv Display8.inventory.tmp Display8.mark Display8.go"
# Display8 out of the d3d8 slot. An older Display8 stood in front of a d3d8_real.dll: one
# that is not the vendored d3d8to9 (retire.txt takes that one out) is put back in the slot.
# With --all, its settings and logs go too.
display_out() {
    if is_display8 "$game/d3d8.dll"; then
        rm -f "$game/d3d8.dll"
        if [ -f "$game/d3d8_real.dll" ] && ! is_d3d8to9 "$game/d3d8_real.dll"; then
            mv -f "$game/d3d8_real.dll" "$game/d3d8.dll"
        fi
    fi
    if [ "${1:-}" = --all ]; then
        local n
        for n in $DISPLAY_FILES; do rm -f "$game/$n"; done
    fi
    return 0
}
# DXVK by content, never by name: GOG puts a d3d8.dll there too.
is_dxvk() { grep -a -q -i 'dxvk' "$1" 2>/dev/null; }
# crosire's d3d8to9 1.16.0, the old chain's (retire/known.txt), by hash.
D3D8TO9_SHA=122928cfe225c25d30decf7184a5d37e490cecf3b58256ba3206c7e1853f8ab8
is_d3d8to9() { [ -f "$1" ] && [ "$(sha256 "$1")" = "$D3D8TO9_SHA" ]; }
# GOG's own d3d8.dll (its d3d8to9 build), as the game ships it. Kept as
# d3d8.dll.gog-backup, the name platform/d3d8-chain.py gives it, when replaced.
GOG_D3D8_SHA=735dbb81a5fa0368c6436349fc5bf97a9ae15daeb6829dfe95a17704294e6719
is_gog_d3d8() { [ -f "$1" ] && [ "$(sha256 "$1")" = "$GOG_D3D8_SHA" ]; }
CHAIN_BACKUP=.a2chain-backup   # any other d3d8.dll / d3d9.dll an install before 15.6.0 replaced
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
    display_out --all
    for n in Render3D UI QOL QOLRules Online; do
        rm -f "$game/$n.asi" "$game/$n.ini" "$game/$n.log"
    done
    rm -f "$game/armada2-refit-layers.txt"   # the remembered choice of layers
    retire_table
    recipes_out
    if [ -n "$PY" ] && [ -d "$here/models" ]; then
        echo "  the models back to stock:"; sods --revert
    fi
    unproxy
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
    # What an install replaced, back where it took it out: GOG's d3d8.dll (kept as
    # d3d8.dll.gog-backup), and what installs before 15.6.0 kept as .a2chain-backup.
    for f in d3d8.dll d3d9.dll; do
        [ -e "$game/$f" ] && continue
        if [ "$f" = d3d8.dll ] && [ -f "$game/d3d8.dll.gog-backup" ]; then
            mv -f "$game/d3d8.dll.gog-backup" "$game/d3d8.dll"
        elif [ -f "$game/$f$CHAIN_BACKUP" ]; then
            mv -f "$game/$f$CHAIN_BACKUP" "$game/$f"
        elif [ "$f" = d3d8.dll ] && [ -f "$game/d3d8.dll.a2display-backup" ]; then
            mv -f "$game/d3d8.dll.a2display-backup" "$game/d3d8.dll"
        fi
    done
    rm -f "$game/d3d8.dll.a2display-backup"
    old_bloom_out
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

# ------------------------------------------------------------------ the layers to install

# layers.txt (tools/layers.py, baked by package.sh): id, select, requires, scope. A layer
# can be left out when its select is on or off; core ones are the Direct3D chain's and
# Display8's, and the legacy and retired ones are not a choice.
LAYERS_FILE="$game/armada2-refit-layers.txt"
layer_select() { awk -F'\t' -v id="$1" '$1 == id { print $2 }' "$here/layers.txt"; }
want=""
if [ "$all" = 0 ]; then
    for w in $(echo "${A2_SKIP:-} $without" | tr ',' ' '); do want="$want $w"; done
fi
if [ -n "${want// /}" ]; then
    for w in $want; do
        sel="$(layer_select "$w")"
        case "$sel" in
            on|off) case "$SKIP" in *" $w "*) ;; *) SKIP="$SKIP$w " ;; esac ;;
            "")     echo "unknown layer: $w (layers.txt lists them)" >&2; exit 2 ;;
            *)      echo "layer $w is not a choice (select $sel): it cannot be left out" >&2; exit 2 ;;
        esac
    done
elif [ "$all" = 0 ] && [ -f "$LAYERS_FILE" ]; then
    # An earlier choice, kept across the upgrade.
    while read -r kw w _ || [ -n "$kw" ]; do
        if [ "$kw" = without ] && [ -n "$w" ]; then
            case "$(layer_select "$w")" in on|off) SKIP="$SKIP$w " ;; esac
        fi
    done < "$LAYERS_FILE"
    [ "$SKIP" = " " ] || echo "keeping your earlier choice: without${SKIP% } (--all for every layer)"
fi
if [ "$SKIP" = " " ]; then
    rm -f "$LAYERS_FILE"
else
    { echo "# armada2-refit: layers left out of the install; --all brings them back"
      for w in $SKIP; do echo "without $w"; done; } > "$LAYERS_FILE"
fi

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

# A2EngineTest.ini is the bench's fault injection (crates/a2engine/README.md); no package
# ships it, and a stray one would stand hooks down: taken out, in every case.
rm -f "$game/A2EngineTest.ini"

step prereqs "prerequisites"
echo "prerequisites:"
while IFS= read -r line || [ -n "$line" ]; do
    if [ -n "$line" ]; then prereq "$line"; fi
done < "$here/prereqs.txt"

# ------------------------------------------------------------------ Display8

# Display8 (display/), the game's d3d8.dll of ours, as the repository's display/install
# puts it, and Display8.ini written if missing, else merged. It goes into the d3d8.dll slot
# when that is empty, GOG's (kept as d3d8.dll.gog-backup: --uninstall puts it back), the
# old chain's d3d8to9 or a DXVK d3d8 (both retired), or an older Display8; a d3d8.dll that
# is anything else is the player's own and stays, and Display8, Render3D.asi and UI.asi
# stay out. Re-run, Display8 is replaced and Display8.ini merged again.
#
# The merge is tools/ini-merge.py's, without Python: every key the shipped file has and the
# player's lacks is added, in its section, and the player's values stay -- except that a
# value still at an older set's default moves to the new one (Defaults=, numbered in the
# shipped file). INI_UPGRADES is tools/ini-merge.py's UPGRADES (Display8.ini's and UI.ini's
# Defaults 2), one "<section with Defaults=> <n> <section> <key> <old value>", separated by
# ";"; ini_retired is its RETIRED (a key whose meaning changed: its old default is
# replaced wherever it still holds it). Keep them in step. Every shipped .ini goes through
# this, never over the player's (put, below).
# Display8.ini's Defaults 3 is code in each (not a table row): ini_merge's awk, Merge-Ini.
INI_UPGRADES="display8 2 display8 mode forward;display8 2 display8 backend d3d9;ui 2 ui cursor 0;ui 2 menus enable 0;ui 2 loading enable 0"
for k in bar info minimap selection unitview resources buttons messages pause replay overlay chat comm dropplayer loadwait; do
    INI_UPGRADES="$INI_UPGRADES;ui 2 hud $k 0"
done
ini_retired() {   # installed file name -> "<section> <key> <old value>;..."
    case "$(echo "$1" | tr 'A-Z' 'a-z')" in
        render3d.ini) echo "lights beamlight 1.0;lights beamreach 40;nebulae litexplosion 0.75" ;;
    esac
}
ini_merge() {   # shipped installed upgrades retired
    local new="$2.a2merge"
    awk -v label="$(basename "$2")" -v up="$3" -v ret="${4:-}" -v out="$new" -v dxvk="$(dirname "$2")/dxvk.conf" '
    function trim(s) { sub(/^[ \t\r]+/, "", s); sub(/[ \t\r]+$/, "", s); return s }
    function secname(l) {
        if (l !~ /^[ \t]*\[[^]]+\]/) return ""
        sub(/^[ \t]*\[/, "", l); sub(/\].*$/, "", l); return tolower(l)
    }
    function iskey(l) { return l !~ /^[ \t]*;/ && l ~ /^[ \t]*[A-Za-z0-9_]+[ \t]*=/ }
    function keyof(l) { sub(/=.*$/, "", l); return tolower(trim(l)) }
    function nameof(l) { sub(/=.*$/, "", l); return trim(l) }
    function valof(l) { sub(/^[^=]*=/, "", l); return trim(l) }
    { sub(/\r$/, "") }   # line ends as Python reads them: CRLF comes out LF
    NR == FNR {
        sn = secname($0)
        if (sn != "") { cur = sn; if (!(sn in shead)) { ns++; sorder[ns] = sn; shead[sn] = $0 }; next }
        if (cur != "" && iskey($0)) {
            k = keyof($0); sval[cur SUBSEP k] = valof($0)
            nk[cur]++; skl[cur, nk[cur]] = $0; skk[cur, nk[cur]] = k
        }
        next
    }
    { n++; L[n] = $0 }
    END {
        cur = ""
        for (i = 1; i <= n; i++) {
            sn = secname(L[i])
            if (sn != "") { cur = sn; hsec[sn] = 1; continue }
            if (cur != "" && iskey(L[i])) { k = keyof(L[i]); hidx[cur SUBSEP k] = i; hval[cur SUBSEP k] = valof(L[i]) }
        }
        # The defaults upgrade, before any key is added.
        dsec = ""
        for (s = 1; s <= ns; s++) if ((sorder[s] SUBSEP "defaults") in sval) { dsec = sorder[s]; break }
        shipn = 1; if (((dsec SUBSEP "defaults") in sval) && sval[dsec SUBSEP "defaults"] != "") shipn = sval[dsec SUBSEP "defaults"] + 0
        haven = 1; if (((dsec SUBSEP "defaults") in hval) && hval[dsec SUBSEP "defaults"] != "") haven = hval[dsec SUBSEP "defaults"] + 0
        nu = split(up, U, ";")
        for (u = 1; u <= nu; u++) {
            if (split(U[u], f, " ") < 5 || f[1] != dsec) continue
            un = f[2] + 0
            if (!(haven < un && un <= shipn)) continue
            sk = f[3] SUBSEP f[4]
            if ((sk in hidx) && (sk in sval) && hval[sk] == f[5] && hval[sk] != sval[sk]) {
                i = hidx[sk]; name = nameof(L[i])
                L[i] = name "=" sval[sk]
                printf "  %s: [%s] %s %s -> %s (defaults %d)\n", label, f[3], name, f[5], sval[sk], un
            }
        }
        nr = split(ret, R, ";")
        for (u = 1; u <= nr; u++) {
            if (split(R[u], f, " ") < 3) continue
            sk = f[1] SUBSEP f[2]
            if ((sk in hidx) && (sk in sval) && hval[sk] == f[3] && hval[sk] != sval[sk]) {
                i = hidx[sk]; name = nameof(L[i])
                L[i] = name "=" sval[sk]
                printf "  %s: [%s] %s %s -> %s\n", label, f[1], name, f[3], sval[sk]
            }
        }
        # Defaults 3 of Display8.ini (tools/ini-merge.py, display8_defaults3): Mode and Real go,
        # `d3d9` goes from Backend= (nothing left: the shipped value), and the sampler keys of a marked dxvk.conf
        # go into [Samplers] where the player set none (the shipped line the add
        # step below writes is replaced by the carried value).
        if (dsec == "display8" && haven < 3 && 3 <= shipn) {
            nm = split("mode real", DK, " ")
            for (u = 1; u <= nm; u++) {
                sk = "display8" SUBSEP DK[u]
                if (sk in hidx) {
                    printf "  %s: [display8] %s dropped (defaults 3)\n", label, nameof(L[hidx[sk]])
                    gone[hidx[sk]] = 1; delete hidx[sk]
                }
            }
            sk = "display8" SUBSEP "backend"
            if (sk in hidx) {
                i = hidx[sk]; cur = valof(L[i]); nt = split(cur, T, ","); kept = ""; nkept = 0; dropped = 0
                for (u = 1; u <= nt; u++) {
                    t = trim(T[u]); if (t == "") continue
                    if (tolower(t) == "d3d9") { dropped = 1; continue }
                    kept = kept (nkept ? "," : "") t; nkept++
                }
                if (dropped) {
                    if (kept == "") kept = (sk in sval) ? sval[sk] : "auto"
                    printf "  %s: [display8] Backend %s -> %s (defaults 3)\n", label, cur, kept
                    L[i] = nameof(L[i]) "=" kept
                }
            }
            ok = 0
            while ((getline dl < dxvk) > 0) {
                sub(/\r$/, "", dl)
                if (trim(dl) == "# generated by tools/renderer-config.sh") ok = 1
                else if (dl !~ /^[ \t]*#/ && dl ~ /^[ \t]*[A-Za-z0-9_.]+[ \t]*=/) { dk = tolower(keyof(dl)); dv = valof(dl); sub(/[ \t].*$/, "", dv); dconf[dk] = dv }
            }
            close(dxvk)
            if (ok) {
                nd = split("d3d9.sampleranisotropy anisotropy d3d9.samplerlodbias lodbias d3d9.clampnegativelodbias clampnegativelodbias", DM, " ")
                for (u = 1; u < nd; u += 2) {
                    if (!(DM[u] in dconf)) continue
                    key = DM[u + 1]; v = dconf[DM[u]]
                    if (key == "clampnegativelodbias") { if (tolower(v) == "true") v = "1"; else if (tolower(v) == "false") v = "0" }
                    sk = "samplers" SUBSEP key
                    ship = (sk in sval) ? sval[sk] : "auto"
                    if ((v ~ /^-?[0-9.]+$/ && ship ~ /^-?[0-9.]+$/) ? (v + 0 == ship + 0) : (tolower(v) == tolower(ship))) continue
                    if (sk in hidx) {
                        if (tolower(hval[sk]) != "auto") continue
                        i = hidx[sk]; L[i] = nameof(L[i]) "=" v
                    } else {
                        for (j = 1; j <= nk["samplers"]; j++) if (skk["samplers", j] == key) skl["samplers", j] = substr(skl["samplers", j], 1, index(skl["samplers", j], "=")) v
                    }
                    printf "  %s: [samplers] %s -> %s (from dxvk.conf, defaults 3)\n", label, key, v
                }
            }
        }
        if (((dsec SUBSEP "defaults") in hidx) && haven < shipn) {
            i = hidx[dsec SUBSEP "defaults"]; L[i] = nameof(L[i]) "=" shipn
        }
        # Take the dropped lines out now: the insertion below shifts line numbers, which a
        # list of lines to skip afterwards would not follow.
        if (length(gone)) {
            m = 0
            for (i = 1; i <= n; i++) if (!(i in gone)) L[++m] = L[i]
            for (i = m + 1; i <= n; i++) delete L[i]
            n = m; delete gone
        }
        # Every key the player lacks, after the last line of its section (or the section
        # added at the end).
        for (s = 1; s <= ns; s++) {
            sec = sorder[s]; cnt = 0
            for (j = 1; j <= nk[sec]; j++)
                if (!((sec SUBSEP skk[sec, j]) in hidx)) { cnt++; miss[cnt] = skl[sec, j] }
            if (!cnt) continue
            if (sec in hsec) {
                at = n + 1; cur = ""
                for (i = 1; i <= n; i++) {
                    sn = secname(L[i])
                    if (sn != "") { if (cur == sec) { at = i; break }; cur = sn }
                }
                while (at > 1 && trim(L[at - 1]) == "") at--
                for (i = n; i >= at; i--) L[i + cnt] = L[i]
                for (j = 1; j <= cnt; j++) L[at + j - 1] = miss[j]
                n += cnt
            } else {
                L[++n] = ""; L[++n] = shead[sec]
                for (j = 1; j <= cnt; j++) L[++n] = miss[j]
            }
        }
        for (i = 1; i <= n; i++) print L[i] > out
        close(out)
    }' "$1" "$2"
    # Nothing to add (line ends aside): the player's file stays byte for byte.
    if cmp -s <(tr -d '\r' < "$2") "$new"; then rm -f "$new"; else mv -f "$new" "$2"; fi
}
# One key of an installed .ini, by an awk of its own: ini_op get|set|del FILE SECTION KEY
# [VALUE]. get prints the value (a ; comment dropped) and fails when the key is absent;
# set replaces the key's first line, else adds it after the last line of its section (the
# section is added at the end if missing); del removes every line of the key.
ini_op() {
    local op="$1" f="$2"
    awk -v op="$op" -v rsec="$3" -v rkey="$4" -v val="${5:-}" -v out="$f.a2edit" '
    function trim(s) { sub(/^[ \t\r]+/, "", s); sub(/[ \t\r]+$/, "", s); return s }
    { sub(/\r$/, ""); n++; L[n] = $0 }
    END {
        sec = tolower(rsec); key = tolower(rkey); cur = ""; insec = 0; last = 0; first = 0
        for (i = 1; i <= n; i++) {
            t = trim(L[i])
            if (t ~ /^\[/) {
                s = t; sub(/^\[/, "", s); sub(/\].*$/, "", s); cur = tolower(trim(s))
                if (cur == sec) { insec = 1; last = i }
                continue
            }
            if (cur != sec) continue
            if (t != "") last = i
            if (t !~ /^[;#]/ && t ~ /=/) {
                nm = t; sub(/=.*$/, "", nm)
                if (tolower(trim(nm)) == key) {
                    if (!first) {
                        first = i; v = t; sub(/^[^=]*=/, "", v); sub(/;.*$/, "", v); v = trim(v)
                    }
                    if (op == "del") drop[i] = 1
                }
            }
        }
        if (op == "get") { if (first) { print v; exit 0 } exit 1 }
        if (op == "set") {
            if (first) { nm = L[first]; sub(/=.*$/, "", nm); L[first] = nm "=" val }
            else if (insec) { for (i = n; i > last; i--) L[i + 1] = L[i]; L[last + 1] = rkey "=" val; n++ }
            else { L[++n] = ""; L[++n] = "[" rsec "]"; L[++n] = rkey "=" val }
        }
        for (i = 1; i <= n; i++) if (!(i in drop)) print L[i] > out
        close(out)
    }' "$f" && { [ "$op" = get ] || mv -f "$f.a2edit" "$f"; }
}
lc() { echo "$1" | tr 'A-Z' 'a-z'; }
# Settings that moved from one file to another (moved.txt, baked by package.sh from the
# settings tables): a value other than the new default goes to the new key, and the old key
# is removed either way -- tools/settings/move-keys.py's job in the repository's ./install.
apply_moved() {
    [ -s "$here/moved.txt" ] || return 0
    local ff fs fk tf ts tk def v nv
    while IFS=$'\t' read -r ff fs fk tf ts tk def || [ -n "$ff" ]; do
        [ -n "$ff" ] && [ -f "$game/$ff" ] || continue
        v="$(ini_op get "$game/$ff" "$fs" "$fk")" || continue
        # The one rule (tools/settings/README.md, "Moved keys"): a new key the player set
        # (present, not empty, not the default) wins; else the old value moves if it is not
        # the default; the old key goes either way.
        nv=""; if [ -f "$game/$tf" ]; then nv="$(ini_op get "$game/$tf" "$ts" "$tk")" || nv=""; fi
        if [ -n "$nv" ] && [ "$(lc "$nv")" != "$(lc "$def")" ]; then
            echo "  $ff [$fs] $fk=$v dropped ($tf [$ts] $tk=$nv is set already)"
        elif [ "$(lc "$v")" != "$(lc "$def")" ]; then
            if [ ! -f "$game/$tf" ]; then
                echo "  $tf is not there; $ff [$fs] $fk=$v stays"; continue
            fi
            ini_op set "$game/$tf" "$ts" "$tk" "$v"
            echo "  moved $ff [$fs] $fk=$v to $tf [$ts] $tk"
        fi
        ini_op del "$game/$ff" "$fs" "$fk"
    done < "$here/moved.txt"
}

step display "Display8"
d8="$game/d3d8.dll"
if [ -e "$d8" ] && ! is_display8 "$d8" && ! is_gog_d3d8 "$d8" && ! is_d3d8to9 "$d8" && ! is_dxvk "$d8"; then
    echo "  Display8 skipped: d3d8.dll is none of GOG's, the old chain's or ours -- yours stays"
else
    if is_gog_d3d8 "$d8"; then
        [ -e "$d8.gog-backup" ] || cp -p "$d8" "$d8.gog-backup"
    elif ! is_display8 "$d8" && ! is_d3d8to9 "$d8" && [ -e "$d8" ]; then
        # A d3d8.dll whose hash we do not know (a DXVK's, the player's own build of it) is
        # kept; the uninstall puts it back.
        [ -e "$d8$CHAIN_BACKUP" ] || cp -p "$d8" "$d8$CHAIN_BACKUP"
        echo "  d3d8.dll: not one we know, replaced (the old one is d3d8.dll$CHAIN_BACKUP)"
    elif ! is_display8 "$d8" && [ -e "$d8" ]; then
        echo "  d3d8.dll: the old chain's, replaced"
    fi
    # The new one beside it first: a copy that fails leaves the slot as it was.
    copy_into "$here/layers/display/d3d8.dll" "$game/d3d8.dll.a2display-new"
    mv -f "$game/d3d8.dll.a2display-new" "$d8"
    manifest_drop d3d8.dll   # an install before 15.6.0 recorded the d3d8to9 it put here
    echo "  d3d8.dll: Display8 (on Vulkan or Direct3D 12; the system's own d3d8.dll if neither starts)"
    if [ ! -f "$game/Display8.ini" ] && [ -f "$game/dxvk.conf" ] && grep -qF "# generated by tools/renderer-config.sh" "$game/dxvk.conf"; then
        # A dxvk.conf of ours with no Display8.ini beside it: merge into a stub at Defaults=1
        # so its sampler values are carried (retire.txt takes the dxvk.conf out after).
        printf '[Display8]\nDefaults=1\n' > "$game/Display8.ini"
    fi
    if [ -f "$game/Display8.ini" ]; then
        ini_merge "$here/layers/display/Display8.ini" "$game/Display8.ini" "$INI_UPGRADES"
        echo "  Display8.ini (yours, brought up to date)"
    else
        copy_into "$here/layers/display/Display8.ini" "$game/Display8.ini"
        echo "  Display8.ini"
    fi
fi
step retire "taking out what earlier versions installed"
echo "retired:"
retire_table
[ "$RETIRED" = 1 ] || echo "  nothing to take out"

echo "the mod:"

# ------------------------------------------------------------------ plugins

# A file of a layer into the game directory. An .ini the player has is merged with the
# shipped one (ini_merge: the keys it lacks are added, the values stay), never replaced.
put() {   # layer file
    local src="$here/layers/$1/$2" d="$game/$2"
    if [ -f "$d" ] && [ "${2##*.}" = ini ]; then
        ini_merge "$src" "$d" "$INI_UPGRADES" "$(ini_retired "$2")"
        echo "  $2 (yours, brought up to date)"
        return 0
    fi
    copy_into "$src" "$d"
    echo "  $2"
}
# A layer left out is taken out when an earlier install left it.
layer_out() {   # name... -- each plugin with its ini and log
    local n
    for n in "$@"; do
        rm -f "${game:?}/${n:?}.asi" "${game:?}/${n:?}.ini" "${game:?}/${n:?}.log"
    done
}

# Render3D.asi and UI.asi draw on Display8's device: without Display8 as d3d8.dll each would
# stand down to stock.
display8=0
if is_display8 "$game/d3d8.dll"; then display8=1; fi

step render3d "the 3D view"
if skipped render3d; then
    layer_out Render3D; recipes_out
    echo "  the 3D view left out (you chose to)"
elif [ "$display8" = 1 ]; then
    put render3d Render3D.asi; put render3d Render3D.ini
    recipes_out
    for d in $RECIPES; do
        mkdir -p "$game/$d"
        for f in "$here/layers/render3d/$d"/*.ini; do copy_into "$f" "$game/$d/$(basename "$f")"; done
        echo "  $d/ ($(ls "$here/layers/render3d/$d" | wc -l | tr -d ' ') recipes)"
    done
else
    layer_out Render3D; recipes_out
    echo "  Render3D.asi skipped: it needs Display8 as d3d8.dll -- the 3D view stays the engine's"
fi
step ui "the HUD and the menus"
if skipped ui; then
    layer_out UI
    echo "  the HUD and menus left out (you chose to)"
elif [ "$display8" = 1 ]; then
    put ui UI.asi; put ui UI.ini
else
    # The UI cannot draw, and there is no older plugin to fall back on: the game keeps its own.
    layer_out UI
    echo "  UI.asi skipped: it needs Display8 as d3d8.dll -- the HUD and menus stay the engine's"
fi
step qol "quality of life"
if skipped qol; then layer_out QOL; echo "  quality of life left out (you chose to)"
else put qol QOL.asi; put qol QOL.ini; fi
step rules "rules"
# Pay on queue: every player of a network game needs it (a game with anyone lacking it
# plays stock rules); against the computer only, it stands down in other transports.
if skipped rules; then layer_out QOLRules; echo "  rules left out (you chose to)"
else put rules QOLRules.asi; put rules QOLRules.ini; fi
step models "models"
if skipped models; then
    if [ -n "$PY" ] && [ -d "$here/models" ]; then
        echo "  the models back to stock (you chose to leave them out):"; sods --revert
    else
        echo "  models left out (you chose to)"
    fi
elif [ -n "$PY" ]; then
    echo "  the moons, the selection bubble, the hulls and the asteroids, from your own SOD files:"
    sods --install
else
    echo "  moons, selection bubble, hulls and asteroids skipped: needs python3"
fi
# Internet - Online, on our own transport: Wine's dpnet.dll cannot host.
step online "online multiplayer"
if skipped online; then layer_out Online; echo "  online multiplayer left out (you chose to)"
else put online Online.asi; put online Online.ini; fi

step cutscenes "cutscene player"
if skipped cutscenes; then
    unproxy   # one an earlier install put in goes too
    echo "  cutscene player skipped (left out)"
else
    # The proxy forwards every call to the stock DLL as binkw32_orig.dll.
    if ! is_proxy "$game/binkw32.dll"; then
        [ -f "$game/binkw32.dll" ] || { echo "no binkw32.dll in $game" >&2; exit 1; }
        cp -p "$game/binkw32.dll" "$game/binkw32.dll.a2neb-backup"
        cp -p "$game/binkw32.dll" "$game/binkw32_orig.dll"
    fi
    [ -f "$game/binkw32_orig.dll" ] || { echo "binkw32.dll is the proxy but binkw32_orig.dll is missing" >&2; exit 1; }
    put cutscenes binkw32.dll; put cutscenes BinkProxy.ini
fi

# Settings that moved between files since the version the player had.
apply_moved

# ------------------------------------------------------------------ old bloom

# Packages before 13.0.0 set up a vkBasalt bloom; Render3D.asi's own replaced it, and
# both at once would bloom the frame twice.
step bloom "taking out the old bloom"
old_bloom_out

# ------------------------------------------------------------------ what is left

step done "done"
# Wine prefers its own builtin DLLs; these make it load the game directory's.
overrides="winmm=n,b;d3d8=n,b"   # no d3d9 since 15.6.0: nothing of ours is on that slot
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
echo "Uninstall: $0 --uninstall \"$game\""
