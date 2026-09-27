#!/usr/bin/env bash
# Build a replacement movie from its paid layers -- offline, free, repeatable.
#
#   cutscenes/binkproxy/build-movie.sh <Name>...      e.g.  build-movie.sh Intro
#
# A movie is laid out like a texture target -- a recipe in the repository, everything
# else in $A2_DATA/movies/<Name>/ (a2env.sh):
#
#   cutscenes/movies/<Name>/movie.conf   committed -- every decision the build needs
#   ai/upscale/    PAID: raw upscaler output, one file per piece      (ai-movie.sh)
#   ai/interp/     PAID: raw AI interpolation, when interp=apollo     (ai-movie.sh)
#   src/           derived: upscaled.mkv (joined, lossless, source fps)
#                           interp.mp4   (target fps; x264 crf 10)
#   out/           derived: <Name>.mp4 (AV1) + <Name>.wav -- what install.sh copies
#
# Everything under ai/ cost money and nothing here writes to it.  src/ and out/ can
# be deleted at any time and rebuilt from ai/ + movie.conf.  The stock .bik is read
# from the game directory and never written.
#
# Each stage is skipped when its output exists and a stamp of its inputs (the conf
# keys it reads, and the size and mtime of every input file) is unchanged, because
# minterpolate over the whole intro takes ~25 minutes.  --force rebuilds everything.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MOVIES="$(cd "$here/.." && pwd)/movies"        # recipes: <Name>/movie.conf
. "$(cd "$here/../.." && pwd)/a2env.sh"; GAME="$A2_GAME"
MDATA="$A2_DATA/movies"                       # layers and builds: <Name>/ai, src, out

force=0
names=()
for a in "$@"; do
    case "$a" in
        --force) force=1 ;;
        -*) echo "unknown option: $a" >&2; exit 2 ;;
        *) names+=("$a") ;;
    esac
done
[ ${#names[@]} -gt 0 ] || { echo "usage: build-movie.sh [--force] <Name>..." >&2; exit 2; }

conf_get() {   # conf_get <file> <key> [default]
    local v
    v="$(grep -E "^$2=" "$1" | tail -1 | cut -d= -f2- | sed 's/[[:space:]]*#.*$//; s/[[:space:]]*$//' || true)"
    printf '%s' "${v:-${3:-}}"
}

# Name, size, mtime -- not the path, so src/ and out/ stay fresh when moved.
files_stamp() { local f; for f in "$@"; do printf '%s %s\n' "${f##*/}" "$(stat -c '%s %Y' "$f")"; done; }

# stage_fresh <output> <stampfile> <stamp text>: 0 if the output can be kept
stage_fresh() {
    [ "$force" = 0 ] && [ -e "$1" ] && [ -e "$2" ] && [ "$(cat "$2")" = "$3" ]
}

frames() { ffprobe -v error -count_frames -select_streams v -show_entries stream=nb_read_frames -of csv=p=0 "$1"; }

# join_pieces <dir> <scale> <fps> <out> <piece ranges...> -- <joins...>
# Piece i covers source frames [s_i, e_i) at `scale` output frames per source frame.
# It contributes [prev join, next join); overlapping pieces meet at the join.  The
# trailing fps= normalises timestamps, which minterpolate reads -- the installed
# intro was joined with it, and leaving it out changes the interpolated frames.
join_pieces() {
    local dir="$1" scale="$2" rate="$3" out="$4"; shift 4
    local ranges=() joins=() inputs=() filt="" labels="" pad="" i n s e k0 k1
    # An interpolator returns scale*n - (scale-1) frames for n: nothing after the last
    # real one.  Repeat that frame into the missing slots, or every join would lose
    # them and the video would drift ahead of the audio.
    (( scale > 1 )) && pad="tpad=stop_mode=clone:stop=$(( scale - 1 )),"
    while [ "$1" != "--" ]; do ranges+=("$1"); shift; done; shift
    joins=("$@")
    n=${#ranges[@]}
    [ ${#joins[@]} -eq $((n - 1)) ] || { echo "movie.conf: $n pieces need $((n - 1)) joins" >&2; exit 1; }
    for ((i = 0; i < n; i++)); do
        [ -f "$dir/p$i.mp4" ] || { echo "missing $dir/p$i.mp4" >&2; exit 1; }
        inputs+=(-i "$dir/p$i.mp4")
        s=${ranges[$i]%-*}; e=${ranges[$i]#*-}
        k0=$(( i == 0 ? s : joins[i - 1] )); k1=$(( i == n - 1 ? e : joins[i] ))
        (( k0 >= s && k1 <= e && k0 < k1 )) || { echo "piece $i ($s-$e) does not cover $k0-$k1" >&2; exit 1; }
        filt+="[$i:v]${pad}trim=start_frame=$(( (k0 - s) * scale )):end_frame=$(( (k1 - s) * scale )),setpts=PTS-STARTPTS,setsar=1[v$i];"
        labels+="[v$i]"
    done
    ffmpeg -v error -y "${inputs[@]}" -filter_complex "${filt}${labels}concat=n=$n:v=1:a=0,fps=$rate[v]" \
        -map '[v]' -c:v ffv1 "$out"
}

build_one() {
    local name="$1" dir="$MDATA/$1" conf bik upscale pieces joins interp fps srcfps mi crf
    local up="$dir/src/upscaled.mkv" ip="$dir/src/interp.mp4" stamp expect got
    conf="$MOVIES/$name/movie.conf"
    [ -f "$conf" ] || { echo "no $conf" >&2; exit 1; }
    bik="$GAME/$(conf_get "$conf" bik)"
    [ -f "$bik" ] || { echo "stock movie not found: $bik" >&2; exit 1; }
    upscale="$(conf_get "$conf" upscale)"
    read -ra pieces <<< "$(conf_get "$conf" pieces)"
    read -ra joins  <<< "$(conf_get "$conf" joins)"
    interp="$(conf_get "$conf" interp minterpolate)"
    fps="$(conf_get "$conf" fps 30)"
    crf="$(conf_get "$conf" crf 28)"
    srcfps="$(ffprobe -v error -select_streams v -show_entries stream=r_frame_rate -of csv=p=0 "$bik")"
    mkdir -p "$dir/src" "$dir/out"
    echo "== $name  (upscale=$upscale, interp=$interp -> $fps fps, crf $crf)"

    # --- stage 1: join the upscaled pieces --------------------------------------------
    stamp="$(printf 'pieces=%s\njoins=%s\n' "${pieces[*]}" "${joins[*]}"; files_stamp "$dir"/ai/upscale/p*.mp4)"
    if stage_fresh "$up" "$dir/src/.upscaled.stamp" "$stamp"; then
        echo "   src/upscaled.mkv   up to date"
    else
        echo "   src/upscaled.mkv   joining ${#pieces[@]} pieces"
        join_pieces "$dir/ai/upscale" 1 "$srcfps" "$dir/src/.upscaled.tmp.mkv" "${pieces[@]}" -- "${joins[@]}"
        mv -f "$dir/src/.upscaled.tmp.mkv" "$up"
        printf '%s' "$stamp" > "$dir/src/.upscaled.stamp"
    fi
    expect=$(( ${pieces[-1]#*-} - ${pieces[0]%-*} ))
    got=$(frames "$up")
    [ "$got" = "$expect" ] || { echo "src/upscaled.mkv has $got frames, the pieces cover $expect" >&2; exit 1; }

    # --- stage 2: interpolate to $fps ---------------------------------------------------
    # The intermediate carries the stock audio only so that -shortest trims the video
    # to it, which is how the installed intro was made; out/ takes its WAV from the .bik.
    case "$interp" in
    minterpolate)
        mi="$(conf_get "$conf" minterpolate)"
        stamp="$(printf 'interp=minterpolate\nfps=%s\nminterpolate=%s\n' "$fps" "$mi"; files_stamp "$up" "$bik")"
        if stage_fresh "$ip" "$dir/src/.interp.stamp" "$stamp"; then
            echo "   src/interp.mp4     up to date"
        else
            echo "   src/interp.mp4     minterpolate (slow: ~25 min for the intro)"
            ffmpeg -v error -y -i "$up" -i "$bik" -map 0:v -map 1:a \
                -vf "minterpolate=fps=$fps:$mi" \
                -c:v libx264 -crf 10 -preset slow -pix_fmt yuv420p -c:a aac -b:a 192k -shortest \
                "$dir/src/.interp.tmp.mp4"
            mv -f "$dir/src/.interp.tmp.mp4" "$ip"
            printf '%s' "$stamp" > "$dir/src/.interp.stamp"
        fi
        ;;
    apollo)
        # Untested end to end: no paid ai/interp/ exists yet.  The join and the cut
        # repair are the same arithmetic as stage 1 at `scale` output frames per
        # source frame; see ai-movie.sh for how the pieces are cut.
        local scale cuts=() drop="" c
        scale=$(awk -v a="$fps" -v b="$srcfps" 'BEGIN { split(b, r, "/"); q = r[2] ? r[1] / r[2] : r[1]; printf "%d", a / q + 0.5 }')
        read -ra cuts <<< "$(conf_get "$conf" cuts)"
        stamp="$(printf 'interp=apollo\nfps=%s\ncuts=%s\n' "$fps" "${cuts[*]}"; files_stamp "$dir"/ai/interp/p*.mp4 "$bik")"
        if stage_fresh "$ip" "$dir/src/.interp.stamp" "$stamp"; then
            echo "   src/interp.mp4     up to date"
        else
            echo "   src/interp.mp4     joining ai/interp, repairing ${#cuts[@]} cuts"
            join_pieces "$dir/ai/interp" "$scale" "$fps" "$dir/src/.interp.join.mkv" "${pieces[@]}" -- "${joins[@]}"
            # The frame Apollo invents across a cut at source frame c is output frame
            # scale*c - 1 (for 2x; for higher factors, the scale-1 frames before scale*c).
            # Drop them and let fps= repeat the last real frame into the gap.
            for c in "${cuts[@]}"; do
                for ((k = 1; k < scale; k++)); do drop+="+eq(n\\,$(( c * scale - k )))"; done
            done
            ffmpeg -v error -y -i "$dir/src/.interp.join.mkv" -i "$bik" -map 0:v -map 1:a \
                -vf "select='not(0${drop})',fps=$fps" \
                -c:v libx264 -crf 10 -preset slow -pix_fmt yuv420p -c:a aac -b:a 192k -shortest \
                "$dir/src/.interp.tmp.mp4"
            rm -f "$dir/src/.interp.join.mkv"
            mv -f "$dir/src/.interp.tmp.mp4" "$ip"
            printf '%s' "$stamp" > "$dir/src/.interp.stamp"
        fi
        ;;
    *) echo "movie.conf: unknown interp=$interp" >&2; exit 1 ;;
    esac

    # --- stage 3: what the DLL plays ------------------------------------------------------
    stamp="$(printf 'crf=%s\n' "$crf"; files_stamp "$ip" "$bik")"
    if stage_fresh "$dir/out/$name.mp4" "$dir/out/.stamp" "$stamp" && [ -e "$dir/out/$name.wav" ]; then
        echo "   out/$name.mp4/.wav up to date"
    else
        echo "   out/$name.mp4/.wav encoding"
        bash "$here/make-movie.sh" "$ip" "$dir/out/$name" "$crf" --audio "$bik" >/dev/null
        printf '%s' "$stamp" > "$dir/out/.stamp"
    fi
    ls -l "$dir/out/$name.mp4" "$dir/out/$name.wav" | awk '{print "   " $5 "  " $NF}'
}

for n in "${names[@]}"; do build_one "$n"; done
