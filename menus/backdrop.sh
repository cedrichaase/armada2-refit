#!/usr/bin/env bash
# Build a widescreen, hi-res menu backdrop for Menus.asi: $A2_DATA/backdrops/<name>/wide.bmp.
#
#   menus/backdrop.sh <name> [--blend N] [--reblend] [--force]
#     <name>      menus/backdrops/<name>.conf -- source=, blend=, seed=, prompt=, negative=,
#                 upscaler=pruna|bria (default bria), outpaint=no (centre only),
#                 field=L,T,R,B (design px: the open art inside a drawn frame),
#                 clone=W,H,SX,SY,DX,DY[,F] ... (ai/expanded.png px: patch over a
#                 defect, feathered F px, default 8),
#                 fade=P (sides darken to P% at the screen edge),
#                 dehaze=K[,M] (darken blue haze by K%, ramped in over M px),
#                 keep=L,T,R,B ... (design px: stock UI art the field cuts through;
#                 the plate takes the upscale there, never the outpaint)
#     --blend N   per cent of the AI upscale kept over Lanczos (default: the conf)
#     --reblend   no network: rebuild wide.bmp from the ai/ layers already on disk
#     --force     pay again for both generative steps
#
# The menus are 800x600 GDI screens, drawn by the shell 1:1 into Menus.asi's offscreen
# DC and stretched to the screen height, pillarboxed. So the art cannot be swapped in
# bitmaps/ -- the shell would still draw 800x600 of it -- and Menus.asi composites this
# plate itself: its sides fill the pillarboxes and its centre replaces the stretched
# stock wherever the shell is still showing the stock background. See menus.c.
#
# Same recipe as models/loading-panel.sh, and the same order for the same reason:
# upscale the stock art first, then outpaint, then feather the upscale back over the
# centre so the model's contribution is the sides alone.
#   ai/stock.png     the stock 800x600 BMP
#   ai/up.png        its generative upscale, 4x                     (~$0.02)
#   ai/expanded.png  the BLENDED plate at 2048x1536, outpainted to
#                    3680x1536 (bria/expand, $0.02) -- sides only
#   wide.bmp         3450x1440, 24-bit: what install.sh copies to Menus/<name>.bmp
#
# Geometry. The plate is authored at the screen height it is shown at (1440, the one
# screen this has been built for) so Menus.asi never resamples it; any other height is
# stretched once when the backdrop is first shown. The centre 4:3 is H*4/3 wide and
# Menus.asi assumes it is horizontally centred, so the plate width minus 1920 must be
# even: 3680x1536 -> 3450x1440 is x0.9375 exactly and puts the centre at 765..2685.
# 3450/1440 = 2.396:1 covers 21:9 (2.389) with 5px to spare each side.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
. "$(cd "$here/.." && pwd)/a2env.sh"; GAME="$A2_GAME"
UP=pruna/p-image-upscale@7j4n1pr7
UPB=bria/increase-resolution@6yjh1gjj
EX=bria/expand@1d8zqs34
H=1440 EH=1536 EW=3680 PW=2048

name=${1:-}; [ -n "$name" ] && [ "${name#-}" = "$name" ] || { sed -n '2,38p' "$0" | sed 's/^# \?//'; exit 2; }
shift
conf="$here/backdrops/$name.conf"; [ -f "$conf" ] || { echo "no $conf" >&2; exit 1; }
BLEND="" REBLEND=0 FORCE=0
while [ $# -gt 0 ]; do case $1 in
  --blend) BLEND=$2; shift 2;; --reblend) REBLEND=1; shift;; --force) FORCE=1; shift;;
  *) echo "unknown argument: $1" >&2; exit 2;; esac; done
cfg () { grep -E "^$1=" "$conf" | head -1 | cut -d= -f2- || true; }
blend=${BLEND:-$(cfg blend)}; blend=${blend:-35}
seed=$(cfg seed); seed=${seed:-1}
# field=: the part of the 800x600 art that is open picture rather than a drawn frame.
# The outpaint sees only this, and the plate replaces everything outside it -- frame
# included, which is invisible in game only because Menus.asi shows the plate wherever
# the shell still draws the stock background. Default: the whole screen.
IFS=, read -r fL fT fR fB <<<"$(cfg field),"
fL=${fL:-0} fT=${fT:-0} fR=${fR:-800} fB=${fB:-600}
src="$GAME/$(cfg source)"; [ -f "$src" ] || { echo "stock source not found: $src" >&2; exit 1; }
D="$A2_DATA/backdrops/$name"; A="$D/ai"   # every layer and the plate; the conf stays here
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
mkdir -p "$A"

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
  [ -n "$url" ] || { echo "  !! $1 returned no image:" >&2; tail -c 600 "$T/r.json" >&2; return 1; }
  curl -sfL -o "$3" "$url"
}

# --- 1. stock + generative upscale ------------------------------------------------
magick "$src" -alpha off "PNG24:$A/stock.png"
read -r sw sh < <(magick "$A/stock.png" -format '%w %h\n' info:)
[ "$sw" = 800 ] && [ "$sh" = 600 ] || { echo "$src is ${sw}x${sh}, not 800x600" >&2; exit 1; }
if [ "$REBLEND" = 0 ] && { [ "$FORCE" = 1 ] || [ ! -s "$A/up.png" ]; }; then
  # upscaler=pruna in the conf picks the project's usual model: 8MP on 800x600 rounds
  # to round(sqrt(8e6/480000)) = 4. bria is the default because pruna has hung in
  # "running" or returned 503 on both days these were built. Both are 4x; read back below.
  if [ "$(cfg upscaler)" = pruna ]; then
    echo "$name: upscaling 800x600 (pruna, 8MP)"
    printf '{"image":"%s","megapixels":8,"output_format":"png","enhance_details":false,"enhance_realism":false}\n' \
      "$A/stock.png" > "$T/up.json"
    run "$UP" "$T/up.json" "$A/up.png"
  else
    echo "$name: upscaling 800x600 (bria/increase-resolution, 4x)"
    printf '{"image":"%s","desired_increase":4}\n' "$A/stock.png" > "$T/up.json"
    run "$UPB" "$T/up.json" "$A/up.png"
  fi
fi
read -r uw uh < <(magick "$A/up.png" -format '%w %h\n' info:)
magick "$A/stock.png" -filter Lanczos -resize "${uw}x${uh}!" "PNG24:$T/lz.png"
magick "$T/lz.png" "$A/up.png" -define compose:args="$blend" -compose blend -composite \
    "PNG24:$T/plate.png"

# --- 2. outpaint to 2.4:1 ---------------------------------------------------------
# outpaint=no: the hi-res centre alone, 1920x1440, and Menus.asi keeps the sides black.
# (A screen drawn inside its own black frame wants field= instead -- see singleplay.conf.)
if [ "$(cfg outpaint)" = no ]; then
  C=$(( H * 4 / 3 ))
  magick "$T/plate.png" -filter Lanczos -resize "${C}x${H}!" -alpha off "PNG24:$D/wide.png"
  magick "$D/wide.png" -alpha off -type TrueColor BMP3:"$D/wide.bmp"
  magick "$D/wide.png" -resize 1720x "$D/preview.png"
  echo "$name: wide.bmp ${C}x${H}, centre only (outpaint=no), blend $blend"
  exit 0
fi
# bria caps a canvas at 5000px, so the 4x plate (3200x2400) is outpainted at 2048x1536;
# the centre of the final plate comes from the full 4x plate, not from this.
ox=$(( (EW - PW) / 2 ))
# the field alone, at the outpaint's scale, placed where it sits in the 2048 centre
exL=$(( ox + fL * PW / 800 )) exT=$(( fT * EH / 600 ))
exW=$(( (fR - fL) * PW / 800 )) exH=$(( (fB - fT) * EH / 600 ))
magick "$T/plate.png" -crop "$(( (fR - fL) * uw / 800 ))x$(( (fB - fT) * uh / 600 ))+$(( fL * uw / 800 ))+$(( fT * uh / 600 ))" +repage \
    -filter Lanczos -resize "${exW}x${exH}!" "PNG24:$T/plate-ex.png"
if [ "$REBLEND" = 0 ] && { [ "$FORCE" = 1 ] || [ ! -s "$A/expanded.png" ]; }; then
  echo "$name: outpainting ${exW}x${exH} at $exL,$exT -> ${EW}x${EH} (seed $seed)"
  python3 - "$T/plate-ex.png" "$EW" "$EH" "$exL" "$exT" "$exW" "$exH" "$seed" "$(cfg prompt)" "$(cfg negative)" > "$T/ex.json" <<'PY'
import json, sys
img, ew, eh, ex, ey, w, h, seed, prompt, neg = sys.argv[1:]
print(json.dumps({"image": img, "canvas_size": [int(ew), int(eh)],
                  "original_image_location": [int(ex), int(ey)],
                  "original_image_size": [int(w), int(h)],
                  "prompt": prompt, "negative_prompt": neg, "seed": int(seed)}))
PY
  run "$EX" "$T/ex.json" "$A/expanded.png"
fi
read -r ew eh < <(magick "$A/expanded.png" -format '%w %h\n' info:)
[ "$ew" = "$EW" ] && [ "$eh" = "$EH" ] ||
  { echo "ai/expanded.png is ${ew}x${eh}, wanted ${EW}x${EH} -- stale? use --force" >&2; exit 1; }

# clone=: an outpaint that is right apart from one small defect -- singleplay seed 2 wrote
# a line of fake glyphs into empty sky -- is kept, and the defect covered with a nearby
# patch of the same sky, feathered F px (default 8; small where the patch must stop at
# a hard edge in the art). Space-separated for several. ai/ stays as paid.
cp "$A/expanded.png" "$T/expanded.png"
for c in $(cfg clone); do
  IFS=, read -r cw ch sx sy dx dy fe <<<"$c"
  fe=${fe:-8}
  magick "$T/expanded.png" \
    \( "$T/expanded.png" -crop "${cw}x${ch}+$sx+$sy" +repage \
       \( -size "${cw}x${ch}" xc:black -fill white -draw "rectangle $fe,$fe $((cw-1-fe)),$((ch-1-fe))" -blur "0x$(( fe / 2 ))" \) \
       -alpha off -compose copy-opacity -composite \) \
    -geometry "+$dx+$dy" -compose over -composite -alpha off "PNG24:$T/expanded.png"
done

# --- 3. final plate: sides from the outpaint, centre from the 4x plate, feathered --
# Feathered for the reason loading-panel.sh gives: bria does not return the original
# region untouched, and a hard paste draws a line down the seam. F px INSIDE the field;
# its left and right edges always ramp (the outpaint is beyond them), its top and
# bottom only when the field stops short of the screen edge.
W=$(( EW * H / EH )); C=$(( H * 4 / 3 )); cx=$(( (W - C) / 2 )); F=64
mL=$(( fL * C / 800 )) mR=$(( fR * C / 800 )) mT=$(( fT * H / 600 )) mB=$(( fB * H / 600 ))
[ "$fT" = 0 ] && mT=-$F; [ "$fB" = 600 ] && mB=$(( H + F ))
magick "$T/expanded.png" -alpha off -filter Lanczos -resize "${W}x${H}!" "PNG24:$T/sides.png"
magick "$T/plate.png" -filter Lanczos -resize "${C}x${H}!" "PNG24:$T/centre.png"
magick -size "${C}x${H}" xc:black -fx \
  "clamp((i-($mL))/$F)*clamp(($mR-1-i)/$F)*clamp((j-($mT))/$F)*clamp(($mB-1-j)/$F)" \
  -alpha off "PNG24:$T/mask.png"
# keep=: UI art that the field edge cuts through (singleplay's panel bar starts at 30,
# the field at 32). Left to the mask above, the plate there is the outpaint's redrawn
# copy of it, blended into the upscale's across the feather: two misaligned drawings of
# one edge, which wobbles. Idle it shows; hovered the shell covers it, so it flickers.
for k in $(cfg keep); do
  IFS=, read -r kL kT kR kB <<<"$k"
  magick "$T/mask.png" \( -size "${C}x${H}" xc:black -fill white \
      -draw "rectangle $(( kL * C / 800 )),$(( kT * H / 600 )) $(( kR * C / 800 - 1 )),$(( kB * H / 600 - 1 ))" \
      -blur 0x3 \) -compose lighten -composite -alpha off "PNG24:$T/mask.png"
done
magick "$T/sides.png" \
  \( "$T/centre.png" "$T/mask.png" -alpha off -compose copy-opacity -composite \) \
  -geometry "+$cx+0" -compose over -composite -alpha off "PNG24:$D/wide.png"
# fade=P: darken the sides towards the screen edges, to P per cent at the edge. The
# outpaint lights its sides as evenly as its centre, which pulls the eye outwards; this
# keeps the menu the brightest thing on screen. 1.0 at the 4:3 boundary, so the centre
# is untouched and there is no seam; (1-d)^1.5 so it starts darkening at once.
fade=$(cfg fade)
if [ -n "$fade" ]; then
  magick "$D/wide.png" \( -size "${W}x${H}" xc:black -fx \
    "$fade/100 + (1 - $fade/100)*pow(1 - min(max(max(($cx-i)/$cx, (i-$cx-$C+1)/$cx), 0), 1), 1.5)" \
    \) -compose multiply -composite -alpha off "PNG24:$D/wide.png"
fi
# dehaze=K[,M]: darken blue haze, by K per cent of how far blue exceeds red and green
# ((b - max(r,g)) / b), so orange gas, white stars and grey UI art keep their light. The
# haze straddles the 4:3 edge -- the outpaint continues stock's own corner glow -- so
# the weight ramps from 0 at M plate px inside the centre (default 150) to full at M
# outside it, and the edge carries no step. Inside the centre the plate only shows
# where the shell still draws stock background, so this is safe there too.
IFS=, read -r dhK dhM <<<"$(cfg dehaze),"
if [ -n "$dhK" ]; then
  dhM=${dhM:-150}
  magick "$D/wide.png" -fx \
    "u*(1 - $dhK/100*clamp((max($cx-i, i-($cx+$C-1)) + $dhM)/(2*$dhM))*clamp((b - max(r,g))/(b + 0.02)))" \
    -alpha off "PNG24:$D/wide.png"
fi
magick "$D/wide.png" -alpha off -type TrueColor BMP3:"$D/wide.bmp"
magick "$D/wide.png" -resize 1720x "$D/preview.png"
echo "$name: wide.bmp ${W}x${H}, centre ${C} at x $cx, blend $blend"
