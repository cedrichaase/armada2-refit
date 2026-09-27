# ChatGPT / DALL·E prompts

> **Status: currently unused.** Every texture installed by this project is derived from
> its own stock art via `textures/tools/upscale-stock.sh`, not generated. Upscaling stock beat
> generation on the one texture where both were built and judged — `Mnebula4` shipped
> generated, was rebuilt from stock, and the stock version won. These prompts are kept
> for textures whose stock art is too small or too damaged to carry detail, but they
> have **not** been exercised against the current pipeline; expect to re-tune.
>
> If you do use them, the target is `source=gen` in `target.conf` (the default), which
> is the path that trims, squares, insets to `fill`% and stretches the noise floor —
> all the positioning `source=stock` skips.

Ask for a **square** image each time, otherwise you may get 16:9. 1024x1024 is ideal.
Generate 4 per atlas — re-run the same prompt, or follow with
"another variation, same style, different shape".

Save as PNG into `$A2_DATA/textures/<NAME>/src/`.

Do **not** hand-fix colour casts or slight background haze — the build greyscales,
sets the black point and masks the edges. Only the shape and structure matter.

**Three things the build cannot fix, so the prompt has to get them right:**

1. **Density.** The cloud must be a substantial body of mid-grey gas, not a bright core
   with faint wisps. Brightness is only lifted 2.5x at most, because anything more turns
   stray specks into stars. A too-faint source stays too faint.
2. **Fill.** It should occupy most of the frame. A small object in a large black field
   gets scaled up, which costs resolution.
3. **Softness.** Soft-edged smoke, not sharp filaments or lightning. The stock art is
   diffuse, and a very bright core just clips to flat white.

---

## Mnebula4  → textures/targets/Mnebula4/

A black and white visual effects plate for compositing: a thick, soft cloud of
luminous gas isolated on a completely pure black background. Broad billowing arms
spread outward from a gently glowing centre in a loose radial swirl, overlapping and
merging into one another. The body of the cloud is substantial and evenly lit in mid
greys — soft-edged smoke throughout, not thin strands. Greyscale only, no colour.
The cloud fills most of the square frame, leaving only a narrow black margin at the
edges. The background is entirely empty — no stars, no planets, no scattered dots or
points of light anywhere.

## Mnebula2  → textures/targets/Mnebula2/     (the only one that keeps colour)

A visual effects plate for compositing: a dense, clumpy cloud of glowing gas in deep
red and orange, isolated on a completely pure black background. The gas is turbulent
and lumpy, with bright fiery highlights and dark voids threaded through it. It fills most of the
square frame, leaving only a narrow black margin at the edges. The background is
entirely empty — no stars, no planets, no scattered dots or points of light.

## Mnebula5  → textures/targets/Mnebula5/

A black and white visual effects plate for compositing: a bright shattered cloud of
gas isolated on a completely pure black background. The structure looks fractured and
granular, like clumped granular ash, with bright flecks
scattered through it and deep black gaps between them. Strong contrast but still soft-edged smoke. Greyscale
only, no colour. It fills most of the square frame, leaving only a narrow black margin
at the edges. The background is entirely empty — no stars, no planets, no scattered
dots or points of light.

## Mnebula1  → textures/targets/Mnebula1/

A black and white visual effects plate for compositing: a dense billowing cloud
isolated on a completely pure black background. The mass is lumpy and rounded, like
cauliflower or thick churning smoke, granular in texture, with bright crests and deep
shadowed hollows. Greyscale only, no colour. It fills most of the square frame, leaving only a
narrow black margin at the edges. The background is entirely empty — no stars, no
planets, no scattered dots or points of light.

## mtachyonneb  → in/mtachyonneb/

A black and white visual effects plate for compositing: a thin, torn veil of smoke
isolated on a completely pure black background. It is sparse, irregular and ragged,
with no bright centre — an even, soft, low-contrast wisp drifting across the frame in
tatters. Greyscale only, no colour. It fills most of the square frame, leaving only a narrow
black margin at the edges. The background is entirely empty — no stars, no planets, no
scattered dots or points of light.

## MFluidicNeb  → in/MFluidicNeb/

A black and white visual effects plate for compositing: soft smoke tendrils spiralling
outward from a glowing centre, isolated on a completely pure black background. Only a
few broad, smooth, gently curving arms — organic and flowing rather than finely
detailed or filamentary. Greyscale only, no colour. It fills most of the square frame,
leaving only a narrow black margin at the edges. The background is entirely empty —
no stars, no planets, no scattered dots or points of light.

---

## If it misbehaves

- **Stars appear anyway.** Negation is weak in these models. Reply: "Regenerate with a
  completely empty black background. Remove every star and speck — only the cloud."
- **The cloud fills the whole frame.** Reply: "Pull it back so the cloud occupies the
  middle two thirds, surrounded by empty black."
- **It comes out dark navy rather than black.** Harmless — the build crushes it.
- **It looks like a finished artwork** with glow, vignette or border. Reply: "No glow,
  no vignette, no border. A flat isolated element on black, like a VFX stock plate."

---

# Skybox faces (the backdrop, a separate system)

These are the `mbg*` textures — the sky *behind* everything, drawn as a cube model
named by each map's `.bzn`. Completely separate from the map-plane puffs above, and
the rules are almost inverted:

| Puffs (`Mnebula*`)              | Skybox (`mbg*`)                          |
|---------------------------------|------------------------------------------|
| Isolated subject on black       | **Fills the frame edge to edge**         |
| Fades to black at the edges     | **Bright right up to the border**        |
| Greyscale, engine tints         | **Keeps its own colour**                 |
| 4 per atlas, order irrelevant   | **One image per face, order matters**    |
| 64x64 source                    | 256x256 source                           |

**There is no horizontal band.** An earlier draft of this file claimed every face
carries a galactic-plane band across the middle. Measured, that is false. The 6-face
sets (`MbgDom1`, `MbgBorg`, `MbgKling`) are built around a **bright core in the centre
of the frame**, fading outward, with a hard **black notch cut out of each of the four
corners** — corner 24x24 blocks measure exactly 0 while the centre measures 47-55. Ask
for a centred nebula, not a band.

`MBG02` is the exception to almost everything here — see its own section below.

Workflow: drop images in `$A2_DATA/textures/<Set>/src/`, sorted by filename → face 1..6.

    ./a2tex build <Set>   ./a2tex install <Set>   ./a2tex revert all

## Only three sets cover the whole Federation campaign

| Set        | Missions        | Faces | Look |
|------------|-----------------|-------|------|
| `MBG02`    | fed01 + 14 more | 1 file, **4 tiles** | Very dark, saturated blue |
| `MbgDom1`  | fed02, 05, 06   | 6     | Sparse magenta and violet, dark |
| `MbgBorg`  | fed07–10        | 6     | Dense yellow-green, much brighter |

### MBG02  → $A2_DATA/textures/MBG02/src/   (**4 images**)

`MBG02.tga` is not one sky — it is a **2x2 atlas of four 128x128 tiles**, the same
layout as the map-plane puff atlases, and the cube faces sample quadrants out of it.
Verified two ways: the strongest column-to-column and row-to-row discontinuities in
the whole texture sit at x=127|128 and y=127|128, and drawing those two lines over the
image lands them exactly on visible breaks in the cloud.

So: **four images, not one.** Feeding it a single picture would put a quarter of that
picture on each face.

It is also the only set with no corner notches — it fills the square completely — and
it is worth doing first: `MBG02.tga` is the sky for fed01, tutorial3, three Borg and
two Klingon missions, and eight multiplayer maps. `MbgBlue.SOD` is byte-identical to
`Mbg02.SOD` and uses the same texture, so "the blue skybox" and MBG02 are one thing.

**Measured target** (what the build script matches you to): mean 15/255 overall, with
the blue channel at 30 against red 8 and green 7 — roughly **4:1 blue**, a near-pure
blue image. Peak blue is 152; nothing in the texture exceeds 47/255 in grey. Only 4%
of pixels are brighter than 24/255. Within a tile the density is flat — 8 to 24 top to
bottom, no gradient worth naming.

Generate it **brighter than the target** and let the build dim it. Asking a model
for something this dark directly gives muddy, low-contrast sludge; a rich image scaled
down keeps its structure. Stock only uses about 48 distinct levels anyway, so the
dimming costs nothing you can see.

### What actually shipped

**ChatGPT (the web UI) produced the best image of anything tried**, better than any
model reachable through `belt`. It is archived as `source-MBG02-chatgpt.png`; the
seamless version actually built from is `source-MBG02-seamless.png`.

| Route | Outcome |
|---|---|
| **ChatGPT web UI** | **used** — dense filament web, even coverage, good scale variation |
| `openai/gpt-image-2-5-flare` (high, $0.053) | same idea but a too-regular honeycomb of uniform cells; no scale variation |
| `pruna/flux-2-klein-4b` ($0.001) | good composition, but wispy plasma — 21% pure black against stock's 9.8% |
| `bytedance/seedream-4-5` ($0.04) | best *gas*, but always a composed subject and a photographic plate border. Usable via off-centre 4K crops |
| `falai/flux-dev-lora` ($0.035) | terrestrial storm clouds |

The prompt that produced it, run once (one good image is enough — all four tiles are
the same seamless copy, see README):

> A seamlessly tileable square texture of a deep space nebula, filling the frame
> completely edge to edge with no border and no vignette. A dense irregular web of
> glowing electric blue gas filaments spread evenly over the entire square, the bright
> threads branching and joining into a fine network, with rounded pockets of deep black
> empty space enclosed between them. Fractal detail at many scales: broad filament
> bundles made of finer threads made of finer wisps. Uniform overall density across the
> whole picture with no centre, no focal point, no radial pattern and no bright core.
> Pure saturated blue on black, no cyan, no teal, no purple, no white except the
> faintest highlights on the brightest filaments. No stars, no planets, no text.

**Ask for "fractal detail at many scales".** It is what separates the shipped image
from the gpt-image honeycomb — without it models produce cells of one uniform size.

**Neither hue nor tiling needs to be right.** The source had *no red channel at all*
and did not tile (L|R 0.18); `fit()` synthesises the missing channel and
`make-seamless.sh` fixes the tiling. Spend the prompt on structure and composition.

**Phrasings that backfired**, each traced to a specific failure:

| Wording | What it produced |
|---|---|
| "texture plate" | a literal photographic plate, with border and rounded corners |
| "soft cloudy" | terrestrial weather clouds |
| "fibrous / filaments" alone | lightning arcs and electrical veins |
| no explicit "no dots, no specks" | star fields, every time |
| "no centre of interest" alone | radial convergence to a point — say *no radial pattern, no converging lines* |

**Hue does not need to be right.** The first run came back at R10 G35 B76 — visibly
cyan against stock's R8 G7 B30 — and `fit()` corrects it, matching each
channel to the stock file separately. Spend the prompt's effort on composition instead.

**The four tiles do not need to join.** Stock's don't: the seams at the quadrant
boundaries are plainly visible in the original. They only need to share palette and
density, which the follow-up prompt handles.

**Push the contrast.** Stock's peak is about 5x its own mean in the blue channel (152
against 30) and 3x in grey (47 against 15). Because the build script matches the
*mean*, a flat input stays flat after scaling and the sky reads as haze rather than
cloud. That is why the prompt asks for dark voids and bright knots rather than even
coverage of mid-tones — dynamic range is the one thing dimming cannot restore.

**Fibrous beats smooth.** Stock's character comes from fine, *coherent* filaments —
thread-like structures that curve and branch, not soft blobs and not speckle. This is
the single hardest property to get, and it is the reason a real image model beats the
procedural fallback (see README). Say "fibrous", "filaments", "wispy strands".

### MbgDom1  → $A2_DATA/textures/MbgDom1/src/   (6 images)

A deep space nebula background filling the square frame. Ragged magenta and violet
clouds of gas and dark dust against black, densest and brightest at the centre of the
frame and thinning outward toward the edges. Mostly dark overall. The four corners of
the square fade to pure black. No stars, no planets, no bright points of light, no
text.

### MbgBorg  → $A2_DATA/textures/MbgBorg/src/   (6 images)

A deep space nebula background filling the square frame. Dense, roiling clouds of
yellow-green gas laced with dark dust lanes, blazing brightest at the centre of the
frame and darkening outward toward the edges. Rich and saturated, noticeably brighter
than a typical night sky. The four corners of the square fade to pure black. No stars,
no planets, no bright points of light, no text.

**Keep the six faces of a set consistent** — same prompt, same palette, same core
brightness. Asking for "another variation of the same nebula, same colours, same
centred core" works better than re-running from scratch.

**The corner notches have to come from the prompt.** Stock cuts a hard black notch out
of all four corners of every face in these sets (a 24x24 corner block measures exactly
0). the build does not mask them in, so the "corners fade to pure black" clause
above is currently the only thing producing them, and it will give a soft fade rather
than stock's hard cut. If that reads wrong in game, the fix is a corner mask in the
build script, not a longer prompt.
