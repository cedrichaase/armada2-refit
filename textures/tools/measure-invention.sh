#!/usr/bin/env bash
# How much does the generative layer INVENT, and how much does it ERASE, judged at the
# size the object is actually drawn?
#
#   textures/tools/measure-invention.sh <target> [--at N[,N...]] [texture...]
#     --at     on-screen widths to judge at. Default 81,231,377,1055 -- a SWEEP, not one
#              figure, because how big a thing draws depends on the zoom and the player
#              controls that. All four are MEASURED, off four 3440x1440 captures of the
#              same scene at different zooms, taking the Galaxy-class as the yardstick:
#
#                  81px   zoomed all the way out
#                 167px   the zoom the player calls their usual maximum
#                 231px   a moderate zoom
#                1055px   zoomed all the way in (the starbase beside it clears 1377px
#                         and fills the viewport height)
#
#              So the practical range tops out near 380px, where a 1024 texture is
#              minified at least 2.7x and invented detail is comfortably sub-pixel. The
#              1055 rung is the honest worst case: there the texture approaches 1:1 and
#              the sub-pixel argument stops holding entirely.
#
# An earlier revision judged everything at a single 340px and called that "the size the
# object is drawn". It is not; it is one zoom level, and the player moves it over a
# 13x range. A blend is only safe if it is safe at the CLOSE end, and the argument for
# a high blend -- that invented detail lands under a pixel -- is precisely the argument
# that stops holding when the player zooms in. README.md's 340 figure carries the same
# flaw and is corrected there.
#
# Three columns, because two of them are not enough:
#   invented  = the blend carries a saturated marking where plain Lanczos does not
#   erased    = plain Lanczos carries one where the blend does not
#   displaced = invented, but only counting pixels more than 2px from ANY saturated
#               stock pixel -- i.e. saturation that appeared somewhere NEW
#
# THE METRIC HAS FOUR KNOWN FALSE POSITIVES. It assumes markings are small and localised
# on a neutral field, and over-reports wherever that does not hold. Every one of these was
# found by rendering a texture the numbers had condemned:
#
#   1. INTENSIFICATION -- the model saturating a marking stock already has. This is what
#      `displaced` exists to subtract.
#   2. LARGE FLAT COLOUR -- a saturated region that grows by more than the 2px dilation
#      still scores as displaced. Fsensor is gold chevrons and a logo on white; it reads
#      5.2% displaced and is simply crisper.
#   3. ADDITIVE GLOW ON BLACK -- Lanczos spreads each blob into a halo and the model puts
#      the light back in the blob, which counts as ERASURE of the halo. Ftransport reads
#      8.8-11.6% erased and is correct; verify.py's per-channel means are the real check
#      there, because for an additive texture the mean IS the light contributed.
#   4. NO SATURATED MARKINGS AT ALL -- on organic or near-greyscale art the numbers go
#      flat across every blend and say nothing. 8472_passive2 reads 0.037-0.040% invented
#      from blend 20 to 100 while visibly gaining speckle. Silence is not a pass.
#
# So: this tool RANKS textures for attention. It does not decide them. Anything it flags
# gets looked at, and cases 3 and 4 mean a clean number is not proof either.
#
# `invented` alone has a large, benign false positive: the model INTENSIFYING a marking
# that stock already has. A desaturated red panel sitting between the two thresholds
# crosses the upper one when the model saturates it, and scores as invention although
# it is the right marking in the right place. The gap between the two columns is the
# size of that effect, and it is usually most of the number: fsrepairb reads 0.578%
# invented and 0.042% displaced, fcruise1 0.170% and 0.010%. Judge on `displaced`; read
# `invented` as "how much the model pushed saturation about".
#
# Saturation, not luminance. The co-registration test -- mean luminance under lit texels
# against unlit ones -- was tried first and does not separate these textures at all: the
# Sovereign, the known positive, scores 1.02. Hull markings are small and sit on light
# grey, so they barely move a mean. What they do move is chroma.
#
# THE SIZE TRAP, which cost a wrong conclusion here: `--mp 4` on a 256px source returns
# 2048, an 8x lift, not the 4x the megapixel figure suggests. Compositing that against a
# Lanczos layer built at 1024 does not fail -- ImageMagick quietly composites the 2048
# image onto the 1024 canvas at the origin, so the comparison is the TOP-LEFT QUARTER of
# one plate against the whole of the other. It reads as catastrophic erasure: markings
# "vanish" because they are not in the crop. The Lanczos layer here is therefore built
# at the ai/ plate's own size, read off the file, never at an assumed one.
set -uo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd); cd "$ROOT"
. "$ROOT/lib/common.sh"; . "$ROOT/../a2env.sh"
t=${1:?usage: measure-invention.sh <target> [--at N] [texture...]}; shift
AT=81,231,377,1055
while [ $# -gt 0 ]; do case $1 in --at) AT=$2; shift 2;; *) break;; esac; done
IFS=, read -r -a SIZES <<< "$AT"
dir="$A2_DATA/textures/$t"; tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
# Q16 ImageMagick: -threshold takes a PERCENTAGE. "60" would mean 60/65535.
hi=$(python3 -c 'print(round(100*60/255,4))')   # "saturated" = 60/255
lo=$(python3 -c 'print(round(100*25/255,4))')   # "not"       = 25/255
if [ $# -gt 0 ]; then list=("$@"); else mapfile -t list < <(stock_bases "$dir/stock" | sed 's/\.[^.]*$//'); fi
blend=$(grep -E '^blend=' "targets/$t/target.conf" 2>/dev/null | head -1 | cut -d= -f2- | tr -d ' ' || true)
[ -n "$blend" ] || blend=35
printf 'judged at %s px on screen; target blend=%s\n' "${SIZES[*]}" "$blend"
printf '%-20s %6s %6s %9s %10s %11s\n' texture at blend erased invented displaced
for B in "${list[@]}"; do
  f=$(cd "$dir/stock" && tex_path "$B"); [ -n "$f" ] || continue
  [ -s "$dir/ai/$B.png" ] || { printf '%-20s  (no ai/ layer)\n' "$B"; continue; }
  read -r W H < <(magick "$dir/ai/$B.png" -format '%w %h\n' info:)
  magick "$dir/stock/$f" -alpha off -filter Lanczos -resize "${W}x${H}!" "PNG24:$tmp/lz.png"
  # the fixed ladder plus the target's own setting, without listing it twice when it
  # happens to be one of the rungs
  mapfile -t steps < <(printf '%s\n' 20 35 50 70 100 "$blend" | sort -n -u)
  for at in "${SIZES[@]}"; do
   magick "$tmp/lz.png" -filter Lanczos -resize "${at}x${at}" -colorspace HSL -channel S -separate "PNG24:$tmp/ls.png"
   for n in "${steps[@]}"; do
    magick "$tmp/lz.png" "$dir/ai/$B.png" -define compose:args="$n" -compose blend -composite "PNG24:$tmp/b.png"
    magick "$tmp/b.png" -filter Lanczos -resize "${at}x${at}" -colorspace HSL -channel S -separate "PNG24:$tmp/bs.png"
    er=$(magick \( "$tmp/ls.png" -threshold "$hi%" \) \( "$tmp/bs.png" -threshold "$lo%" -negate \) \
         -compose Multiply -composite -format '%[fx:100*mean]' info:)
    iv=$(magick \( "$tmp/bs.png" -threshold "$hi%" \) \( "$tmp/ls.png" -threshold "$lo%" -negate \) \
         -compose Multiply -composite -format '%[fx:100*mean]' info:)
    # Dilate stock's saturated set by 2px before subtracting, so intensifying an
    # existing marking does not read as inventing one.
    magick "$tmp/ls.png" -threshold "$lo%" -morphology Dilate Disk:2 -negate "PNG24:$tmp/new.png"
    dp=$(magick \( "$tmp/bs.png" -threshold "$hi%" \) "$tmp/new.png" \
         -compose Multiply -composite -format '%[fx:100*mean]' info:)
    mark=""; [ "$n" = "$blend" ] && mark="  <- target"
    printf '%-20s %6s %6s %8.3f%% %9.3f%% %10.3f%%%s\n' "$B" "$at" "$n" "$er" "$iv" "$dp" "$mark"
   done
  done
done
