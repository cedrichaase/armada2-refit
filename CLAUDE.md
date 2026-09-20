# Working in this directory

Tooling for replacing Star Trek: Armada II nebula textures with generated art.

**Read `README.md` first.** It holds the engine reference — sprite format, the two
nebula systems, texture inventory, pitfalls. Do not re-derive any of it; it was
established by measurement and several wrong turns.

`REMASTERING.md` generalises this work to the other ~1300 base textures. **Read it
before touching any texture that is not a nebula** — over half the set has a real alpha
channel that these scripts would destroy.

`SETUP.md` covers the Heroic/Proton side — DLL overrides, the widescreen patch, the
d3d8 proxy chain, the Hyprland virtual desktop, the save format. Read it before
touching anything renderer-adjacent. **It records a live regression: Heroic redeploys
DXVK on every launch and reverts the d3d8 chain fix.**

## Environment

- Game: `/home/cedric/Games/Heroic/Star Trek Armada II` — GOG release, patch 1.1 plus
  Patch Project 1.2.5, run through Heroic with Proton-CachyOS.
- Textures: `Textures/RGB/`, flat, ~2100 files, **mixed `.tga` / `.TGA` case**.
- Available: ImageMagick 7 (`magick`), `python3`, `ffmpeg`.
  **Not available: numpy, PIL.** Do image work through ImageMagick, not Python.
- Scratchpad for intermediates; this directory is the user's, keep it tidy.

## Hard rules

1. **Never write into `Textures/RGB/` directly.** `./a2tex build` writes to
   `targets/<NAME>/out/`; `./a2tex install` copies it across, making a `.a2neb-backup`
   on first touch. `./a2tex revert all` undoes everything.
2. **Match the stock TGA format exactly**: image type 2, uncompressed, no ID field, no
   colour map — and **the same bit depth as the file you are replacing.** The nebula
   textures are 24-bit; **1113 of the 2118 textures in the game are 32-bit with a live
   alpha channel.** `write_tga()` now reads the depth off the stock file it is given as
   its third argument and writes to match, and `attach_alpha()` carries the stock mask
   across, Lanczos-upscaled. **The alpha never goes through the generative upscaler** —
   a mask has only edges, which Lanczos resolves exactly, and a model that invents
   plausible detail into a mask invents holes in the object.
   **Resize colour and alpha separately, always.** Resizing an RGBA image associates
   alpha and then un-associates it, dividing colour back out by a near-zero alpha; that
   put `Mmoon`'s Lanczos layer at mean 137 against stock's 43, and `fit()` then *hid* it
   by scaling the whole plate to match the mean. See `REMASTERING.md`.
   For 24-bit nebula work: `-alpha off -type TrueColor -compress None`, then
   `tools/bottomup.py <file>`. Verify the header after writing; a format mismatch will
   not announce itself. ImageMagick 7 cannot be told to write bottom-up TGAs — every
   `-define tga:image-origin=...` spelling is ignored — so the row flip must be a
   post-pass. `bottomup.py` is in-place and idempotent.
   **Stock is not uniformly bottom-up — that earlier claim was measured and is false.**
   Of the 135 skybox faces, 53 are `0x00` and 82 are `0x20`, and the split runs *within*
   a single set: `MbgBorg.2` and `.5` are bottom-up while the other four faces are
   top-down. Both render correctly in the retail game, so the engine honours the
   descriptor. `write_tga()` takes the stock file as its third argument and copies
   *its* origin (`bottomup.py --like`), which makes an accidental vertical flip
   impossible by construction rather than by argument.
3. **Always glob both extension cases.** `$(ls "$n.tga" "$n.TGA" 2>/dev/null | head -1 || true)`
   — and note the `|| true`: under `set -o pipefail` the failing `ls` aborts the script.
4. **Don't touch `Sprites/nebula.spr`.** UVs are normalised against `@reference=128`, so
   they are fractions and larger textures land on the same quadrants unchanged. Editing
   it is never the fix.

## The pipeline

One entry point, `./a2tex` — `list`, `build [-j N]`, `install`, `revert`, `diff`.
A target is `targets/<NAME>/` holding `target.conf`, `stock/`, `src/`, `out/`.
**To add work, drop images in `src/` and build.** Do not write new per-texture scripts;
add a target directory instead.

`-j` is safe because every build gets its own `mktemp -d` scratch. Keep it that way:
nothing in `lib/` may write to a fixed path.

**Always write intermediates with the `PNG24:` prefix.** A greyscale intermediate is
stored as a 2-channel PNG and `%[fx:mean.g]` then reads exactly **0** — which makes the
channel-synthesis branch "repair" channels that were never broken, and makes a
reference's green/blue gain a silent no-op. Neither `-type TrueColor` nor
`-colorspace sRGB` fixes it; only the on-disk format does. `fit()` refuses a reference
with a zero green and blue channel for this reason.

Two bash traps that have each cost a debugging round here:

- `local a=$1 b="$a"` — the whole `local` expands before any assignment lands, so `b`
  gets the *old* `a`.
- `read -r w h < <(magick ... -format '%w %h' info:)` — `read` returns 1 at EOF without
  a trailing newline, and under `set -e` that **aborts the script silently**. It killed
  `upscale-stock.sh` between the paid upscale and the blend, leaving a full `ai/` and an
  empty `src/`. Put `\n` in the format.

## Generative upscaling, in five rules

1. **`enhance_details` / `enhance_realism` are not a quality dial.** Off makes invented
   detail *more* visible, not less — the output is sharper, so the hallucinations read
   clearly. On merely blurs them. Use a blend toward Lanczos instead:
   `magick lanczos.png ai.png -define compose:args=35 -compose blend -composite out.png`
2. **Measure invention, don't eyeball it.** `-fx "abs(r-g)"` against the stock file's
   own value catches invented chroma (MBG02: stock 1.89, Lanczos 1.97, AI 5.90).
3. **`MONOHUE=1` only on single-hue textures.** It rebuilds colour from luminance and so
   removes chroma invention by construction — but on a multi-hue texture it flattens
   real colour. Check hue spread first, with `tools/huespread.py` — **not** with the
   plain `-colorspace HSL -channel H -separate` standard deviation, which is dominated
   by hue noise in the near-black areas that make up most of a skybox and scores
   single-hue plates like `MbgDom1` at 102°. `huespread.py` weights each pixel's hue by
   the chroma it actually carries; `MBG02`, the one accepted `MONOHUE=1`, scores 0.8°.
4. **How much a generative upscaler invents depends on the scale factor, steeply.**
   `MBG02` was 16x from a 128px quadrant and tripled its R-G deviation (1.89 → 5.90).
   The 6-face sets are 8x from 256px and the same model at the same settings invents
   *no* measurable chroma: `MbgBorg`'s R-G ratio against stock is 0.998–1.000 across all
   six faces. So `MONOHUE` is needed for the atlas and is **not** needed for the 6-face
   sets, where per-channel matching keeps the real hue variation instead of flattening
   it. Measure before reaching for it.
5. **Keep the AI layer.** `targets/<T>/` holds three layers: `stock/` (original),
   `ai/` (the raw upscale — what the credits bought), `src/` (`ai/` blended over
   Lanczos — what `a2tex build` reads). `tools/upscale-stock.sh --reblend --blend N`
   re-derives `src/` offline and free. For `MBG02` those layers were scratch and were
   deleted, which is why its blend percentage became expensive to re-tune.

## The four things most easily got wrong

- **There are two nebula systems**, and they need opposite treatment. Map puffs
  (`Mnebula*`) are greyscale, engine-tinted, isolated on black, 4 per atlas. Skybox
  (`mbg*`) keeps its own colour, fills the frame edge to edge, 6 faces, order matters.
  A nebula that appears on the minimap is a map puff; the skybox never does.
- **`MBG02` breaks the skybox rules.** It is not one face — it is a 2x2 atlas of four
  128x128 tiles, so it needs four images, and `kind=sky-atlas` in its `target.conf` routes it
  through the atlas path. It also has no corner notches, unlike every other
  6-face set. Before assuming any skybox texture is a single picture, run the seam test
  at x=127|128 and y=127|128 and compare against a baseline column. Measure first.
- **The skybox is not in any config file**, and **`strings` will lie to you about it.**
  Each map's binary `.bzn` holds the background name in a fixed field after the marker
  `02 00 00 00 64 00 00 00`. It is either `<name>.sod` (a cube model) *or* a bare
  `<prefix>` resolving to `<prefix>0.tga`..`<prefix>5.tga` with no SOD at all — 38 of
  the 72 maps use the prefix form. `strings | grep '\.sod'` silently skips all of them
  and emits buffer-tail fragments (`s.sod`, `e.sod`) for some. **Read the field.**
  Recipe in `README.md`.
- **Blending is additive, so black is transparent.** Any non-black pixel at a map-puff
  quadrant edge renders as a glowing square in space. Skybox faces are the opposite and
  must stay bright to their borders.

## Method

Measure, don't eyeball. Every quality decision here has a number behind it:

| Check | Target |
|---|---|
| Mean luminance | match the stock file it replaces (additive blend ⇒ mean ≈ light contributed) |
| Quadrant edge maxima | `0` for map puffs |
| Peak | below 255 — anything at 255 is clipping |
| Greyscale | R, G, B means identical — true for most puffs but **not** `Mnebula1` (28/26/22) or `Mnebula2` (33/9/8). Match the stock file, not the rule |
| Skybox hue | each of R, G, B matches the stock file's channel mean — `fit()` in `lib/common.sh` does this |
| Face resolution | `size=` in `target.conf` = per-FACE pixels; one face fills the viewport, so this is what the eye sees |
| Chroma invention | R-G deviation vs the stock file's (MBG02: 1.89). A generative upscale triples it; `MONOHUE=1` fixes it |
| Seamless-able? | only homogeneous sources. Five methods failed on composed ones — see README before trying a sixth |
| Tile self-seam | L\|R and T\|B RMSE below stock's 0.023 / 0.058 — `make-seamless.sh` |
| TGA descriptor byte | **the same as the stock file's** — `bottomup.py --like`. ImageMagick always writes `0x20`; stock is a mix |
| Is it an atlas? | diff columns at x=127\|128 and y=127\|128 vs a baseline column |
| Corner notches | 24x24 corner mean — `0` on the 6-face sets, non-zero on `MBG02`. Upscaling stock preserves them for free |

**ImageMagick is Q16 here, so `-threshold N` means N/65535, not N/255.** Writing
`-threshold 40` to mean "40/255" thresholds at 0.15/255 instead and reports nearly the
whole image as bright. Always use a percentage: `-threshold $((v*100/255))%`. This
produced a completely wrong brightness histogram for `MBG02` before it was caught.

`a2tex build` prints `q0:38->28(x0.746)` per quadrant: source mean, result, gain.
**Gain pinned at 2.5 means the input was too faint** and no amount of processing will
fix it — the generation prompt has to change. Gain near 1.0 is ideal. Dimming (< 1.0)
is safe and uncapped.

Two corrections that are baked into the scripts and should not be undone:

- **Linear stretch, never gamma, for density matching.** A gamma steep enough to lift
  mean 7 to 28 maps background 1/255 to 32/255 and fills the sky with fake stars.
- **Noise floor = mean of the darkest small corner, capped at 3%** — not the max over a
  large corner, which reads real gas once a cloud fills the frame.

## When changing prompts

`PROMPTS.md` is the single source of truth and the artifact reads from it
directly — do not restate prompts elsewhere. Written for ChatGPT, which has **no
negative prompt**, so constraints are phrased positively and the whole thing is framed
as an isolated VFX plate; that framing is what keeps stars out.

What the build cannot fix, so the prompt must:

1. **Density** — a substantial body of mid-grey gas, not a bright core with faint wisps.
2. **Fill** — occupy most of the frame; a small subject in a black field wastes resolution.
3. **Softness** — soft-edged smoke. A very bright core just clips to flat white.

Generation: **the ChatGPT web UI beat every model reachable through `belt`**, including
`openai/gpt-image-2-5-flare` at high quality. If the user offers an image, prefer it —
and ask them to save it to disk, since pasted images do not persist anywhere readable.
For a tile the binding constraints are compositional, not aesthetic: fill the frame, no
focal point, no stars, fractal detail at many scales. See `PROMPTS.md`.

If no image model is reachable, `./gen-nebula.sh <seed> <out.png>` produces a usable
tile offline. It matches stock statistics exactly but **cannot produce coherent fibrous
filaments** — three alternative noise formulations were tried and were worse. The
comparison table is in `README.md`; read it before re-tuning the weights.

Tiling repetition is *not* visible in game — the installed `Mnebula4` uses the same
image in all four quadrants and reads fine, because billboards overlap at many scales.
Don't insist on four distinct generations.

## State

**A 4096x4096 atlas (48 MB TGA) is confirmed working in game.** Texture size is not a
constraint worth worrying about here; face resolution is. **But a texture with a
hand-authored mip chain is a hard exception** — there the base size is pinned to the
chain, and changing one without the other crashes the game. `mips=N` in `target.conf`.

**Everything installed by this project is derived from its own stock art.** None of the
generated art shipped: `Mnebula4` was the only texture built both ways, and the stock
upscale won. Reach for `PROMPTS.md` only when stock is too small or damaged to carry
detail.

**`SIZE` is per-face, and the face fills the whole viewport** — sizing the atlas is not
the same thing and was the reason a 4x increase still looked soft. See the resolution
table in `README.md`. Prebuilt `MBG02` candidates are in `archive/mbg02-candidates/`;
swap with `./a2tex build` + `./a2tex install`.

**`MBG02` is settled: candidate D, accepted by the user.** Stock art, AI-upscaled,
blended 35% toward Lanczos, `monohue=1`, face 2048 in a 4096x4096 atlas. `./a2tex build
MBG02` reproduces it byte-for-byte from `targets/MBG02/src/`. The other three candidates
are in `archive/mbg02-candidates/` and can be copied straight over the game file.

Only its blend percentage is still expensive to change: `MBG02` predates the `ai/` layer,
so its Lanczos and AI intermediates were scratch and are gone. Re-tuning it means
re-upscaling the four stock quadrants (~$0.16). Every later target keeps `ai/` precisely
so this does not recur.

Do not re-litigate the alternatives without reading `README.md` first: generated sources
(B, C) were tried and the user preferred stock's own composition.

`Mnebula4` (map puff) and `MBG02` (skybox) are installed, and `./a2tex build` reproduces
both — MBG02 byte-for-byte against the accepted candidate D.

**The skybox class is finished and confirmed in game by the user.** All 23 sets — the
`MBG02` atlas plus 22 six-face sets, 134 files — are upscaled from their own stock art,
installed, and verified rendered, not just measured. `Mbgstars` is deliberately left
stock (starfield). Do not re-open this without a specific reason.

**`Mnebula2` has a hand-authored mip chain (`Mnebula2_1..._4`, each half the previous)
and upscaling its base while leaving the chain alone CRASHED the game** in the Klingon
campaign. Fixed with `mips=4` in its `target.conf`: the build emits the whole chain and
`install` validates it pre-flight, refusing the entire target rather than writing a
partial one. `revert <target>` restores the chain too. It is the only one of the 142
textures installed here with a chain — but `REMASTERING.md` counts 347 across the full
set, so `mips=` will be needed constantly once this moves to hull textures.

**All 8 puffs are done**, and the claim above them — that a puff could not usefully be
upscaled from its own stock — was wrong. All 8 atlases are now `source=stock`,
1024x1024, installed. `Mnebula4`'s earlier generated art is preserved in
`targets/Mnebula4/src-generated/` and `archive/mnebula4-generated/`; put it back in
`src/` and set `source=gen` to return to it.

Generated art via `PROMPTS.md` remains the route for anything where stock composition is
not worth keeping — but measure the upscale first, because on both classes here it won.

**Dilithium is not a nebula.** It is a moon: `mdmoon.tga` (256x256) plus the glow
`Mdmoonglo4` (64x64) and the "Dmoon nimbus pulse" animation in `Sprites/animation.spr`.
No target was made for it; the nebula-looking resource cloud people mean is usually
`Mlatinum`, which is covered.

**Planets and moons are done — 15 textures, installed, not yet seen in game.** Eleven
24-bit planet maps at `kind=plain` (an alias for `sky-faces`: one file, no alpha, no
chain — structurally a single skybox face), all at face 2048, all from their own stock.
`MQonos` is the one at `blend=15`; on the other ten the AI's contribution follows
existing structure, on `MQonos` it was free-floating hairlines over a smooth surface.

Then four **32-bit** ones — `mdmoon` (+ a rebuilt 4-level chain), `Mmoon`, `Mbakurng`,
`earth` — which are the first alpha-bearing textures this pipeline has touched and the
reason rule 2 above changed. `blackedge=N` exists for `Mmoon` alone: its three sun
sprites are additive, so a stray 1-3/255 on a quadrant boundary draws a glowing square.
`Mdmoonglo`/`Mdmoonglo4` stay stock — soft gradients, nothing to recover.

**The UI stretch is a layout-canvas bug, not a texture problem.** `misc/gui_<race>.cfg`
declares `screenWidth = 1600 / screenHeight = 1200` and the engine scales that canvas to
the back buffer independently on each axis — 2.15x across against 1.20x down at
3440x1440. `tools/ui-widescreen.py` re-declares the canvas and moves the right-anchored
and centred panels; `--revert` undoes it. Details and the one unhandled case (the bridge
display) are in `SETUP.md`. **Applied but not yet seen in game.**

Originals are backed up in the game directory (`.a2neb-backup`) and in each
`targets/<NAME>/stock/`; `./a2tex revert all` restores every one of them. The UI configs
have their own `.a2neb-backup` in `misc/` and are reverted by `ui-widescreen.py`, not by
`a2tex`.

(An older note here claimed the installed `Mnebula4.TGA` was still top-down. Measured:
it is `1800`, the same as `out/Mnebula4.tga` and the same as every stock puff. All nine
puff sources are bottom-up; only the skyboxes are mixed.)

The old "corner notches are not reproduced" gap is **closed, and was only ever a gap for
*generated* faces.** Upscaling a set's own stock faces keeps the composition — notches
included — by construction: `MbgBorg.1`'s 24x24 corner mean is 0 in stock and 0 in the
2048px build.

Open items are listed at the end of `README.md`; the plan for the rest of the game's
textures is in `REMASTERING.md`.
