#!/usr/bin/env bash
# Capture and numerically compare two in-game frames, so "did that setting do anything"
# gets an answer instead of an impression.
#
#   ab-shot.sh grab <label>          capture the screen now -> archive/ab/<label>.png
#   ab-shot.sh diff <a> <b>          compare two labels: RMSE, peak, and a diff image
#   ab-shot.sh diff <a> <b> W H X Y  compare one region only (see below)
#   ab-shot.sh list                  show what has been captured
#
# WHY A REGION USUALLY MATTERS MORE THAN THE WHOLE FRAME
#
# A whole-frame diff of this game is dominated by things that changed for reasons that
# have nothing to do with the renderer setting: ships drift, sprites animate, the
# starfield twinkles.  That noise can easily exceed the effect being measured and will
# make any setting look like it "did something".
#
# So aim at a region holding a LARGE, STATIC surface seen at a SHALLOW angle -- a
# starbase, a planet, the skybox near a screen edge.  That is where anisotropic
# filtering can act at all.  Do NOT aim at nebula or resource clouds: those are
# camera-facing billboards, they are never oblique, and AF cannot touch them by
# construction.  (A mip LOD bias CAN, which is why stage 2 tends to be the visible one
# in this game and stage 1 the subtle one.)
#
# REPEATABILITY IS ON YOU, AND IT IS THE WEAK POINT
#
# There is no fixed-camera debug mode.  Load the SAME save, do not move the mouse or
# the camera, and grab immediately.  Between the two grabs change exactly ONE thing.
# If the two frames disagree about where the ships are, the number is meaningless --
# check the diff image before trusting the number.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$here/archive/ab"
OUTPUT="${A2_OUTPUT:-DP-3}"

usage() { sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'; exit "${1:-0}"; }

cmd="${1:-}"; shift || true

case "$cmd" in
grab)
    label="${1:-}"; [ -n "$label" ] || usage 2
    mkdir -p "$OUT"
    command -v grim >/dev/null || { echo "grim not installed" >&2; exit 1; }
    grim -o "$OUTPUT" "$OUT/$label.png"
    # Report the mean so an all-black grab (wrong output, or the game not focused)
    # is obvious immediately rather than after the comparison.
    read -r w h mean < <(magick "$OUT/$label.png" \
        -format '%w %h %[fx:mean*255]\n' info:)
    echo "captured $OUT/$label.png  ${w}x${h}  mean ${mean}"
    awk -v m="$mean" 'BEGIN{ if (m+0 < 2) print "  WARNING: frame is almost black -- wrong output, or the game was not on screen" }'
    ;;

diff)
    a="${1:-}"; b="${2:-}"; [ -n "$a" ] && [ -n "$b" ] || usage 2
    A="$OUT/$a.png"; B="$OUT/$b.png"
    for f in "$A" "$B"; do [ -f "$f" ] || { echo "no such capture: $f" >&2; exit 1; }; done

    crop=""
    if [ $# -ge 6 ]; then
        crop="${3}x${4}+${5}+${6}"
        echo "region: $crop"
    fi

    tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
    if [ -n "$crop" ]; then
        magick "$A" -crop "$crop" +repage "PNG24:$tmp/a.png"
        magick "$B" -crop "$crop" +repage "PNG24:$tmp/b.png"
    else
        magick "$A" "PNG24:$tmp/a.png"
        magick "$B" "PNG24:$tmp/b.png"
    fi

    sa=$(magick "$tmp/a.png" -format '%wx%h\n' info:)
    sb=$(magick "$tmp/b.png" -format '%wx%h\n' info:)
    [ "$sa" = "$sb" ] || { echo "size mismatch: $sa vs $sb" >&2; exit 1; }

    # compare writes its metric to stderr; keep it off the pipeline so pipefail and a
    # non-zero "images differ" exit status cannot abort the script.
    rmse=$(magick compare -metric RMSE "$tmp/a.png" "$tmp/b.png" "$tmp/d.png" 2>&1 >/dev/null || true)
    peak=$(magick "$tmp/a.png" "$tmp/b.png" -compose difference -composite \
           -colorspace Gray -format '%[fx:maxima*255]\n' info:)
    mad=$(magick "$tmp/a.png" "$tmp/b.png" -compose difference -composite \
          -colorspace Gray -format '%[fx:mean*255]\n' info:)

    # Name the diff after the region too: without it a whole-frame diff and a region
    # diff of the same pair write to one filename, and the image silently stops
    # meaning what the last line of output said it meant.
    dname="diff-$a-vs-$b${crop:+-$crop}.png"
    cp "$tmp/d.png" "$OUT/$dname"
    printf 'RMSE           %s\n' "$rmse"
    printf 'mean abs diff  %.3f / 255\n' "$mad"
    printf 'peak diff      %.0f / 255\n' "$peak"
    echo   "diff image     $OUT/$dname"
    echo
    awk -v m="$mad" 'BEGIN{
        if (m+0 < 0.05)
            print "Verdict: indistinguishable. The setting changed nothing measurable here.";
        else if (m+0 < 0.5)
            print "Verdict: a real but small change. Look at the diff image to see WHERE --";
        else
            print "Verdict: a large change. Check the diff image that it is the surface you";
    }'
    awk -v m="$mad" 'BEGIN{
        if (m+0 >= 0.05 && m+0 < 0.5) print "         if it is scattered over moving objects, it is drift, not the setting.";
        else if (m+0 >= 0.5) print "         aimed at and not the camera having moved between grabs.";
    }'
    ;;

list)
    [ -d "$OUT" ] || { echo "nothing captured yet"; exit 0; }
    for f in "$OUT"/*.png; do
        [ -e "$f" ] || continue
        printf '  %-34s %s\n' "$(basename "$f")" "$(magick "$f" -format '%wx%h' info:)"
    done
    ;;

*) usage 2 ;;
esac
