#!/usr/bin/env bash
# render3d against the plugins it replaces, on the bench (render3d/PERF.md).
#
#   render3d/perf/compare.sh OLD_CHECKOUT [OUT_DIR]
#
# OLD_CHECKOUT is a checkout from before the per-part plugins were removed from the
# repository (or a copy of one, e.g. an archive of that commit) whose
# ./install puts in the stock plugins; its install builds into it, so pass a copy you
# do not mind being built in. For each scene (SCENES, default "showcase match nebula")
# and each configuration (CFGS, default "old new"), one session at a time: start it,
# wait for the scene, wait 80 s, keep Render3D.log as OUT_DIR/<cfg>-<scene>.log, stop.
# "old" is OLD_CHECKOUT's install with this checkout's Render3D.asi in measure-only mode
# on top; "new" is this checkout's install with Timing=1. Then summary.py OUT_DIR.
set -u
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
old=${1:?usage: compare.sh OLD_CHECKOUT [OUT_DIR]}
old=$(cd "$old" && pwd)
out=${2:-${A2_DATA:-$HOME/.local/share/armada2-refit}/perf/$(date +%Y%m%d-%H%M%S)}
res=${RES:-1920x1080}
mkdir -p "$out"
[ -f "$repo/render3d/build/Render3D.asi" ] || "$repo/render3d/build.sh"
cd "$repo"
for scene in ${SCENES:-showcase match nebula}; do
  for cfg in ${CFGS:-old new}; do
    if [ "$cfg" = old ]; then
      inst=(--install "$old" --install "$repo/testbench/scene" --install "$here/probe-old")
    else
      inst=(--install "$repo" --install "$repo/testbench/scene" --install "$here/probe-new")
    fi
    echo "=== $cfg $scene $(date +%T)"
    o=$(SCENE=$scene ./a2test session start --res "$res" "${inst[@]}" --args '-nointro a2_borg01' 2>&1)
    id=$(echo "$o" | sed -n 's/^session \([0-9-]*\).*/\1/p' | head -1)
    if [ -z "$id" ]; then echo "$o" | tail -5; continue; fi
    g=~/.cache/a2test/sessions/$id/game
    for _ in $(seq 1 60); do grep -q "scene ready" "$g/Scene.log" 2>/dev/null && break; sleep 3; done
    sleep 80
    # the log is in the prefix's LocalAppData (13.50.0), or beside the exe in older builds
    rl=$(ls ~/.cache/a2test/sessions/$id/prefix/pfx/drive_c/users/*/AppData/Local/armada2-refit/logs/Render3D.log "$g/Render3D.log" 2>/dev/null | head -1)
    cp "$rl" "$out/$cfg-$scene.log" 2>/dev/null || echo "no Render3D.log"
    ./a2test drive --session "$id" shot "$cfg-$scene" >/dev/null 2>&1
    ./a2test session stop "$id" >/dev/null 2>&1
    echo "    session $id"
    sleep 3
  done
done
python3 "$here/summary.py" "$out"
