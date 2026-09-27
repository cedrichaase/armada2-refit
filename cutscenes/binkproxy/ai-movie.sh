#!/usr/bin/env bash
# The PAID stages of a replacement movie, through belt (inference.sh).
#
#   cutscenes/binkproxy/ai-movie.sh <Name> upscale [--dry-run] [--force]
#   cutscenes/binkproxy/ai-movie.sh <Name> interp  [--dry-run] [--force]
#
# upscale  cuts movie.conf's `pieces` from the stock .bik (read from the game
#          directory, never written) and runs the upscaler on each:
#          ai/upscale/p<N>.mp4.
# interp   cuts the same pieces from src/upscaled.mkv (build-movie.sh stage 1) and
#          runs AI frame interpolation on each: ai/interp/p<N>.mp4.  Only used when
#          movie.conf says interp=apollo; build-movie.sh then joins these instead
#          of running minterpolate.
#
# Everything this writes cost money, so it never overwrites: an existing piece is
# skipped, and --force renames it aside (p<N>.<timestamp>.mp4) before replacing it.
# --dry-run cuts the pieces and prints belt's cost estimate, and runs nothing.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOVIES="$(cd "$here/.." && pwd)/movies"        # recipes: <Name>/movie.conf
. "$(cd "$here/../.." && pwd)/a2env.sh"; GAME="$A2_GAME"
MDATA="$A2_DATA/movies"                       # layers and builds: <Name>/ai, src, out
BRIA=bria/video-increase-resolution
TOPAZ_INTERP=topaz/frame-interpolation

name="${1:?usage: ai-movie.sh <Name> upscale|interp [--dry-run] [--force]}"
stage="${2:?usage: ai-movie.sh <Name> upscale|interp [--dry-run] [--force]}"
shift 2
dry=0; force=0
for a in "$@"; do
    case "$a" in
        --dry-run) dry=1 ;;
        --force)   force=1 ;;
        *) echo "unknown option: $a" >&2; exit 2 ;;
    esac
done

dir="$MDATA/$name"
conf="$MOVIES/$name/movie.conf"
[ -f "$conf" ] || { echo "no $conf" >&2; exit 1; }
conf_get() {
    local v
    v="$(grep -E "^$1=" "$conf" | tail -1 | cut -d= -f2- | sed 's/[[:space:]]*#.*$//; s/[[:space:]]*$//' || true)"
    printf '%s' "${v:-${2:-}}"
}
frames() { ffprobe -v error -count_frames -select_streams v -show_entries stream=nb_read_frames -of csv=p=0 "$1"; }

bik="$GAME/$(conf_get bik)"
[ -f "$bik" ] || { echo "stock movie not found: $bik" >&2; exit 1; }
read -ra pieces <<< "$(conf_get pieces)"
srcfps="$(ffprobe -v error -select_streams v -show_entries stream=r_frame_rate -of csv=p=0 "$bik")"
fps="$(conf_get fps 30)"
scale=$(awk -v a="$fps" -v b="$srcfps" 'BEGIN { split(b, r, "/"); q = r[2] ? r[1] / r[2] : r[1]; printf "%d", a / q + 0.5 }')

case "$stage" in
    upscale)
        from="$bik"; out="$dir/ai/upscale"
        [ "$(conf_get upscale)" = bria ] || { echo "only upscale=bria is implemented" >&2; exit 1; }
        factor="$(conf_get upscale_factor 2)" ;;
    interp)
        from="$dir/src/upscaled.mkv"; out="$dir/ai/interp"
        [ -f "$from" ] || { echo "no $from -- run build-movie.sh $name first (stage 1 is free)" >&2; exit 1; }
        model="$(conf_get apollo_model apollo)" ;;
    *) echo "stage must be upscale or interp" >&2; exit 2 ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
[ "$dry" = 1 ] || mkdir -p "$out"

run_belt() {   # run_belt <app> <input.json> -> prints the output video URL
    local log="$work/run.log"
    belt app run "$1" --input "$2" > "$log" 2>&1 || true
    if ! grep -q 'Completed' "$log"; then cat "$log" >&2; return 1; fi
    grep -o '"video": "[^"]*' "$log" | cut -d'"' -f4
}

for i in "${!pieces[@]}"; do
    s=${pieces[$i]%-*}; e=${pieces[$i]#*-}
    dst="$out/p$i.mp4"
    if [ -e "$dst" ] && [ "$force" = 0 ]; then echo "p$i: kept (exists; --force to redo)"; continue; fi

    piece="$work/$name-$stage-p$i-$$.mp4"
    ffmpeg -v error -y -i "$from" -an \
        -vf "trim=start_frame=$s:end_frame=$e,setpts=PTS-STARTPTS" \
        -c:v libx264 -crf 8 -preset slow -pix_fmt yuv420p -r "$srcfps" "$piece"
    secs=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$piece")

    if [ "$stage" = upscale ]; then
        awk -v d="$secs" 'BEGIN { exit !(d <= 30.0) }' \
            || { echo "p$i is ${secs}s; bria takes at most 30 s -- split it in movie.conf" >&2; exit 1; }
        echo "p$i: frames $s-$e (${secs}s), bria x$factor, ~\$$(awk -v d="$secs" 'BEGIN { printf "%.2f", d * 0.02 }')"
        [ "$dry" = 1 ] && continue
        belt file upload "$piece" >/dev/null
        url="$(belt file list --json | python3 -c "
import json, sys
for f in json.load(sys.stdin)['items']:
    if f['filename'] == '$(basename "$piece")': print(f['uri']); break")"
        [ -n "$url" ] || { echo "p$i: upload not found in belt file list" >&2; exit 1; }
        printf '{"video_url":"%s","desired_increase":%s,"preserve_audio":false}' "$url" "$factor" > "$work/in.json"
        result="$(run_belt "$BRIA" "$work/in.json")"
        want=$(( e - s ))
    else
        printf '{"video":"%s","model":"%s","target_fps":%s,"slowmo":1}' "$piece" "$model" "$fps" > "$work/in.json"
        echo "p$i: frames $s-$e (${secs}s), $TOPAZ_INTERP $model -> $fps fps"
        if [ "$dry" = 1 ]; then belt app estimate "$TOPAZ_INTERP" --input "$work/in.json" 2>&1 | grep -i estimated || true; continue; fi
        result="$(run_belt "$TOPAZ_INTERP" "$work/in.json")"
        want=$(( (e - s) * scale ))
    fi

    curl -sfL -o "$work/got.mp4" "$result"
    got=$(frames "$work/got.mp4")
    # Interpolators return scale*n - (scale-1) frames: nothing after the last real one.
    if [ "$got" -ne "$want" ] && [ "$got" -ne $(( want - scale + 1 )) ]; then
        echo "p$i: got $got frames, expected $want -- kept as $out/p$i.suspect.mp4" >&2
        mv "$work/got.mp4" "$out/p$i.suspect.mp4"
        exit 1
    fi
    [ -e "$dst" ] && mv "$dst" "$out/p$i.$(date +%Y%m%d-%H%M%S).mp4"
    mv "$work/got.mp4" "$dst"
    echo "p$i: $got frames -> $dst"
done
