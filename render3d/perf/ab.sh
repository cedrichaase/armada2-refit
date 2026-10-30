#!/usr/bin/env bash
# Frame time of two checkouts or two commits, A against B, on the bench (render3d/PERF.md, "A/B").
#
#   render3d/perf/ab.sh A B [OUT_DIR]
#
# A and B are each a checkout directory or a git revision of this repository (a commit,
# branch or tag). A revision is exported with `git archive` into
# ~/.cache/armada2-refit/perf-trees/<sha>/ (kept and reused; its install builds into it),
# so neither the user's checkout nor a worktree is built in. A directory is used as it
# is, and its install builds into it.
#
# For each scene (SCENES, default "showcase match nebula"), each side and each of RUNS
# passes (default 1), one session at a time: start the scene with that side's ./install
# and probe-new (Timing=1), wait for the scene, wait SETTLE seconds (default 80), keep
# Render3D.log as OUT_DIR/<a|b>[.<run>]-<scene>.log and a shot, stop. Sides alternate
# within a pass (a b, then b a) so a drift in the machine does not land on one side.
# Then summary.py OUT_DIR. RES (default 1920x1080) is the resolution.
set -u
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
[ $# -ge 2 ] || { sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }
out=${3:-${A2_DATA:-$HOME/.local/share/armada2-refit}/perf/ab-$(date +%Y%m%d-%H%M%S)}
res=${RES:-1920x1080}
runs=${RUNS:-1}
settle=${SETTLE:-80}
trees=${XDG_CACHE_HOME:-$HOME/.cache}/armada2-refit/perf-trees

# A checkout directory as it is, or a revision exported once into $trees/<sha>.
resolve() {
    if [ -d "$1" ]; then (cd "$1" && pwd); return; fi
    local sha
    sha=$(git -C "$repo" rev-parse --verify --quiet "$1^{commit}") ||
        { echo "ab.sh: $1 is neither a directory nor a revision" >&2; return 1; }
    if [ ! -f "$trees/$sha/install" ]; then
        mkdir -p "$trees/$sha.tmp"
        git -C "$repo" archive "$sha" | tar -x -C "$trees/$sha.tmp"
        mv "$trees/$sha.tmp" "$trees/$sha"
    fi
    echo "$trees/$sha"
}
a=$(resolve "$1") || exit 1
b=$(resolve "$2") || exit 1
mkdir -p "$out"
printf 'a\t%s\t%s\nb\t%s\t%s\n' "$1" "$a" "$2" "$b" > "$out/sides.tsv"
cd "$repo"

take() {  # side tree scene label
    local inst=(--install "$2" --install "$repo/testbench/scene" --install "$here/probe-new")
    echo "=== $4 $3 $(date +%T)"
    local o id g
    o=$(SCENE=$3 ./a2test session start --res "$res" "${inst[@]}" --args '-nointro a2_borg01' 2>&1)
    id=$(echo "$o" | sed -n 's/^session \([0-9-]*\).*/\1/p' | head -1)
    if [ -z "$id" ]; then echo "$o" | tail -5; return; fi
    g=~/.cache/a2test/sessions/$id/game
    for _ in $(seq 1 60); do grep -q "scene ready" "$g/Scene.log" 2>/dev/null && break; sleep 3; done
    sleep "$settle"
    # the log is in the prefix's LocalAppData (13.50.0), or beside the exe in older builds
    rl=$(ls ~/.cache/a2test/sessions/$id/prefix/pfx/drive_c/users/*/AppData/Local/armada2-refit/logs/Render3D.log "$g/Render3D.log" 2>/dev/null | head -1)
    cp "$rl" "$out/$4-$3.log" 2>/dev/null || echo "no Render3D.log"
    ./a2test drive --session "$id" shot "$4-$3" >/dev/null 2>&1
    ./a2test session stop "$id" >/dev/null 2>&1
    echo "    session $id"
    sleep 3
}

for scene in ${SCENES:-showcase match nebula}; do
    for r in $(seq 1 "$runs"); do
        sfx=""; [ "$runs" -gt 1 ] && sfx=".$r"
        if [ $((r % 2)) -eq 1 ]; then order="a b"; else order="b a"; fi
        for side in $order; do
            if [ "$side" = a ]; then tree=$a; else tree=$b; fi
            take "$side" "$tree" "$scene" "$side$sfx"
        done
    done
done
python3 "$here/summary.py" "$out"
