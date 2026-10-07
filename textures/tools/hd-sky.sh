#!/usr/bin/env bash
# Higher-fidelity skybox faces: each face of an existing sky-faces target, edited by an
# image model into a crisper rendering of the SAME picture. Spends credits (belt); with
# --reblend it spends nothing.
#
#   textures/tools/hd-sky.sh [opts] BASE...      e.g.  hd-sky.sh MbgBorg mbgpur
#     --blend N        per cent of the model's face kept over BASE's own src/ face
#                      (default 100: the model's face as it is); written to the HD target's target.conf
#     --quality Q      model quality: low medium high xhigh max (default high)
#     --model ID       belt app (default openai/gpt-image-2-5-sunburst, the edit model
#                      that keeps composition)
#     --concurrency N  tasks in flight (default 4)
#     --reblend        no network: rebuild src/ from the ai/ already on disk
#     --force          re-generate faces that already have an ai/ file
#
# For BASE it makes the target <BASE>HD (recipe textures/targets/<BASE>HD/, work directory
# $A2_DATA/textures/<BASE>HD/):
#   ai/   the raw edits, one PNG per face           (what the credits bought -- never delete)
#   src/  ai/ blended over BASE's src/              (what `a2tex build <BASE>HD` reads)
# BASE's own src/ (the stock upscale, `upscale-stock.sh`) is the model's input, so BASE must
# have been upscaled first. <BASE>HD carries `optin=yes`: `a2tex install` with no target
# named skips it, and `a2tex install <BASE>HD` / `a2tex install <BASE>` switches between
# the two; `a2tex revert <BASE>HD` (or `all`) returns to stock.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
. "$ROOT/lib/common.sh"; . "$ROOT/../a2env.sh"
BLEND=""; QUALITY=high; MODEL=openai/gpt-image-2-5-sunburst; CONC=4; REBLEND=0; FORCE=0
while [ $# -gt 0 ]; do case $1 in
  --blend) BLEND=$2; shift 2;; --quality) QUALITY=$2; shift 2;; --model) MODEL=$2; shift 2;;
  --concurrency) CONC=$2; shift 2;; --reblend) REBLEND=1; shift;; --force) FORCE=1; shift;;
  -*) echo "unknown option $1" >&2; exit 2;; *) break;; esac; done
[ $# -gt 0 ] || { sed -n '2,21p' "$0" | sed 's/^# \?//'; exit 2; }

PROMPT='Remaster this image as a higher-fidelity version of the same artwork. It is a 2001 video-game space skybox: a nebula painted in soft, rich gas. Keep the exact composition, every cloud shape in its place, every colour, and the overall brightness and contrast. Keep the four black square corners pure black. Add crisp fine detail only inside the existing structures: sharper fibrous gas filaments, finer wispy texture, cleaner edges and subtle depth. Do not add stars, planets, ships, text, borders or new objects, and do not change the art style.'

for base in "$@"; do
  t=${base}HD; rec="$ROOT/targets/$t"; bdir="$A2_DATA/textures/$base"; dir="$A2_DATA/textures/$t"
  [ "$(grep -E '^kind=' "$ROOT/targets/$base/target.conf" | cut -d= -f2-)" = sky-faces ] ||
    { echo "$base: not a sky-faces target" >&2; exit 1; }
  [ -n "$(ls "$bdir/src" 2>/dev/null)" ] || { echo "$base: no src/ -- upscale it first" >&2; exit 1; }
  if [ ! -d "$rec" ]; then
    mkdir -p "$rec"; cp "$ROOT/targets/$base/stock.sha256" "$rec/"
    { grep -vE '^(blend|note|optin|base)=' "$ROOT/targets/$base/target.conf"
      echo "note=$base's faces edited by an image model for finer detail (tools/hd-sky.sh)"
      echo "base=$base"; echo "optin=yes"; echo "blend=${BLEND:-100}"; } > "$rec/target.conf"
  fi
  if [ -n "$BLEND" ]; then sed -i -E "s/^blend=.*/blend=$BLEND/" "$rec/target.conf"; fi
  blend=$(grep -E '^blend=' "$rec/target.conf" | head -1 | cut -d= -f2-)
  mkdir -p "$dir/ai" "$dir/src"
  mapfile -t faces < <(cd "$bdir/src" && ls -1 *.png | sort)

  # Up to three passes: an upload sometimes 404s before the model sees it, which --retry
  # does not cover, and only the faces still missing are sent again (and billed).
  for pass in 1 2 3; do
    [ "$REBLEND" = 0 ] || break
    jl=$(mktemp "$A2_DATA/.scratch/hd.XXXXXX.jsonl" 2>/dev/null || { mkdir -p "$A2_DATA/.scratch"; mktemp "$A2_DATA/.scratch/hd.XXXXXX.jsonl"; })
    todo=()
    for f in "${faces[@]}"; do
      [ "$FORCE" = 0 ] && [ -s "$dir/ai/$f" ] && continue
      todo+=("$f")
      # Sent as a scratch copy tagged with the pass: re-sending byte-identical input
      # to an upload that has already 404'd 404s again (mbgaqu3 did, every time).
      mkdir -p "$A2_DATA/.scratch/hd-in"
      magick "$bdir/src/$f" -set comment "$t pass $pass" "$A2_DATA/.scratch/hd-in/$t-$f"
      PROMPT="$PROMPT" IMG="$A2_DATA/.scratch/hd-in/$t-$f" Q="$QUALITY" python3 -c '
import json,os
print(json.dumps({"prompt":os.environ["PROMPT"],"images":[os.environ["IMG"]],"width":2048,"height":2048,"quality":os.environ["Q"],"output_format":"png"}))' >> "$jl"
    done
    if [ ${#todo[@]} -gt 0 ]; then
      echo "$t: ${#todo[@]} face(s) -> $MODEL ($QUALITY)"
      res=$(mktemp "$A2_DATA/.scratch/hd.XXXXXX.out")
      belt app run "$MODEL" --batch "$jl" --json --concurrency "$CONC" --retry 2 > "$res" || true
      # results come back out of order; `index` is the 0-based input line.
      while IFS=$'\t' read -r i url; do
        curl -sfL -o "$dir/ai/${todo[$i]}" "$url" && echo "  ${todo[$i]}"
      done < <(python3 -c '
import json,sys
for l in open(sys.argv[1]):
    l=l.strip()
    if not l.startswith("{"): continue
    r=json.loads(l)
    if r.get("status")=="completed": print("%d\t%s"%(r["index"],r["output"]["images"][0]))' "$res")
      rm -f "$res"
    fi
    rm -f "$jl"
    rm -rf "$A2_DATA/.scratch/hd-in"
    [ ${#todo[@]} -gt 0 ] || break
    FORCE=0
  done

  miss=0
  for f in "${faces[@]}"; do
    [ -s "$dir/ai/$f" ] || { echo "$t: ai/$f missing (re-run to retry)" >&2; miss=1; continue; }
    # the model returns the size asked for, but read it off the file (hard rule 6)
    sz=$(magick "$bdir/src/$f" -format '%wx%h\n' info:)
    magick "$bdir/src/$f" -alpha off "(" "$dir/ai/$f" -alpha off -resize "${sz}!" ")" \
      -define compose:args="$blend" -compose blend -composite "PNG24:$dir/src/$f"
  done
  [ "$miss" = 0 ] && echo "$t: src/ blended at $blend%  ->  ./a2tex build $t"
done
