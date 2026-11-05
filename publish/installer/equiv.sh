#!/usr/bin/env bash
# Do the installers do the same thing?   equiv.sh [--no-pwsh] [--no-repo] [<zip>]
#
# The release install exists twice, install.sh (bash 3.2) and install.ps1 (PowerShell), and
# the repository's ./install does it a third way, layer script by layer script.
# tools/layers.py --check compares the ::step ids they print; this compares what they DO.
# Each scenario starts two (or three) copies of the same mock game directory, runs one
# installer into each, step by step, and after every step compares the trees: every path,
# the bytes of every file, each .ini key by key where one differs (equiv-diff.py), the
# ::step lines, and the exit codes. Then the ini mergers on their own fixtures
# (equiv-ini.sh). A difference fails the run unless equiv-known.txt lists it, with a
# reason; those are printed as "known".
#
# <zip> is a release package (CI's `package` job's); without one, publish/package.sh
# --stub stages one from placeholder plugins, in the same layout. The mock game is a few
# stand-in files (an "exe" of three bytes, a stand-in for GOG's d3d8.dll and the stock
# binkw32.dll), never game content; the package is copied and its scripts and retire.txt
# told the stand-ins' hashes, as test.sh does, so the GOG, d3d8to9 and DXVK cases run.
#
#   --no-pwsh   skip install.ps1 (also when pwsh is missing, unless A2_EQUIV_REQUIRE_PWSH=1)
#   --no-repo   skip the repository's ./install
#
# What it does not cover is printed at the end (and in README.md, "Equivalence"): it runs
# PowerShell 7 on Linux, not Windows PowerShell 5.1 on Windows; the GUI installer; the
# macOS branch of install.sh (test.sh fakes it); and of ./install only what runs without
# a game, a build or assets. publish/installer/README.md, "Equivalence".
set -euo pipefail
src="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$src/../.." && pwd)"
use_pwsh=1 use_repo=1 zip=""
while [ $# -gt 0 ]; do
    case "$1" in
        --no-pwsh) use_pwsh=0 ;;
        --no-repo) use_repo=0 ;;
        -h|--help) sed -n '2,/^set -e/p' "$0" | grep '^#' | sed 's/^# \{0,1\}//'; exit 0 ;;
        -*)        echo "unknown option: $1" >&2; exit 2 ;;
        *)         zip="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")" ;;
    esac
    shift
done
T="$(mktemp -d "${TMPDIR:-/tmp}/a2equiv.XXXXXX")"; trap 'rm -rf "$T"' EXIT
T="$(cd "$T" && pwd)"
KNOWN="$src/equiv-known.txt"
PWSH="${PWSH:-pwsh}"
if [ "$use_pwsh" = 1 ] && ! command -v "$PWSH" >/dev/null 2>&1; then
    if [ "${A2_EQUIV_REQUIRE_PWSH:-}" = 1 ]; then echo "no pwsh, and A2_EQUIV_REQUIRE_PWSH=1" >&2; exit 1; fi
    echo "SKIP  pwsh not found: install.ps1 is not compared here (CI compares it; README.md says how to get pwsh)"
    use_pwsh=0
fi
fails=0 checks=0
skipped=()

# ------------------------------------------------------------------ the package

if [ -z "$zip" ]; then
    echo "== staging a package from placeholder plugins (package.sh --stub)"
    zip="$(bash "$root/publish/package.sh" --stub "$T/dist")"
fi
unzip -q "$zip" -d "$T/unz"
P0="$(ls -d "$T"/unz/armada2-refit-*/)"; P0="${P0%/}"
sum() { sha256sum "$1" | cut -d' ' -f1; }
# Stand-ins for GOG's d3d8.dll, crosire's d3d8to9 and DXVK's d3d9.dll; the package's copy
# is told their hashes in place of the real ones, in both scripts and in retire.txt.
printf 'gog d3d8.dll stand-in\n' > "$T/gog.dll"
printf 'd3d8to9 stand-in\n' > "$T/d3d8to9.dll"
printf 'old d3d9.dll stand-in\n' > "$T/d3d9.dll"
P="$T/pkg"; cp -R "$P0" "$P"
gog_real="$(sed -n 's/^GOG_D3D8_SHA=//p' "$P/install.sh")"
d2_real="$(sed -n 's/^D3D8TO9_SHA=//p' "$P/install.sh")"
d9_real="$(awk '$1 == "chain-hash" && $2 == "d3d9.dll" { print $3 }' "$P/retire.txt")"
[ -n "$gog_real" ] && [ -n "$d2_real" ] && [ -n "$d9_real" ] || { echo "the hashes to stand in for were not found" >&2; exit 1; }
for f in install.sh install.ps1 retire.txt; do
    sed -i "s/$gog_real/$(sum "$T/gog.dll")/g; s/$d2_real/$(sum "$T/d3d8to9.dll")/g; s/$d9_real/$(sum "$T/d3d9.dll")/g" "$P/$f"
done
grep -v ' retire.txt$' "$P/SHA256SUMS" > "$T/sums"; echo "$(sum "$P/retire.txt")  retire.txt" >> "$T/sums"; mv "$T/sums" "$P/SHA256SUMS"
# Each script must have taken every stand-in: a hash only one of them knows is a divergence.
for h in "$(sum "$T/gog.dll")" "$(sum "$T/d3d8to9.dll")"; do
    for f in install.sh install.ps1; do
        grep -q "$h" "$P/$f" || { echo "FAIL  $f does not know the hash the other script has for a stand-in"; fails=$((fails + 1)); }
    done
done

# ------------------------------------------------------------------ the environment

# install.sh refuses while an Armada2.exe runs anywhere (a bench game): pgrep sees none.
mkdir -p "$T/bin"; printf '#!/bin/sh\nexit 1\n' > "$T/bin/pgrep"; chmod +x "$T/bin/pgrep"
export PATH="$T/bin:$PATH"
export XDG_DATA_HOME="$T/xdg"            # install.sh's old vkBasalt bloom
export LOCALAPPDATA="$T/localappdata"    # install.ps1's Display8 pipeline cache
mkdir -p "$T/win/SysWOW64" "$T/win/System32"; : > "$T/win/SysWOW64/vcruntime140.dll"; : > "$T/win/System32/vcruntime140.dll"
export WINDIR="$T/win"                   # install.ps1 looks for the Visual C++ runtime there
export A2_PYTHON="$(command -v python3)" # the models and hud-files.py, in both
unset A2_SKIP A2_GAME 2>/dev/null || true

# ------------------------------------------------------------------ the mock games

# mock DIR KIND -- fresh (GOG's d3d8.dll), own (a d3d8.dll of the player's), empty (no
# d3d8.dll), unknown (a d3d8.dll naming DXVK, of no known hash), old (what a 14.x install
# left: the chain, an old Display8, retired plugins, old inis, the hud fixes' backups).
mock() {
    local G="$1" n
    mkdir -p "$G"
    printf 'exe' > "$G/Armada2.exe"; printf 'stock binkw32 stand-in\n' > "$G/binkw32.dll"
    printf '; mine\r\n[Render3D.Modules]\r\nhulls=0\r\n' > "$G/RefitPlayer.ini"
    case "$2" in
        fresh)   cp "$T/gog.dll" "$G/d3d8.dll" ;;
        own)     printf 'a d3d8.dll of the player'"'"'s own\n' > "$G/d3d8.dll" ;;
        empty)   ;;
        unknown) printf 'a dxvk d3d8.dll of no known hash\n' > "$G/d3d8.dll" ;;
        old)
            cp "$T/gog.dll" "$G/d3d8.dll.gog-backup"
            printf 'MZ Display8 stopped the game: a 14.x build\n' > "$G/d3d8.dll"
            cp "$T/d3d8to9.dll" "$G/d3d8_real.dll"; cp "$T/d3d9.dll" "$G/d3d9.dll"
            printf 'd3d9.dll %s\n' "$(sum "$T/d3d9.dll")" > "$G/armada2-refit-prereqs.txt"
            printf '# generated by tools/renderer-config.sh\nd3d9.samplerAnisotropy = 16\nd3d9.samplerLodBias = -1.25\nd3d9.clampNegativeLodBias = False\n' > "$G/dxvk.conf"
            printf '[Display8]\nMode=forward\nReal=auto\nBackend=d3d9,vulkan\nVSync=0\n' > "$G/Display8.ini"
            printf '[UI]\r\nEnable=1\r\nCursor=0\r\nKeepClip=0\r\nNative=1\r\n\r\n[HUD]\r\nBar=0\r\nInfo=1\r\nBarKeys=0\r\n\r\n[Menus]\r\nEnable=0\r\n' > "$G/UI.ini"
            printf '[Lights]\nBeamLight=1.0\nBeamReach=30\n\n[Nebulae]\nLitExplosion=0.75\n' > "$G/Render3D.ini"
            printf '[QOL]\nPanSpeed = 7\nViewDistance=500\n' > "$G/QOL.ini"
            printf 'old\n' > "$G/QOL.asi"
            for n in HUD GridLayout Menus MenuScale MSAA Sky Planets; do printf 'old\n' > "$G/$n.asi"; printf 'old\n' > "$G/$n.ini"; done
            mkdir -p "$G/MenuScale" "$G/Sky" "$G/Surfaces"
            printf 'BM\n' > "$G/MenuScale/plate.bmp"
            cp "$root/render3d/content/sky/mbgaqu.ini" "$G/Sky/mbgaqu.ini"   # a shipped copy: by hash
            printf '[Sky]\n' > "$G/Sky/myown.ini"; printf '[Surface]\n' > "$G/Surfaces/myown.ini"
            mkdir -p "$G/misc" "$G/Sprites" "$G/Textures/RGB"
            printf 'stock-cfg\n' > "$G/misc/gui_fed.cfg.a2neb-backup"; printf 'wide-cfg\n' > "$G/misc/gui_fed.cfg"
            printf 'stock-spr\n' > "$G/Sprites/FontFinal4_12.spr.a2font-backup"; printf 'cond-spr\n' > "$G/Sprites/FontFinal4_12.spr"
            printf 'stock-tga\n' > "$G/Textures/RGB/FontFinal4_12.tga.a2font-backup"; printf 'cond-tga\n' > "$G/Textures/RGB/FontFinal4_12.tga"
            printf '[bloom]\n' > "$G/A2Bloom.ini"; cp "$G/A2Bloom.ini" "$G/ReShadePreset.ini"
            printf '[Hooks]\n' > "$G/A2EngineTest.ini"
            printf '# armada2-refit: layers left out\nwithout rules\n' > "$G/armada2-refit-layers.txt"
            mkdir -p "$XDG_DATA_HOME/armada2-refit-bloom"
            printf '# written by armada2-refit install.sh\n' > "$XDG_DATA_HOME/armada2-refit-bloom/vkBasalt.conf"
            ;;
        *) echo "mock: unknown kind $2" >&2; exit 2 ;;
    esac
}

# ------------------------------------------------------------------ the runs

# The neutral spelling of a run -- install, install:without=a,b, install:all, uninstall --
# in each script's own.
args_sh() {
    case "$1" in
        install) ;; install:all) echo --all ;; uninstall) echo --uninstall ;;
        install:without=*) echo "--without ${1#install:without=}" ;;
    esac
}
args_ps() {
    case "$1" in
        install) ;; install:all) echo -All ;; uninstall) echo -Uninstall ;;
        install:without=*) echo "-Without ${1#install:without=}" ;;
    esac
}
run_sh() {   # game action
    # shellcheck disable=SC2046
    (cd "$T" && A2_PROGRESS=1 bash "$P/install.sh" $(args_sh "$2") "$1") > "$T/out.sh" 2>&1
}
run_ps() {
    # shellcheck disable=SC2046
    (cd "$T" && A2_PROGRESS=1 "$PWSH" -NoProfile -NonInteractive -File "$P/install.ps1" $(args_ps "$2") "$1") > "$T/out.ps" 2>&1
}
steps() { grep '^::step ' "$1" | cut -d' ' -f2 || true; }

# compare SCENARIO STEP PAIR NAME_A DIR_A NAME_B DIR_B
compare() {
    checks=$((checks + 1))
    if python3 -I "$src/equiv-diff.py" --trees --pair "$3" --scenario "$1/$2" --known "$KNOWN" --used "$T/used" \
            "$4" "$5" "$6" "$7" > "$T/diff"; then
        echo "ok    $1/$2: $4 and $6 leave the same tree"
    else
        echo "FAIL  $1/$2: $4 and $6 leave different trees"; fails=$((fails + 1))
    fi
    sed 's/^/      /' "$T/diff"
}

# scenario NAME KIND ACTION... -- a mock game per installer, each action run in each,
# compared after every one. "edit:<what>" changes both copies alike between runs.
scenario() {
    local name="$1" kind="$2" a rs rp; shift 2
    local S="$T/s/$name"; rm -rf "$S"; mkdir -p "$S"
    mock "$S/sh" "$kind"; [ "$use_pwsh" = 0 ] || mock "$S/ps" "$kind"
    echo "== $name ($kind): $*"
    for a in "$@"; do
        case "$a" in
            edit:*) edit "${a#edit:}" "$S/sh"; [ "$use_pwsh" = 0 ] || edit "${a#edit:}" "$S/ps"; continue ;;
        esac
        rs=0; run_sh "$S/sh" "$a" || rs=$?
        if [ "$use_pwsh" = 0 ]; then
            checks=$((checks + 1))
            if [ "$rs" = 0 ]; then echo "ran   $name/$a: install.sh alone (no pwsh to compare with)"
            else echo "FAIL  $name/$a: install.sh exited $rs"; tail -5 "$T/out.sh" | sed 's/^/      sh| /'; fails=$((fails + 1)); fi
            continue
        fi
        rp=0; run_ps "$S/ps" "$a" || rp=$?
        checks=$((checks + 1))
        if [ "$rs" != "$rp" ]; then
            echo "FAIL  $name/$a: install.sh exited $rs, install.ps1 $rp"; fails=$((fails + 1))
            tail -5 "$T/out.sh" | sed 's/^/      sh| /'; tail -5 "$T/out.ps" | sed 's/^/      ps| /'
        fi
        if [ "$(steps "$T/out.sh")" != "$(steps "$T/out.ps")" ]; then
            echo "FAIL  $name/$a: the ::step lines differ: sh $(steps "$T/out.sh" | tr '\n' ' '), ps $(steps "$T/out.ps" | tr '\n' ' ')"
            fails=$((fails + 1))
        fi
        compare "$name" "$a" sh-ps install.sh "$S/sh" install.ps1 "$S/ps"
    done
}
# What a player does between two runs, the same in each copy.
edit() {
    local G="$2"
    case "$1" in
        player)   # an edited QOL.ini (a value, CRLF, a key gone), a recipe of their own
            printf '[QOL]\r\nPanSpeed=7\r\n; mine\r\n' > "$G/QOL.ini"
            printf '[Surface]\n' > "$G/Surfaces/myown.ini"
            printf 'mine\n' > "$G/UltimateASILoader-license.txt"   # a prerequisite changed since: left
            ;;
        *) echo "edit: unknown $1" >&2; exit 2 ;;
    esac
}

scenario fresh fresh install install edit:player install \
    install:without=qol,online install install:without=ui,render3d,models,cutscenes install:all uninstall
scenario upgrade old install install uninstall
scenario own-d3d8 own install uninstall
scenario empty-slot empty install uninstall
scenario unknown-d3d8 unknown install uninstall

# ------------------------------------------------------------------ the repository's ./install

# ./install builds each plugin and installs it with its layer's script. Here it runs from a
# copy of this checkout whose build scripts are stand-ins: each copies the package's
# placeholder (or real) plugin where the build would put it. The asset layers (textures,
# the menu plates, the movies) find nothing built in an empty A2_DATA and install
# nothing, as for any player without a pack. Compared with install.sh on the same mock.
if [ "$use_repo" = 1 ]; then
    R="$T/repo"; mkdir -p "$R"
    (cd "$root" && git ls-files -z --cached --others --exclude-standard | while IFS= read -r -d '' f; do
        [ -e "$f" ] || continue; mkdir -p "$R/$(dirname "$f")"; cp -p "$f" "$R/$f"; done)
    stub_build() {   # layer-dir file
        printf '#!/usr/bin/env bash\nmkdir -p "$(dirname "$0")/build"\ncp "%s" "$(dirname "$0")/build/%s"\n' "$3" "$2" > "$R/$1/build.sh"
    }
    # The stand-ins' hashes, as in the package (platform/d3d8-chain.py, retire/known.txt).
    for f in platform/d3d8-chain.py retire/known.txt; do
        sed -i "s/$gog_real/$(sum "$T/gog.dll")/g; s/$d2_real/$(sum "$T/d3d8to9.dll")/g; s/$d9_real/$(sum "$T/d3d9.dll")/g" "$R/$f"
    done
    stub_build display d3d8.dll "$P/layers/display/d3d8.dll"
    stub_build qol QOL.asi "$P/layers/qol/QOL.asi"
    stub_build rules QOLRules.asi "$P/layers/rules/QOLRules.asi"
    stub_build online Online.asi "$P/layers/online/Online.asi"
    stub_build render3d Render3D.asi "$P/layers/render3d/Render3D.asi"
    stub_build ui UI.asi "$P/layers/ui/UI.asi"
    stub_build cutscenes/binkproxy binkw32.dll "$P/layers/cutscenes/binkw32.dll"
    # models/install.sh stops on a game without the stock SODs, which no mock has; the
    # release scripts run the same model scripts, say they failed and go on, writing
    # nothing. So here it is a no-op, and the models are not compared (listed below).
    printf '#!/usr/bin/env bash\necho "models: not run by equiv.sh (no SODs in a mock game)"\n' > "$R/models/install.sh"
    # tools/settings/move-keys.py, which ./install runs last, edits through the a2ini host
    # tool, which tools/settings/host.sh builds with cargo. A stand-in takes its place:
    # `a2ini edit FILE Sec:Key=value` and `a2ini unset FILE Sec:Key` by install.sh's own
    # ini_op, cut out of the package's install.sh. So move-keys.py's decisions (which value
    # moves, which is dropped) are compared, and a2ini's writer is not (listed below).
    {
        echo '#!/usr/bin/env bash'
        echo 'set -euo pipefail'
        awk '/^ini_op\(\) /{p=1} p{print} p&&/^}/{exit}' "$P/install.sh"
        cat <<'A2INI'
cmd="$1" f="$2"; shift 2
for w in "$@"; do
    [ "$w" != --unset ] || { cmd=unset; continue; }
    sec="${w%%:*}" rest="${w#*:}"
    case "$cmd" in
        edit)  ini_op set "$f" "$sec" "${rest%%=*}" "${rest#*=}" ;;
        unset) ini_op del "$f" "$sec" "$rest" ;;
        *)     echo "a2ini stand-in: no $cmd" >&2; exit 2 ;;
    esac
done
A2INI
    } > "$T/a2ini"
    chmod +x "$T/a2ini"
    grep -q '^ini_op() ' "$T/a2ini" || { echo "could not cut ini_op out of install.sh" >&2; exit 1; }
    printf '#!/usr/bin/env bash\necho "%s"\n' "$T/a2ini" > "$R/tools/settings/host.sh"
    # And no cargo at all: a build this harness has not replaced fails loudly here, as on
    # a CI runner without a default toolchain.
    mkdir -p "$T/nocargo"
    printf '#!/bin/sh\necho "equiv.sh: ./install reached cargo ($*): stand it in" >&2\nexit 1\n' > "$T/nocargo/cargo"
    chmod +x "$T/nocargo/cargo"
    mkdir -p "$T/a2data" "$T/home"
    run_repo() {   # game action
        local sel=""
        case "$2" in install) ;; install:without=*) sel="--without ${2#install:without=}" ;;
                     *) echo "./install has no $2" >&2; return 2 ;; esac
        # shellcheck disable=SC2086
        (cd "$R" && PATH="$T/nocargo:$PATH" HOME="$T/home" A2_GAME="$1" A2_DATA="$T/a2data" A2_PREFIX="$T/prefix" ./install $sel) > "$T/out.repo" 2>&1
    }
    repo_scenario() {   # name kind action...
        local name="$1" kind="$2" a rs rr; shift 2
        local S="$T/r/$name"; rm -rf "$S"; mkdir -p "$S"
        mock "$S/sh" "$kind"; mock "$S/repo" "$kind"
        echo "== ./install: $name ($kind): $*"
        for a in "$@"; do
            rs=0; run_sh "$S/sh" "$a" || rs=$?
            rr=0; run_repo "$S/repo" "$a" || rr=$?
            checks=$((checks + 1))
            if [ "$rs" != 0 ] || [ "$rr" != 0 ]; then
                echo "FAIL  $name/$a: install.sh exited $rs, ./install $rr"; fails=$((fails + 1))
                tail -8 "$T/out.repo" | sed 's/^/      repo| /'
            fi
            compare "repo-$name" "$a" sh-repo install.sh "$S/sh" ./install "$S/repo"
        done
    }
    repo_scenario fresh fresh install install:without=qol,online
    repo_scenario upgrade old install
    repo_scenario own-d3d8 own install
    repo_scenario empty-slot empty install
    skipped+=("./install's asset layers with assets built (textures, the menu plates, the movies): A2_DATA is empty here, so each is only seen installing nothing")
    skipped+=("./install's builds and its models layer: the build scripts are stand-ins that copy the package's plugins, and models/install.sh a no-op (it stops on a game without the stock SODs)")
    skipped+=("the a2ini host tool's writer, which move-keys.py edits through: a stand-in (install.sh's ini_op) takes its place, so no cargo is needed; crates/a2ini is host-tested on tools/settings/fixtures")
    skipped+=("./install --uninstall: there is none; a2mod stock snapshots instead, a different contract")
fi

# ------------------------------------------------------------------ the ini mergers

echo "== the ini mergers on the shared fixtures (equiv-ini.sh)"
checks=$((checks + 1))
if [ "$use_pwsh" = 1 ]; then ini_env=(); else ini_env=(PWSH=/nonexistent); fi
if env ${ini_env[@]+"${ini_env[@]}"} bash "$src/equiv-ini.sh" > "$T/ini" 2>&1; then
    grep -v '^ok' "$T/ini" | sed 's/^/      /'; echo "ok    $(tail -1 "$T/ini")"
else
    sed 's/^/      /' "$T/ini"; echo "FAIL  the ini mergers"; fails=$((fails + 1))
fi

# ------------------------------------------------------------------ what is not covered

echo "== not covered here"
skipped+=("install.ps1 under Windows PowerShell 5.1 on Windows: here pwsh 7 on Linux, a case-sensitive file system, / as the separator; test.ps1 runs it on a Windows runner, but compares nothing")
skipped+=("the GUI installer (gui-installer/): it runs these scripts, and gui-installer/test.sh checks it on its own")
skipped+=("install.sh's macOS branch (CrossOver discovery, its messages): test.sh fakes uname, the trees are the same code")
skipped+=("the model rewrites on real SODs: the mock has none, so models/*.py find nothing to do in every installer")
skipped+=("what Windows alone has: the GOG registry lookup, the Visual C++ runtime warning, the ReShade bloom preset, the pipeline cache in LOCALAPPDATA")
[ "$use_pwsh" = 1 ] || skipped+=("install.ps1: no pwsh on this machine (README.md, Equivalence, says how to get one)")
[ "$use_repo" = 1 ] || skipped+=("./install: --no-repo")
for s in "${skipped[@]}"; do echo "  - $s"; done
# Known lines no comparison needed: stale, or for a part skipped here.
if [ "$use_pwsh" = 1 ] && [ "$use_repo" = 1 ]; then
    grep -vE '^(#|$)' "$KNOWN" | while IFS= read -r k; do
        grep -qxF "$k" "$T/used" 2>/dev/null || echo "note  equiv-known.txt line not needed this run: $k"
    done
fi

echo "equivalence: $checks checks, $fails failed"
[ "$fails" = 0 ]
