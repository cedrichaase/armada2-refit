# Armada II nebula regeneration

Tooling and reference notes for replacing Star Trek: Armada II's nebula textures with
higher-resolution generated art. Built against the GOG release running under
Heroic/Proton on Arch.

Game directory: `/home/cedric/Games/Heroic/Star Trek Armada II`
Textures live in `Textures/RGB/` (flat, 2115 `.tga` files, 196 MB stock).

> Sizes here are **byte totals**, not `du`. `du` reports 205 MB for the same stock set,
> because 2115 small files carry about 9 MB of filesystem block slack. Older notes in
> these documents quote the `du` figure; where the two disagree, this is why.

---

## Layout and workflow

    a2tex                 the only entry point
    lib/                  common.sh (fit, TGA writing), puff.sh, sky.sh
    tools/                bottomup.py, upscale-stock.sh, huespread.py, verify.py,
                          make-seamless.sh, gen-nebula.sh, tgapad.py,
                          ui-widescreen.py, fix-enterprise-registry.py
    targets/<NAME>/       target.conf, stock/, ai/, src/, out/  -- one per texture
                          (+ src-alpha/ when the target sets alpha=ai)
    archive/              material that cannot be regenerated cheaply or at all --
                          upscale candidates, reference plates, stock mip chains, and
                          the madExcept capture of the end-of-mission crash
    .scratch/             transient, safe to delete at any time, and periodically is

Four layers per target, and the order matters:

| | |
|---|---|
| `stock/` | the pristine original, copied out of the game. Never written |
| `ai/` | the raw generative upscale, one file per face. **This is what the credits bought** — it is the only layer that cannot be recreated for free, and it is not in git |
| `src/` | what `a2tex build` reads: `ai/` blended over a plain Lanczos upscale, or hand-supplied art |
| `out/` | the finished TGA, ready for `a2tex install` |

What each layer costs on disk, across all 52 targets, and what it takes to lose it:

| layer | size | committed | to recreate |
|---|---|---|---|
| `stock/` | 76 MB | **yes** | copy out of the game, or from `.a2neb-backup` |
| `ai/` | 869 MB | no | **money** — this is the only layer credits bought |
| `src/` | 534 MB | no (see below) | free and offline: `--reblend` |
| `src-alpha/` | 0.7 MB | no | free and offline: `--reblend` |
| `out/` | 2.2 GB | no | free: `./a2tex build` |

**Back up `ai/`.** Everything else in `targets/` is either committed or one command away;
`ai/` is neither.

`targets/MBG02/src/` is the one exception to `src/` being derived, and it is committed on
purpose: `MBG02` predates the `ai/` layer, its intermediates were scratch and are gone,
and those four tiles are the only remaining way to reproduce the accepted candidate D
byte-for-byte. `.gitignore` says so too.

A target with `alpha=ai` gets a further directory, `src-alpha/`: the upscaled alpha plate, blended
the same way, one PNG per face. It is its own directory rather than another file in
`src/` because `build_sky_faces` requires exactly one `src/` image per stock face, and
an extra `<base>.alpha.png` would trip that count check as a "needs 3 images, found 6"
skip — which reads like a missing file, not like a design.

`src/`, `src-alpha/` and `out/` are all derivable, so all three are gitignored; `ai/` is
gitignored only because it is 869 MB.

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
| `kind` | `puff` (2x2 atlas, additive, engine-tinted), `sky-atlas` (2x2, keeps colour), `sky-faces` (N files, one image each), `plain` (alias for `sky-faces`, for targets that are not skies) |
| `size` | output edge **per face/quadrant**, not per atlas. One skybox face fills the whole viewport, so this is what the eye sees |
| `uniform` | sky-atlas only: match every tile to the whole stock texture rather than its own quadrant. Needed when all four tiles are one seamless image, otherwise the differing quadrant means put a brightness step at each cube-face join |
| `monohue` | rebuild all channels from luminance using the stock hue ratio. Only valid on single-hue textures; removes chroma invention by construction |
| `blend` | per cent of the AI layer kept over Lanczos in `src/`. Read and written by `upscale-stock.sh`, so a set tuned away from the default keeps that setting. `mbgrg` is the only one at 20 |
| `source` | puff only: `gen` (default) treats `src/` as generated art of unknown framing — trim, square, inset to `fill`%, stretch the noise floor. `stock` treats it as this atlas's own upscaled quadrants and skips all of that, **including the greyscale conversion** |
| `mips` | number of hand-authored mip levels the stock texture has. The build emits `<name>_1..N`, each exactly half the previous. **Required** for any texture with `_N` siblings — `install` refuses the base otherwise |
| `keepcolour` | puff only: skip the greyscale conversion. `Mnebula2` is the one stock puff with its own colour |
| `fill` | puff only: percent of the quadrant the subject occupies (default 92) |
| `sheet` | `GxP`: pack the units into GxG contact sheets with a P-pixel gutter and upscale the sheet, then slice back. A quality setting before a cost one — the app's `megapixels` is an integer, so a lone 64px icon gets a 16x lift at the 1MP floor where a 512px sheet of 49 gets 4x |
| `maxsize` | refuse to install this target if any output is wider than N. `install` measures first and refuses the **whole** target. The UI targets declare 256, because a `@tmaterial=interface` sprite above that **crashes the game** |
| `blackedge` | force the outer N texels of the image and of each 2x2 quadrant to exact black. Additive sprite atlases only — `Mmoon`, whose sun quadrants would otherwise draw a faintly glowing square. Runs on the RGB plate **before** alpha is attached |
| `alpha` | `ai` sends the **alpha plate** through the upscaler too, at the same blend, leaving the result in `src-alpha/`. Off by default and wrong for a coverage mask; correct for a hull texture, whose alpha is a self-illumination map. See the hull section |
| `install` | `no` means built but deliberately not shipped. `UImid` is the case: it carries the fog-of-war and minimap textures, and doubling them froze the game |
| `note` | free text, ignored by the tooling — why this target is configured the way it is |

### Parallel builds

`-j` is safe. Every build gets its own `mktemp -d` scratch directory and writes nothing
to a fixed path — the previous scripts shared `work/_s.png` and `work/_q0.png`, which
silently corrupt each other when two targets run at once.

## Current state

**52 targets, 734 files built, 708 shipped.** `./a2tex verify` checks every one of them
against the file it replaces, from the raw TGA bytes, and currently reports **0
problems** over 720 textures — the count differs because the 14 mip levels are checked
as part of their base rather than on their own. Run it after every build and before
every install.

| class | targets | files | state |
|---|---|---|---|
| skyboxes | 23 | 134 | **confirmed in game.** 22 six-face sets at face 2048, plus the `MBG02` 4096 atlas (candidate D) |
| map puffs | 8 | 12 | **confirmed in game.** 1024x1024, 512 per quadrant, upscaled from their own quadrants. Includes `Mnebula2`'s rebuilt 4-level chain (`mips=4`) — upscaling its base alone had crashed the Klingon campaign |
| named planets | 11 | 11 | **confirmed in game.** `kind=plain`, 2048, from their own stock |
| class planets | 2 | 16 | installed. `PB_CLSS*` grounds and `PA_*` cloud layers, 2048 |
| moons / suns / rings | 4 | 8 | installed. The first **32-bit** textures here. `mdmoon` carries a rebuilt 4-level chain |
| UI | 3 | 544 | **confirmed in game**, 518 of them. Icons 64→256 via contact sheets, panels at 256. `UImid` (26) is `install=no` — see the crash section |
| hull | 1 | 9 | **confirmed in game.** The Sovereign: `Fbattle`, `FEntE`, `Fbattle_b` at 1024 with rebuilt 2-level chains, and the first target whose **alpha goes through the upscaler** (`alpha=ai`). `blend=70`, not the usual 35, and the registry's two Cs re-cut by hand — see why below |

Four things cost a crash, a freeze or a visible artifact, and each one is written up
below rather than only fixed: **a UI sprite over 256x256 crashes the game**, **doubling
a fog-of-war or minimap texture freezes it**, **compositing onto an alpha-bearing image
corrupts colour**, and **`earth.tga`'s fourth byte is padding, not alpha**.

Every installed file has a `.a2neb-backup` beside it; `./a2tex revert all` undoes the
lot. `Textures/RGB` went from 196 MB to 2.27 GB.

There are **734 backups against 708 shipped files.** The 26 extra are `UImid`, which was
installed, froze the game, and was reverted; its backups stay beside the stock files they
restored. `./a2tex verify` skips `install=no` targets for exactly this reason, and the 26
game files have been confirmed byte-identical to their backups.

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

**Set the dial on a render at the object's real on-screen size.** 35 is right for a
skybox because a skybox face fills the viewport — 1:1 and the game agree about what is
visible. A *ship* does not: a Sovereign draws about 340 px wide, its saucer takes 140 of
256 texels, and the invented hull glyphs that 35 exists to suppress land at well under a
pixel. Judged at 1:1, 35 looked careful; in game it was indistinguishable from stock.
That target now runs at 70. Crop the region, resize it to the pixel width the object
actually occupies, and compare *there* — for anything drawn small, the invention budget
is much larger than pixel-peeping suggests. See the Sovereign section below.

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

There are **24 skybox sets, 135 files**, all 256x256 24-bit except the `MBG02` atlas.
23 of those sets were upscaled (134 files); `Mbgstars` is the one left stock.
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

## Upscaling the puffs from their own quadrants

Same idea as the skyboxes, three differences that matter.

**The unit is a quadrant, not a file.** A puff is one 128x128 atlas whose four 64x64
quadrants are carved by `Sprites/nebula.spr` at `@reference=128`. `upscale-stock.sh`
slices them into `.units/` and the rest of the flow is shared with `sky-faces`.

**64px is small, so `--mp 1`, not the default 4.** 4 MP from a 64px quadrant is a 32x
lift. 1 MP is 16x, downsampled to 512/quadrant — an 8x delivered lift, the same ratio
that invented nothing on the skyboxes.

**`source=stock` skips the greyscale conversion, and that is not an optimisation.**
Greyscale exists because the engine tints these sprites, so coloured *generated* art
would be tinted twice. When the source is the stock art it already carries stock's exact
colour, and grey-then-re-tint is a lossy round trip. Measured on `Mnebula1`, whose stock
is 28/26/22 rather than neutral:

| | gains | peak R | clipped px |
|---|---|---|---|
| greyscale, then `fit()` rebuilds the warmth | 0.93–0.96 | 255 (stock 246) | 109 |
| keep the source's colour | 0.97–1.00 | 248 | **0** |

Rebuilding a warm tint by stretching red back out of grey is what clipped it. Note this
also means the CLAUDE.md rule "R, G and B means identical for map puffs" describes most
of the set but **not all of it**: `Mnebula1` is 28/26/22 and `Mnebula2` is 33/9/8. Match
the stock file, not the rule.

**Quadrant edges come out at exactly 0** on all eight atlases — the vignette still runs,
and it has to: stock's own quadrant borders reach 11/255, and the sprite UVs
(`0 0 64 64` of a 128px reference) sample the edge pixels exactly, so stock itself has a
faint additive seam that the rebuild removes.

**The upscaler is deterministic.** `Mnebula4` and `MFluidicNeb` are byte-identical stock
files, were upscaled in two independent runs, and produced byte-identical `ai/` layers
and byte-identical output. Do not expect run-to-run variation to give you free variety.

## Mip chains, and the crash they caused

**`Mnebula2` was the first texture installed here with a hand-authored
mip chain, and upscaling its base while leaving that chain alone crashed the game.**

A mip chain is the set of pre-shrunk copies of a texture the GPU uses when the surface
is small on screen. Without them, drawing a 1024px texture across ten screen pixels
makes each pixel grab one essentially arbitrary texel, and the result crawls and
sparkles as the camera moves. The hardware picks the level whose texel density matches
the pixel density and blends between two of them. Normally the driver generates the
chain automatically by repeated box-filtering; Armada II instead lets an artist ship one
explicitly, as sibling files:

    Mnebula2.TGA    128x128     level 0
    Mnebula2_1.tga   64x64      level 1
    Mnebula2_2.tga   32x32      level 2
    Mnebula2_3.tga   16x16      level 3
    Mnebula2_4.tga    8x8       level 4

Each level is exactly half the previous one. That is not a convention, it is what the
API requires, and it is why the crash happened: a 1024x1024 base above a 64x64 level 1
is not a valid chain, and creating the texture fails. The d3d8to9 -> DXVK path is
stricter about this than the original runtime would have been.

**Why hand-author a chain at all?** Because averaging is the wrong shrink for some
images. An additive nebula puff box-filtered down loses its bright core and fades toward
grey mush, and since the blend is additive, a duller small mip means the nebula visibly
*dims* as the camera pulls back. An artist can keep the core punchy and the total light
roughly constant instead. The same logic covers alpha-tested foliage (averaging alpha
eats the coverage) and anything that has to stay readable when tiny. In 2001 there was
also a practical reason: driver mip generation varied between cards, so shipping the
levels guaranteed what players saw.

**The fix.** `mips=N` in `target.conf` makes the build emit the chain alongside the
base, and `a2tex install` validates it: a texture with `_N` siblings whose base size
changes is installed only if this build supplies every level at exactly half the one
above. The check is a **pre-flight over the whole target**, not per file — the mips sort
before the base, so an in-loop check installed `_1`/`_2`/`_4` and only then refused the
base, leaving a stock base under upscaled mips. That is the same invalid chain, inverted.
`a2tex revert <target>` restores the chain too, for the same reason: `stock/` holds only
the base, so it used to leave 512px mips over a 128px base — reached by the command
meant to make things safe.

**Use a Box filter, and no peak lift.** At exact powers of two a box filter is a pure
area average, so it preserves the mean — and with additive blending the mean *is* the
light contributed, so a filter that dims the small levels makes the nebula fade as the
camera pulls back. Stock's authored chain does lift the peak at its two smallest levels
(16px: 228 against a box filter's 217), but reproducing that as a per-level `-level`
compounds — it drifted the mean 33 → 37 over four levels where stock holds 33 — and it
lifted the quadrant edges off black. It only matters at sizes a 1024 base never reaches.

The rebuilt chain against stock's, per level:

| | stock | rebuilt |
|---|---|---|
| sizes | 128 / 64 / 32 / 16 / 8 | 1024 / 512 / 256 / 128 / 64 |
| mean | 33, 33, 33, 34, 36 | 33 at every level |
| peak | 249, 255, 246, 228, 175 | 253, 251, 250, 249, 248 |
| worst quadrant edge | 11, 14, 19, 47, 78 | 0, 0, 1, 2, 7 |

Stock never forced its mip edges to black, and box-filter bleed makes them worse at every
level; the rebuild is better than stock on that axis by a wide margin.

`REMASTERING.md` counts **363 hand-authored chains, 782 files**, across the whole texture
set. For the nebulae it is a single texture; for the hull textures it will be most of
them.

## Planets and moons, and the first 32-bit textures

Fifteen textures, done in two passes. They are the first thing here that is not a
nebula, and the second pass is what finally made the pipeline alpha-aware.

### The eleven planet maps

`Mearth` `Mearthbg` `Mbaku` `MBarisa` `Mborgpl` `MEridon` `MKrios` `MLankal` `MRemus`
`MRomulus` `MQonos` — all 256x256, 24-bit, **no alpha and no mip chain**, which makes
each one structurally identical to a single skybox face. They run the `sky-faces` code
under the alias `kind=plain`; a planet target reading `kind=sky-faces` looked like a
mistake, and that was the whole reason for the alias.

They are **equirectangular sphere maps**, so two things matter that did not for a
skybox:

- **They wrap horizontally**, and the upscaler does not know that — it sees two picture
  borders. Measured before and after: column 0 against the last column, compared with
  two columns the same distance apart in the middle of the picture (one *stock* texel =
  8 built columns, not 1, or the baseline is unfairly tight). Stock's wrap difference is
  0.6-1.1x its local variation; the 2048 builds come out at 1.0-1.45x. The seam is
  inside the range of normal variation and no wrap-padding pass was needed.
- **The poles are the stretched top and bottom bands**, where a huge number of texels
  cover very little sphere. Nothing was done about this; it costs resolution but
  reproduces stock exactly.

Measured across all eleven: every channel mean matches stock (one off-by-one on
`MLankal`'s red), R-G deviation is within 1% of stock's own on every file, and no file
clips where stock does not — `Mearth` has 20.8% of pixels at 255 against stock's 20.9%,
which is its polar ice, inherited rather than introduced.

**`MQonos` is the one at `blend=15`.** Difference maps against the Lanczos layer are
the useful tool here: on ten of them the AI's contribution *follows existing structure*
— coastlines on `Mbaku` and `MRemus`, Borg circuitry on `Mearthbg`, lava veins on
`MKrios`, band turbulence on `MBarisa` and `MLankal`. On `MQonos` it was sparse, long,
free-floating hairlines over an otherwise empty field: invention, not recovery, and
visible against a smooth surface. That is what the dial is for.

### The four 32-bit ones

| | | |
|---|---|---|
| `mdmoon` | 256x256 + **4 mips** | the dilithium moon; alpha is a network of fissures |
| `Mmoon` | 256x256, 2x2 atlas | the moon (`alphathreshold`) and three suns (`additive`) |
| `Mbakurng` | 256x256 | the Ba'ku ring; alpha is the band mask |
| `earth` | 128x128 | a pre-rendered Earth billboard; alpha bits **0**, so unused |

Four things had to change, and all of them are the general 32-bit problem, not
something about moons:

1. **`bottomup.py` accepts 32-bit.** The stride is `w * bpp/8`, and — this is the part
   that is easy to miss — the descriptor's **low nibble is the alpha bit count**, not
   padding. `--like` now copies the reference's *whole* descriptor when the depths
   match, because stock is not consistent even there: the moons carry `0x08` and
   `earth.tga` carries `0x00` at the same 32 bpp.
2. **`write_tga()` takes its depth from the stock file**, exactly as it already took its
   row origin. With no stock file the old 24-bit behaviour stands, which is what every
   nebula target relies on.
3. **`attach_alpha()` carries the stock alpha across, Lanczos-upscaled, and the alpha
   never goes near the upscaler.** Alpha here is a mask — a fissure network, a hard
   cutout disc — and a model that invents plausible detail into a mask invents holes in
   the object. There is also nothing to recover: a mask has no texture, only edges, and
   Lanczos resolves an edge exactly.
4. **Colour and alpha are resized separately, everywhere.** This is not fastidiousness.
   Resizing an RGBA image associates alpha and then un-associates it, dividing colour
   back out by a near-zero alpha — and the Lanczos layer for `Mmoon` came out at mean
   137 against stock's 43 because of exactly that. The failure is nasty because
   `fit()` *hides* it: it scales the whole plate down to make the mean match, so the
   only symptom is a correct average over a picture that is uniformly too dark.

`mdmoon`'s chain is rebuilt with `mips=4`, 1024/512/256/128, colour and alpha box-
filtered separately. Its colour mean holds 57 flat at every level, matching stock's 57;
its alpha holds 230 where stock's chain *drops* 230 -> 203 as it shrinks. That drop is
the artist thickening the fissures so they stay visible at distance, and it is
deliberately not reproduced, for the same reason the peak lift is not: stock's drop
starts below 128px, and this chain bottoms out *at* 128px.

`Mmoon` needed one thing the others did not. Its three sun sprites are
`@tmaterial=additive`, so black is transparent and any stray value on a quadrant
boundary draws a faintly glowing square around the sprite. Stock is exactly 0 on all
eight boundary lines; the upscale came back 1-3. `blackedge=8` forces the outer 8 texels
of each quadrant to black — 8 because stock's tightest quadrant has a guaranteed
all-black margin of exactly one texel, times the 8x scale. It restores stock's own
values and cannot clip real art. It applies to **additive atlases only**.

### What was left alone

`Mdmoonglo` (8x8) and `Mdmoonglo4` (64x64) are soft radial gradients with no structure
to recover; Lanczos and a generative upscale give the same thing, so they stay stock.

## The class planets: the ones actually on the maps

The eleven `M*` maps above are the **named story** planets — Earth, Qo'noS, Romulus,
Remus, Ba'ku and friends. They are not what most maps place. That is a second family
bound in the ODFs:

    groundTextureName     = "PB_CLSSD" | "PB_CLSSH" | "PB_CLSSK" | "PB_CLSSL" | "PB_CLSSM"
    atmosphereTextureName = "PB_CLSSJ"

Class D, H, J, K, L, M — the engine appends `1` or `2` for the two variants of each, so
eleven `PB_*` files — plus five `PA_*` cloud layers (`PA_BORG1/2`, `PA_ECFR0`,
`PA_ECNA1/2`) which are 32-bit, alpha being cloud density. The user reported a planet
looking "muddy" after the named set was done; it was `PB_CLSSH1`, still stock, 256px
carrying a sphere about 600px across — a 4.7x magnification. At 2048 the same sphere is
*minified*, which is the whole difference.

**These are not equirectangular.** They are a four-lobe gore unwrap — two mirrored
diamonds with unused black wedges between them — so the horizontal-wrap reasoning from
the `M*` maps does not transfer. What matters instead is that the upscaler not bleed
across the lobe boundaries, and the difference maps against the Lanczos layer show it
does not: the contribution stops dead at the gore edges and follows terrain inside them.
`PB_CLSSJ`, a soft-banded gas giant, gets almost nothing, which is the right answer for
a source with no structure to recover.

Channel means match stock on all sixteen and R-G deviation comes out at or slightly
*below* stock's own on every file.

## The UI: 544 textures, and why they went up in sheets

**The stretch had to be fixed first** — see `SETUP.md`. Upscaling a UI drawn through a
1600x1200 canvas stretched 1.79x sideways would have been sharpening the wrong picture.

`Sprites/gui_*.spr` and `cursor.spr` resolve to 569 distinct textures. Almost all of
them are `@tmaterial=interface` or `default`, neither of which forbids filtering — the
`#No filtering, ever.` bar that rules out the fonts does not apply here. 2285 of the
2317 resolvable sprite entries have `@reference` equal to their texture's real width, so
the UVs are true fractions and a bigger texture lands on the same rectangle; the 32
exceptions are all cursors, where `@reference` is the *frame* size of a horizontal
strip, which is equally scale-invariant.

### Sheets are a quality decision

The upscaler takes `megapixels` as an **integer**, so 1 is the floor — and 1MP from a
64x64 icon is a 16x lift, the regime where this model invents most. Packing 49 icons
into one 512px sheet (7x7 cells, 8px gutters: `8 + 7*(64+8) = 512` exactly) and asking
for 4MP returns 2048: a **4x** lift, the gentlest used anywhere in this project.

The app rounds to an integer scale factor — `scale = round(sqrt(mp*1e6/(w*h)))` — so a
sheet edge and a megapixel target pin the lift exactly. Verified against the same icon
taken both routes: the sheet keeps stock's shapes where the 16x individual upscale
restyles them (the two hands in `gbtrade` gain plausible but wrong fingers). It also
costs 1/49th as much, which is not the argument but is not nothing: 408 textures for
$0.055.

`sheet=GxP` in `target.conf` turns it on. `tools/upscale-stock.sh` packs, uploads,
slices back to per-unit `ai/` files, and the blend stage downstream is unchanged.

| target | n | stock → built | route |
|---|---|---|---|
| `UIicon` | 382 | 64 → **256** | `sheet=7x8`, 8 sheets |
| `UImid` | 26 | 128 → **256** | `sheet=3x32`, 3 sheets |
| `UIpanel` | 136 | 256 → **256** | individual at 1MP (4x), *supersampled* back down |

**256 is a hard engine ceiling, not a choice — see below.** `UIpanel` therefore ships at
stock's own size, but resolved from a 4x upscale and downsampled rather than resampled
from stock: cleaner edges for the same bytes.

The panels are **not** sheeted. They are 9-slice pieces whose edges are load-bearing —
they abut each other on screen — and a neighbour across a gutter is a risk with no
payoff when one panel already fills a 4x request on its own.

Sizes come from how big the things are actually drawn. `paletteSingleButtonArea` is
80x80 in the canvas, x1.2 on screen = 96px, so a 64px icon is magnified 1.5x and 256
leaves it 2.7x oversampled. That is comfortable, and it is also the most the engine will
take.

All 544 verified: no channel mean off by more than 1, no alpha mean off by more than 1,
no channel-count change, and every TGA header byte-identical to the file it replaces.

### A UI sprite over 256x256 crashes the game

This is the second hard crash this project has caused, and like the first it is a limit
that announces itself only by killing the process.

The first build shipped `UImid` and `UIpanel` at **512**. The game then crashed on every
campaign start -- Klingon and Borg both -- at the cinematic-to-HUD transition, which is
exactly when `gui_<race>.cfg` is parsed and the race's whole UI texture set loads. The
madExcept dump faults at `rep movsd` inside `Armada2.exe` at `0x4e97fd`: a memcpy, which
is what an allocation that was too small and never checked looks like.

The census that settled it: **exactly one stock texture in the entire game is 512x512**
-- `WshladSW.tga`, a weapon texture -- and **no stock `@tmaterial=interface` sprite
exceeds 256x256.** 2001-era UI code sizing a scratch buffer for the largest UI texture
it shipped with is the obvious reading, and 512 is the first time in the game's life
that buffer was asked to hold four times its content.

Reverting the 162 files at 512 and keeping the 382 icons at 256 fixed it, confirmed in
game. What this does *not* distinguish is a per-texture cap from exhaustion of the
resident UI set as a whole (the 512 build was 247MB of UI against stock's 43MB; the
shipped one is 125MB). It did not need distinguishing -- the same ceiling fixes both.

**3D model textures are not affected.** 2048 skybox faces, 2048 planets and a 4096 atlas
have all been running for weeks. The limit is specific to the sprite/UI path.

`maxsize=N` in `target.conf` now makes this machine-readable: `a2tex install` measures
every output first and refuses the **whole target** if any file exceeds it, the same
pre-flight shape as the mip-chain guard and for the same reason -- a partial install is
the bug, not a lesser version of it. The three UI targets declare `maxsize=256`.

### Deliberately left stock

- **20 cursors** — still not *upscaled*, but no longer untouched, and the reason given
  here was wrong. Deliberate pixel art, and the strips are non-square (so `fit()` would
  square them); but **"drawn at native pixel size" is false.** At 3440x1440 a 32x32
  frame draws at ~138x77 screen px, a 4.30x/2.40x blow-up, because the cursor code
  scales against a hard-coded 800x600 — so they were 1.79x too wide like everything
  else. `tools/cursor-aspect.py` squashes the art to 18 texels across, about each
  sprite's `@origin` hotspot, which is the *only* lever: the UI cursor is a hardware
  cursor sized from the sprite's texel extent, so no `.spr` number reaches it and the
  4.30 px/texel horizontal density is fixed no matter what. **An AI upscale cannot help
  them**, and that is now read out of the binary rather than inferred: `DrawCursor` is
  `xor eax,eax; ret`, and `SetCursor` allocates the cursor texture at
  `texW * [device+0x18]` by `texH * [device+0x1c]` — so the engine magnifies it itself
  by a constant, and more source texels just draws a bigger cursor at the same density.
  The lever for sharpness is those two device floats, which is code (an ASI hook like
  `tools/menuscale/`), not art. Derivation, and the in-game test that established the
  two draw paths, are in `SETUP.md`.
- **`colors`** — an 8x8 colour lookup table. Interpolating it blends the cells.
- **`logos`** — Activision, Bink, GameSpy and Mad Doc trademarks, on a splash screen
  shown once. Invented shapes where there is a right answer, for no gain.
- **`gminicon`, `gminisys`** — 32x32, drawn at minimap blip scale.
- **`MBuild`** — the line-art build overlay: `add_nomipf`, and the only UI texture in
  the game with a mip chain.
- **The 12 font atlases**, for the reasons in `REMASTERING.md`, which have not changed.

### Two bugs this pass found

- **`fit()`'s greyscale guard was a heuristic and had a false positive.** It refused any
  reference whose green and blue means were both 0 — meant to catch a 2-channel PNG
  written without `PNG24:`. `Gmneb2`, the red minimap nebula icon, is genuinely 107/0/0,
  and the guard `die`d on it: the target shipped 25 of 26 files and the only symptom was
  a blank column in the build report. It now tests the actual on-disk channel count,
  which is what it always meant.
- **`sheet=$(grep …)` with no match exits 1**, and under `set -euo pipefail` that kills
  the script *at the assignment*, before anything has been printed. A 136-file upscale
  reported success and did nothing. Same family as the non-matching-glob trap in
  CLAUDE.md hard rule 3; both greps in `upscale-stock.sh` now end in `|| true`.

## Two crashes, a freeze, and a white box: what the UI pass actually cost

Worth reading before touching another class. All four were found in game, none by the
checks that were in place at the time, and each one changed the tooling.

### 1. A UI sprite over 256x256 crashes the game

Covered above. The lesson that generalises: **census the largest size stock ships for
the class you are about to touch.** `maxsize=N` in `target.conf` now enforces it.

### 2. Doubling a fog-of-war or minimap texture freezes the game

`UImid` went 128 -> 256 and the game stopped crashing and started *hanging*, at the same
moment -- the cinematic-to-HUD transition. It was not memory: the build that worked was
121MB of UI and the one that hung was 125MB. And it was not `UIpanel`, whose 136 files
are dimensionally identical to the files they replace.

`UImid` was the only target in that install whose **dimensions** changed, and it carried

    Gfog  Gshroud  Gminimap  Gmabelt  Gmneb1..5  Gshadow

which are not panel art at all. They are the fog-of-war, shroud and minimap-overlay
textures, swept into a "UI" list purely because they are reachable from `gui_map.spr`
and `gui_global.spr`. Those systems are created exactly when the HUD appears and then
run every frame; something in them is written against a fixed cell size, and doubling
the texture spins rather than faults.

`REMASTERING.md` had carried a warning about minimap art which this session dismissed on
the grounds that `Gmneb*` are flat single-hue blobs. They are — but **"safe to upscale
as an image" and "safe to change the size of" are different questions**, and conflating
them cost a diagnosis. The whole target is now `install=no`: built, kept for
reproducibility, never shipped.

### 3. blackedge corrupted Mmoon, and the verification could not see it

In game: a white box around the yellow sun sprite. In the file: every quadrant's RGB
mean several times stock's (Yellowsun 31 -> 228), while the **alpha** was perfect.

The cause is the associated-alpha trap for the third time in this project. `blackedge`
composed its mask onto the plate *after* `attach_alpha` had attached alpha, so
ImageMagick worked in premultiplied space and un-premultiplied on write, dividing colour
back out by a small alpha. `-channel RGB` does not prevent it. The fix is ordering:
blackedge now runs on the RGB-only plate, before alpha is attached.

**The rule that keeps coming back: colour and alpha are handled apart and joined only at
the very end.** It has now bitten in `upscale-stock.sh` (the Lanczos layer at mean 137
against stock's 43), in `gen_mips` (avoided by construction), and here.

The more uncomfortable half is why it shipped. `Mmoon` *was* verified and passed — and
then `blackedge` was added, and only the newly-added property (quadrant edges at 0) was
re-checked. **Changing a build invalidates the whole invariant set, not just the part
you changed.** More discipline is not the fix; one command is.

### 4. earth.tga's fourth byte is padding, not alpha

Caught by that new command on its first run. `earth.tga` is the only file in the game
with `bpp=32` and a descriptor declaring **zero alpha bits** — 24-bit colour in a 32-bit
container, where the fourth byte is padding. ImageMagick reads it as opaque, so
`-alpha extract` returns 255 where stock stores 0. `tools/tgapad.py` detects the case
and `attach_alpha` reproduces the padding byte instead.

### `./a2tex verify`

`tools/verify.py` checks every built texture against the stock file it replaces, **from
the raw TGA bytes**:

| check | |
|---|---|
| header | idlen / colour-map / image-type / bpp / descriptor identical to stock |
| size | a power of two, and a whole multiple of stock |
| maxsize | honours `maxsize=` |
| channels | exact raw mean R, G, B and A within 1.0 of stock's |
| mips | if `mips=N`, exactly N levels each half the previous |
| installed | the file in the game directory is byte-identical to `out/` |

Raw bytes rather than ImageMagick **because of alpha**: ImageMagick has opinions about
associated versus unassociated alpha, and those opinions are what corrupted `Mmoon`. A
checker built on the library that holds the opinion cannot reliably see the damage. The
engine reads bytes; so does this.

It samples nothing — the first version strided every 11th pixel and reported ±3 errors
on high-variance icons, the same magnitude as a real defect, which makes a tolerance
meaningless. Strided slice sums over the whole plane are exact and fast enough.

Current state: **720 textures, 0 problems.**

## The first hull texture: the Sovereign, and alpha that is not a mask

`Fbattle` (the Sovereign-class battleship), `FEntE` (the *Enterprise*-E, also used by
`fedpod16.sod`) and `Fbattle_b` (the Borg-assimilated variant). All three are 256x256,
32-bit, with a **2-level** hand-authored chain — `_1` at 128 and `_2` at 64. Built at
**1024** with `mips=2`, so the chain comes out 1024 / 512 / 256.

`Fbattle_b` is worth a note: no SOD references it. The engine finds it by name when a
Sovereign is assimilated, which is why it is in the target even though nothing points at
it. Any hull target has to look for the `_b` sibling itself; there are 100 of them in
the set.

### Why this ship first

It is the smallest self-contained hull set in the game that is also the most
recognisable, which is exactly what a first target of a new class should be: three files
and six mips, one model each, and a result that is obvious on screen rather than
something that needs a measurement to see. It also happens to carry every property the
class has — 32-bit alpha, a hand-authored chain, a `_b` variant, and hull lettering the
upscaler can get wrong — so nothing about it is a special case that would fail to
generalise.

### 1024, not 2048

The app takes `megapixels` as an integer and rounds the scale factor, so a 256px source
is 4x at 1MP and 8x at 4MP — and both cost $0.005. The size was chosen on **memory**,
not price -- or so it was argued at the time. Armada2.exe is a 32-bit LAA process and
`Textures/RGB` held 2.27 GB. **That second figure is disk, not memory, and the argument
leans on it as though the two were the same.** See "2.27 GB resident was never a memory
figure" below; the conclusion (1024, not 2048) still stands on disk cost and on invention
falling off with scale factor, but not on the memory reasoning as written.
At 1024 a 32-bit base plus its chain is 5.3 MB, so the Sovereign set is 16 MB. At 2048
the same three files would be 64 MB, and the ~600 hull bases behind them would be
unreachable at any size. 4x is also where this model's invention is mildest — see
"Invention falls off steeply with the scale factor" above.

### The alpha is a self-illumination map, and it does go through the upscaler

The standing rule in `attach_alpha()` is that **alpha never goes through the generative
upscaler**: a coverage mask has no texture, only edges, Lanczos resolves an edge exactly,
and a model that invents detail into a mask invents holes in the object. That rule was
written against the moons, and it is still right for them.

A hull texture breaks its premise. This alpha is 81% exact zero with 234 distinct values
above it, and what it actually holds is a **night-lights map**: rows of lit windows down
the saucer rims, the deflector, the nacelle and impulse glow. That is picture content
with real high-frequency structure, co-registered with the windows painted into the RGB
— and Lanczos at 4x smears a row of 1px window dots into a bar.

So `alpha=ai` was added, off by default and set per target. It extracts the stock alpha,
sends it through the same model at the same `--mp`, blends it at the same `blend%` and
leaves the plate in `src-alpha/`, which `attach_alpha()` takes as its fifth argument.
The `tgapad.py` padding case still outranks it: a padding byte is not a picture.

Measured over all three files, AI alpha against the Lanczos alpha it replaces:

| | Fbattle | FEntE | Fbattle_b |
|---|---|---|---|
| mean, Lanczos → AI → blend 35% | 11.61 → 11.29 → 11.49 | 13.12 → 12.72 → 12.97 | 11.61 → 11.29 → 11.49 |
| **invented** — AI > 40 where Lanczos < 10 | 0.001% | 0.001% | 0.001% |
| **sharpened** — Lanczos > 40 where AI < 10 | 0.759% | 0.974% | 0.759% |

Invention is the number that decides it, and it is a rounding error: the model adds
essentially no light where stock has none. The 0.76–0.97% in the other direction is not
loss but the point of the exercise — Lanczos spreads each window's brightness into a
soft halo, and the AI puts it back in the window. Counted the naive way that reads as
"a percent of the glow disappeared", which is why both directions are measured.

### Judge the blend at the size the thing is drawn, not at 1:1

This shipped at `blend=35` first, and in game it was **indistinguishable from stock**.
That was the right number arrived at the wrong way.

The model does invent on the colour plate: it reads faint grey smudges on the hull as
lettering and draws plausible glyphs into them, and it rounds stock's square windows
into lozenges. At 35 those stay smudges; at 60 they are legible; at 100 they are
confident and wrong. (`NCC-1701-E` itself survives all the way to 100 — it is large
enough to be read correctly. It is the invented markings *around* it that do not.) So 35
looked like the careful choice.

It was chosen by pixel-peeping at 1:1, which is not a viewing distance this game has. A
Sovereign at ordinary RTS zoom draws about 340 px wide, and the saucer top-view occupies
roughly 140 of stock's 256 texels — so those invented glyphs land at well under a pixel
and are invisible, while the sharpening given up to suppress them is the only thing the
eye had to go on. Rendering the same crop down to 340 px makes the whole argument
visible at a glance: stock, 35, 60 and 85 in a row, and 35 sits on top of stock.

**Now 70.** Re-blending is free and offline — `ai/` is on disk, so `--reblend --blend 70`
costs nothing and changes no pixels that were paid for. The general rule this target
adds: *tune the invention dial on a render at the object's real on-screen size.* Every
earlier target in this project was a skybox, a planet or a UI sprite — things drawn at or
near 1:1, where pixel-peeping and the game agree. A hull texture is the first class where
they do not.

### One blend dial, not two

The colour plate wants a blend low enough to keep invention plausible; the alpha,
measured at 0.001% invention, could take a much higher one. They are still driven by
**one** `blend=`, deliberately: the lit windows in alpha and the painted windows in RGB
must stay consistent with each other, and a crisp light in a soft socket is its own
artifact. The measurement is recorded above so a later pass can split the dial with
evidence rather than by taste.

### The registry: the one place the model had to be corrected by hand

Stock's `NCC-1701-E` is **three texels tall**. At that size a C and an O are the same
pixels, so the string in stock is an illegible smear — the legible registry on the ship
today is *entirely* the upscaler's reconstruction. It got eight of ten glyphs right and
closed both Cs into O shapes, so the *Enterprise* flew as `NOO-1701E`.

This is the "never hallucinate into letterforms" rule from `REMASTERING.md` arriving on a
texture that is not a font atlas, and it needed a different answer than "leave it stock":
leaving it stock means an illegible smear, and no filter can do better, because the
aperture was never resolved in the source to begin with. Re-setting the type would mean
inventing a typeface.

So `tools/fix-enterprise-registry.py` does the minimum that is certainly correct — it
cuts the aperture back into the two glyphs the model closed and touches nothing else.
The string is mirrored in the atlas (the UVs flip it back), so each C opens to the
**left** there:

    glyph rows      908..920      the box, 13px tall, at 1024
    aperture rows   912..916      5px, centred on the glyph at y=914 (~38% of height)
    C1 left stroke  x 156..158    core x157, exterior x155, interior x159
    C2 left stroke  x 175..177    core x176, exterior x174, interior x178

Each aperture row is filled by interpolating across the stroke from the exterior sample
column to the interior one, per channel. Both sides are the same light hull grey, so the
cut disappears into the plate rather than needing a matched fill colour. The script
verifies the core column is actually dark and the flanks actually light before it writes
anything, which both catches a moved glyph and makes a second run a refusal rather than
a second cut.

**This is a one-off and is meant to stay one.** The *Enterprise* is iconic enough that
wrong lettering reads as a bug; no other hull texture in the game carries type anyone can
name. **Re-run it after any `--reblend`** — `src/` is derived and gitignored, so a
re-blend rewrites the plate and takes the fix with it. The order is `upscale-stock.sh
--reblend` → the script → `./a2tex build` → `install`.

### What this class cannot fix

Even at 70 the gain is real but modest, and that is the honest ceiling. The upscaler
cleans and sharpens; it does not manufacture structure that was never in 256x256. The
larger remaining lever is not the texture at all — the saucer is viewed at a steep
oblique angle, which is precisely where **anisotropic filtering** decides sharpness, and
`dxcfg.ini` currently leaves it at `application`, i.e. whatever a 2001 renderer asked
for. A `dxvk.conf` beside the exe with `d3d9.samplerAnisotropy = 16` would sharpen every
oblique surface in the game at once, for free, and is reversible by deleting the file.
Untried — it touches the graphics stack, which has an open regression in `SETUP.md`.

## The Federation hull class, after the Sovereign

The Sovereign proved the shape of a hull target. Extending it to the rest of the
Federation ships turned up four things it could not have shown, because they are
properties of the *set* rather than of one texture.

### The set, measured

76 base textures carry an `F`/`f` prefix once fonts, wireframes, `fedui*`, `Fguib04`/
`Fguif04`, the Ferengi strays and the two 8472 `FluidicRift` plates are excluded — a
count that still includes the three orphans stock leaves behind (`FpremNew_B_2`,
`Fsensor_B_1`, `Fsensor_B_2`), which present as bases because they are not valid levels
of anything. Scoped to **ships** — matched
by scanning every `.sod` for the texture name as a plain string, then mapping the SOD to
its `odf/ships/*.odf` `unitName` — the working set is 44 bases in six targets:

| target | files | source | built | alpha | note |
|---|---|---|---|---|---|
| `FedCapital` | 14 | 256 | 1024 | `ai` | Galaxy hull+saucer, Akira, Steamrunner, Intrepid, Nebula A+B, and Borg siblings |
| `FedCombat` | 13 | 256 | 1024 | `ai` | Defiant, Sabre, Aegian, Iwo Jima, Incursion, Venture |
| `FedSupport` | 10 | 256 | 1024 | `ai` | Cargo, Colony, Repair |
| `FedMask` | 3 | 256 | 1024 | Lanczos | `Ffreight`, `fdata` — alpha is coverage, not light |
| `FedFlat` | 2 | 256 | 1024 | none | `Fconst` — 24-bit, no alpha at all |
| `FedSmall` | 2 | 128 | 512 | Lanczos | `Fbee`, `fedpod10` — 128px sources, so their own `size=` |

All six are built, verified and **installed**; what none of them has had yet is a look
in game. `./a2tex revert all` undoes every target this project has ever installed, and
`./a2tex revert FedCapital` (or any one name) undoes just that one, chain included.

### The test that actually separates a light map from a mask

`alpha=ai` is justified for a night-lights map and for nothing else. The Sovereign
section above reasons from "81% exact zero with 234 distinct values", and that generalises
badly on its own: `fcargo` is 98.3% zero with 205 distinct values and *is* a light map —
a freighter simply has few windows — while `Ffreight` has 212 distinct values and is a
mask.

Two other tests were tried and are recorded here because they do not work:

- **Co-registration by luminance** — mean RGB luminance under lit texels against unlit
  ones. The Sovereign, the known positive, scores **1.02**. Hull markings are small and
  sit on light grey, so they barely move a mean. Useless as a discriminator.
- **Fraction of graded values** — `fcargo` (1.7% between the extremes) and `Fbattle`
  (27%) are the same kind of channel, four hundred windows apart.

What does separate them, cleanly and cheaply: **is the RGB painted where the alpha is
zero?** A coverage mask hides a region the artist never painted, so the colour under it
is black. A self-illumination map is a second layer over hull art that is painted
everywhere.

| | alpha = 0 | luminance there | reading |
|---|---|---|---|
| `Fbattle` (Sovereign) | 81.2% | 150.1 | hull art continues underneath — light map |
| `F_GalaxyHull` | 89.7% | 100–160 | light map |
| `fcargo` | 98.3% | 101.7 | light map, sparse |
| `fdata` | 97.5% | — | 2 distinct alpha values; no picture content either way |
| `Ffreight` | **0.0%** | *undefined* | mostly opaque — see below |

**`Ffreight` is the one this test cannot answer, and an earlier revision of this section
claimed it could.** That revision reported its luminance-under-transparent as 0.0 and
called the question settled. It has **no alpha-zero texels at all** — 0% zero, 97.2%
exactly 255 — so the 0.0 was a mean over an empty set, printed by a guard that returns
zero rather than dividing by it. A number computed from nothing is not evidence, and it
happened to agree with the conclusion already expected, which is how it survived a
reading.

It stays out of `alpha=ai` regardless, on the geometry rather than on that number: a
night-lights map is *mostly dark* with graded structure in the light, and `Ffreight` is
mostly opaque with 2.8% graded. Whether that channel is a soft-edged coverage mask or a
specular map is still unknown, and `alpha=ai` is justified for a night-lights map and
for nothing else yet.

### Stock ships duplicates, and they must stay duplicates

A byte-identity sweep over the Federation set:

    fcolony.tga == ftransco.tga          (and their whole chains)
    F_GalaxySaucer.tga == fspecialA.tga  (the Nebula reuses the Galaxy saucer)
    FpremNew_B_1.TGA == FpremNew_B_2.TGA
    Fsensor_B_1.TGA == Fsensor_B_2.TGA

The first two are processed as separate units and their outputs come back
byte-identical, as `Mnebula4` and `MFluidicNeb` do — the upscaler is deterministic, so a
*difference* between them would be the signal worth chasing.

The last two are the tell for the irregular siblings: a "level 1" and a "level 2" that
are the same file, at the same size as their base, are a placeholder rather than a
chain. See hard rule 5 in `CLAUDE.md`; `a2tex install` refuses those targets.

Worth noting what this sweep *disproved*. The census table showed `fcruise2_B` and
`Fcruise2` with identical alpha statistics — %zero, distinct count and mean all equal —
and several other `_B` pairs likewise. They are not identical files. Matching summary
statistics is not identity, and grouping them as duplicates on that basis would have
shipped one ship's hull on another.

### `blend=70` holds, but not for the reason it was nearly rejected for

The first pass at this concluded, with a measurement and two rendered contact strips,
that 70 was destroying the Galaxy's deflector dish, the Bussard collectors and the
impulse strips, and that the whole class needed a blend near zero. That conclusion was
**wrong**, and the way it was wrong is worth more than the result.

`--mp 4` on a 256px source returns **2048**, not 1024 — the megapixel figure is not the
scale factor. The comparison built its Lanczos layer at an assumed 1024 and composited
the 2048 `ai/` plate onto it. ImageMagick does not object; it composites at the origin.
So every "blend 70" image was the **top-left quarter** of the AI plate laid over the
whole Lanczos plate, and the markings that appeared to have been erased were simply
outside the crop. Judged at the plate's real size, the same textures at blend 70 measure:

| | erased | invented |
|---|---|---|
| `F_GalaxyHull` | 0.013% | 0.116% |
| `fcruise1` | 0.100% | 0.170% |
| `Fcruise2` | 0.020% | 0.119% |
| *`Fbattle` (accepted at 70)* | *0.016%* | *0.095%* |

Same regime as the Sovereign, so 70 carries over. `tools/measure-invention.sh` now reads
the plate size off the file, reports both directions at the on-screen size, and carries
the trap in its header comment.

The measure itself is new and is saturation-based, not luminance-based: **erased** is a
saturated marking in the Lanczos layer that the blend does not have, **invented** is one
in the blend that Lanczos does not. Both directions, because the Sovereign section
already records how misleading one direction alone is.

### 8x then downsampled beats 4x, which reads backwards

`FedCapital` was run at `--mp 4` (an 8x lift to 2048, brought to 1024 by `fit()`) and
the same fourteen textures at `--mp 1` (4x to 1024, the Sovereign's own setting).
Measured at 340px, blend 70, the 8x is level or slightly cleaner on every row —
`F_GalaxyHull` 0.013%/0.116% against 0.100%/0.161%.

That contradicts CLAUDE.md rule 4 only in appearance. Rule 4 compares lifts at their
**native output sizes**; here the larger lift is halved on the way to `size=`, and that
downsample is itself an invention filter. The rejected 4x layers are kept in
`archive/fedcapital-4x-lift/` with the full table.

### The metric has a benign false positive, and a real bug hid behind a name

Two things surfaced only because the per-target work was fanned out and each target's
numbers were read against the others'.

**`invented` over-reports by counting intensification.** `Ffreight_B` measured 1.322%
invented at blend 70, ten times `FedCapital`'s range, and `fdata` 1.011%. Neither is
inventing anything. `fdata`'s alpha hides 97.5% of its plate, and restricting the
measure to the 2.5% the engine draws gives **0.012%**. `Ffreight_B` is opaque
throughout, but its "invention" is the model saturating markings stock already has: a
desaturated panel sitting between the two thresholds crosses the upper one and scores,
though it is the right marking in the right place. Requiring an invented pixel to be
more than 2px from *any* saturated stock pixel — the `displaced` column — separates the
two: `fsrepairb` 0.578% → 0.042%, `fcruise1` 0.170% → 0.010%. Chroma confirms it
independently; R-G deviation against stock is 0.98–1.02 on every flagged texture.

**`verify.py` was silently skipping textures whose name is another texture's name plus a
digit.** `fdestroy2` is the Sabre Class; `fdestroy` is the Defiant. Folding mip levels
back onto their base by *name* read the Sabre as "level 2 of the Defiant" and dropped it
from the checked set entirely — no header check, no channel means, no chain check, and
**no failure**: just 12 textures reported where 13 were built. Across the repo it was
hiding 43, 42 of them in the UI targets (`commMenu11`, `gbbresear2`, and 28 more).

The fix is the same width test `mip_name` already used — level 2 of a 1024px base is
256px, and `fdestroy2` is 1024 — and the lesson is one this project keeps relearning in
new clothes: **a checker that can skip work must say how much it checked.** The count
line was always printed and always believed; nothing compared it against how many
textures the target actually holds.

### "2.27 GB resident" was never a memory figure

Every sizing decision in this project up to here rested on a sentence that says the
textures "already hold 2.27 GB" inside a 32-bit LAA process, and the Sovereign's choice
of 1024 over 2048 was made on it. **It is the size of `Textures/RGB` on disk.** Nothing
established that Armada II holds all 2115 of them in memory at once, and there is good
reason to think it does not: a Federation-versus-Klingon match has no cause to load
Cardassian hulls.

What is actually known, now that it has been looked at:

- `ART_CFG.h` in the game directory is read at runtime and carries
  `int ST3D_PRELOAD_TEXTURES = 1;`, alongside `GameOpenPreLoad() took %d seconds.` in
  the executable. So there **is** a preload pass — but it runs per game, against what
  that game needs, not across the whole directory.
- The renderer is the d3d8 → d3d9 → DXVK chain from `SETUP.md`, so textures live in
  **VRAM**, which on this machine is 8 GB (Navi 10). The 32-bit address space holds
  DXVK's bookkeeping and staging, not the texture bodies.
- The disk total did pass 2 GB some time ago and nothing has fallen over.

So the ceiling is **unmeasured**, and the disk total is a poor proxy for it — an upper
bound on a quantity that is probably several times smaller in practice. The figure that
matters is the per-match working set, which is roughly two factions' hulls plus the map
and the UI, not the sum over every faction.

The honest test is to launch a heavy match — two remastered factions, a big map — and
watch RSS and VRAM. Until someone does that, sizing by "how much is on disk" is
cargo-culting a number that was never measured. If it ever does bite, the lever is
`ST3D_PRELOAD_TEXTURES = 0`: lazy loading, paying in stutter instead of memory.

The figures below are kept because disk cost is still real — it is just not the same
question as memory.

### The measurement ranks textures; it does not decide them

`tools/measure-invention.sh` has four known false positives, all found the same way — by
rendering a texture whose numbers had condemned it. It assumes markings are small and
localised on a neutral field, and over-reports wherever that does not hold:

| | case | what it reads | what is happening |
|---|---|---|---|
| 1 | intensification | high `invented` | the model saturating a marking stock already has — this is what `displaced` subtracts |
| 2 | large flat colour | high `displaced` | `Fsensor`, gold chevrons on white, reads 5.2% displaced and is simply crisper; a region that grows by more than the 2px dilation still scores |
| 3 | additive glow on black | high `erased` | `Ftransport`, rows of light blobs, reads 8.8–11.6% erased. Lanczos spreads each blob into a halo and the model puts the light back in the blob. The halo's loss is the point of the exercise |
| 4 | no saturated markings | **flat, near zero** | `8472_passive2` reads 0.037–0.040% invented from blend 20 to 100 while visibly gaining speckle. The metric cannot see it |

Case 3 has a proper check behind it: for an additive texture the channel mean *is* the
light contributed, and `verify.py` compares raw per-channel means against stock within
1.0. Case 4 is the dangerous one, because **a clean number there is not evidence**. On
organic or near-greyscale art the blend has to be set on a render.

The honest summary is that this tool is a ranker. It says which textures to look at, in
what order. It has never once decided a blend correctly on its own, and three of the four
station textures it flagged loudest turned out to be improvements.

### What the class cost the tooling

Four changes, all of which apply to every faction that follows:

- `mip_name()` / `mip_strays()` / `stock_bases()` in `lib/common.sh`, and the install
  and revert paths in `a2tex` rebuilt on top of them. The old glob, `${a}_[0-9]*`,
  could not see two of the chains in this set at all — the guard against the
  Klingon-campaign crash was blind to them, and `revert` would have restored a base
  while leaving its levels upscaled.
- Mip levels are matched to **their own** stock file for header as well as name.
  `fresearch` is a 32-bit base (`desc 0x08`) over two 24-bit levels (`desc 0x00`), and
  taking depth from the base wrote a chain stock never shipped. `verify.py` checks
  per-level headers now; it previously skipped levels entirely.
- `mips=auto`, because one number per target cannot describe a set where `fdestroy` has
  two levels and `fdestroy_b` has none.
- `stock/` now carries each texture's levels so a target is self-describing about its
  chain spelling — and `stock_bases()` keeps them out of both the src-image count and
  the **paid** upscale. Without it `FedCapital` alone would have bought 28 upscales of
  art that `gen_mips` then overwrites by downsampling the base.

- `alphafilter=` in `target.conf`, default Lanczos so nothing already built moves.
  Lanczos rings, and on a mask that is mostly zero with small isolated opaque blobs the
  ringing clips asymmetrically — the undershoot below 0 has nowhere to go — so the
  channel mean rises. `fedpod10` (91.1% zero, 8.5% opaque, 13 distinct values) drifts
  22.24 → 23.36 at 4x and fails verify's tolerance of 1.0; Mitchell gives 22.43,
  Triangle 22.24 exactly. It is one texture's geometry and not a property of masks —
  `Fbee`, `fdata`, `Ffreight` and `mdmoon` are all masks and all drift under 0.16 — so
  the knob is per target rather than automatic, which also leaves shipped targets alone.

`tools/selftest-mips.py` pins all of this against the real texture directory.

## The other five factions, and a texture that is two things at once

Klingon, Romulan, Cardassian, Borg and Species 8472 — **198 textures in 12 targets**,
built, verified and installed on the recipe the Federation pinned: 256px sources to 1024,
`mips=auto`, `blend=70`, `alpha=ai` where the channel is a night-lights map.

Grouping was done by `tools/classify-alpha.py` rather than by hand, because at this scale
hand-classification is where mistakes come from. Each faction splits into a `Lit` target
(alpha is picture content) and a `Plain` one (coverage mask, binary mask, or no alpha at
all), plus three cross-faction targets for the odd source sizes — `size=` is per target,
so a 128px plate cannot share one with a 256px plate.

### borgUI3: the ceiling that `maxsize=` cannot enforce

`@tmaterial=interface` sprites above 256x256 crash the game, and `maxsize=256` in
`target.conf` has enforced that since the UI pass. It is not sufficient, and this faction
pass found out how.

**`borgUI3` is both things.** It is an interface sprite in `Sprites/gui_borg.spr` *and*
it is referenced by `Borpod16.SOD`. The faction census collects "textures a SOD points
at", so it was collected as a Borg hull texture, placed in `BorgPlain` — a target with no
`maxsize`, because hull targets have no reason to declare one — and **installed at
1024x1024**. It sat in the game directory that way until the verify that follows an
install reported `borgUI3: installed file differs from out/`, which is `UIpanel` noticing
that something had overwritten its 256px copy.

The lesson is not "add maxsize to hull targets". It is that **a texture's target
membership does not tell you what the engine thinks it is**, and `maxsize` is a target
opting in to a rule it has to know applies. The rule has to be applied by what the
texture *is*:

- `tools/interface-textures.py` reads every `.spr` in `Sprites/`, tracking
  `@tmaterial` as state, and prints the 267 textures declared under `interface`.
- `a2tex install` now refuses any output over 256px whose name is in that set,
  independently of the target's own `maxsize`.

An audit of every installed texture against that set found `borgUI3` and nothing else.
`UIpanel`'s 256px build was restored and `borgUI3` removed from `BorgPlain` entirely.

### What the fan-out was worth

Five agents, one per faction. Two things are worth recording honestly.

They found a real bug the author missed: `verify.py` was silently folding `fdestroy2`
(the Sabre) into `fdestroy` (the Defiant) as a mip level and dropping it from the checked
set — no failure, just a count one short — and across the repo it was hiding 43 textures.
An agent traced the cause correctly and refused to touch shared code, as instructed.

And **two of the three collisions in this project's history were caused by the author, not
the agents**, both by starting work on a target whose agent was still running. Neither
corrupted anything, because `build_sky_faces` refuses a target whose src image count does
not match its stock base count — but the symptom, `SKIP -- needs 24 images, found 25`,
reads like a missing texture rather than a collision. The cause was `upscale-stock.sh`
writing `src/.lz-<name>.png`, a fixed path inside a directory whose file *count* is
load-bearing. `lib/common.sh` has carried the rule since `-j` was introduced — "every
function takes its scratch directory as an argument and writes nothing to a fixed path" —
and that script had never followed it. It uses a `mktemp -d` now.

All four remaining agents ultimately died on API timeouts, after their upscales and
builds had landed. `CardassianLit` was rebuilt from scratch and diffed against the build
that had raced: byte-identical.

## The mission loading screen, widened

The loading screen is **not a sprite and not a menu** — it is a 3D scene, which is why
it can be widened at all. `RenderLoopAssetLoadStatusUpdate` (`0x598ba0`) draws
`mbg02.sod` (the skybox, centred on the camera) and then `logo.sod` 700 units in front of
it, through the ordinary perspective camera. `logo.sod` is six flat quads, 288 units
square, in a 3x2 grid — 864x576, exactly the 3:2 of `LOADING1..6` (256px, 32-bit,
alpha a flat 255). The camera fits that to the screen **height**: at 3440x1440 it spans
x 646..2792, measured off `before.mp4`, and the MBG02 skybox shows either side.
Because it is a model texture, hard rule 4's 256 ceiling does not apply; the tiles go
to 1024.

Widening is a geometry change **and** new art, never one alone. `tools/logo-sod.py`
scales the quads' x coordinates to `panel=` units in `targets/LOADING/target.conf`
(1400; 3440x1440 needs 1385) — same byte count, same UVs, every vertex checked against
the stock grid first. `tools/loading-panel.sh` makes the art: the six tiles assembled,
AI-upscaled 4x and blended 35 over Lanczos (the skybox recipe — the panel fills the
screen height, so 1:1 is what is seen), then **outpainted** with `bria/expand` to the
panel's aspect, then the blended plate feathered back over the centre so the model's
contribution is the sides only, then cut into six squares, each a third of the width
squashed square. `a2tex install LOADING` installs the SOD with the textures, and both
reverts put it back, because stock art on a wide panel is stretched and wide art on the
stock panel is squashed.

What the outpaint needed, measured:

- **A feathered paste, not a hard one.** bria does not return the region it was given
  untouched (RMSE 0.015 against its own input), and a hard paste drew a line through
  Earth at the left seam: column mean 133 → 127 in one pixel. A 96px fade inside the
  original region is step-free.
- **A prompt that names the planet.** "Nebula and stars" continued Earth's limb
  correctly and then read its night side as sky and filled it with stars — a
  see-through planet. Describing the night side as a solid surface fixed it on the
  first try.

The upscale is **`bria/increase-resolution`, not the project's usual pruna model** —
pruna returned 503s for the whole session this was built in. `upscaler=` in
`target.conf` selects it; `upscaler=pruna` plus `tools/loading-panel.sh --force` goes
back. The 35% blend toward Lanczos bounds what either model invents, so this is not
expected to matter, but it is the one target built on a different upscaler.

`fit=none` is new for this target: each tile is a third of a wider picture, not its
namesake, so per-tile channel matching would put a brightness step at every join, and
`verify` compares only alpha for it.

## Beyond the nebulae

`REMASTERING.md` carries the measured inventory of all 2115 textures and what would be
needed to apply this pipeline to them. The short version, because it changes how these
scripts should be read:

- **1113 of 2115 textures are 32-bit with a live alpha channel.** No longer a blocker:
  `write_tga()` takes its depth from the stock file and `attach_alpha()` carries the
  mask across. Proved on four textures, not four hundred — see the planets-and-moons
  section above for what that cost. On a **hull** texture that channel is not a mask at
  all but a night-lights map, and `alpha=ai` upscales it like colour; the Sovereign
  section above has the measurement.
- **363 hand-authored mip chains** (782 files) must be regenerated, not ignored.
  `gen_mips()` does this for every kind now, colour and alpha separately.
- **Fonts, UI, cursors, wireframes and minimap art must never be hallucinated into.**
- Upscaling every base texture 4x would take the set from 196 MB to about 10 GB;
  the 708 textures done so far already account for 2.27 GB. On the hull class this is the
  binding constraint, not cost — see "1024, not 2048" above.

## Still open

**708 textures are shipped and verified.** Every one is derived from its own stock art —
**none of the generated art shipped in the end.** The nebula systems, the planets and
the UI are all confirmed rendering in game; what remains unseen is narrow.

Needing a look in game:

- **The 16 class planets** (`PB_CLSS*`, `PA_*`) and **`mdmoon`.** `mdmoon` is the one to
  check first: it is on nearly every map, and it is a rebuilt mip chain on a 32-bit
  texture — the class of mistake that crashed the Klingon campaign.
- **Whether the comm and objectives pop-ups land centred.** They moved with the
  widescreen canvas change in `SETUP.md` and have not been opened since.
- **The Sovereign's registry.** `blend=70` is confirmed in game and clearly better; the
  two C apertures were then re-cut by hand and that build has not been looked at yet.
- **All 44 Federation ship textures.** Built, verified and installed, never yet seen in
  game. The first thing to look at is a Galaxy or an Akira at ordinary RTS zoom, and the
  second is anything Borg-assimilated, since the `_B` plates are the ones with no SOD
  pointing at them and so the ones a naming mistake would hide. Resident texture load is
  now 2.47 GB **on disk**, up from 2.27 GB. That is not a memory figure and should not
  be read as one — see the subsection on it above. Whether memory is a constraint at all
  is still unmeasured; the test is a heavy match with RSS and VRAM watched.

- **The widened loading screen** (`LOADING` + `SOD/logo.SOD`). Installed and verified,
  approved from the preview, not yet seen in game. Launch any mission: the panel should
  reach both screen edges with no skybox showing, and nothing should be squashed.

Accepted as-is, with reasons, so they are not re-litigated:

- **`Mbgstars` stays stock.** Sparse starfield — an upscaler turns a 1px star into a
  blob. Plain Lanczos is the only safe option and buys nothing.
- **`mbgpur2` clips** at 0.037% of pixels where stock does not. Plain Lanczos already
  reaches 254-255 on those star cores, so the blend is not the lever.
- **`mbgrg` is the one skybox at `blend=20`**, because its two darkest faces measured HF
  2.98 and 2.85 against a 3.0 threshold. Taste, not correctness.
- **`Mnebula4` and `MFluidicNeb` are identical**, as in stock — the same file, and the
  upscaler is deterministic, so two independent runs produced byte-identical output.
- **`MbgBaku` carries 7 files, not 6.** `MbgBaku1` is an orphan, not a duplicate; it is
  processed correctly as a seventh unit.

**What is left is inventoried, not guessed.** `INVENTORY.md`, generated by
`tools/inventory.py --md`, classifies all 2115 texture files as done, could-be-done or
should-not-be, with the reason for each category and the list of files in it. The
summary: 1517 installed, 189 mip levels that follow a skipped base, 97 that could be
done (weapons and effects are the largest group, and the largest remaining win), and 312
that should not be — 192 of those referenced by nothing in the game at all.

Genuinely outstanding:

- **Menu animations stall while the cursor is moving.** Reported in game after
  `MenuScale.asi` went in, not yet diagnosed, and **not yet shown to be a regression** —
  the first test is to remove the plugin and watch stock, because a 2001 GDI shell
  flooding its own message queue on mouse-move is plausible stock behaviour. The
  hypothesis and the three experiments that discriminate between the causes are in
  `tools/menuscale/README.md`. Everything else about the menus is confirmed: they scale,
  they centre, and input follows the picture.
- **The options, load/save and multiplayer menu screens have not been seen scaled.**
  They go through the same two code paths as the two screens that are confirmed, so they
  are expected to follow. The multiplayer screens are the ones to doubt: they use real
  Win32 child controls, which Windows draws itself and the offscreen redirect does not
  cover.
- **The d3d8/DXVK regression in `SETUP.md`**, unfixed and still blocked on Heroic being
  closed. The cutscene-crash fix has consequently never been tested.
- **The end-of-mission crash** — a `Wine C++ Runtime Library` R6025 box on finishing a
  mission, then madExcept. Captured in `archive/error-mission-finish/`. **It first
  occurred before any modding**, so it is not this project's, and nothing here has been
  shown to affect it either way. Recorded so it is not mistaken for a texture problem.
- **DXVK now renders the game, and `dxvk.conf` applies.** Confirmed in game at
  3440x1440 with 16x anisotropic filtering and a -0.5 mip LOD bias live, no DXVK
  errors. Four things are required together and all four are in place:
  DXVK's `d3d8.dll` **and** `d3d9.dll` in the **game directory** (the prefix is not
  durable — Proton restores it from symlinks), `d3d9=n,b` in `WINEDLLOVERRIDES`, and
  the **Wine virtual desktop off** — inside it DXVK's display-mode change fails and the
  game collapses to 640x480. `tools/d3d8-chain.py` and `tools/virtual-desktop.py` set
  and reverse all of it; full write-up in `SETUP.md`.
  **This corrects a claim that stood in these notes for a long time:**
  `syswow64/d3d8.dll` at 320548 bytes was recorded as DXVK's; it is byte-identical to
  *Wine's builtin*, so until now the game — and every texture in it — rendered through
  wined3d/OpenGL, never Vulkan.
- **All three renderer stages are installed and confirmed in game** — 16x anisotropic
  filtering, a -0.5 mip LOD bias and `seamlessCubes`, verified applied in DXVK's own log
  with no errors. **They have not been separated**, though: they went live across three
  launches but were never A/B'd, so how much each contributes is unmeasured.
  `tools/renderer-config.sh --stage 1` and `tools/ab-shot.sh` are what settle it, and
  the expectation on record is that the LOD bias does more here than AF, because the
  dominant visual layer is camera-facing billboards that AF cannot touch.
- **Bloom is installed and confirmed in game** — vkBasalt + MagicBloom, threshold 6,
  intensity 0.08, Home toggles it live. Built per-user by `tools/vkbasalt/build.sh`,
  configured by `tools/postfx.py`; details and the measurements behind the numbers in
  `SETUP.md`, Tier 3. What is left in the post-processing family is SMAA/CAS, untried.
- **Anti-aliasing: `MSAA.asi`, 8x, installed and confirmed in game (2026-09-23).**
  `tools/msaa/`. It hooks the engine's own `CreateDevice` path and sets
  `MultiSampleType`, `SwapEffect=DISCARD` and a non-lockable back buffer. DXVK has no
  key for this, and the `dxcfg.ini` route died with the chain change. `MSAA.log`
  says what the device actually got. The first thing to check in game is the
  **minimap**: it is the one back-buffer read-back, and it relies on DXVK's CopyRects
  resolving a multisampled source. Details in `tools/msaa/README.md`.
- **`Mnebula4`'s generated art is preserved but unused** — `targets/Mnebula4/src-generated/`
  and `archive/mnebula4-generated/`. Move it back into `src/` and set `source=gen` to
  return to it.
- **`PROMPTS.md` is now unexercised.** Nothing in the game currently uses generated art,
  so those prompts are untested against the current pipeline.
- **`UImid`'s 17 non-map textures are built but unshipped.** The target is `install=no`
  because of the 9 fog/minimap textures in it; the other 17 (`gbfpod100`, `shipinfo`,
  `ferwireframe`, …) are ordinary interface sprites of the same class as the 382 that
  work. Splitting them into their own target would recover a 2x on 17 icons, for one
  more launch to verify. Marginal, and nobody has asked.
- **The rest of the hull class, beyond the Federation.** The Federation ships are done
  and are covered by their own section above; what remains is the other factions. The
  Federation pass measured the memory question rather than estimating it: its 44 bases
  and their chains come to **220 MB** at 1024 — `FedCapital`'s 73.5 MB measured on
  disk, the rest computed from the same 5.25 MB per 32-bit 1024 base-plus-chain —
  on top of the 2.27 GB already on disk: the directory measures **2.47 GB** with all six
  targets installed, the +0.20 GB predicted. Disk, not memory.
  Doing it by faction is the right unit. Still untested for the other factions is the
  **scale** —
  ~600 hull bases, and contact sheets only help where a whole group shares one size,
  which at 256px they largely do. Memory is the real ceiling: at 1024 the whole class is
  roughly 3 GB of disk on top of the 2.27 GB already spent -- a real cost, though not
  the memory ceiling it was once read as -- so it
  wants doing by faction or by ship class and checking as it goes, not in one pass.
  There are also `*bump` textures throughout the set, which are normal or bump maps and
  must not be treated as colour, and the alpha on a **station** or **weapon** texture has
  not been looked at — `alpha=ai` is justified for a night-lights map and for nothing
  else yet.

Closed since the last revision of this list, recorded so they are not re-opened:

- ~~`MBG02` peak brightness is short, built 86 against stock 152~~ — that was the
  *generated* source. Candidate D peaks at 160 against stock's 152.
- ~~Corner notches are not reproduced by `sky-faces`~~ — only ever a problem for
  generated faces; upscaling stock preserves them by construction.
- ~~1024x1024 textures are untested~~ — 4096x4096 is confirmed working in game.
- ~~`build.sh` requires exactly 4 images per atlas~~ — the `puff` and `sky-atlas` kinds
  repeat the supplied images, so one is enough.
- ~~A puff cannot usefully be upscaled from its own stock, because the point of replacing
  it is filament structure a 128px quadrant does not contain~~ — argued here, then
  disproved. All 8 were upscaled from stock and the result beat the generated art on
  every measure taken, including on `Mnebula4`, where both existed.
- ~~`Mnebula2` is the only installed texture with a hand-authored chain~~ — `mdmoon` is
  the second, and the first 32-bit one.
- ~~Verifying a build once is enough~~ — `Mmoon` was verified, passed, and then the
  build changed (`blackedge` was added) and only the new property was re-checked. It
  shipped with every quadrant's RGB several times stock's and rendered as a white box
  around a sun sprite. `./a2tex verify` exists so this is one command, not a habit.
- ~~ImageMagick is a sufficient tool for checking the output~~ — not where alpha is
  involved. Its opinions about associated alpha are what caused the `Mmoon` bug, so it
  cannot be the thing that checks for it. `tools/verify.py` reads the raw bytes.
- ~~R, G and B means are identical for map puffs~~ — true for most of the set, but
  `Mnebula1` is 28/26/22 and `Mnebula2` is 33/9/8. Match the stock file, not the rule.
- ~~Generative upscaling is non-deterministic, so two runs give free variety~~ —
  `Mnebula4` and `MFluidicNeb` are byte-identical stock files, upscaled in two separate
  runs, and produced byte-identical `ai/` layers and byte-identical output.
