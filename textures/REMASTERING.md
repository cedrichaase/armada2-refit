# Remastering the rest of the textures

What any texture that is not a nebula needs, and why. Read `README.md` first for the
engine reference and `CLAUDE.md` for the working rules. For **what is left**, read
`INVENTORY.md`, which is generated from the game directory — done, could be done and
should not be done, each with its reason. This file is the reasoning those categories
rest on; re-run the inventory rather than quoting its numbers.

---

## The set, measured

`Textures/RGB/` — flat, **2115 `.tga` files, 196 MB** (byte total; `du` says 205 MB,
which is block slack across 2115 small files), plus three strays
(`StarTrek.pal`, `Mgalaxy.1`, `pspbrwse.jbf`). TGA here is uncompressed, so the
on-disk size *is* the decoded size.

| | files | decoded |
|---|---|---|
| base textures | 1333 | 193 MB |
| mip siblings (`name_1`, `name_2`, …) | 782 | 12 MB |

Every file is TGA image type 2, uncompressed, no colour map. Sizes are almost entirely
powers of two: 64x64 (829), 256x256 (686), 128x128 (524), then a long tail down to 8x8
and a handful of non-square (192x32, 256x128, 160x32). **Exactly one 512x512 exists in
stock** — `WshladSW.tga`, a weapon texture — and that fact matters: see the 256x256 UI
ceiling below.

Extension case is split 1795 `.tga` / 320 `.TGA` and is **not** consistent within a set.

---

## Five things that break a naive mass upscale

### 1. More than half the textures have a real alpha channel

**1113 of 2115 are 32-bit** — 611 base textures and 502 mips. The alpha is genuinely
used, not a padded constant: graded values with structure, not just 0/255.

    magick F.tga -alpha extract -format '%[fx:int(255*mean)] %[fx:int(255*minima)] %[fx:int(255*maxima)] %[fx:int(255*standard_deviation)]\n' info:
    # min=255 -> alpha unused. high sd -> binary mask. otherwise -> graded

What the channel *means* varies by class. On the moons it is a mask — a fissure network,
a cutout disc. On hulls it is usually a **self-illumination map**: on the Sovereign
(`Fbattle`, `FEntE`, `Fbattle_b`) 81% exact zero with 234 distinct values above it,
holding rows of lit windows, the deflector and the nacelle glow, co-registered with the
windows painted into the RGB. That is picture content, and the opt-in `alpha=ai`
upscales it like colour. `textures/tools/classify-alpha.py` tells the two apart; the
test that works — is the RGB painted where alpha is zero? — is in the Federation hull
section of `README.md`, with the two that do not.

The rules, which any tool here must keep:

- detect 32-bit input and keep 4 channels — `write_tga()` reads the depth off stock,
- **never send alpha through the upscaler *when the alpha is a mask*.** A mask has no
  texture, only edges; Lanczos resolves an edge exactly, and a model that invents
  plausible detail into a mask invents holes in the object. The test is what the channel
  *means*, not that it is the fourth one: a hull night-lights map is a picture and the
  opt-in `alpha=ai` upscales it, measured at 0.001% invented light. Default stays off,
- **resize colour and alpha separately.** Resizing an RGBA image associates alpha and
  then un-associates it, dividing colour back out by a near-zero alpha. `Mmoon`'s
  Lanczos layer came out at mean 137 against stock's 43 this way, and `fit()` hid it by
  scaling the whole plate down to make the mean match — so the only visible symptom was
  a picture that was uniformly too dark. Add `-alpha off` *before* the resize, not
  `PNG24:` after it,
- **copy the stock descriptor byte whole, not just the origin bit.** At 32 bpp its low
  nibble is the alpha bit count, and stock is inconsistent there: the moons carry `0x08`
  and `earth.tga` carries `0x00` (and its fourth byte is padding — `tgapad.py`),
- never apply `MONOHUE` to an alpha-bearing texture without checking what alpha means.

### 2. There are 363 hand-authored mip chains, and getting one wrong CRASHES the game

782 files are mip levels with strictly descending sizes — `Wborgbore_1` (64), `_2` (32),
`_3` (16). Each level is **exactly half** the one above; that is not a convention, it is
what the graphics API requires of a mip chain. Installing a 1024x1024 `Mnebula2` over
its stock 64/32/16/8 mips crashed Armada II outright, in the Klingon campaign, whenever
the ion nebula came on screen: an invalid chain hard-fails rather than misbehaving
quietly.

They must be **regenerated, not ignored**, and they must not themselves be sent to an
upscaler:

- **Downsample with a Box filter at exact powers of two.** It is a pure area average, so
  it preserves the mean — which for an additive texture is the light contributed. A
  filter that dims the small levels makes the object fade as the camera pulls back.
- **Do not try to reproduce an artist's peak lift per level.** Stock's chain does lift
  the peak at its smallest levels (16px: 228 against a box filter's 217), but applying
  that as a per-level `-level` compounds — it drifted the mean 33 -> 37 over four levels
  where stock holds 33, and lifted quadrant edges off black.
- **Validate before writing anything.** Mips sort before their base (`X_1.tga` <
  `X.tga`), so a per-file check installs the mips and only then refuses the base,
  leaving a stock base under upscaled mips — the same invalid chain, inverted. `a2tex`
  does a pre-flight pass over the whole target; so must any general tool. The same
  applies to *revert*: restoring only the base is what the crash looks like too.

**Detection is not a regex over names.** Two real chains are spelled without the
underscore — `fcruise1_B1`/`_B2` under `fcruise1_B`, and `fresearch1`/`2` under
`fresearch` — so `^.*_\d+\.(tga|TGA)$` cannot see them, and a guard built on it waves
through exactly the install it exists to refuse. `mip_name()` in
`textures/lib/common.sh` tries both spellings per level and **validates by width**. The
width test is not a refinement, it is the whole thing — `FluidicRift2` is 128 beside a
256 `FluidicRift`, which is level 2 by name and level 1 by size, and a name-only rule
takes it and breaks a texture that was never a chain. Stock also ships siblings named
like levels that are not levels at any size (`Fsensor_B_1`, `FpremNew_B_2`);
`mip_strays()` finds those and the install is refused rather than leaving one stale
under a bigger base.

A chain can also **change bit depth partway down**: `fresearch` is a 32-bit base over
two 24-bit levels. Each level is matched to its own stock file for header and name, not
to the base.

The detection does **not** match the font atlases (`FontFinal4_10a.tga`), which is
correct — those are point sizes, not mips.

### 3. A UI sprite over 256x256 crashes the game

`@tmaterial=interface` textures at 512x512 kill the process at the cinematic-to-HUD
transition, in a `rep movsd` inside `Armada2.exe` -- a memcpy into a buffer that is too
small. Exactly one stock texture in the whole game is 512 (`WshladSW`, a weapon) and no
stock interface sprite exceeds 256, so the ceiling is almost certainly a scratch buffer
sized in 2001 for the largest UI texture that shipped.

**3D model textures are not affected** -- 2048 skybox faces, 2048 planets and a 4096
atlas run fine -- so do not generalise this into a global texture cap. It is the sprite
path.

`maxsize=N` in `target.conf` enforces it per target, and `a2tex install` refuses any
interface texture over 256 whatever target builds it (`interface-textures.py`): a
texture's target does not tell you what the engine thinks it is (`borgUI3`, in
`README.md`).

The general lesson is worth more than the number: **before upscaling a class, census the
largest size stock ships for that class.** One `magick -format '%w'` over the set finds
it before it costs a crash, and it is the same question that the mip-chain guard
answers in a different form -- what shape is the engine expecting?

A related limit that is about size, not quality: **doubling the fog-of-war and minimap
textures** (`Gfog`, `Gshroud`, `Gminimap`, `Gmabelt`, `Gmneb1`-`5`) **froze the game**
as the HUD appeared. They upscale fine as images; they may not change size. `UImid`,
which carries them, is `install=no`.

### 4. Some textures must never be hallucinated into

**Fonts are forbidden by the engine, not just by taste.** All 8 font atlases declare

    @tmaterial=font    #No filtering, ever.

so glyph textures are sampled nearest-neighbour. An 8x atlas means the GPU picks one
texel out of each 8x8 block: not a sharper glyph, a point-sampled one, with thin strokes
dropping out depending where the sample lands. Strictly worse than stock.

The deeper reason is that **the premise of this whole project does not hold for them.**
Everything here rests on a texture being starved — a nebula quadrant spreads 64 texels
over a thousand screen pixels, a 16x deficit worth filling. Fonts ship as a separate
atlas per point size (10, 12, 13, 15, 16, 19, 20, 24) precisely because glyphs are drawn
near 1:1 with screen pixels. There is no magnification to fight, so upscaling only adds
resampling error. And unlike gas, where "plausible" is indistinguishable from "correct",
a glyph has a right answer: an invented stroke turns an 8 into a B.

**UI is not in that class.** It uses `@tmaterial=interface`, with no filtering
prohibition; hard-edged alpha is carried across by `attach_alpha()` and never sent
through the model, and the fixed drawn size sets the target resolution rather than
arguing against having one. The real bar for UI is the 256x256 ceiling above. The UI
section of `README.md` has how it is done.

> **Tooling trap:** the 8 font `.spr` files contain enough non-text that `file` reports
> `data`, and **plain `grep` skips them silently** — a survey of `@tmaterial` values
> across `Sprites/` reports *zero* font materials until it is re-run with `grep -a`.
> Same family as `strings` lying about the `.bzn` backgrounds. Use `-a` on anything in
> `Sprites/`.

A generative upscaler invents plausible detail. That is the point for gas and hull
plating, and destructive for anything carrying glyphs or exact shapes.

The hull class has an awkward third case: a texture that is **not** a font atlas but
carries a few glyphs anyway. The *Enterprise*'s `NCC-1701-E` is 3 texels tall in stock —
illegible, so "leave it stock" buys nothing, and no filter can recover an aperture that
was never resolved. The model reconstructs 8 of 10 glyphs correctly and closes both Cs
into Os. The answer is neither of the usual two: `textures/tools/fix-enterprise-registry.py`
cuts the two apertures back by hand and touches nothing else. Deliberately a one-off — no
other hull texture carries type anyone can name.

Left stock, each for its own reason:

- **12 font atlases** (`FontFinal4_*`) — letterforms, forbidden by the engine.
- **20 cursors** (`curs_*`) — deliberate pixel art, and the engine magnifies them by a
  constant, so more texels only draw a bigger cursor (`README.md`, UI section).
- **`colors`** — an 8x8 colour *lookup table*. Interpolating it blends the cells.
- **`logos`** — third-party trademarks on a splash screen shown once.
- **`MBuild`** — line-art build overlay, `add_nomipf`, and the only UI texture with a
  mip chain.
- **`gminicon`, `gminisys`** — 32x32, drawn at minimap blip scale.
- **bump and normal maps** — they encode vectors, not colour (`INVENTORY.md`).

Two that look as though they belong on that list and do not: **wireframes**
(`*WIREFRAME*`) are not line art but scattered small ship-part silhouettes on
transparent, and a 4x upscale reproduces them faithfully; and **`Gmneb*`** are flat
single-hue blobs — so flat that `Gmneb2` is exactly 107/0/0, which once broke `fit()`'s
greyscale guard. (They stay stock for the size reason in section 3, not for this one.)

### 5. Disk cost is severe because TGA is uncompressed

Upscaling every base texture 4x linear (16x area) takes the set from 196 MB to about
**10 GB**. At 2x linear it is 2.5 GB. The engine reads dimensions from the header and
does not care; load time will. This is why hulls are built at 1024, not 2048.

**Memory is a separate question.** The disk total is not what the process holds:
`ART_CFG.h`'s `ST3D_PRELOAD_TEXTURES = 1` preloads per game rather than across the
directory, and under the DXVK chain the texture bodies live in VRAM (8 GB on this
machine). The quantity that matters is the per-match working set — roughly two
factions' hulls plus the map and the UI — and it has never been measured. See "2.27 GB
resident was never a memory figure" in `README.md`.

This argues for **triage rather than a blanket pass**.

---

## What generalises from the nebula work

**The target pattern scales.** A recipe in `textures/targets/<NAME>/` (`target.conf`,
`stock.sha256`) and a work directory in `$A2_DATA/textures/<NAME>/`, with one driver and
`-j`, is how to add textures in bulk — a new class needs a new `kind` in
`textures/lib/`, not a new script. Parallel safety depends on nothing in
`textures/lib/` writing to a fixed path.

**Match the stock file, per channel.** `textures/lib/common.sh`'s `fit()` scales each of
R, G, B so its mean equals the stock file's. This fixes hue drift and density in one step
and works for any texture, generated or upscaled.

**Channel synthesis for a dead channel.** Zero times anything is zero; a source with
`mean.r` of 0 can never be scaled up to a target. When a channel's source mean is 0, or
the gain it needs exceeds `MAXGAIN`, rebuild it *from* the strongest channel. Keeps
blacks black, unlike adding a constant. Never test with a ratio or an absolute floor on
the source alone: both fire on *dimming*, which is always safe (`README.md`, "Channel
synthesis"). And a *target* mean of 0 blanks the channel (`MbgOmega`'s blue).

**`MONOHUE=1` where it is valid.** Rebuild all channels from luminance using the stock
hue ratio. Valid **only** when the stock texture is effectively single-hue — test it
with `textures/tools/huespread.py`, not the raw HSL hue standard deviation, which the
near-black areas dominate. Under about 25° is single-hue and `MONOHUE` is safe; it then
removes chroma invention by construction. On a multi-hue hull texture it would flatten
real colour and is wrong.

**The blend dial.** Blend the AI upscale back toward a plain Lanczos upscale to
attenuate invented detail uniformly:

    magick lanczos.png ai.png -define compose:args=35 -compose blend -composite out.png

35 for skyboxes and planets, which fill the screen; 70 for hulls, which are drawn small.
Set it per texture *class*, on a render at the object's real on-screen size, not at 1:1.

**Quantify invention rather than eyeballing it.** Two measures that caught real problems:

    # chroma invention: compare against the same measure on the stock file
    magick F.tga -fx "abs(r-g)" -format '%[fx:1000*mean]\n' info:
    # high-frequency energy
    magick F.tga -colorspace Gray -write mpr:x +delete \( mpr:x \) \( mpr:x -blur 0x3 \) \
        -compose difference -composite -format '%[fx:1000*mean]\n' info:

For `MBG02`: stock R-G deviation 1.89, Lanczos 1.97, AI upscale **5.90**. That is the
number that proved the upscaler was inventing colour. For hulls,
`textures/tools/measure-invention.sh` measures both directions at the on-screen size —
and ranks, it does not decide.

**`enhance_details` / `enhance_realism` are a trap.** Turning them *off* makes invented
filaments **more** visible, not less, because the output is sharper — enhance-on merely
blurs its own inventions. Do not reach for them as a quality dial; use the blend.

**4096x4096 works.** Confirmed in game as a 48 MB TGA. Texture size is not the
constraint for 3D textures; disk and load time are.

**Copy the row origin from the file you are replacing, don't assume it.** ImageMagick 7
cannot write bottom-up TGAs and silently ignores every `-define tga:image-origin=`
spelling, so the origin is always a post-pass — but "stock is bottom-up" is **false**.
Among the skybox faces alone, 53 are `0x00` and 82 are `0x20`, mixed within a single
set. `textures/tools/bottomup.py --like <stock> <built>` makes the question go away.
Never generalise a format fact from a sample of one.

**Prefer upscaling a texture's own stock art over generating a replacement.** It
preserves composition exactly, needs no prompt engineering, has no seam or tiling
problem, keeps details like corner notches for free, and at 8x invents almost nothing
measurable. It also beat the generated art on both textures where both existed,
`Mnebula4` and `MBG02`. **Every installed texture is stock-derived; none of the
generated art ships.** Generation is for cases where the stock art is genuinely too small
or too damaged.

**Invention scales with the magnification, steeply.** 16x from 128px tripled `MBG02`'s
R-G deviation; 8x from 256px left `MbgBorg`'s at a ratio of 0.998-1.000. Two consequences:
keep the scale factor down where you can, and **re-measure rather than reusing another
texture's correction** — `MONOHUE=1` is necessary at 16x and actively harmful at 8x,
where it would flatten real hue variation that nothing is threatening.

**Keep the raw AI layer on disk.** It is the only artifact that costs money and cannot be
reproduced deterministically. Everything downstream of it — the blend, the channel
matching, the resize, the TGA — is free to redo. `$A2_DATA/textures/<T>/ai/` exists for
this; `upscale-stock.sh --reblend` re-derives the rest offline.

---

## Triage

Order work by **how much screen area the texture covers**, since that is what makes a
texture worth doing — see the resolution section of `README.md`. Skyboxes (one face fills
the viewport) and map puffs gave the highest return per texture; planets and large props
next; hulls have the highest file count and the lowest return per file, and disk rather
than credits limits them. `INVENTORY.md` groups what is left the same way.

## Cost

`pruna/p-image-upscale` is $0.005 up to 4 MP, $0.02 up to 16 MP. Upscaling all 1333 base
textures is roughly **$7 to $27** depending on target resolution — not the limiting
factor. The limiting factors are the alpha handling, the mip regeneration, and disk.

Measured: **135 skybox faces, 256x256 to 2048x2048, cost about $0.68 and ran in
minutes** across five parallel workers. Output is 1.7 GB of TGA and the retained `ai/`
layer another 0.84 GB, which is the shape of the disk problem at scale — the whole stock
texture set is 196 MB. The **32 puff quadrants added about $0.16** at 1 MP each.

**Contact sheets** make small textures cheaper still: 49 64x64 icons packed into one
512px sheet is one $0.005 request instead of 49, and it is also a 4x lift instead of the
16x a lone 64px icon gets at the API's 1MP floor. 408 UI textures cost **$0.055**.

`belt app run <id> --batch inputs.jsonl` runs 4 at a time and is the right interface for
bulk work; a batch of 8 took about two minutes.
