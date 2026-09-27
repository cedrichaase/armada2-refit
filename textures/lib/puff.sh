# Map-plane nebula puffs (Mnebula*, MFluidicNeb, mtachyonneb, Mlatinum).
#
# `source=` in target.conf picks between two very different jobs:
#   gen    (default) src/ is generated art of unknown framing -- trim it, square it,
#          inset it to FILL% and stretch out its noise floor.
#   stock  src/ is this atlas's own four quadrants, upscaled. Keep the composition
#          exactly; every positioning step above is skipped.
# 2x2 atlas of quadrants carved by Sprites/nebula.spr. Additive blending, so black is
# transparent and ANY non-black pixel at a quadrant edge renders as a glowing square.
# Engine-tinted, so sources are converted to greyscale unless the atlas keeps its own
# colour (Mnebula2 is the only stock one that does).
build_puff () {
  local name=$1 dir=$2 tmp=$3
  local size=${SIZE:-256} fill=${FILL:-92}
  local stock; stock=$(cd "$dir/stock" && tex_path "$name")
  [ -n "$stock" ] || die "$name: no stock/ copy"
  stock="$dir/stock/$stock"

  mapfile -t src < <(find "$dir/src" -maxdepth 1 -type f \
      \( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.webp' \) | sort)
  [ ${#src[@]} -gt 0 ] || { echo "SKIP  $name (no src)"; return 1; }
  # Repeat to fill four quadrants: tiling repetition is invisible in game, because
  # billboards overlap at many scales. Mnebula4 shipped with one image in all four.
  while [ ${#src[@]} -lt 4 ]; do src+=("${src[$(( ${#src[@]} % ${#src[@]} ))]}"); done

  local half; half=$(magick "$stock" -format '%[fx:int(w/2)]' info:)
  # Greyscale exists because the engine tints these sprites, so coloured generated art
  # would be tinted twice. It is pointless when source=stock: the stock art ALREADY
  # carries exactly the colour we are trying to match, and the grey-then-re-tint round
  # trip is lossy. Mnebula1's stock is 28/26/22, and rebuilding that warmth by
  # stretching red out of grey clipped 109 pixels to 255 against stock's peak of 246.
  local gray="-colorspace Gray"
  { [ "${KEEPCOLOUR:-0}" = 1 ] || [ "${SOURCE:-gen}" = stock ]; } && gray=""
  local report="" i
  for i in 0 1 2 3; do
    local ox=$(( (i%2)*half )) oy=$(( (i/2)*half ))
    magick "$stock" -crop ${half}x${half}+${ox}+${oy} +repage "PNG24:$tmp/ref.png"

    if [ "${SOURCE:-gen}" = stock ]; then
      # The source IS this quadrant, upscaled -- so every step below that exists to
      # POSITION unknown art is wrong here and would destroy the composition we are
      # trying to keep. No trim (there is no subject to find), no FILL inset (the puff
      # already sits where the artist put it), and no noise-floor stretch (the floor is
      # stock's own; removing it would push the quadrant darker than the file it
      # replaces). Just resize and let fit() match the means.
      magick "${src[$i]}" -auto-orient $gray -resize ${size}x${size}! "PNG24:$tmp/f$i.png"
    else
      # Noise floor = mean of the DARKEST 64px corner, capped at 3%. Not the max over a
      # large corner: once a cloud fills the frame that reads real gas and crushes it.
      local floor
      floor=$(for g in NorthWest NorthEast SouthWest SouthEast; do
          magick "${src[$i]}" -gravity $g -crop 64x64+0+0 +repage \
            -format '%[fx:int(255*mean)]\n' info:; done | sort -n | head -1)
      local clamp; clamp=$(python3 -c "print(min(3.0, round(($floor+3)/255*100,3)))")

      # Trim to the subject, square it, inset to FILL% so the falloff lands inside the
      # quadrant, then a LINEAR stretch -- never gamma, which maps background 1/255 up to
      # 32/255 and fills the sky with fake stars.
      magick "${src[$i]}" -auto-orient $gray -fuzz 1% -trim +repage \
          -background black -gravity center -extent '%[fx:max(w,h)]x%[fx:max(w,h)]' \
          -resize $((size*fill/100))x$((size*fill/100)) -extent ${size}x${size} \
          -level ${clamp}%,100% "PNG24:$tmp/f$i.png"
    fi

    read cur got gain < <(fit "$tmp/f$i.png" "$tmp/ref.png" "$tmp/q$i.png" "$tmp" "$size")
    report+=$(printf " q%d:%.0f→%s(x%s)" "$i" "$cur" "$got" "$gain")
  done

  # Force the quadrant edges to exact black. Additive blending makes any stray value a
  # visible glowing seam, and the vignette alone does not always reach zero.
  magick -size ${size}x${size} xc:black -fill white \
      -draw "rectangle 11,11 $((size-12)),$((size-12))" -blur 0x5 "$tmp/vig.png"
  for i in 0 1 2 3; do
    magick "$tmp/q$i.png" "$tmp/vig.png" -compose Multiply -composite "$tmp/v$i.png"
  done
  magick \( "$tmp/v0.png" "$tmp/v1.png" +append \) \( "$tmp/v2.png" "$tmp/v3.png" +append \) \
      -append "$tmp/atlas.png"
  mkdir -p "$dir/out"; write_tga "$tmp/atlas.png" "$dir/out/$name.tga" "$stock"

  # Hand-authored mip chain, when the stock texture has one. gen_mips() in
  # textures/lib/common.sh carries the whole rationale -- Box filter, no peak lift.
  local mips=${MIPS:-0}
  [ "$mips" -gt 0 ] && gen_mips "$tmp/atlas.png" "$dir/out" "$name" "$stock" "$mips" "$tmp"
  [ "$mips" -gt 0 ] && report+=" +${mips} mips"

  printf "built %-14s |%s\n" "$name" "$report"
}
