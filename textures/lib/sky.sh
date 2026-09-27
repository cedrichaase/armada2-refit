# Skybox textures (mbg*). Opposite treatment to the puffs: fill the frame edge to edge,
# keep their own colour, no trim, no centring, no falloff.
#   sky-atlas  one file that is a 2x2 grid of four tiles (MBG02)
#   sky-faces  N files, one generated image each (MbgBorg, MbgDom1, ...)
# SIZE is the per-FACE resolution -- one face fills the whole viewport, so that, not the
# atlas size, is what the eye sees.
build_sky_atlas () {
  local name=$1 dir=$2 tmp=$3 size=${SIZE:-512}
  local stock; stock=$(cd "$dir/stock" && tex_path "$name"); [ -n "$stock" ] || die "$name: no stock/"
  stock="$dir/stock/$stock"
  mapfile -t src < <(find "$dir/src" -maxdepth 1 -type f \
      \( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.webp' \) | sort)
  [ ${#src[@]} -gt 0 ] || { echo "SKIP  $name (no src)"; return 1; }
  while [ ${#src[@]} -lt 4 ]; do src+=("${src[0]}"); done

  local half; half=$(magick "$stock" -format '%[fx:int(w/2)]' info:)
  local report="" i
  for i in 0 1 2 3; do
    # UNIFORM=1: match every tile to the WHOLE stock texture, not its own quadrant.
    # Required when the four tiles are the same seamless image -- stock's quadrant means
    # differ (MBG02: 11/16/17/16) and per-quadrant targets would put a brightness step
    # at every cube-face join, undoing the point of making them seamless.
    if [ "${UNIFORM:-0}" = 1 ]; then magick "$stock" "PNG24:$tmp/ref.png"
    else magick "$stock" -crop ${half}x${half}+$(( (i%2)*half ))+$(( (i/2)*half )) +repage "PNG24:$tmp/ref.png"; fi
    read cur got gain < <(fit "${src[$i]}" "$tmp/ref.png" "$tmp/q$i.png" "$tmp" "$size")
    report+=$(printf " q%d:%.0f→%s(x%s)" "$i" "$cur" "$got" "$gain")
  done
  magick \( "$tmp/q0.png" "$tmp/q1.png" +append \) \( "$tmp/q2.png" "$tmp/q3.png" +append \) \
      -append "$tmp/atlas.png"
  mkdir -p "$dir/out"; write_tga "$tmp/atlas.png" "$dir/out/$name.tga" "$stock"
  printf "built %-14s |%s\n" "$name" "$report"
}

build_sky_faces () {
  local name=$1 dir=$2 tmp=$3 size=${SIZE:-512}
  mapfile -t faces < <(stock_bases "$dir/stock")
  mapfile -t src < <(find "$dir/src" -maxdepth 1 -type f \
      \( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.webp' \) | sort)
  [ ${#src[@]} -gt 0 ] || { echo "SKIP  $name (no src)"; return 1; }
  [ ${#src[@]} -eq ${#faces[@]} ] || { echo "SKIP  $name -- needs ${#faces[@]} images, found ${#src[@]}"; return 1; }

  # Pair src to stock BY NAME where possible, and only fall back to sort position.
  # Two independent `sort` calls is not a pairing rule: glibc collation ignores
  # punctuation, so MbgBaku.1 / MbgBaku1 / MbgBaku.2 tie and are separated only by the
  # byte-level tiebreak. It happens to agree on both sides here, which is luck. Two
  # names differing only in punctuation would silently swap two faces of a cube -- a
  # failure that looks like bad art, not like a bug. upscale-stock.sh names every src
  # after its stock face, so the by-name path is the normal one; hand-supplied art with
  # arbitrary names still works, positionally, exactly as before.
  local paired=1 i f base hit
  local -a match=()
  for i in "${!faces[@]}"; do
    base="${faces[$i]%.*}"; hit=""
    for f in "${src[@]}"; do
      [ "$(basename "${f%.*}")" = "$base" ] && { hit=$f; break; }
    done
    [ -n "$hit" ] || { paired=0; break; }
    match+=("$hit")
  done
  if [ "$paired" = 1 ]; then src=("${match[@]}")
  else note "$name: src/ names do not match stock/, pairing by sort order instead"; fi

  local report=""
  mkdir -p "$dir/out"
  for i in "${!faces[@]}"; do
    f="${faces[$i]}"; base="${faces[$i]%.*}"
    # Scratch named after the face, not a fixed f.png: fit()'s channel-synthesis note
    # reports basename "$dst", and every such warning used to read "f.png", which made
    # it impossible to tell which face it came from without cross-referencing means.
    # fit() is RGB-only and its output has no alpha; attach_alpha puts the stock file's
    # own alpha back, Lanczos-upscaled, when the stock file is 32-bit. For a 24-bit
    # stock -- every skybox face -- it is a copy and nothing changes.
    read cur got gain < <(fit "${src[$i]}" "$dir/stock/$f" "$tmp/$base.rgb.png" "$tmp" "$size")

    # blackedge=N: force the outer N texels of the image AND of each 2x2 quadrant to
    # exact black. Only for an ADDITIVE sprite atlas -- Mmoon, whose three sun sprites
    # are @tmaterial=additive, so black is transparent and any stray value on a quadrant
    # boundary draws a faintly glowing square around the sprite. Stock is exactly 0 on
    # every boundary line and the upscale came back 1-3; N is measured as stock's
    # guaranteed all-black margin (1 texel on Mmoon's tightest quadrant) times the scale
    # factor, so this restores stock's own values and cannot clip real art.
    #
    # THIS MUST RUN BEFORE attach_alpha, on the RGB-only plate. Composing onto an image
    # that already carries alpha makes ImageMagick work in associated (premultiplied)
    # space and un-associate on write, dividing colour back out by a small alpha:
    # Mmoon's quadrants came out at RGB mean 211 against stock's 43 and rendered in game
    # as a white box around the sun. `-channel RGB` does not prevent it. Same trap as
    # the Lanczos layer in upscale-stock.sh and the mip downsample in gen_mips -- the
    # rule everywhere in this pipeline is that colour and alpha are handled apart and
    # only joined at the end.
    local be=${BLACKEDGE:-0}
    if [ "$be" -gt 0 ]; then
      local hf=$((size/2))
      magick -size ${size}x${size} xc:black -fill white \
        -draw "rectangle $be,$be $((hf-1-be)),$((hf-1-be))" \
        -draw "rectangle $((hf+be)),$be $((size-1-be)),$((hf-1-be))" \
        -draw "rectangle $be,$((hf+be)) $((hf-1-be)),$((size-1-be))" \
        -draw "rectangle $((hf+be)),$((hf+be)) $((size-1-be)),$((size-1-be))" \
        "PNG24:$tmp/be.png"
      magick "$tmp/$base.rgb.png" -alpha off "$tmp/be.png" -compose Multiply -composite \
        "PNG24:$tmp/$base.be.png"
      mv "$tmp/$base.be.png" "$tmp/$base.rgb.png"
    fi

    # src-alpha/<base>.png, when upscale-stock.sh built one under alpha=ai, is the
    # upscaled self-illumination plate; with no such file attach_alpha falls back to
    # Lanczos-resizing stock's own alpha, which is what every other target does.
    attach_alpha "$tmp/$base.rgb.png" "$dir/stock/$f" "$tmp/$base.png" "$tmp" \
        "$dir/src-alpha/$base.png"
    write_tga "$tmp/$base.png" "$dir/out/$base.tga" "$dir/stock/$f"
    # "auto" is not a number, so this cannot be the arithmetic test it used to be.
    [ "${MIPS:-0}" != 0 ] &&
      gen_mips "$tmp/$base.png" "$dir/out" "$base" "$dir/stock/$f" "${MIPS}" "$tmp"
    report+=$(printf " %s:%.0f→%s" "$base" "$cur" "$got")
  done
  [ "${MIPS:-0}" != 0 ] && report+=" +${MIPS} mips"
  printf "built %-14s |%s\n" "$name" "$report"
}
