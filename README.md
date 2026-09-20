# Armada II nebula regeneration

Tooling and reference notes for replacing Star Trek: Armada II's nebula textures with
higher-resolution generated art. Built against the GOG release running under
Heroic/Proton on Arch.

Game directory: `/home/cedric/Games/Heroic/Star Trek Armada II`
Textures live in `Textures/RGB/` (flat, ~2100 files).

---

## Layout and workflow

    a2tex                 the only entry point
    lib/                  common.sh (fit, TGA writing), puff.sh, sky.sh
    tools/                bottomup.py, upscale-stock.sh, huespread.py,
                          make-seamless.sh, gen-nebula.sh
    targets/<NAME>/       target.conf, stock/, ai/, src/, out/  -- one per texture
    archive/              candidates and comparisons that cost credits to make
    .scratch/             transient, safe to delete at any time

Four layers per target, and the order matters:

| | |
|---|---|
| `stock/` | the pristine original, copied out of the game. Never written |
| `ai/` | the raw generative upscale, one file per face. **This is what the credits bought** — it is the only layer that cannot be recreated for free, and it is not in git |
| `src/` | what `a2tex build` reads: `ai/` blended over a plain Lanczos upscale, or hand-supplied art |
| `out/` | the finished TGA, ready for `a2tex install` |

`src/` and `out/` are both derivable, so both are gitignored; `ai/` is gitignored only
because 135 faces of it is 0.84 GB. Back it up rather than regenerate it.

    ./a2tex list                       every target, its config and state
    ./a2tex build [target...] [-j N]   default: every target that has files in src/
    ./a2tex install [target...]        copies out/ into the game, backing up first
    ./a2tex revert [target...|all]     restores from the .a2neb-backup copies
    ./a2tex diff [target...]           measured comparison against stock

To work on a texture: drop images in `targets/<NAME>/src/`, then build. Nothing else
needs editing — `target.conf` already carries the right flags for every nebula target.

### `target.conf`

| key | meaning |
|---|---|
| `kind` | `puff` (2x2 atlas, additive, engine-tinted), `sky-atlas` (2x2, keeps colour), `sky-faces` (N files, one image each) |
| `size` | output edge **per face/quadrant**, not per atlas. One skybox face fills the whole viewport, so this is what the eye sees |
| `uniform` | sky-atlas only: match every tile to the whole stock texture rather than its own quadrant. Needed when all four tiles are one seamless image, otherwise the differing quadrant means put a brightness step at each cube-face join |
| `monohue` | rebuild all channels from luminance using the stock hue ratio. Only valid on single-hue textures; removes chroma invention by construction |
| `blend` | per cent of the AI layer kept over Lanczos in `src/`. Read and written by `upscale-stock.sh`, so a set tuned away from the default keeps that setting. `mbgrg` is the only one at 20 |
| `keepcolour` | puff only: skip the greyscale conversion. `Mnebula2` is the one stock puff with its own colour |
| `fill` | puff only: percent of the quadrant the subject occupies (default 92) |

### Parallel builds

`-j` is safe. Every build gets its own `mktemp -d` scratch directory and writes nothing
to a fixed path — the previous scripts shared `work/_s.png` and `work/_q0.png`, which
silently corrupt each other when two targets run at once.

## Current state

| | |
|---|---|
| `Mnebula4` | **installed** — 512x512, verified in game, looks good |
| `MBG02` | **installed** — 4096x4096 atlas, face 2048, candidate D |
| 22 six-face skybox sets, 133 faces | **installed** — upscaled from their own stock faces, face 2048. Not yet seen in game |
| 7 puff atlases | stock — they need art, and that is the remaining work |

Every installed file has a `.a2neb-backup` beside it; `./a2tex revert all` undoes the
lot. `Textures/RGB` went from 254 MB to 1.9 GB.

Originals are backed up twice: beside each file in the game directory as
`<name>.a2neb-backup`, and here in each `targets/<NAME>/stock/`.
`./a2tex revert all` restores everything.

---

## The two nebula systems

Armada II draws nebulae twice, with almost opposite rules. Getting these confused
wastes a lot of time — the first pass of this project targeted the wrong one.

| | Map puffs (`Mnebula*`) | Skybox (`mbg*`) |
|---|---|---|
| What | Billboard sprites you fly into | The cube of sky behind everything |
| Defined in | `Sprites/nebula.spr` | `SOD/Mbg*.SOD`, named in each map's `.bzn` |
| Source size | 128x128 atlas = **four 64x64 puffs** | 256x256 per face |
| Drawn at | up to 400x400 world units | — |
| Colour | **Greyscale, engine tints it** | **Keeps its own colour** |
| Framing | Isolated subject, fades to black | **Fills frame edge to edge** |
| Per set | 4 images, order irrelevant | 6 faces, **order matters** |
| Appears on minimap | yes | no |

A quick way to tell them apart in a screenshot: nebulae that show up as coloured blobs
on the minimap are map puffs. The skybox never does.

---


## What the `puff` build does, and why

Each step exists because something failed without it.

1. **Greyscale** (all but `Mnebula2`). The engine tints these; supplied colour fights it.
2. **Trim to content, square-pad, scale to 92% of the quadrant.** Generated subjects are
   rarely framed the way the engine wants, and a stock puff fills ~99% of its quadrant
   against a typical generation's ~59%. Without this the nebulae come out too small.
3. **Black clamp** at the measured noise floor, capped at 3%.
4. **Linear stretch** toward the stock quadrant's mean. Not gamma — see pitfalls.
   Lift is capped at **2.5x**; dimming is uncapped because it cannot amplify anything.
5. **Square vignette** over the outer ~14px. Blending is additive, so black is
   transparent and any non-black pixel at a quadrant edge becomes a glowing square
   hanging in space. Square rather than radial because stock art runs into the corners.
6. **Assemble 2x2** in sprite order and write 24-bit uncompressed TGA.

The build prints `q0:38->28(x0.746)` per quadrant — source mean, result, gain applied.
**A gain pinned at 2.5 means the input was too faint**; near 1.0 is ideal.

`fit()` matches **each channel separately** against the stock file rather than
matching overall luminance. Generated art drifts in hue — the MBG02 tiles came back at
R10 G35 B76 against stock's R8 G7 B30, plainly cyan — and per-channel matching fixes
hue and density in one step. It is a strict generalisation: on a greyscale source it
behaves exactly like matching the grey mean.

The `sky-*` kinds skip steps 1, 2 and 5 entirely — skybox faces keep their colour and
fill the frame.

---

## Engine reference

### Texture format

- **24-bit uncompressed TGA, image type 2**, no ID field, no colour map.
- The engine **reads dimensions from the TGA header** — sizes are not hardcoded.
- **4096x4096 is confirmed working in game** (48 MB uncompressed, as `MBG02`). Sizes are
  read from the header; the stock install's largest is a single 512x512 (`WshladSW.tga`)
  but that is not a limit. Above 4096 is untested.
- **Row order.** Every stock file has descriptor byte `0x00` (bottom-left origin).
  ImageMagick 7 always writes `0x20` (top-down) and offers no option to change it —
  `-define tga:image-origin=...` is silently ignored. `Mnebula4` shipped as `0x20` and
  rendered correctly, so the engine does appear to honour the flag, but the build
  scripts now run `./bottomup.py` as a last step to reverse the rows and clear bit 5,
  so output matches stock byte for byte. It is in-place and idempotent.
- File extension case is inconsistent — both `.tga` and `.TGA` occur, sometimes for
  files in the same set. Always glob both.

### `Sprites/*.spr`

    @reference=128        # UV values below are measured against a 128px texture
    @tmaterial=additive   # black is transparent
    # name    file       U   V   W   H
    ion1      Mnebula2   0   0   64  64
    @sprite_node ion1 ion1 red1.colour (80,80) (.2,.1,.1) billboard
    #            name sprite animation  (w,h)   (r,g,b)

- **`@reference` makes UVs fractions, not pixels.** This is the single most useful fact
  here: a 512x512 replacement lands on the same quadrants with no config edit at all.
- `(w,h)` on a sprite node is **world units**. The `big*` variants go to 400x400 from a
  64x64 source — 6.25 world units per texel. That ratio is the blur.
- The engine tints: `Mnebula4` is pure greyscale and is drawn green as metrion *and*
  purple as mutura from the same file.
- Quadrant order from the UV table: `(0,0)`=TL, `(0,64)`=BL, `(64,0)`=TR, `(64,64)`=BR.

### Map puff inventory

| Atlas | In game | Colour |
|---|---|---|
| `Mnebula1` | radiation | greyscale (engine tints yellow) |
| `Mnebula2` | ion storm | **keeps its own red/orange** |
| `Mnebula3` | — | orphaned, see pitfalls |
| `Mnebula4` | metrion **and** mutura | greyscale (tinted green / purple) |
| `Mnebula5` | neutral, cerulean, impenetrable | greyscale (tinted blue) |
| `MFluidicNeb` | fluidic space (Species 8472) | greyscale |
| `mtachyonneb` | tachyon | greyscale |

`Mnebula2_1` through `_4` (64, 32, 16, 8 px) are a hand-authored mip chain for
`Mnebula2` only. Left alone; they are only used when drawn small.

### Skybox

- Named in each map's **binary `.bzn`**. Not referenced from any `.spr`, `.odf`, `.h`
  or `.set` file — grepping config finds nothing.
- Faces are 256x256 and **carry their own colour** (no engine tint).
- **Not seamless.** Matching every face edge against every other in both orientations
  gives a best fit of 0.077 RMSE against an unrelated baseline of 0.204 — better than
  chance, nowhere near continuous. Per-face generation is therefore fine.
- **There is no horizontal band.** An earlier draft of these notes said every face
  carries a galactic-plane band; measured, that is false. See below.

#### Two naming conventions, and how to read them out of a `.bzn`

Every `.bzn` stores its background in a fixed field introduced by the 8-byte marker
`02 00 00 00 64 00 00 00`, followed by a NUL-terminated name. That name takes one of
two forms, and **only the first is a SOD**:

| Form | Example | Resolves to |
|---|---|---|
| `<name>.sod` | `mbg02.sod` | cube model `SOD/Mbg02.SOD`, which names its textures |
| `<prefix>` | `mbgpur` | six textures `Textures/RGB/<prefix>0.tga` .. `<prefix>5.tga`, no SOD |

    python3 - <<'EOF'
    import glob, os
    marker = bytes.fromhex('0200000064000000')
    for f in sorted(glob.glob('bzn/*.bzn')):
        d = open(f, 'rb').read(); i = d.find(marker) + 8
        print(os.path.basename(f), d[i:d.find(b'\x00', i)].decode())
    EOF

**Do not use `strings` for this.** It misses the prefix form entirely — `strings | grep
'\.sod'` returns nothing for 38 of the 72 maps, which is what produced the earlier and
wrong note that those maps "name no background model". Worse, it returns misleading
fragments: `a2_fed03` shows `s.sod` and `a2_borg01` shows `e.sod`, which are the tails
of a longer name left in the buffer (`mbgstars.sod` and `mbgblue.sod` respectively)
after a shorter one was written over the front of it. Read the field, don't scan it.

#### Complete background usage, all 72 maps

| Background | Maps | Form |
|---|---|---|
| `mbg02.sod` | 14 | SOD — **2x2 atlas**, see below |
| `mbgpur` | 9 | prefix (`mbgpur0`-`5`) — incl. `a2_fed03`, `a2_fed04` |
| `mbgborg.sod` | 6 | SOD, 6 faces |
| `mbgred` | 6 | prefix |
| `mbgkling.sod` | 6 | SOD, 6 faces |
| `mbgrg` | 5 | prefix |
| `mbgaqu` | 5 | prefix |
| `mbgdom1.sod` | 5 | SOD, 6 faces |
| `mbgdk` | 5 | prefix |
| `mbgkl` | 4 | prefix |
| `mbgblue.sod` | 2 | SOD — byte-identical to `Mbg02.SOD`, same texture |
| `mbgflu` | 2 | prefix |
| `mbggb` | 2 | prefix |
| `mbgbaku.sod` | 1 | SOD, 6 faces |

Nothing is unaccounted for. The eight lowercase `mbg*0`-`5` texture sets, previously
unexplained, are exactly the prefix-form backgrounds.

#### SOD internals

`strings` on a `SOD/Mbg*.SOD` gives both node and texture names, which look alike:

    Mbg02.SOD      -> MBG02, MBG2_1
    MbgBorg.SOD    -> Mbg02_1 .. mbg02_6, MbgBorg

`Mbg02_1`-`_6` are the **six cube-face node names**, reused verbatim across every set;
they are not files and looking for them as textures is a dead end. The texture is the
other name — `MbgBorg`, which resolves to `MbgBorg.1.tga` .. `.6.tga`. Attempting to
recover face->quadrant UVs by scanning the SOD for 4-byte-aligned floats in `[0,1]`
does not work — the runs come back all-zero. Not pursued further; it does not change
the art brief, since the four MBG02 tiles are interchangeable.

Federation campaign coverage is only **13 textures**:

| Set | Missions | Faces | Look |
|---|---|---|---|
| `MBG02` | fed01 + 14 more maps | 1 file, **4 tiles** | very dark, saturated blue (mean 15, B:R 4:1) |
| `MbgDom1` | fed02, 05, 06 | 6 | sparse magenta/violet, dark (face means 8-18) |
| `MbgBorg` | fed07-10 | 6 | dense yellow-green, bright (face means 35-45) |

`a2_fed03` and `a2_fed04` use `mbgpur`, the prefix form — see the table above.
The full set across all factions is 22 backgrounds / 135 files.

#### `MBG02` is a 2x2 atlas, not a face

The single biggest correction to the earlier notes. `MBG02.tga` is 256x256 holding
**four 128x128 tiles**, exactly like the map-puff atlases; the cube faces sample
quadrants out of it. Two independent confirmations: the largest column-to-column and
row-to-row differences anywhere in the texture are at x=127|128 (2.22) and y=127|128
(3.54) against a 1.77 baseline, and overlaying those two lines on the image puts them
precisely on visible breaks in the cloud. It therefore needs **four** generated
images, and `kind=sky-atlas` routes it through the atlas path.

Per-quadrant stock means, which the build matches tile by tile:

| | left | right |
|---|---|---|
| **top** | 11 | 16 |
| **bottom** | 17 | 16 |

`MbgBlue.SOD` is **byte-identical** to `Mbg02.SOD` and references the same texture, so
"the blue skybox" and MBG02 are the same asset. Between them they are the sky for
`a2_fed01`, `a2_tutorial3`, `a2_tutorial_UI`, `a2_borg02/04/08/09`, `a2_kling06/10`
and eight multiplayer maps — far more reach than any other background.

#### The 6-face sets have a centred core and black corner notches

`MbgBorg`, `MbgDom1` and `MbgKling` are built
around a **bright core at the centre** fading outward, and each has a **hard black
notch cut out of all four corners**: a 24x24 corner block measures exactly 0 while the
centre measures 47-55. `MBG02` has neither — no notches, no core, flat density (8-24
top to bottom within a tile).

    # corner-notch test
    magick F.tga -crop 24x24+0+0 +repage -format '%[fx:int(255*mean)]\n' info:

    # atlas test: seam at the midpoint vs an arbitrary baseline column
    for x in 63 127; do magick \( F.tga -crop 1x256+$x+0 +repage \) \
      \( F.tga -crop 1x256+$((x+1))+0 +repage \) -compose difference -composite \
      -colorspace Gray -format "x=$x %[fx:255*mean]\n" info:; done

### Not part of either system

`Gmneb1`-`Gmneb5` and `Gfog` are minimap and interface art (`Sprites/gui_map.spr`,
`gui_global.spr`). They never reach the map plane.

The starfield is **procedural**, not a texture — see `DETAIL_*_STARFIELD_*` in
`ART_CFG.h`. Don't generate stars into nebula art; they will swim against it.

---

## Pitfalls

Each of these cost real time.

- **Plain upscaling achieves nothing.** Lanczos 4x and mild detail injection are visually
  identical to the stock texture at game magnification, because the GPU's bilinear filter
  already does exactly that and there is no finer detail in the source to recover.
  Only *adding* structure helps.
- **Do not use gamma to match density.** A curve steep enough to lift mean 7 to 28 maps a
  background value of 1/255 to 32/255, turning the noise floor into a field of fake stars.
  Use a linear stretch with a capped gain.
- **Do not estimate the noise floor from corner maxima.** It works until a cloud fills the
  frame, then it reads 35 and clamps away real gas. Take the *mean of the darkest small
  corner*, and cap the clamp.
- **`Mnebula3` looks used but probably isn't.** Its four sprites are declared in
  `nebula.spr`, but every `mutura` sprite node points at the `metrion` sprites
  (`Mnebula4`) instead. Excluded from the batch. If a nebula fails to change, add it.
- **"Galaxy & Nebula retextures" on ArmadaFiles is not nebulae.** It retextures the
  Galaxy-*class* and Nebula-*class* starships. Trek naming trap.
- **Tiling repetition does not show in game.** All four quadrants of the installed
  `Mnebula4` are the same image and it is not noticeable, because many billboards overlap
  at different scales and rotations. Generating four distinct variants is optional.
- **`ls a.tga a.TGA | head -1` under `set -o pipefail` aborts the script.** The failing
  `ls` poisons the pipeline's exit status. Append `|| true`.
- **Web sources:** moddb.com and pcgamingwiki.com return HTTP 403 to automated fetches;
  armadafiles.com has a broken TLS certificate. Use plain `http` with `curl` for the last.

---

## Verification recipes

    # TGA header — must read 24bpp, imagetype 2
    python3 -c "
    import struct; d=open('F.tga','rb').read(18)
    w,h=struct.unpack_from('<HH',d,12)
    print(f'{w}x{h} {d[16]}bpp type={d[2]} idlen={d[0]} cmap={d[1]}')"

    # density, against the stock file it replaces
    magick F.tga -format 'mean=%[fx:int(255*mean)] max=%[fx:int(255*maxima)] sd=%[fx:int(255*standard_deviation)]\n' info:

    # quadrant edges must be 0 for map puffs (additive blending)
    magick F.tga -crop 512x1+0+0 +repage -format '%[fx:int(255*maxima)]\n' info:

    # is it actually greyscale? R, G and B means will be identical
    magick F.tga -format 'R%[fx:int(255*mean.r)] G%[fx:int(255*mean.g)] B%[fx:int(255*mean.b)]\n' info:

    # which background model does a map use?
    strings -a bzn/a2_fed07.bzn | grep -ioE 'mbg[a-z0-9_]*\.sod' | sort -u

---

## Resolution: the FACE is the unit, not the atlas

The game stretches **one cube face across the whole viewport**. On the 3440x1440
monitor here that means a single tile is magnified about 7x at `SIZE=512`, which still
looks soft — the original complaint, surviving a 4x atlas increase, because the atlas
is not what the eye sees. Size the *face*, not the atlas:

| `SIZE` (face) | atlas | TGA | magnification at 1440px tall |
|---|---|---|---|
| 128 (stock) | 256 | 192 KB | 11x |
| 512 | 1024 | 3 MB | 2.8x |
| 1024 | 2048 | 12 MB | 1.4x |
| **2048** | **4096** | **48 MB** | **0.7x — below 1:1** |

`size=` in `target.conf` sets it. **A 4096x4096 atlas — a 48 MB
uncompressed TGA — loads and renders fine**, confirmed in game. That retires the old
"512 is proven safe, 1024 is untested" note entirely: this engine reads the dimensions
from the TGA header and does not care how large they are, at least up to 4096 through
the d3d8to9 -> DXVK chain. Prebuilt candidates are in `archive/mbg02-candidates/`.

## Can you just upscale the stock texture?

Worth separating from the earlier finding that "plain upscaling achieved nothing". That
was *classical* upscaling — Lanczos cannot invent detail the source lacks, and the GPU
already does bilinear. A **generative** upscaler is a different operation: it
hallucinates plausible structure. `pruna/p-image-upscale` at $0.005-0.04 does this.

It was tried properly: each stock 128x128 quadrant upscaled 16x to 4222px
(`enhance_details` and `enhance_realism` on), reassembled. Verdict: **better than
stock, worse than a generated source.**

- It is genuinely sharper than stock and keeps the original composition exactly.
- But 16x from 128px is beyond what the model can invent — the result is still soft and
  cloudy, just cleanly soft rather than blockily soft.
- It **hallucinated chroma**: red and green mottling across what was a near-monochrome
  blue plate, plus a fern-like artifact in one corner. Per-channel matching pulls the
  hue back but the mottling stays.

Kept as candidate A because it is the only option that preserves the stock art exactly.
Candidate B (a generated source, upscaled from 1254px, a much gentler 3.4x) has far
finer filament structure.

## Seamless tiling only works on homogeneous sources

`make-seamless.sh` heals invisibly on a fractal, homogeneous texture and **cannot** heal
a source with large-scale composition — a bright plume, a big dark void. Five approaches
were tried on the Seedream crops and all failed:

| Approach | Result |
|---|---|
| roll + patch heal (quarter-rolled copy) | the patch is the wrong *content*; reads as a soft-edged rectangle |
| wider feather (96 -> 250 -> 450) | makes the bad region bigger, not subtler; edge RMSE identical, since the heal never touches the border |
| roll + blur heal over a narrow band | a blurred discontinuity is still a discontinuity — the cross stays visible |
| mirror tiling (2x2 flip/flop) | exactly seamless, but an unmistakable kaleidoscope |
| searching all 2048 windows for naturally matching edges | best found was L\|R 0.10 / T\|B 0.25 against stock's 0.023 / 0.058 |

The reason is structural: seamless tiling needs either homogeneous texture or invented
content, and nothing local can invent a transition between two genuinely different
regions. Note also that roll-and-heal **moves** the seam rather than removing it — from
the tile border to the tile centre. On a face that fills the viewport, a line through
the middle is worse than a line at the edge.

**So the choice is real, not a defect to fix:** a homogeneous source can be seamless but
looks like uniform texture; a composed source looks better per-face but shows joins.

## Controlling how much a generative upscaler invents

Upscaling stock looked best, but the model invents detail that was never in a 128px
plate — fine filaments, a frost-fern structure in one corner, and red/green mottling
across what was a single-hue blue image. Two dials, and one non-dial:

**`enhance_details` / `enhance_realism` are not the dial.** Turning both off is *worse*
for the invented filaments, not better: the output is sharper, so the fern reads more
clearly. Enhance-on merely blurs its own inventions. Measured on quadrant 0:

| | R-G deviation | high-freq energy |
|---|---|---|
| stock (128px native) | 1.89 | — |
| Lanczos to 2048 | 1.97 | 0.594 |
| AI, enhance **off** | 3.10 | 1.685 |
| AI, enhance **on** | 5.90 | 2.016 |

**Dial 1 — blend toward Lanczos.** `-define compose:args=N -compose blend` between a
plain Lanczos upscale and the AI one attenuates every invention uniformly. 35% keeps a
visible sharpness gain while pushing the fern close to invisible.

**Dial 2 — `MONOHUE=1`.** Rebuilds all three channels from *luminance* using the
reference's hue ratio. Valid whenever the stock texture is effectively single-hue, which
MBG02 is, so its colour carries nothing its luminance does not. This removes chroma
invention by construction rather than by degree: R-G deviation 7.61 -> **2.47**, against
stock's 2.51.

`MAXGAIN` deliberately does **not** apply on the `MONOHUE` path. Rec.709 grey weights
blue at 0.114, so a blue-dominant plate needs a gain near 3 to reach its own blue mean;
capping at 2.5 undershot the whole texture to mean 12 / B22 instead of 15 / B30.

Candidate **D** = stock, AI-upscaled, blended 35%, `MONOHUE=1`. Matches stock on mean and
hue with a third of the high-frequency invention of candidate A.

    ./a2tex build MBG02        # size=2048 uniform=0 monohue=1 in target.conf

## Cube-face seams, and why all four tiles are the same image

Observed in game: the six faces meet at visible edges, so a tile that does not join
itself shows a hard line at every cube edge. This drives three decisions.

**1. The source is made seamlessly tileable** — `./make-seamless.sh <in> <out> [feather]`.
Rolling by exactly half puts formerly-adjacent pixels at the left and right borders, so
the outer boundary becomes seamless for free and the discontinuity moves to the centre
cross, where it can be healed without touching the edges. The heal blends in a
quarter-rolled copy of the same texture through a smooth mask.

Build the mask with `-fx`, not with drawn rectangles. A `-draw rectangle` plus `-blur`
leaves hard corners where the two bands cross and a visible step where the border clamp
begins; both were plainly visible as rectangular blocks on the dark texture. The `-fx`
version uses raised cosines. Note `-fx` rejects `d` as a variable name — it parses as
an operator and the error (`Expected operand`) does not say so.

| | L\|R | T\|B |
|---|---|---|
| source as generated | 0.183 | 0.206 |
| after `make-seamless.sh` | **0.027** | **0.027** |
| stock `MBG02` for reference | 0.023 | 0.058 |

**2. All four tiles are the same seamless image.** Six faces draw from four quadrants,
and which quadrant lands beside which is decided by SOD UVs that could not be parsed.
Cross-tile joins therefore cannot be guaranteed — but if every tile is the *same*
self-tiling image, every join matches by construction. Repetition is the price, and it
is one the stock game already pays; `Mnebula4` ships the same image in all four
quadrants and reads fine.

**3. `UNIFORM=1` is required when doing this.** Stock's quadrant means differ
(11/16/17/16), so per-quadrant matching would hand identical tiles *different* gains
and put a brightness step at every face join — undoing the whole exercise. `UNIFORM=1`
matches every tile to the whole stock texture instead. Result: all four quadrants at
mean 15, R8 G7 B30, and a self-seam of 0.0169 against stock's 0.0227.

    # uniform=1 in target.conf, then: ./a2tex build MBG02 && ./a2tex install MBG02

## Channel synthesis: when a channel is zero

`fit()` matches each channel by multiplication, and **zero times anything is
zero**. This is not hypothetical: the shipped source came back with `mean.r` of 0.49
against `mean.b` of 91 — effectively no red at all — and no gain could reach stock's
R8, leaving the sky cyan.

It is rebuilt *from* the strongest channel, scaled to the target mean. That keeps blacks
black — unlike adding a constant — and preserves structure. The build prints a note when
it fires.

**The trigger took two goes to get right, and both wrong versions failed the same way:
by firing on the safe direction.** `t` is the target mean, `v` the source's.

| test | what it broke |
|---|---|
| `v < 0.15 * strongest` (original) | a *ratio*, so it misfires on any strongly single-hue plate. The Borg sky is G69/B3 — its blue is real and scalable at 4% of green, and this threw it away and rebuilt blue from green on all six faces |
| `v < 1.0 or t/v > MAXGAIN` | the absolute floor still fires on *dimming*. `MbgKlin4`'s blue is v 0.39 → t 0.32, a gain of **0.82**, and it was still discarded and rebuilt from red — on `MbgKlin4`, `MbgKlin2`, `MbgDom2` and `MbgRom2` |
| `v <= 0 or t/v > MAXGAIN` (current) | — |

    synthesise if   v <= 0   or   t/v > MAXGAIN

The channel literally cannot be scaled, or the gain needed is beyond what a stretch does
without banding. **Dimming a faint channel is always safe; only lifting is dangerous**,
and any test with a floor on `v` alone cannot tell the two apart. Synthesis now fires on
no skybox set at all. Neither shipped texture ever changed: `Mnebula4` is greyscale and
`MBG02` takes the `MONOHUE` path.

## Procedural fallback: `gen-nebula.sh`

    ./gen-nebula.sh <seed> <out.png> [size] [r] [g] [b]

Multi-octave fractal plasma, crushed toward black, with a screened filament layer and
exact per-channel colour weights (`0.27 0.23 1.00` reproduces MBG02's measured R8 G7
B30). It exists because it needs no network, no account and no credits, and because
the histogram is directly controllable — the four tiles it produced for `MBG02` match
stock's mean, per-channel hue and all four quadrant means exactly.

**What it cannot do is fibre.** Stock's character comes from fine *coherent* filaments
that curve and branch. Procedural noise gives either soft blobs or incoherent speckle.
Three alternatives were tried and all were worse:

| Attempt | Result |
|---|---|
| Flatter octave weights, finer high octaves | mid-frequency haze filled the voids; lower contrast |
| Ridged noise via `-solarize 50%` at each octave | still smooth — upscaling small plasma kills the high frequencies |
| Full-resolution `+noise Random` blurred at 1.2/2.6/5.0px | reads as speckle and fake stars, not gas |

The weights in the script are the tuned version. Don't re-tune without re-reading this
table. **A real image model is the right answer for this texture** — confirmed: the shipped
`MBG02` is model-generated. The fallback is for when one is not reachable.

### inference.sh / `belt` — the route that produced the shipped MBG02

`belt` is at `~/.local/bin/belt` and authorized. **`MBG02` was generated this way and
is installed.** Total spend for the whole exercise, including four models' worth of
failed experiments: **$0.31**.

    belt balance                      # refuses to submit at $0.00, with a clear error
    belt app get <id>                 # input schema
    belt app estimate <id> --input …  # price before running
    belt app run <id> --batch in.jsonl   # one JSON object per line, runs 4 at a time

Model choice is in `PROMPTS.md` and is counter-intuitive: the **$0.001** model
beat the $0.04 and $0.035 ones, because the binding constraints for a tile are
compositional (fill the frame, no focal point, no stars) and the bigger models insist
on composing a subject. Check `size`/`aspect_ratio` before spending — Seedream's "2K"
is 2560x1440, not square; only its 4K is.

Outputs are URLs; `curl -sL -o` them. Klein returns `.jpg` even when asked for png.

## The skybox sets: a census

There are **23 skybox sets, 136 files**, all 256x256 24-bit except the `MBG02` atlas.
Reading the background field out of all 72 `.bzn` maps (recipe in the engine-reference
section above) accounts for every map and says which sets actually matter:

| maps | set | | maps | set |
|---|---|---|---|---|
| 16 | `MBG02` (14 as `mbg02.sod`, 2 as `mbgblue.sod` — byte-identical models) | | 5 | `mbgdk` |
| 9 | `mbgpur` | | 4 | `mbgkl` |
| 6 | `MbgBorg` | | 2 | `mbgflu` |
| 6 | `mbgred` | | 2 | `mbggb` |
| 6 | `MbgKling` | | 1 | `MbgBaku` |
| 5 | `mbgrg` | | 0 | `MbgCard`, `MbgDom2`, `MbgIkol`, `MbgKlin2`, |
| 5 | `mbgaqu` | | | `MbgKlin3`, `MbgKlin4`, `MbgOmega`, `MbgRom1`, |
| 5 | `MbgDom1` | | | `MbgRom2`, `MbgRom3`, `Mbgstars` |

Thirteen sets cover all 72 stock maps; eleven are unused by the shipped campaign and
skirmish maps and are presumably there for mods and the map editor.

Two files are not part of any set:

- **`Mbgstars.tga` is a starfield, and must not go near a generative upscaler.** Mean 3,
  maximum 255, and only 4% of its pixels above 32/255 — it is sparse bright points on
  black. An upscaler has no way to keep a 1px star a point; it turns each one into a
  blob or a small galaxy. This is the same category as the fonts and UI in
  `REMASTERING.md`: leave it alone.
- **`MbgBaku1.tga` is not a duplicate of `MbgBaku.1.tga`.** Different content (58/38/54
  against 68/33/20), different row origin, and no set of six around it — an orphan. It
  is nonetheless carried as a 7th file in the `MbgBaku` target, which is correct rather
  than accidental: `sky-faces` pairs `src/` to `stock/` by sorted name, `MbgBaku.N` sorts
  before `MbgBaku1` in both lists, and it gets its own upscale matched to its own means
  (50, 57/37/54). A set does not have to be exactly six files.

## Upscaling a skybox from its own stock faces

This is the `MBG02` candidate-D recipe applied to the 6-face sets, and it needs **no
generated art at all** — the source is the game's own texture.

    tools/upscale-stock.sh [--blend N] [--mp N] [--reblend] [--force] TARGET...
    ./a2tex build TARGET

Per face: `stock/` 256x256 → generative upscale to 2048x2048 → blended `--blend`%
(default 35) over a plain Lanczos upscale of the same stock face → `fit()` matches every
channel back to the stock face's mean → TGA. About $0.005 and 17 seconds per face.

**Keep `ai/`, and the blend becomes free.** `--reblend` rebuilds `src/` from the `ai/`
layer already on disk, offline and at no cost, so the one dial that is genuinely a
matter of taste can be turned as often as you like. It is deterministic: a reblend at
the same setting reproduces the TGA byte-for-byte (the PNG in `src/` gets a fresh
timestamp, so compare pixels, not md5sums). The setting lives in `target.conf` as
`blend=`, not in a comment — otherwise a bare `--reblend` silently reverts a
deliberately lowered set to the default, which is exactly what would have happened to
`mbgrg`. `MBG02` predates this and its
intermediates were scratch; re-tuning its blend still means paying to upscale again.

### Invention falls off steeply with the scale factor

The finding that drove `MONOHUE` was measured at **16x from a 128px** `MBG02` quadrant,
where the upscaler tripled the R-G deviation. The 6-face sets are **8x from 256px**, and
the same model at the same settings invents nothing measurable:

| | R-G deviation vs stock | high-frequency energy |
|---|---|---|
| `MBG02` quadrant, 16x from 128px | 1.89 → 5.90 (**3.1x**) | 0.594 → 2.016 |
| `MbgBorg` faces, 8x from 256px | ratio **0.998 – 1.000**, all six faces | 2.80 → 14.78 raw, 5.94 at blend 35 |

So `MONOHUE=1` is right for the atlas and **wrong for the 6-face sets** — there is no
chroma invention for it to remove, and it would flatten the real hue variation that
per-channel matching preserves. Measure before reaching for it, with
`tools/huespread.py`, not with the raw HSL hue standard deviation (see below).

### Measuring hue spread honestly

`magick F.tga -colorspace HSL -channel H -separate -format '%[fx:standard_deviation]'`
is the obvious test and it is **misleading on a skybox**. Hue is undefined for near-black
and near-grey pixels, a skybox is mostly near-black, and the noise in the dead areas
dominates the number: `MbgDom1`, a single-hue violet plate, scores 102 that way.

`tools/huespread.py` weights each pixel's hue by the chroma it actually carries
(saturation x luminance) and takes the circular standard deviation. On the same files:

| set | raw HSL sd | weighted | set | raw HSL sd | weighted |
|---|---|---|---|---|---|
| `MBG02` | 30 | **0.8°** | `MbgDom1` | 102 | 40.8° |
| `mbgflu` | 2 | 3.9° | `mbgpur` | 99 | 54.6° |
| `MbgKlin4` | 11 | 3.4° | `MbgCard` | 77 | 72.9° |
| `MbgKling` | 93 | 10.5° | `mbgkl` | 83 | 122.6° (mean 2/255 — noise) |

`MBG02`, the one texture whose `MONOHUE=1` a human accepted, scores 0.8°. Under about
25° a plate is effectively single-hue. On a set as dark as `mbgkl` the number means
nothing either way and the decision has to come from the invention measurements.

### The stock row origin is mixed, and the build now matches it per file

`bottomup.py`'s original docstring claimed every stock TGA is bottom-up. **Measured, that
is false**: of the 135 skybox faces, 53 are `0x00` (bottom-up) and 82 are `0x20`
(top-down) — and the split runs *within* a single set, with `MbgBorg.2` and `.5`
bottom-up against the other four top-down. Both render correctly in the retail game, so
the engine honours the descriptor byte.

All nine puff sources, by contrast, are uniformly `0x00`.

`write_tga()` now takes the file being replaced as a third argument and copies its
origin via `bottomup.py --like`, so a replacement can never be flipped relative to the
original. The round trip is byte-exact, and both shipped textures still reproduce
byte-for-byte.

### `belt` batch notes

`--download` is **silently ignored** when `--batch` is combined with `--json`; nothing is
written and nothing complains. Parse the JSONL instead — each result line carries an
`index`, the 0-based line of the input file, which is the only reliable way to map a
result back to its input, because results come back out of order. Then `curl` the URLs.

### A zero target channel must be annihilated, not left alone

The worst bug of the skybox pass, and it was invisible to every per-set check because
only one texture in the game triggers it. `MbgOmega`'s stock blue is **exactly** 0 on all
six faces. In `fit()` the per-channel gain is `g = min(MAXGAIN, t/v)`, which is 0 when the
target mean `t` is 0, and the only guard was against dividing by zero:

    print(round(100/g,4) if g>0 else 100)      # 100 == -level 0%,100% == identity

So the channel that most needed to be removed was the one channel passed through
untouched. `MbgOmega` built with blue at **33–88/255** against stock's 0 — an amber sky
rendered violet. `fit()` now blanks the channel outright, on both the per-channel and the
`MONOHUE` path, and says so:

    MbgOmega.1.png: B target mean is 0, channel blanked

The general lesson: a fallback chosen to avoid an arithmetic error is still a *choice
about the image*, and "do nothing" is the wrong choice exactly when the target is zero.
Audited across every stocked texture, `MbgOmega`'s six faces are the only ones in the
game with an exactly-zero channel — `MbgKlin4`, which looks identical in `a2tex diff`
(`17/8/0`), is really 0.22–0.45 and scaled correctly all along.

### Two checks that are weaker than they look

- **R-G deviation is insensitive on a strongly-hued plate.** `MbgKling`'s stock R-G is
  242 and `MbgBaku`'s 140 — dominated by real hue, so invented noise cannot move the
  ratio. Near-parity on those sets is not strong evidence; on `mbgaqu` (R-G ~31) the same
  check is genuinely sensitive. Where the metric is weak, the high-frequency-energy
  comparison carries the signal instead.
- **"maxima below 255" cannot pass on a set whose stock already clips.** `MbgKling`,
  `MbgBaku`, `MbgCard` and three `MbgDom1` faces have stock maxima of 255 (stars). The
  criterion is really **"no clipping beyond stock"**, measured as the saturated-pixel
  *share*: on `MbgKling`/`MbgBaku`/`MbgDom1` that share fell (0.71% against stock 1.34%),
  on `MbgCard` it rose 1.4–2.5x, which is what an 8x upscale of point highlights does —
  it spreads each clipped star over more pixels. `mbgpur2` is the one face that clips
  where stock does not (0.037% of pixels), and the blend is not the lever: plain Lanczos
  with zero AI contribution already reaches 254–255 there.

### Pair `src/` to `stock/` by name, not by sort position

`build_sky_faces` used to `sort` both lists independently and pair by index. That is not
a pairing rule: glibc collation ignores punctuation, so `MbgBaku.1` / `MbgBaku1` /
`MbgBaku.2` tie and are separated only by a byte-level tiebreak. It agreed on both sides
here, which is luck. Two names differing only in punctuation would silently swap two
faces of a cube — a failure that looks like bad art rather than a bug. Pairing is now by
name, falling back to sort order (with a warning) for hand-supplied art whose names do
not match.

## Beyond the nebulae

`REMASTERING.md` carries the measured inventory of all 2120 textures and what would be
needed to apply this pipeline to them. The short version, because it changes how these
scripts should be read:

- **1113 of 2118 textures are 32-bit with a live alpha channel.** `write_tga()` ends in `-alpha off -type TrueColor`, which is right for the 24-bit
  additive nebula textures and would destroy everything else.
- **347 hand-authored mip chains** (782 files) must be regenerated, not ignored.
- **Fonts, UI, cursors, wireframes and minimap art must never be hallucinated into.**
- Upscaling every base texture 4x would take the set from 254 MB to about 12 GB.

## Still open

- **`MBG02` is FINAL** — candidate D (stock upscaled, 35% blend, `MONOHUE=1`, face
  2048), accepted. `./a2tex build MBG02` reproduces it byte-for-byte.
- **`MBG02` was previously installed** (1024x1024, seamless, `UNIFORM=1`, per-channel
  matched to stock with red synthesised from blue). Not yet seen in game — the thing to
  check is whether four identical faces read as repetition from inside the map.
- Peak brightness is short: built max 86 against stock 152. The mean and hue are exact;
  stock simply has a few isolated hot pixels the source does not. `MbgBorg` is the next best target: six faces, four
  missions, brightest of the three.
- The `MbgBorg`/`MbgDom1`/`MbgKling` prompts still describe a centred core only in
  prose; the **corner notches are not yet reproduced by the `sky-faces` kind**, which would
  need a corner mask. Generated faces for those sets will have square corners where
  stock has black ones. Not yet seen in game, so the visual cost is unknown.
- ~~`build.sh` requires exactly 4 images per atlas~~ — done: the `puff` and `sky-atlas`
  kinds now repeat the supplied images to fill four quadrants, so 1 image is enough.
- 1024x1024 textures are untested.
