#!/bin/bash
# The promo's YouTube thumbnail (concept A): one Enterprise-E split by an amber
# diagonal, stock ("2001") on the left, refit ("NOW") on the right.
#
#   films/promo/thumbnail.sh [BG ...]        BG: planet (default), nebula
#
# Reads the 4K stills $A2_DATA/films/promo/thumbnail/final/still-<BG>-{stock,refit}.png,
# filmed on the bench from films/promo/thumbnail.ini (the nebula variant swaps the
# planet for mnebula8 at 2400,-100,1750), the same camera in both:
#   orbit ente 305 24 170, after a far orbit (400) and back, shot live.
# Writes final/thumbnail.png (planet) or final/thumbnail-<BG>.png, 1280x720, and a
# 168x94 check beside each (-small). Nothing is read from or written to the checkout.
set -euo pipefail
. "$(dirname "$0")/../../a2env.sh"
D=$A2_DATA/films/promo/thumbnail/final
W=$D/work
mkdir -p "$W"

BLK=$(fc-match -f '%{file}' 'Noto Sans:width=75:weight=black')   # Noto Sans Condensed Black
BOLD=$(fc-match -f '%{file}' 'Noto Sans:weight=bold')
AMB='#F2A33A'
CROP=2560x1440+760+380        # 1.5x into the 4K still: the ship at ~60% of the width
GRADE=(-level 1%,90%,1.05)    # the same grade on stock and refit; blacks stay black
# The divider: through the neck, so the nacelles are stock and the saucer, with its
# NCC-1701-E, is refit. (x at the top, x at the bottom) of a 1280x720 frame.
X0=630; X1=440

txt() {  # txt <in> <out> <font> <pt> <fill> <gravity> <+x+y> <kerning> <text>
  magick "$1" \( -size 1280x720 xc:none -font "$3" -pointsize "$4" -kerning "$8" -gravity "$6" \
      -fill black -annotate "$7" "$9" -blur 0x10 -channel A -evaluate multiply 1.7 +channel \) -composite \
    -font "$3" -pointsize "$4" -kerning "$8" -gravity "$6" -fill "$5" -annotate "$7" "$9" "$2"
}

for bg in "${@:-planet}"; do
  out=$D/thumbnail.png; [ "$bg" = planet ] || out=$D/thumbnail-$bg.png
  for v in stock refit; do
    magick "$D/still-$bg-$v.png" -crop "$CROP" +repage -filter Lanczos -resize 1280x720! \
      "${GRADE[@]}" "PNG24:$W/$bg-$v.png"
  done
  # stock left of the divider
  magick -size 1280x720 xc:black -fill white -draw "polygon 0,0 $X0,0 $X1,720 0,720" "$W/mask.png"
  magick "$W/$bg-refit.png" "$W/$bg-stock.png" "$W/mask.png" -composite "$W/$bg-split.png"
  # a dark band under the title, the same on both halves, so the type reads over the planet
  magick "$W/$bg-split.png" \
    \( -size 1280x300 gradient:none-black -channel A -evaluate pow 1.6 -evaluate multiply 0.8 +channel \
       -background none -gravity south -extent 1280x720 \) -composite \
    \( -size 1280x720 xc:none -stroke "$AMB" -strokewidth 16 -draw "line $X0,0 $X1,720" -blur 0x9 \) -composite \
    -stroke "$AMB" -strokewidth 5 -draw "line $X0,0 $X1,720" "$W/$bg-base.png"
  txt "$W/$bg-base.png" "$W/$bg-t1.png" "$BLK" 150 '#d9d9d9' NorthWest +40+12 2 '2001'
  txt "$W/$bg-t1.png" "$W/$bg-t2.png" "$BLK" 150 "$AMB" NorthEast +44+12 2 'NOW'
  txt "$W/$bg-t2.png" "$W/$bg-t3.png" "$BLK" 72 white SouthWest +40+64 1 'ARMADA II'
  txt "$W/$bg-t3.png" "$W/$bg-t4.png" "$BOLD" 32 "$AMB" SouthWest +44+26 24 'REFIT'
  magick "$W/$bg-t4.png" -strip -define png:compression-level=9 "PNG24:$out"
  magick "$out" -filter Lanczos -resize 168x94! "PNG24:${out%.png}-small.png"
  echo "$out ($(stat -c %s "$out") bytes)"
done
