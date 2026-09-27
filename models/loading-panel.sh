#!/usr/bin/env bash
# Build the widened mission loading-screen art: $A2_DATA/textures/LOADING/src/LOADING1..6.png.
#
#   models/loading-panel.sh [--blend N] [--reblend] [--force]
#     --blend N   per cent of the AI upscale kept over Lanczos (default: target.conf)
#     --reblend   no network: rebuild src/ from the ai/ layers already on disk
#     --force     pay again for both generative steps
#
# The loading screen is six 256px tiles on six quads of SOD/logo.SOD, a 3:2 panel the
# camera fits to the screen height -- so on a 21:9 screen the skybox shows either side.
# Widening it is two changes that only make sense together: models/logo-sod.py widens
# the quads to `panel=` units, and this widens the picture to the same aspect. See the
# docstring of logo-sod.py for the engine side.
#
# Layers, kept for the same reason upscale-stock.sh keeps ai/ (re-tuning must be free):
#   ai/stock-plate.png  the six stock tiles assembled, 768x512
#   ai/up.png           the raw generative upscale of that, 4x   (upscaler=, ~$0.02)
#   ai/expanded.png     the BLENDED plate outpainted to the panel aspect (bria/expand,
#                       $0.02) -- only its sides are used
#   src/LOADING<n>.png  what `a2tex build LOADING` reads
#
# Order matters, and is deliberate: upscale the stock art first, then outpaint. The
# other way round, the centre -- the logo and six ships, all the art anyone looks at --
# would be a 2x upscale of a Lanczos-enlarged image rather than a 4x of the real one.
# After outpainting, the blended plate is pasted back over the centre, so the centre is
# exactly the upscale-stock recipe and the model's contribution is the sides alone.
#
# Each tile then holds ONE THIRD of the wider picture squashed into its square, because
# logo-sod.py stretches the quads rather than adding any. That is the only anisotropy,
# and it is undone exactly on screen.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
. "$ROOT/a2env.sh"
R="$ROOT/textures/targets/LOADING"          # the recipe: target.conf
D="$A2_DATA/textures/LOADING"; S="$D/stock"; A="$D/ai"
UP=pruna/p-image-upscale@7j4n1pr7
UPB=bria/increase-resolution@6yjh1gjj
EX=bria/expand@1d8zqs34
BLEND="" REBLEND=0 FORCE=0
while [ $# -gt 0 ]; do case $1 in
  --blend) BLEND=$2; shift 2;; --reblend) REBLEND=1; shift;; --force) FORCE=1; shift;;
  *) sed -n '2,30p' "$0" | sed 's/^# \?//'; exit 2;; esac; done
cfg () { grep -E "^$1=" "$R/target.conf" | head -1 | cut -d= -f2- | tr -d ' ' || true; }
if [ -n "$BLEND" ]; then sed -i -E "s/^blend=.*/blend=$BLEND/" "$R/target.conf"; fi
blend=$(cfg blend); blend=${blend:-35}
panel=$(cfg panel); [ -n "$panel" ] || { echo "target.conf has no panel=" >&2; exit 1; }
size=$(cfg size)
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
mkdir -p "$A" "$D/src"

# run APP JSON OUT -- one belt task, result image downloaded to OUT
run () {
  "$HOME/.local/bin/belt" app run "$1" --input "$2" --no-input --json 2>/dev/null > "$T/r.json" || true
  local url; url=$(python3 -c '
import json, sys
for line in open(sys.argv[1]):
    try: r = json.loads(line)
    except ValueError: continue
    o = r.get("output") or {}
    if o.get("image"): print(o["image"]); break' "$T/r.json")
  [ -n "$url" ] || { echo "  !! $1 returned no image:" >&2; tail -c 600 "$T/r.json" >&2; exit 1; }
  curl -sfL -o "$3" "$url"
}

# --- 1. stock plate + generative upscale ------------------------------------------
magick \( "$S/LOADING1.tga" "$S/LOADING2.tga" "$S/LOADING3.tga" +append \) \
       \( "$S/LOADING4.tga" "$S/LOADING5.tga" "$S/LOADING6.tga" +append \) -append \
       -alpha off "PNG24:$A/stock-plate.png"
if [ "$REBLEND" = 0 ] && { [ "$FORCE" = 1 ] || [ ! -s "$A/up.png" ]; }; then
  # upscaler= in target.conf. pruna is the project's usual model; bria is here because
  # pruna returned 503s for the whole afternoon this target was built. Both are a 4x lift
  # -- for pruna, 6MP on 768x512 rounds to round(sqrt(6e6/393216)) = 4. Read back, below.
  case "$(cfg upscaler)" in
    bria)
      echo "LOADING: upscaling the 768x512 plate (bria/increase-resolution, 4x)"
      printf '{"image":"%s","desired_increase":4}\n' "$A/stock-plate.png" > "$T/up.json"
      run "$UPB" "$T/up.json" "$A/up.png" ;;
    *)
      echo "LOADING: upscaling the 768x512 plate (pruna, 6MP)"
      printf '{"image":"%s","megapixels":6,"output_format":"png","enhance_details":false,"enhance_realism":false}\n' \
        "$A/stock-plate.png" > "$T/up.json"
      run "$UP" "$T/up.json" "$A/up.png" ;;
  esac
fi
read -r pw ph < <(magick "$A/up.png" -format '%w %h\n' info:)
magick "$A/stock-plate.png" -filter Lanczos -resize "${pw}x${ph}!" "PNG24:$T/lz.png"
magick "$T/lz.png" "$A/up.png" -define compose:args="$blend" -compose blend -composite \
    "PNG24:$T/plate.png"

# --- 2. outpaint to the panel's aspect --------------------------------------------
# The prompt NAMES THE PLANET on purpose. A generic "nebula and stars" prompt continued
# Earth's limb correctly but read its unlit hemisphere as sky and sprinkled it with
# stars -- a see-through planet at the lower left. Describing the night side as a solid
# surface fixed it on the first try (both tested on the same plate, seeds 1 and 2).
# Canvas height is the plate's; width is what makes it panel x 576 units. bria caps a
# canvas at 5000px, which is why the panel stops at ~2.44:1 at this plate height.
cw=$(python3 -c "print(round($ph * $panel / 576))")
ox=$(( (cw - pw) / 2 ))
[ "$cw" -le 5000 ] || { echo "canvas ${cw}px exceeds bria's 5000" >&2; exit 1; }
if [ "$REBLEND" = 0 ] && { [ "$FORCE" = 1 ] || [ ! -s "$A/expanded.png" ]; }; then
  echo "LOADING: outpainting ${pw}x${ph} -> ${cw}x${ph}"
  cat > "$T/ex.json" <<EOF
{"image":"$T/plate.png","canvas_size":[$cw,$ph],"original_image_location":[$ox,0],
 "original_image_size":[$pw,$ph],
 "prompt":"a large planet fills the lower left corner, its unlit night side a smooth solid dark blue-grey surface; above and to the right, deep space with purple and violet nebula clouds and scattered small stars",
 "negative_prompt":"stars on the planet, transparent planet, text, letters, logo, spaceship, starship, frame, border","seed":2}
EOF
  run "$EX" "$T/ex.json" "$A/expanded.png"
fi
read -r ew eh < <(magick "$A/expanded.png" -format '%w %h\n' info:)
[ "$ew" = "$cw" ] && [ "$eh" = "$ph" ] ||
  { echo "ai/expanded.png is ${ew}x${eh}, wanted ${cw}x${ph} -- stale? use --force" >&2; exit 1; }
# Pasted back with a FEATHER, not a hard edge. bria does not return the original region
# untouched -- RMSE 0.015 against its own input -- so a hard paste left a visible
# vertical line through Earth at the left seam (column mean 133 -> 127 in one pixel).
# Fading from bria's copy to ours over F px INSIDE the original region puts the join on
# pixels both versions describe; measured step-free afterwards.
F=96
magick -size "${pw}x${ph}" xc:white \
  \( -size "${ph}x$F" gradient:black-white -rotate -90 \) -geometry +0+0 -compose over -composite \
  \( -size "${ph}x$F" gradient:black-white -rotate 90 \) -geometry "+$((pw-F))+0" -compose over -composite \
  "PNG24:$T/mask.png"
magick "$A/expanded.png" -alpha off \
  \( "$T/plate.png" "$T/mask.png" -alpha off -compose copy-opacity -composite \) \
  -geometry "+$ox+0" -compose over -composite -alpha off "PNG24:$T/wide.png"

# --- 3. squash into 3 x 2 square tiles --------------------------------------------
magick "$T/wide.png" -filter Lanczos -resize "$((3*size))x$((2*size))!" "PNG24:$T/tiles.png"
for i in 1 2 3 4 5 6; do
  c=$(( (i-1) % 3 )); r=$(( (i-1) / 3 ))
  magick "$T/tiles.png" -crop "${size}x${size}+$((c*size))+$((r*size))" +repage \
      "PNG24:$D/src/LOADING$i.png"
done
magick "$T/wide.png" -resize 1720x "$A/preview.png"
echo "LOADING: src/ built -- plate ${pw}x${ph} at blend $blend, panel ${cw}x${ph} (${panel} units), tiles ${size}"
