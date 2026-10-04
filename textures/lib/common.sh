# Shared helpers. Sourced by textures/lib/puff.sh and textures/lib/sky.sh; never run directly.
#
# PARALLEL SAFETY: every function takes its scratch directory as an argument and
# writes nothing to a fixed path. The previous scripts used work/_s.png and
# work/_q0.png, which silently corrupt each other when two targets build at once --
# that is the single change that makes -j possible.

MAXGAIN=${MAXGAIN:-2.5}
FLOOR=${FLOOR:-0.15}          # unused since channel synthesis went absolute; see fit()

die () { printf 'error: %s\n' "$*" >&2; exit 1; }
note () { printf '  %s\n' "$*" >&2; }

# first existing spelling of a texture name; TGA case is inconsistent across the set.
# Tests, not `ls | head`: install's mip guard asks this thousands of times. (No stock
# texture exists in both spellings, so which is tried first changes nothing.)
tex_path () {
  if [ -e "$1.tga" ]; then printf '%s\n' "$1.tga"
  elif [ -e "$1.TGA" ]; then printf '%s\n' "$1.TGA"
  fi
  return 0
}

# Width of an image in pixels. A TGA's is the little-endian uint16 at byte 12 of its
# header, read with one `od` instead of starting ImageMagick: `a2tex install`'s mip
# guard asks for thousands of widths, and ImageMagick at ~8 ms a start made that guard
# about 100 s of every install (and so of every bench session). Anything else, or a
# header that cannot be read, goes to ImageMagick as before.
declare -gA TEXW=()
# preload_widths <dir>...: every TGA's width in those directories into TEXW, keyed by
# "<dir>/<name>" exactly as given, read by one process. img_width answers from it, and
# subshells inherit it, so the $(...) callers do too. Whoever overwrites a TGA after
# this updates its entry.
preload_widths () {
  local p w
  while IFS=$'\t' read -r p w; do TEXW[$p]=$w; done < <(python3 - "$@" <<'PY'
import os, struct, sys
for d in sys.argv[1:]:
    try:
        names = os.listdir(d)
    except OSError:
        continue
    for n in names:
        if n[-4:].lower() != '.tga':
            continue
        try:
            with open(os.path.join(d, n), 'rb') as f:
                h = f.read(14)
        except OSError:
            continue
        if len(h) == 14:
            print('%s/%s\t%d' % (d, n, struct.unpack_from('<H', h, 12)[0]))
PY
)
}

img_width () {
  local w
  [ -n "${TEXW[$1]:-}" ] && { printf '%s' "${TEXW[$1]}"; return 0; }
  case "$1" in
    *.tga|*.TGA)
      w=$(od -An -tu2 -j12 -N2 "$1" 2>/dev/null) && w=${w//[[:space:]]/} && [ -n "$w" ] &&
        { printf '%s' "$w"; return 0; } ;;
  esac
  magick identify -format '%w' "$1[0]" 2>/dev/null
}

# A single all-black channel plane the same size as $1, written to $2.
blank_channel () { magick "$1" -colorspace Gray -evaluate multiply 0 "PNG24:$2"; }

# Match $1 to reference $2, writing $3. $4 = scratch dir, $5 = output edge size.
# Honours MONOHUE. Echoes "<src mean> <result mean> <gain>".
fit () {
  local src=$1 ref=$2 dst=$3 tmp=$4 size=$5
  local s="$tmp/s.png" cur got
  # A reference written as a 1- or 2-channel greyscale PNG reports mean.g = mean.b = 0,
  # which silently makes the gain for those channels a no-op. Refuse it rather than ship
  # a texture that is right in red and untouched in green and blue.
  #
  # Test the CHANNEL COUNT, not the means. The means version -- mean.g==0 && mean.b==0
  # && mean.r>0 -- is a heuristic, and it has a real false positive: Gmneb2, the red
  # minimap nebula icon, is 107/0/0 on purpose. It is a perfectly valid 3-channel TGA
  # and the guard killed its build outright, so the target shipped 25 of 26 files with
  # no error beyond a blank column in the report.
  case "$(magick "$ref" -format '%[channels]' info:)" in
    gray*) die "$(basename "$ref"): reference is greyscale on disk -- write it with the PNG24: prefix" ;;
  esac

  # The PNG24: prefix is load-bearing. A greyscale intermediate (the puffs are
  # converted to grey before this) is stored as a 2-channel PNG, and %[fx:mean.g] then
  # reads exactly 0 -- so the channel-synthesis branch below "repairs" two channels
  # that were never broken, and the atlas comes out 28/38/38 instead of 28/28/28.
  # Neither -type TrueColor nor -colorspace sRGB fixes it; only forcing the on-disk
  # format does. Values are preserved exactly.
  magick "$src" -auto-orient -gravity center \
      -resize ${size}x${size}^ -extent ${size}x${size} "PNG24:$s"
  cur=$(magick "$s" -format '%[fx:255*mean]' info:)

  # fit=none: resize only. For a target whose src/ is NOT a picture of the stock file of
  # the same name -- LOADING, where the six tiles are re-cut from a panel widened beyond
  # stock, so LOADING1 now holds scenery stock never had. Matching each tile to its
  # namesake would give the six tiles six different gains and put a brightness step at
  # every join. Whatever tone matching is wanted is done on the whole picture, before
  # it is cut (models/loading-panel.sh).
  if [ "${FIT:-match}" = none ]; then
    magick "$s" -alpha off -type TrueColor "$dst"
    got=$(magick "$dst" -format '%[fx:int(255*mean)]' info:)
    echo "$cur $got 1"; return
  fi

  # MONOHUE: rebuild every channel from luminance using the reference's hue ratio.
  # Only valid when the stock texture is effectively single-hue -- it removes chroma
  # invention by construction, but would flatten real colour on a multi-hue texture.
  # MAXGAIN deliberately does not apply: Rec.709 grey weights blue at 0.114, so a
  # blue-dominant plate needs a gain near 3 just to reach its own blue mean.
  if [ "${MONOHUE:-0}" = 1 ]; then
    magick "$s" -colorspace Gray "$tmp/lum.png"
    local lum=$(magick "$tmp/lum.png" -format '%[fx:mean]' info:) k=0 c
    for c in r g b; do
      local t=$(magick "$ref" -format "%[fx:mean.$c]" info:)
      if [ "$(python3 -c "print(1 if $t<=0 else 0)")" = 1 ]; then
        blank_channel "$tmp/lum.png" "$tmp/c$k.png"; k=$((k+1)); continue
      fi
      local wp=$(python3 -c "print(round(100/($t/$lum),4) if $lum>0 else 100)")
      magick "$tmp/lum.png" -level "0%,${wp}%" "PNG24:$tmp/c$k.png"; k=$((k+1))
    done
  else
    # Per-channel match. Fixes hue drift and density together, and generalises
    # matching the grey mean. A channel at ~0 cannot be scaled up (0 x anything = 0),
    # so below FLOOR of the strongest it is rebuilt FROM the strongest instead --
    # which keeps blacks black, unlike adding a constant.
    local mr mg mb strongest smax k=0 c
    mr=$(magick "$s" -format '%[fx:255*mean.r]' info:)
    mg=$(magick "$s" -format '%[fx:255*mean.g]' info:)
    mb=$(magick "$s" -format '%[fx:255*mean.b]' info:)
    read strongest smax < <(python3 -c "
v={'R':$mr,'G':$mg,'B':$mb}; k=max(v,key=v.get); print(k, v[k])")
    for c in R G B; do
      local t=$(magick "$ref" -format "%[fx:255*mean.${c,,}]" info:)
      local v; case $c in R) v=$mr;; G) v=$mg;; B) v=$mb;; esac
      # Synthesise only when scaling genuinely cannot get there: the channel is
      # effectively zero (0 x anything = 0), or the gain needed is beyond MAXGAIN, where
      # a capped stretch bands badly. The old test -- "below FLOOR of the STRONGEST
      # channel" -- is relative, and so misfires on any strongly single-hue plate: the
      # Borg sky is G69/B3, whose perfectly real blue is 5% of green, and it threw away
      # the source's actual blue detail to rebuild it from green. Dimming a faint
      # channel (gain < 1) is always safe; only the lifting direction is dangerous.
      # A target channel of exactly zero must be ANNIHILATED, not left alone. The old
      # code computed g = min(cap, t/v) = 0 and then fell back to "-level 0%,100%" --
      # the identity -- because the only guard was against dividing by zero. MbgOmega's
      # stock blue is exactly 0 on all six faces and the build passed its source blue
      # straight through at 33-88/255, turning an amber sky violet. This is the one case
      # where the fallback has to be black, not unchanged.
      if [ "$(python3 -c "print(1 if $t<=0 else 0)")" = 1 ]; then
        note "$(basename "$dst"): ${c} target mean is 0, channel blanked"
        blank_channel "$s" "$tmp/c$k.png"; k=$((k+1)); continue
      fi
      # Synthesise ONLY when scaling cannot get there: the channel is exactly zero
      # (0 x anything = 0), or the gain needed exceeds MAXGAIN, where a capped stretch
      # undershoots and bands. An absolute "v < 1.0" floor was tried and is also wrong,
      # for the same reason the original "below 0.15 of the strongest" was: it fires on
      # the DIMMING direction, which is always safe. MbgKlin4's blue is v 0.39 -> t 0.32,
      # a gain of 0.82, and the floor was throwing away the real blue to rebuild it from
      # red. Only lifting is dangerous.
      local use=$c
      if [ "$(python3 -c "print(1 if ($v <= 0 or $t/$v > $MAXGAIN) else 0)")" = 1 ]; then
        # Two decimals on the target, not ${t%%.*}: truncating 0.22 to "0" made this
        # line read exactly like the genuine zero-channel case below, which is a
        # different condition with a different remedy.
        note "$(basename "$dst"): ${c} mean $(printf '%.2f' "$v") -> $(printf '%.2f' "$t") unreachable, synthesised from ${strongest}"
        use=$strongest; v=$smax
      fi
      local wp=$(python3 -c "
v,t,cap=$v,$t,$MAXGAIN
g = 1.0 if v<=0 else min(cap, t/v)
print(round(100/g,4) if g>0 else 100)")
      magick "$s" -channel "$use" -separate -level "0%,${wp}%" "PNG24:$tmp/c$k.png"; k=$((k+1))
    done
  fi

  magick "$tmp/c0.png" "$tmp/c1.png" "$tmp/c2.png" -combine -colorspace sRGB \
      -alpha off -type TrueColor "$dst"
  got=$(magick "$dst" -format '%[fx:int(255*mean)]' info:)
  echo "$cur $got $(python3 -c "print(round($got/max($cur,0.001),3))")"
}

# Bits per pixel of a TGA, read out of its header. 24 or 32 for everything in this game.
tga_bpp () { python3 -c "import sys; print(open(sys.argv[1],'rb').read(17)[16])" "$1"; }

# Write a stock-format TGA: type 2, uncompressed, and THE SAME BIT DEPTH as the file it
# replaces. $3, when given, is that stock file: the output copies its row origin and its
# depth. Stock is not uniformly bottom-up -- 82 of the 135 skybox faces are top-down
# 0x20 -- so "always bottom-up" is a guess where "same as the file I am replacing" is a
# fact, and the same argument applies to depth: 1113 of the 2118 textures are 32-bit and
# an unconditional `-alpha off` silently throws their alpha away. With no $3 the old
# 24-bit behaviour stands, which is what every nebula target relies on.
write_tga () {
  local depth=24
  [ -n "${3:-}" ] && [ -f "$3" ] && depth=$(tga_bpp "$3")
  if [ "$depth" = 32 ]; then
    # -type TrueColorAlpha forces 4 channels even when the source PNG has none, so the
    # header matches stock whether or not attach_alpha had anything to attach.
    magick "$1" -alpha set -type TrueColorAlpha -compress None "$2"
  else
    magick "$1" -alpha off -type TrueColor -compress None "$2"
  fi
  if [ -n "${3:-}" ] && [ -f "$3" ]; then "$ROOT/tools/bottomup.py" --like "$3" "$2"
  else "$ROOT/tools/bottomup.py" "$2"; fi
}

# Give the upscaled RGB in $1 the alpha of stock file $2, resized to match, writing $3.
# $4 is a scratch dir. $5, when given and non-empty, is a PRE-UPSCALED alpha plate to
# use instead of stock's -- see the alpha=ai paragraph below. A 24-bit stock is a
# straight copy, whatever $5 says.
#
# By default the alpha is upscaled with Lanczos and NEVER sent through the generative
# upscaler. Alpha in this game is usually a mask -- mdmoon's is a network of dilithium
# fissures, Mmoon's is a hard cutout disc the engine alpha-thresholds -- and a model
# that invents plausible detail into a mask invents holes in the object. There is also
# nothing to recover: a mask has no texture, only edges, and Lanczos resolves an edge
# exactly.
#
# A HULL texture is the exception, and the reason $5 exists. Its alpha is not coverage
# but a self-illumination map -- rows of lit windows, deflector and nacelle glow -- so
# it is picture content with real high-frequency structure, and Lanczos at 4x smears a
# row of 1px window dots into a bar. `alpha=ai` in target.conf sends it through the same
# model as the colour, at the same blend, and upscale-stock.sh leaves the result in
# src-alpha/. It is opt-in per target and must stay that way: on a coverage mask it is
# the mistake the paragraph above describes.
#
# RGB and alpha are resized SEPARATELY on purpose. Resizing an RGBA image associates
# alpha, which darkens colour wherever alpha is low -- correct for transparency, wrong
# for a mask that means something else, which is what alpha means on most of the hull
# textures this will eventually meet.
attach_alpha () {
  local rgb=$1 stock=$2 dst=$3 tmp=$4 plate=${5:-}
  # alphafilter= in target.conf, default Lanczos so every target built before this
  # existed is unchanged. Lanczos rings, and on a mask that is mostly zero with small
  # isolated opaque blobs the ringing CLIPS ASYMMETRICALLY: the undershoot below 0 has
  # nowhere to go, the overshoot survives, and the channel mean rises. fedpod10 (91.1%
  # zero, 8.5% opaque, 13 distinct values) drifts 22.24 -> 23.36 at 4x, past verify.py's
  # tolerance of 1.0, and the halo it comes from would render as a faint rim around
  # every blob. Non-ringing filters hold it: Mitchell 22.43, Triangle 22.24 exactly.
  #
  # Not made automatic on purpose. The drift is a property of one texture's geometry
  # rather than of masks as a class -- Fbee, fdata, Ffreight and mdmoon are all masks
  # and all drift under 0.16 through Lanczos -- and auto-switching would silently
  # rewrite targets that are already shipped and verified.
  local af=${AFILTER:-Lanczos}
  if [ "$(tga_bpp "$stock")" != 32 ]; then cp "$rgb" "$dst"; return; fi
  local w h; read -r w h < <(magick "$rgb" -format '%w %h\n' info:)
  local a="$tmp/alpha-$(basename "$dst")"
  # A 32-bit TGA whose descriptor declares ZERO alpha bits is 24-bit colour in a 32-bit
  # container -- the fourth byte is padding, not coverage. ImageMagick reads it as
  # opaque, so -alpha extract returns 255 where stock stores 0. Reproduce the padding.
  # This outranks $5: a padding byte is not a picture and there is nothing to upscale.
  local pad; pad=$("$ROOT/tools/tgapad.py" "$stock" || true)
  if [ -n "$pad" ]; then
    magick -size "${w}x${h}" "canvas:$(printf '#%02x%02x%02x' "$pad" "$pad" "$pad")" "PNG24:$a"
  elif [ -n "$plate" ] && [ -f "$plate" ]; then
    # -resize is a no-op at matching size, and the guard against a plate built at a
    # different `size=` than the current build is worth more than skipping it.
    magick "$plate" -filter "$af" -resize "${w}x${h}!" "PNG24:$a"
  else
    magick "$stock" -alpha extract -filter "$af" -resize "${w}x${h}!" "PNG24:$a"
  fi
  magick "$rgb" -alpha off "$a" -compose CopyOpacity -composite "PNG32:$dst"
}

# The stock spelling of mip level $2 of the texture whose stock BASE file is $1, printed
# bare (no extension); empty if there is no such level. $3, when "path", prints the stock
# file's full path instead of the name.
#
# A hand-authored chain is nearly always <base>_<N>, but not always. fcruise1_B's chain
# is fcruise1_B1/fcruise1_B2 and fresearch's is fresearch1/fresearch2 -- no underscore.
# Both were confirmed by box-downsampling the base and measuring RMSE against the
# sibling: 0.022/0.026 and 0.033/0.054, inside the 0.017-0.067 band that the known-good
# underscore chains occupy (fassault_1 0.017, fresear_1 0.067).
#
# This matters twice over. The engine looks a chain up BY NAME, so a level written as
# fcruise1_B_1 is a file nothing reads -- and it leaves stock's 128px fcruise1_B1 sitting
# under a 1024px base, which is precisely the invalid chain that crashed the Klingon
# campaign. And the guard in `a2tex install` globs <base>_[0-9]*, so it does not see
# these chains at all and would wave that install straight through.
#
# The level's WIDTH is what disambiguates, which is why this cannot be a regex over
# names. FluidicRift2 is 128 beside a 256 FluidicRift: level 2 by name, level 1 by size,
# and therefore not a mip of it at all. The width test rejects it for the right reason.
mip_name () {
  local stock=$1 lvl=$2 mode=${3:-name}
  local d b bw want cand f w
  d=${stock%/*}; b=${stock##*/}; b=${b%.*}
  bw=$(img_width "$stock") || return 0
  want=$(( bw >> lvl ))
  for cand in "${b}_${lvl}" "${b}${lvl}"; do
    f=$(cd "$d" && tex_path "$cand"); [ -n "$f" ] || continue
    w=$(img_width "$d/$f") || continue
    [ "$w" = "$want" ] || continue
    if [ "$mode" = path ]; then printf '%s' "$d/$f"; else printf '%s' "$cand"; fi
    return 0
  done
}

# Siblings of the stock base $1 that are NAMED like mip levels but are not levels of it,
# given that levels 1..$2 were validated. Prints one bare name per line.
#
# Stock ships irregular ones. Fsensor_B is 128x128 and so are Fsensor_B_1 and
# Fsensor_B_2; FpremNew_B is 256 with a 128 _1 (a real level) and a 128 _2 (not one).
# The width test in mip_name is right to refuse to call these a chain -- but "not a
# chain" must not quietly become "nothing to worry about", because upscaling the base
# then leaves a stale 128px sibling beside a 1024px base under a name the engine may
# well still read. That is the same shape as the bug that crashed the Klingon campaign,
# and here it would slip through the part of the guard that only counts valid levels.
#
# So: enumerate them, and let the caller refuse and make a human look.
mip_strays () {
  local stock=$1 levels=$2
  local d b claimed lvl f n
  d=${stock%/*}; b=${stock##*/}; b=${b%.*}
  claimed=" "
  for ((lvl=1; lvl<=levels; lvl++)); do claimed+="$(mip_name "$stock" "$lvl") "; done

  # WHICH SPELLINGS can be a stray depends on how this texture's own chain is spelled.
  # The underscore form always can. The bare form can only when level 1 is itself bare,
  # i.e. this texture genuinely uses that convention -- otherwise <base><digit> is just
  # a different texture whose name happens to start with this one's. fdestroy2 is the
  # Sabre Class, not a stray level of fdestroy (the Defiant), and flagging it would make
  # the guard cry wolf on an ordinary target.
  local globs=("$d/$b"_[0-9].tga "$d/$b"_[0-9].TGA)
  [ "$(mip_name "$stock" 1)" = "${b}1" ] && globs+=("$d/$b"[0-9].tga "$d/$b"[0-9].TGA)
  for f in "${globs[@]}"; do
    [ -e "$f" ] || continue
    n=$(basename "$f"); n=${n%.*}
    case "$claimed" in *" $n "*) continue;; esac
    printf '%s\n' "$n"
  done
}

# The BASE textures in a target's stock/ directory, one filename per line -- that is,
# everything that is not a hand-authored mip level of something else in there.
#
# stock/ carries the levels because gen_mips can only name its output the way the
# engine looks it up if it can see how stock spells the chain (fcruise1_B1, not
# fcruise1_B_1). But a level is never a unit of work: build_sky_faces wants one src
# image per base, and upscale-stock.sh would otherwise send all 28 of FedCapital's
# levels through a PAID model to produce art that gen_mips then overwrites by
# downsampling the base. One definition, used by both, so they cannot disagree about
# what a target contains.
stock_bases () {
  local d=$1
  local -a all=() lvls=() out=()
  mapfile -t all < <(cd "$d" && ls 2>/dev/null | sort)
  [ ${#all[@]} -gt 0 ] || return 0
  # Cheap name test first: a target with no levels at all -- every UI and skybox target
  # -- must not pay a `magick identify` per file to discover that.
  if printf '%s\n' "${all[@]}" | grep -qE '[0-9]\.(tga|TGA)$'; then
    local f k m b drop
    for f in "${all[@]}"; do
      for ((k=1; k<=12; k++)); do
        m=$(mip_name "$d/$f" "$k"); [ -n "$m" ] || break
        lvls+=("$m")
      done
    done
    for f in "${all[@]}"; do
      b=${f%.*}; drop=0
      for m in ${lvls[@]+"${lvls[@]}"}; do [ "$b" = "$m" ] && { drop=1; break; }; done
      [ "$drop" = 1 ] || out+=("$f")
    done
  else
    out=("${all[@]}")
  fi
  printf '%s\n' ${out[@]+"${out[@]}"}
}

# Rebuild a hand-authored mip chain under $1: $2 = out dir, $3 = texture name,
# $4 = stock file (for depth and origin), $5 = number of levels, $6 = scratch dir.
#
# Each level is EXACTLY half the previous -- the API requires it, and shipping a base
# without rebuilding the chain crashed the game (see the mip-chain section of README.md).
#
# Box, not Lanczos. An exact power-of-two box filter is a pure area average, so it
# preserves the mean -- and with additive blending the mean IS the light contributed, so
# a filter that dims the small levels makes the object fade as the camera pulls back. It
# is also far gentler across a 2x2 atlas's quadrant boundaries than a ringing filter.
#
# Deliberately NO peak-lift pass. Stock's authored chains do lift the peak slightly at
# their smallest levels (16px: 228 against a box filter's 217), but reproducing that as a
# per-level -level compounds: it multiplied the mean at every step and drifted 33 -> 37
# over four levels where stock holds 33. The lift only matters at sizes these chains
# never reach -- a 1024 base bottoms out at 64px, stock's FIRST mip -- so the honest
# filter is the plain average.
#
# Colour and alpha are downsampled separately, for the reason given in attach_alpha.
gen_mips () {
  local base=$1 out=$2 name=$3 stock=$4 levels=$5 tmp=$6
  local lvl=1 prev="$base" d has_a mname mstock
  # levels=auto: however many stock ships for THIS texture. A single mips=N per target
  # forces every texture in it to share a chain depth, and stock does not oblige --
  # fdestroy has two levels and its Borg variant fdestroy_b has none, Fsensor has four.
  # Splitting a faction into one target per chain depth would multiply the number of
  # hand-set knobs, and a wrong one here is a crash rather than a bad-looking texture.
  if [ "$levels" = auto ]; then
    levels=0
    while [ "$levels" -lt 12 ] && [ -n "$(mip_name "$stock" $((levels+1)))" ]; do
      levels=$((levels+1))
    done
  fi
  d=$(magick "$base" -format '%w' info:)
  while [ "$lvl" -le "$levels" ]; do
    d=$((d/2))
    # Each level is matched to ITS OWN stock file, not to the base, for both its name
    # and its header -- and the two can differ from the base independently.
    #
    # Name: see mip_name. fcruise1_B's chain has no underscore and writing _1/_2 would
    # produce files the engine never reads.
    #
    # Header: fresearch is a 32-bit base (desc 0x08) whose two levels are 24-bit
    # (desc 0x00). Taking depth from the base wrote 32-bit mips over a 24-bit stock
    # chain, and nothing caught it -- verify.py checks headers per BASE and skips the
    # levels. Falling back to the base is only for a target whose stock/ predates
    # carrying its chain; it reproduces the old behaviour exactly.
    mname=$(mip_name "$stock" "$lvl"); [ -n "$mname" ] || mname="${name}_${lvl}"
    mstock=$(mip_name "$stock" "$lvl" path); [ -n "$mstock" ] || mstock=$stock
    has_a=0; [ "$(tga_bpp "$mstock")" = 32 ] && has_a=1
    if [ "$has_a" = 1 ]; then
      magick "$prev" -alpha off -filter Box -resize ${d}x${d}! "PNG24:$tmp/mrgb$lvl.png"
      magick "$prev" -alpha extract -filter Box -resize ${d}x${d}! "PNG24:$tmp/ma$lvl.png"
      magick "$tmp/mrgb$lvl.png" "$tmp/ma$lvl.png" -compose CopyOpacity -composite \
          "PNG32:$tmp/mip$lvl.png"
    else
      # A 24-bit level under a 32-bit base: drop alpha here rather than letting
      # write_tga do it, so the Box average that feeds the NEXT level is taken from the
      # same pixels the file actually holds.
      magick "$prev" -alpha off -filter Box -resize ${d}x${d}! "PNG24:$tmp/mip$lvl.png"
    fi
    write_tga "$tmp/mip$lvl.png" "$out/${mname}.tga" "$mstock"
    prev="$tmp/mip$lvl.png"; lvl=$((lvl+1))
  done
}
