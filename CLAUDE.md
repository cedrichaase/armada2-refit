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
   textures are 24-bit, but **1113 of the 2118 textures in the game are 32-bit with a
   live alpha channel**, and the `-alpha off -type TrueColor` in these scripts would
   silently destroy it. `bottomup.py` rejects non-24-bit input, which is the only reason
   that has not already bitten. See `REMASTERING.md`.
   For 24-bit nebula work: `-alpha off -type TrueColor -compress None`, then
   `./bottomup.py <file>`. Verify the header after writing; a format mismatch will not
   announce itself. ImageMagick 7 cannot be told to write bottom-up TGAs — every
   `-define tga:image-origin=...` spelling is ignored — so the row flip must be a
   post-pass. `bottomup.py` is in-place and idempotent.
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

Watch for `local a=$1 b="$a"` — the whole `local` expands before any assignment lands,
so `b` gets the *old* `a`. This cost a debugging round in `a2tex`.

## Generative upscaling, in three rules

1. **`enhance_details` / `enhance_realism` are not a quality dial.** Off makes invented
   detail *more* visible, not less — the output is sharper, so the hallucinations read
   clearly. On merely blurs them. Use a blend toward Lanczos instead:
   `magick lanczos.png ai.png -define compose:args=35 -compose blend -composite out.png`
2. **Measure invention, don't eyeball it.** `-fx "abs(r-g)"` against the stock file's
   own value catches invented chroma (MBG02: stock 1.89, Lanczos 1.97, AI 5.90).
3. **`MONOHUE=1` only on single-hue textures.** It rebuilds colour from luminance and so
   removes chroma invention by construction — but on a multi-hue texture it flattens
   real colour. Check hue spread first.

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
| Greyscale | R, G, B means identical (map puffs only) |
| Skybox hue | each of R, G, B matches the stock file's channel mean — `fit()` in `lib/common.sh` does this |
| Face resolution | `size=` in `target.conf` = per-FACE pixels; one face fills the viewport, so this is what the eye sees |
| Chroma invention | R-G deviation vs the stock file's (MBG02: 1.89). A generative upscale triples it; `MONOHUE=1` fixes it |
| Seamless-able? | only homogeneous sources. Five methods failed on composed ones — see README before trying a sixth |
| Tile self-seam | L\|R and T\|B RMSE below stock's 0.023 / 0.058 — `make-seamless.sh` |
| TGA descriptor byte | `0x00` after `./bottomup.py` — ImageMagick writes `0x20` |
| Is it an atlas? | diff columns at x=127\|128 and y=127\|128 vs a baseline column |
| Corner notches | 24x24 corner mean — `0` on the 6-face sets, non-zero on `MBG02` |

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
constraint worth worrying about here; face resolution is.

**`SIZE` is per-face, and the face fills the whole viewport** — sizing the atlas is not
the same thing and was the reason a 4x increase still looked soft. See the resolution
table in `README.md`. Prebuilt candidates at 2048 and 4096 are in `compare/`; swap with
`./a2tex build` + `./a2tex install`.

**`MBG02` is settled: candidate D, accepted by the user.** Stock art, AI-upscaled,
blended 35% toward Lanczos, `monohue=1`, face 2048 in a 4096x4096 atlas. `./a2tex build
MBG02` reproduces it byte-for-byte from `targets/MBG02/src/`. The other three candidates
are in `archive/mbg02-candidates/` and can be copied straight over the game file.

Only the blend percentage is still open to taste, and re-blending needs the Lanczos and
AI layers, which were scratch and are gone — redoing it means re-upscaling the four stock
quadrants (~$0.16).

Do not re-litigate the alternatives without reading `README.md` first: generated sources
(B, C) were tried and the user preferred stock's own composition.

`Mnebula4` (map puff) and `MBG02` (skybox) are installed, and `./a2tex build` reproduces
both — MBG02 byte-for-byte against the accepted candidate D.

Targets exist and are stocked for all 7 puff atlases (including `Mlatinum`, the latinum
resource cloud, which is a nebula puff like the rest) and 5 skybox sets. Eleven of the
thirteen have no `src/` yet — that is the work remaining, and it is all art, not code.

**Dilithium is not a nebula.** It is a moon: `mdmoon.tga` (256x256) plus the glow
`Mdmoonglo4` (64x64) and the "Dmoon nimbus pulse" animation in `Sprites/animation.spr`.
No target was made for it; the nebula-looking resource cloud people mean is usually
`Mlatinum`, which is covered. `MBG02` is 1024x1024, built
from a user-supplied ChatGPT image (`source-MBG02-chatgpt.png`), made seamless, and
built with `UNIFORM=1` so all four tiles are identical — see the cube-seam section of
`README.md` before changing that. It matches stock on all three channels; it has **not
yet been seen in game**. Everything else is stock. Originals are
backed up in the game directory (`.a2neb-backup`) and in each `targets/<NAME>/stock/`.

The installed `Mnebula4.TGA` predates `bottomup.py`, so it is still top-down (`0x20`)
while `out/Mnebula4.tga` has since been rebuilt bottom-up. Both render; reinstalling is
optional. `MBG02` has a verified build path (tested against synthetic input — all four
quadrants landed exactly on the stock means 11/16/17/16) and its prompts are in
`PROMPTS.md`.

Known gap: the `sky-faces` kind does not reproduce the **corner notches** the 6-face sets
have, so generated `MbgBorg`/`MbgDom1`/`MbgKling` faces will have square corners where
stock has black ones. `MBG02` is unaffected — it has no notches.

Open items are listed at the end of `README.md`; the plan for the rest of the game's
textures is in `REMASTERING.md`.
