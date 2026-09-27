#!/usr/bin/env bash
# Procedurally generate a nebula tile -- fallback for when no image model is reachable.
# For abstract interstellar gas it holds up, and unlike a diffusion model the histogram
# is controllable, which is the part the build scripts actually care about.
#
#   ./gen-nebula.sh <seed> <out.png> [size] [r] [g] [b]
#
# r/g/b are weights applied to the luminance, so hue is exact rather than eyeballed.
# MBG02 wants 0.27 0.23 1.00 (measured stock: R8 G7 B30).
set -euo pipefail
cd "$(dirname "$0")"
seed=${1:?seed}; out=${2:?out}; S=${3:-1024}
WR=${4:-0.27}; WG=${5:-0.23}; WB=${6:-1.00}
w=$(mktemp -d); trap 'rm -rf "$w"' EXIT

# Four octaves of fractal plasma, coarse to fine. These weights were tuned against
# stock and three alternatives were tried and rejected -- see README "Procedural
# fallback" before changing them. Each is pre-dimmed by its weight so a
# plain mean across the stack is a weighted sum -- the standard 1/f spectrum that makes
# noise read as cloud rather than static.
mk () { magick -seed $2 -size $3x$3 plasma:fractal -colorspace Gray \
          -resize ${S}x${S} -blur 0x$4 -evaluate multiply $5 "$w/$1.png"; }
mk o3 $((seed*7+1)) $((S/24)) 18 0.50
mk o2 $((seed*7+2)) $((S/10))  9 0.28
mk o1 $((seed*7+3)) $((S/4))   3 0.15
mk o0 $((seed*7+4)) $((S/2))   1 0.07
magick "$w/o3.png" "$w/o2.png" "$w/o1.png" "$w/o0.png" \
    -evaluate-sequence Add -auto-level "$w/base.png"

# Crush toward black. Without this the field is a uniform mid-grey fog: gamma < 1 is
# x^(1/g), so 0.40 squares-and-a-bit, leaving gas standing out of real voids.
magick "$w/base.png" -gamma 0.40 -auto-level "$w/body.png"

# Filaments: an independent field, thresholded hard to thin threads, screened on top.
magick -seed $((seed*7+5)) -size $((S/3))x$((S/3)) plasma:fractal -colorspace Gray \
    -resize ${S}x${S} -blur 0x2 -sigmoidal-contrast 26x70% -gamma 0.55 \
    -evaluate multiply 0.55 "$w/fil.png"
magick "$w/body.png" "$w/fil.png" -compose Screen -composite "$w/lum.png"

# Final shaping: lift the knots without flattening the voids.
magick "$w/lum.png" -sigmoidal-contrast 4x40% -auto-level "$w/shaped.png"

# Colourise by channel weight.
magick "$w/shaped.png" \
    \( -clone 0 -evaluate multiply $WR \) \
    \( -clone 0 -evaluate multiply $WG \) \
    \( -clone 0 -evaluate multiply $WB \) \
    -delete 0 -combine -colorspace sRGB "$out"
