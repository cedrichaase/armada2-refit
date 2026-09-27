#!/usr/bin/env bash
# Generatively upscale a texture's own stock art -- the recipe the user accepted for
# MBG02 (candidate D). Handles both kinds: a `sky-faces` set's six face files, and a
# `puff` atlas's four 64x64 quadrants.
#
#   textures/tools/upscale-stock.sh [opts] TARGET...
#     --blend N       per cent of the AI layer kept, over Lanczos. Defaults to the
#                     target's `blend=`, else 35; passing it writes it back there
#     --mp N          upscaler target megapixels (default 4 -> 2048x2048). A puff
#                     quadrant is only 64px, so 1 (-> 1024, a 16x lift) is the sane
#                     setting there; 4 would be a 32x lift from 64 pixels
#     --concurrency N belt tasks in flight (default 4)
#     --reblend       no network: rebuild src/ from the ai/ already on disk
#     --force         re-upscale faces that already have an ai/ file
#
#   `alpha=ai` in target.conf additionally sends each face's ALPHA plate through the
#   same model and blends it the same way, into src-alpha/. Off by default, and wrong
#   for a coverage mask -- see the alpha section of README.md before setting it.
#
# Three layers, on purpose, in the target's work directory $A2_DATA/textures/<T>/
# (the recipe, target.conf, stays in textures/targets/<T>/):
#   stock/  the original 256x256 face                      (never written)
#   ai/     the raw generative upscale, one PNG per face   (what the credits bought)
#   src/    ai/ blended --blend% over a plain Lanczos      (what `a2tex build` reads)
#
# Keeping ai/ is the point. For MBG02 the AI and Lanczos layers were scratch and got
# deleted, so re-tuning the blend -- the one dial still open to taste -- meant paying to
# upscale all over again. With ai/ on disk, `--reblend` is free and offline.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
. "$ROOT/lib/common.sh"; . "$ROOT/../a2env.sh"
# Scratch for the Lanczos layers, OUTSIDE the target. These used to be written as
# src/.lz-<name>.png and src-alpha/.lz-<name>.png -- a fixed path inside a directory
# whose file COUNT is load-bearing: build_sky_faces requires exactly one src image per
# stock base and refuses the target otherwise. Two processes on one target then collide,
# and the symptom is not a clobbered file but "SKIP -- needs 24 images, found 25", which
# reads like a missing texture rather than a collision. textures/lib/common.sh has had the rule
# since -j was introduced ("every function takes its scratch directory as an argument
# and writes nothing to a fixed path"); this script never followed it.
LZ=$(mktemp -d); trap 'rm -rf "$LZ"' EXIT
APP=pruna/p-image-upscale@7j4n1pr7
BLEND="" MP=4 CONC=4 REBLEND=0 FORCE=0
while [ $# -gt 0 ]; do case $1 in
  --blend) BLEND=$2; shift 2;; --mp) MP=$2; shift 2;; --concurrency) CONC=$2; shift 2;;
  --reblend) REBLEND=1; shift;; --force) FORCE=1; shift;;
  -*) echo "unknown option $1" >&2; exit 2;; *) break;; esac; done
[ $# -gt 0 ] || { sed -n '2,29p' "$0" | sed 's/^# \?//'; exit 2; }

for t in "$@"; do
  dir="$A2_DATA/textures/$t"; rec="$ROOT/targets/$t"
  # stock/ is not in git; a2tex fills it from the game and checks it against
  # stock.sha256. Fill it here too -- paying to upscale an incomplete set is the worst
  # way to find out it was incomplete.
  "$ROOT/../a2tex" stock "$t" >/dev/null || { echo "$t: stock/ could not be filled" >&2; exit 1; }

  # A puff is one 128x128 atlas, not N face files: the units to upscale are its four
  # 64x64 quadrants, carved by Sprites/nebula.spr at @reference=128. Slice them into
  # stock/-equivalents first so the rest of this script is identical for both kinds.
  kind=$(grep -E '^kind=' "$rec/target.conf" 2>/dev/null | head -1 | cut -d= -f2- || true)
  unit_dir="$dir/stock"
  if [ "$kind" = puff ]; then
    unit_dir="$dir/.units"; mkdir -p "$unit_dir"
    # `|| true`: the failing half of the two-case glob returns non-zero and set -e
    # would kill the script here. This is CLAUDE.md hard rule 3, and it still bites.
    atlas=$(ls "$dir/stock"/*.tga "$dir/stock"/*.TGA 2>/dev/null | head -1 || true)
    half=$(magick "$atlas" -format '%[fx:int(w/2)]' info:)
    for i in 0 1 2 3; do
      magick "$atlas" -crop ${half}x${half}+$(( (i%2)*half ))+$(( (i/2)*half )) +repage \
          "PNG24:$unit_dir/q$i.png"
    done
  fi

  # The blend lives in target.conf, so what produced src/ is recorded where the rest of
  # the target's settings are. Without this, a --reblend with no --blend silently
  # reverts a deliberately lowered set back to the script default: mbgrg was tuned to
  # 20 for a real reason and had no machine-readable place to say so.
  blend=$BLEND
  [ -n "$blend" ] || blend=$(grep -E '^blend=' "$rec/target.conf" 2>/dev/null | head -1 | cut -d= -f2- | tr -d ' ')
  [ -n "$blend" ] || blend=35
  if [ -n "$BLEND" ]; then
    if grep -qE '^blend=' "$rec/target.conf"; then
      sed -i -E "s/^blend=.*/blend=$BLEND/" "$rec/target.conf"
    else
      printf 'blend=%s\n' "$BLEND" >> "$rec/target.conf"
    fi
  fi
  mkdir -p "$dir/ai" "$dir/src"
  # stock_bases, not a plain ls: a hull target's stock/ carries the hand-authored mip
  # LEVELS beside its bases, because gen_mips needs to see how stock spells the chain.
  # A level is not a unit of work -- it is produced by downsampling the finished base --
  # and sending one here would pay the model to upscale art that is then thrown away.
  # FedCapital alone would have bought 28 of them.
  mapfile -t faces < <(stock_bases "$unit_dir")

  # `sheet=GxP` in target.conf: pack the units into GxG contact sheets with a P-pixel
  # gutter and upscale the SHEET, then slice the result back into per-unit ai/ files.
  #
  # This is a quality decision before it is a cost one. The app takes `megapixels` as an
  # INTEGER, so 1 is the floor -- and 1MP from a 64x64 icon is a 16x lift, the regime
  # where this model invents most (CLAUDE.md rule 4). Seven-by-seven at a 8px gutter is
  # 8 + 7*(64+8) = 512 exactly, and 512 at 4MP comes back 2048: a 4x lift, which is the
  # gentlest this project has used anywhere. Measured against the same icon taken the
  # 16x-then-downsample route, the sheet keeps stock's shapes where the individual
  # upscale restyles them. It also costs 1/49th as much, which is not the argument.
  #
  # The app rounds to an INTEGER scale factor -- scale = round(sqrt(mp*1e6/(w*h))) --
  # so a sheet edge and a megapixel target pin the lift exactly. Verify the returned
  # size rather than assuming it; a short sheet is sliced from whatever came back.
  # `|| true`: grep exits 1 when the key is absent, and under `set -euo pipefail` that
  # kills the script at the assignment -- silently, because nothing has been printed
  # yet. This is CLAUDE.md hard rule 3 in its other form (a failing grep, not a failing
  # glob) and it cost a full 136-file upscale that reported success and did nothing.
  sheet=$(grep -E '^sheet=' "$rec/target.conf" 2>/dev/null | head -1 | cut -d= -f2- | tr -d ' ' || true)
  if [ -n "$sheet" ] && [ "$REBLEND" = 0 ]; then
    grid=${sheet%x*}; gut=${sheet#*x}
    # Every unit must be the same size, or the cell arithmetic is meaningless.
    us=$(magick "$unit_dir/${faces[0]}" -format '%wx%h' info:)
    for f in "${faces[@]}"; do
      [ "$(magick "$unit_dir/$f" -format '%wx%h' info:)" = "$us" ] ||
        { echo "$t: $f is $(magick "$unit_dir/$f" -format '%wx%h' info:), not $us -- sheet mode needs one size" >&2; exit 1; }
    done
    cell=${us%x*}
    edge=$(( gut + grid * (cell + gut) ))
    per=$(( grid * grid ))
    todo=()
    for f in "${faces[@]}"; do
      base="${f%.*}"
      [ "$FORCE" = 0 ] && [ -s "$dir/ai/$base.png" ] && continue
      todo+=("$f")
    done
    if [ ${#todo[@]} -gt 0 ]; then
      nsheets=$(( (${#todo[@]} + per - 1) / per ))
      echo "$t: ${#todo[@]} unit(s) of ${cell}px -> $nsheets sheet(s) of ${edge}px at ${MP}MP"
      jsonl=$(mktemp); tmpd=$(mktemp -d)
      sh=0
      while [ $((sh * per)) -lt ${#todo[@]} ]; do
        args=(-size ${edge}x${edge} xc:black)
        i=0
        while [ $i -lt $per ] && [ $((sh*per+i)) -lt ${#todo[@]} ]; do
          r=$(( i / grid )); c=$(( i % grid ))
          args+=( '(' "$unit_dir/${todo[$((sh*per+i))]}" -alpha off ')'
                  -geometry "+$(( gut + c*(cell+gut) ))+$(( gut + r*(cell+gut) ))" -composite )
          i=$((i+1))
        done
        magick "${args[@]}" "PNG24:$tmpd/sheet$sh.png"
        printf '{"image":"%s","megapixels":%s,"output_format":"png","enhance_details":false,"enhance_realism":false}\n' \
          "$tmpd/sheet$sh.png" "$MP" >> "$jsonl"
        sh=$((sh+1))
      done
      "$HOME/.local/bin/belt" app run "$APP" --batch "$jsonl" --concurrency "$CONC" \
          --no-input --json 2>/dev/null \
        | grep '"task_id"' \
        | python3 -c '
import json, sys
for line in sys.stdin:
    r = json.loads(line)
    print(r["index"], r["status"], (r.get("output") or {}).get("image", ""))' \
        | while read -r idx status url; do
            [ "$status" = completed ] && [ -n "$url" ] \
              || { echo "  !! sheet $idx: $status" >&2; continue; }
            curl -sfL -o "$tmpd/out$idx.png" "$url" || { echo "  !! sheet $idx: download failed" >&2; continue; }
          done
      sh=0
      while [ $((sh * per)) -lt ${#todo[@]} ]; do
        if [ ! -s "$tmpd/out$sh.png" ]; then echo "  !! sheet $sh missing, its units skipped" >&2; sh=$((sh+1)); continue; fi
        ow=$(magick "$tmpd/out$sh.png" -format '%w' info:)
        sc=$(( ow / edge ))
        [ "$sc" -ge 1 ] || { echo "  !! sheet $sh came back ${ow}px, smaller than the ${edge}px input" >&2; sh=$((sh+1)); continue; }
        i=0
        while [ $i -lt $per ] && [ $((sh*per+i)) -lt ${#todo[@]} ]; do
          r=$(( i / grid )); c=$(( i % grid )); base="${todo[$((sh*per+i))]%.*}"
          magick "$tmpd/out$sh.png" -crop "$((cell*sc))x$((cell*sc))+$(( (gut + c*(cell+gut))*sc ))+$(( (gut + r*(cell+gut))*sc ))" \
              +repage "PNG24:$dir/ai/$base.png"
          i=$((i+1))
        done
        echo "  sheet $sh: ${edge} -> ${ow} (${sc}x), sliced"
        sh=$((sh+1))
      done
      rm -rf "$jsonl" "$tmpd"
    fi
  fi

  # --- 1. upscale every face that has no ai/ layer yet -------------------------------
  if [ -z "$sheet" ] && [ "$REBLEND" = 0 ]; then
    jsonl=$(mktemp); todo=()
    for f in "${faces[@]}"; do
      base="${f%.*}"
      [ "$FORCE" = 0 ] && [ -s "$dir/ai/$base.png" ] && continue
      # The upscaler takes a PNG, not a TGA. PNG24: for the usual reason -- see
      # textures/lib/common.sh. `-alpha off` is what makes that a plain channel drop rather than
      # a composite; the alpha is rebuilt from stock at build time by attach_alpha(),
      # and a mask must never go through a model that invents detail.
      magick "$unit_dir/$f" -alpha off "PNG24:$dir/ai/.in-$base.png"
      printf '{"image":"%s","megapixels":%s,"output_format":"png","enhance_details":false,"enhance_realism":false}\n' \
        "$dir/ai/.in-$base.png" "$MP" >> "$jsonl"
      todo+=("$base")
    done
    if [ ${#todo[@]} -gt 0 ]; then
      echo "$t: upscaling ${#todo[@]} face(s) at ${MP}MP"
      # --download is silently ignored in --json batch mode, so fetch the URLs here.
      # `index` is the 0-based line of the input JSONL and is the only reliable way to
      # map a result back to its face -- results arrive out of order.
      "$HOME/.local/bin/belt" app run "$APP" --batch "$jsonl" --concurrency "$CONC" \
          --no-input --json 2>/dev/null \
        | grep '"task_id"' \
        | python3 -c '
import json, sys
for line in sys.stdin:
    r = json.loads(line)
    print(r["index"], r["status"], (r.get("output") or {}).get("image", ""))' \
        | while read -r idx status url; do
            base="${todo[$idx]}"
            [ "$status" = completed ] && [ -n "$url" ] \
              || { echo "  !! $base: $status" >&2; continue; }
            curl -sfL -o "$dir/ai/$base.png" "$url" || echo "  !! $base: download failed" >&2
          done
    fi
    rm -f "$jsonl" "$dir"/ai/.in-*.png
  fi

  # --- 2. blend each AI layer back toward a plain Lanczos upscale --------------------
  # This is the invention dial. enhance_details/enhance_realism are NOT -- turning them
  # off makes invented structure sharper and so more visible, not less.
  for f in "${faces[@]}"; do
    base="${f%.*}"
    [ -s "$dir/ai/$base.png" ] || { echo "  !! $base: no ai/ layer, skipped" >&2; continue; }
    # The \n matters: `read` returns 1 at EOF without a trailing newline, and under
    # `set -e` that aborts the script between the upscale and the blend -- leaving a
    # paid-for ai/ layer and an empty src/.
    read -r w h < <(magick "$dir/ai/$base.png" -format '%w %h\n' info:)
    # `-alpha off` BEFORE the resize, not just PNG24: on the way out. Resizing an RGBA
    # image associates alpha and then un-associates it, which divides colour back out by
    # a near-zero alpha and blows the RGB up wherever the texture is transparent -- the
    # Lanczos layer for Mmoon came out at mean 137 against stock's 43. fit() then hides
    # it by scaling the whole plate down to match the mean, so the only symptom is a
    # correct average over a picture that is uniformly too dark.
    magick "$unit_dir/$f" -alpha off -filter Lanczos -resize "${w}x${h}!" \
        "PNG24:$LZ/lz-$base.png"
    magick "$LZ/lz-$base.png" "$dir/ai/$base.png" \
        -define compose:args="$blend" -compose blend -composite "PNG24:$dir/src/$base.png"
  done

  # --- 3. the ALPHA plate, only when target.conf says alpha=ai -----------------------
  # The standing rule is that alpha NEVER goes through the upscaler, because on every
  # texture this project had met before, alpha was a coverage mask: no texture, only
  # edges, which Lanczos resolves exactly, and a model that invents detail into a mask
  # invents holes in the object.
  #
  # A hull texture breaks that premise. Its alpha is a self-illumination map -- rows of
  # lit windows, deflector and nacelle glow -- which is picture content with real
  # high-frequency structure, and a 4x Lanczos smears a row of 1px window dots into a
  # bar. So this is opt-in per target, never the default, and the README records the
  # measurement that justified it on the target where it is set.
  #
  # It is a separate directory, not another file in src/: build_sky_faces requires
  # exactly one src/ image per stock face and an extra <base>.alpha.png would fail that
  # count check -- as a "needs 3 images, found 6" skip, which reads like a missing file.
  amode=$(grep -E '^alpha=' "$rec/target.conf" 2>/dev/null | head -1 | cut -d= -f2- | tr -d ' ' || true)
  if [ "$amode" = ai ]; then
    mkdir -p "$dir/.alpha-units" "$dir/src-alpha"
    aunits=()
    for f in "${faces[@]}"; do
      base="${f%.*}"
      [ "$(magick "$unit_dir/$f" -format '%[channels]' info:)" = "srgba 4.0" ] || continue
      # PNG24 of the extracted channel, for the reason in textures/lib/common.sh: a 1-channel
      # grey PNG reports mean.g = mean.b = 0 downstream.
      magick "$unit_dir/$f" -alpha extract "PNG24:$dir/.alpha-units/$base.png"
      aunits+=("$base")
    done
    if [ ${#aunits[@]} -eq 0 ]; then
      echo "$t: alpha=ai, but no unit is 32-bit -- nothing to do" >&2
    else
      if [ "$REBLEND" = 0 ]; then
        jsonl=$(mktemp); todo=()
        for base in "${aunits[@]}"; do
          [ "$FORCE" = 0 ] && [ -s "$dir/ai/$base.alpha.png" ] && continue
          printf '{"image":"%s","megapixels":%s,"output_format":"png","enhance_details":false,"enhance_realism":false}\n' \
            "$dir/.alpha-units/$base.png" "$MP" >> "$jsonl"
          todo+=("$base")
        done
        if [ ${#todo[@]} -gt 0 ]; then
          echo "$t: upscaling ${#todo[@]} alpha plate(s) at ${MP}MP"
          "$HOME/.local/bin/belt" app run "$APP" --batch "$jsonl" --concurrency "$CONC" \
              --no-input --json 2>/dev/null \
            | grep '"task_id"' \
            | python3 -c '
import json, sys
for line in sys.stdin:
    r = json.loads(line)
    print(r["index"], r["status"], (r.get("output") or {}).get("image", ""))' \
            | while read -r idx status url; do
                base="${todo[$idx]}"
                [ "$status" = completed ] && [ -n "$url" ] \
                  || { echo "  !! $base alpha: $status" >&2; continue; }
                curl -sfL -o "$dir/ai/$base.alpha.png" "$url" || echo "  !! $base alpha: download failed" >&2
              done
        fi
        rm -f "$jsonl"
      fi
      # alphafilter= governs THIS resize too, not just attach_alpha's. The knob is
      # named for the alpha path and has to mean the whole of it: the Lanczos layer the
      # AI plate is blended over is built here, and it is where the drift actually
      # comes from. 8472_passive2 is a 64px source at a 16x lift whose stock alpha means
      # 43.12; through Lanczos that layer reads 44.64, through Mitchell 43.43, through
      # Triangle 43.12 exactly. At blend=70 the AI plate (43.80) dominates and the
      # result squeaks under verify's tolerance of 1.0; at blend=35 the Lanczos layer
      # dominates and it does not. Same asymmetric ringing as fedpod10, one stage
      # upstream -- and it only shows up where the lift is large.
      afilt=$(grep -E '^alphafilter=' "$rec/target.conf" 2>/dev/null | head -1 | cut -d= -f2- | tr -d ' ' || true)
      [ -n "$afilt" ] || afilt=Lanczos
      for base in "${aunits[@]}"; do
        [ -s "$dir/ai/$base.alpha.png" ] || { echo "  !! $base: no ai/ alpha layer, skipped" >&2; continue; }
        read -r w h < <(magick "$dir/ai/$base.alpha.png" -format '%w %h\n' info:)
        magick "$dir/.alpha-units/$base.png" -filter "$afilt" -resize "${w}x${h}!" \
            "PNG24:$LZ/lza-$base.png"
        magick "$LZ/lza-$base.png" "$dir/ai/$base.alpha.png" \
            -define compose:args="$blend" -compose blend -composite \
            -colorspace Gray "PNG24:$dir/src-alpha/$base.png"
      done
      echo "$t: src-alpha/ has $(ls "$dir/src-alpha"/*.png 2>/dev/null | wc -l) plate(s)"
    fi
  fi

  n=$(ls "$dir/src"/*.png 2>/dev/null | wc -l)
  echo "$t: src/ has $n unit(s) at blend ${blend}%"
done
