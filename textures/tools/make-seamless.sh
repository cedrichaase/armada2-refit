#!/usr/bin/env bash
# Make a texture seamlessly tileable.
#   ./make-seamless.sh <in> <out> [feather]
#
# Rolling by exactly half puts formerly-adjacent pixels at the left and right borders,
# so the OUTER boundary becomes seamless for free and the discontinuity moves to the
# centre cross. The cross is then healed by blending in a quarter-rolled copy of the
# same texture through a smooth mask -- plausible content rather than a smudge.
#
# The mask is built analytically with -fx rather than from drawn rectangles: a drawn
# rectangle plus -blur leaves hard corners where the two bands cross and a visible step
# wherever the border clamp begins, and both show up plainly on a dark texture.
set -euo pipefail
in=${1:?in}; out=${2:?out}; F=${3:-56}; MODE=${MODE:-blur}
W=$(magick "$in" -format '%w' info:); H=$(magick "$in" -format '%h' info:)
w=$(mktemp -d); trap 'rm -rf "$w"' EXIT
M=$((F*3))   # border margin over which the bands taper away to nothing

magick "$in" -roll +$((W/2))+$((H/2)) "$w/rolled.png"
# Two ways to heal the centre cross:
#   patch  blend in a quarter-rolled copy. Invisible on homogeneous fractal texture,
#          but on content with large-scale structure (a bright plume, a dark void) the
#          patched region reads as a soft-edged rectangle of the wrong thing. Widening
#          the feather makes it bigger, not subtler.
#   blur   (default) blend in a heavily blurred copy of the image itself over a NARROW
#          band. The seam becomes a soft gradient rather than a hard edge, and because
#          the content is the image's own local average it never clashes.
# Only the centre cross is at stake either way -- the outer boundary, which is what cube
# faces actually join along, is already exact from the roll.
if [ "$MODE" = patch ]; then
  magick "$w/rolled.png" -roll +$((W/4))+$((H/4)) "$w/patch.png"
else
  magick "$w/rolled.png" -blur 0x$((F/2)) "$w/patch.png"
fi

# raised-cosine bump on each centre line, combined with max(), then tapered to zero
# near all four borders by a second raised cosine so the healed area never reaches them
magick -size ${W}x${H} xc: -colorspace Gray -fx "
  bx = abs(i-w/2) < $F ? 0.5*(1+cos(pi*(i-w/2)/$F)) : 0;
  by = abs(j-h/2) < $F ? 0.5*(1+cos(pi*(j-h/2)/$F)) : 0;
  band = max(bx,by);
  ex = min(i, w-1-i); ey = min(j, h-1-j); em = min(ex,ey);
  edge = em >= $M ? 1 : 0.5*(1-cos(pi*em/$M));
  band*edge" "$w/mask.png"

magick "$w/rolled.png" "$w/patch.png" "$w/mask.png" -compose Over -composite "$out"
