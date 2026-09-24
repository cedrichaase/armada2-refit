# HUD layout — the in-game UI at widescreen

The in-game HUD: its layout canvas (`hud/ui-widescreen.py`, which rewrites
`misc/gui_<race>.cfg`) and the cursors (`hud/cursor-aspect.py`, which reshapes
the cursor art in `Textures/RGB` — `a2mod` counts that under textures, because
that is where the files land). The menus are a different system: `menus/`. The text
is a third: the font ignores the canvas, and its fix is in `font/`.

## The UI stretch, and where it actually comes from

`STA2WidescreenPatch` fixes the 3D view. It does **not** fix the UI, which at 3440x1440
comes out 1.79x too wide — correct only at 4:3. That stretch is not in the patch, not in
`Armada2.exe`, and not in the sprite files. It is in `misc/gui_<race>.cfg`:

    // we no longer assume that these files have a resolution of 640x480
    // it can now be specified here
    screenWidth = 1600
    screenHeight = 1200

Every coordinate in `misc/gui_interface.cfg` and `misc/gui_glob16x12.cfg` is an absolute
pixel position in that canvas — measured across all 356 four-number entries: no `x+w`
exceeds 1600, no `y+h` exceeds 1215. The engine scales the canvas to the back buffer
**independently on each axis**. At 3440x1440 that is 2.15x across against 1.20x down,
and 2.15/1.20 = 1.79. At 4:3 the two factors are equal, which is exactly why 4:3 is the
only aspect that looks right.

`hud/ui-widescreen.py` re-declares the canvas as `1200 x display aspect` — 2867x1200
here — so both scale factors come out at 1.20 and the UI keeps the size it has today
instead of shrinking. Re-declaring alone is not enough: the extra 1267px all appear on
the right, so it also moves the right-anchored and centred panels. 20 values in 8 files,
nothing else touched, CRLF and tab alignment preserved byte-for-byte.

    hud/ui-widescreen.py --dry-run     # show the 20 changes
    hud/ui-widescreen.py               # apply, reading the resolution from ARMADA.PRF
    hud/ui-widescreen.py --revert      # restore from the *.a2neb-backup beside each

The format keeps screen placement in a handful of `<name>PanelArea` keys and makes
everything else relative to its panel, which is why the change is so small — and why
the sub-element coordinates must **not** be touched.

### The popup palette is not on the canvas

The action bar — the row of command buttons for the selected unit — is the one element
that does **not** obey `screenWidth`, and it was missed on the first pass. It sat far
left of the ship display instead of level with it.

`popupPaletteXA` / `popupPaletteXB` in `gui_glob16x12.cfg` are bare scalars, and the
palette code reads them as a fraction of a **hard-coded 1600**, not of the declared
canvas. Measured off a 3440x1440 screenshot with `screenWidth` already at 2867, scale
calibrated against the resource bar, the button panel and the ship display (all three
within 1px of what 2867 predicts):

| | measured | canvas model | 1600 model |
|---|---|---|---|
| palette left edge, screen x | 762.7 | 426 | **763.25** |

Rects in the same file are *not* affected — the palette's own `paletteSingleButtonArea`
(80x80) draws square at ~96px, a clean 1440/1200, and the pause and objectives dialogs
land centred. **Sizes and four-number rects take the canvas; these two scalars take
1600.** Two code paths in one file.

So the tool computes the anchor in canvas space like every other key and then divides it
back into 1600 space. At 3440x1440: `XA 355 -> 554` and `XB 1355 -> 1463`.

**The model is confirmed to sub-pixel accuracy**, against a second screenshot taken with
`XA = 551` (the first attempt, which carried stock's offset through):

| | predicted, image x | measured |
|---|---|---|
| palette left edge, XA=551 | 688.06 | 688.0 |
| info panel left edge, canvas 993 | 692.02 | 692.0 |

The 6.8 screen px still visible at 551 was therefore **not** model error — it is stock's
own offset, 355 against the panel's 360. Five pixels at 4:3, where nobody notices; 6.8
against a 1056px-wide panel, where it reads as a misalignment. So `popupPaletteXA` takes
an `infopanel` anchor — it targets `infoPanelArea`'s x rather than its own stock x — and
the overhang is gone. This is the one place the tool deliberately does not reproduce
stock.

XB was previously shifted to 2622 as though it were a canvas x, which the 1600 reference
renders at screen x **5637** — the locked build palette was off-screen entirely. Fixed
by the same change.

`popupPaletteYA`/`YB` are left alone: the vertical axis is never distorted, since
`screenHeight` stays 1200. Note for anyone tempted to tidy it — the measured button row
sits ~33 canvas px *above* `YA * 1440/1200`, which is unexplained and is stock
behaviour. Don't "fix" it without a measurement.

**Not handled: the bridge display** (`bridgePanelRect`, `*_bridgeBackgroundRect` and
their layer rects, `gui_glob16x12.cfg` lines 393-536). It is a full-screen 1600x1200
backdrop assembled from six tiles with overlay sprites placed against it, so it cannot
be widened without either stretching the art — the thing being removed — or re-tiling
it. Left stock, which leaves it pillarboxed left rather than stretched.

**Confirmed in game.** The model behind it — per-axis scaling from a declared canvas —
was inferred from the file format and the symptom before it was tested, and it was
right: icons square, minimap square, panels flush against the real screen edges instead
of stranded at the 1600px mark. What has *not* been reopened since the change is the
comm and objectives pop-ups, which moved with it.

**The palette correction is confirmed in game**, in two rounds: the first put the action
bar level with the ship display and proved the 1600-reference model to sub-pixel
accuracy, and the second removed stock's five-pixel overhang. The `infopanel` anchor
that removes it has been measured but not yet seen rendered.

### The cursors are not on the canvas either

The mouse cursors stayed stretched after all of the above — the same sideways smear the
panels and glyphs had. They are the **third** screen reference in this game, after the
declared canvas and the palette's hard-coded 1600.

Measured off two 3440x1440 screenshots, against the source texels:

| | source (texels) | on screen (px) | x | y |
|---|---|---|---|---|
| `Curs_Move`, arc centroid spacing | 24.5 x 24.4 | 104.6 x 56.0 | 4.27 | 2.30 |
| `Curs_24` `c_select`, delta bbox | 16 x 21 | 66 x 49 | 4.13 | 2.33 |

Both sources are square — `Curs_Move`'s bright content is 28x28 texels at offset (2,2)
in every one of its five frames — and both draw at about **1.8:1**. The scale pair,
~4.3 across against ~2.4 down, is an **800x600** reference scaled per-axis:
`3440/800 = 4.30`, `1440/600 = 2.40`. (A 1600x1200 reference with the sprite's 32 units
counted double is the same arithmetic and is not distinguishable from outside.) Both
measurements sit 3-4% under that in *both* axes, which is the faint outer antialiased
ring falling below the threshold, not model error — it cancels in the ratio, and the
ratio is the thing: `4.30/2.40 = 1.79`, the same number as the UI, because 1.79 is just
(display aspect) / (4:3).

#### There are two cursor draw paths, and only one of them reads cursor.spr

The first attempt rewrote `Sprites/cursor.spr` so that `W`, `U` and `@referenceWidth`
scaled together — UV rect bit-identical, drawn rect narrowed, `W 32 -> 18`. **In game it
fixed exactly one of the two paths**: the part of the selected-ship cursor that sits on
the map plane came out square, and every cursor on the UI layer was untouched.

That is the measurement that settles the mechanism, and it was worth making:

- **The map-plane part is drawn from the `.spr` `W H` pair.** It responded to W.
- **The UI-layer cursor is a hardware cursor.** `Armada2.exe` imports Win32
  `SetCursor` / `LoadCursorA` / `SetSystemCursor` *and* references D3D8
  `SetCursorProperties`; the engine composes that cursor's surface itself from the
  sprite's **texel extent**, which no number in a `.spr` can reach.
  (`cursors/*.cur` are the shell's own 32x32 Win32 cursors — `cursor1.cur` is a *red*
  arrow, not the in-game pale delta — so they are not this.)

So the drawn size is `texels x (screenW/800, screenH/600)`. The horizontal texel density
is pinned at **4.30 px/texel** by the reference no matter what we do, and the only lever
left is how many texels the art spans.

#### So the correction goes in the art, and cursor.spr goes back to stock

`hud/cursor-aspect.py` squashes the **art** horizontally by `1/1.79` inside its
unchanged 32x32 cell. That fixes *both* paths at once — the map-plane path draws those
same texels into the same 1.79-stretched rect — which is why the `.spr` rewrite was
reverted rather than kept. Keeping both would square the art twice and leave the
map-plane cursor too narrow.

    hud/cursor-aspect.py --dry-run     # 20 textures, frame grids and hotspots
    hud/cursor-aspect.py               # rebuild, reading the resolution from ARMADA.PRF
    hud/cursor-aspect.py --revert      # restore from the *.a2neb-backup beside each

At 3440x1440 the frame goes `32 -> 18` texels across (0.5625 against the ideal 0.5581,
0.8% off square). Each cell is squashed **about its own `@origin` hotspot**, not its
centre, so the click point does not move: `c_arrow` / `c_select` / `standard_cursor` are
`@origin=(0,0)` and squash toward the left edge, keeping the delta's tip on the pixel it
points at; the other 29 entries are `@origin=(16,16)` and squash toward the middle.

**What it costs, honestly: horizontal texels.** A shape described by 28 texels is
described by 16 afterwards. The on-screen *block* size does not change — 4.30 px/texel
before and after — so the cursor does not get blockier, it gets **coarser**, fewer steps
describing the same outline, and it gets smaller, which is the point: the delta goes
from 77x53 screen px to 43x53, the move reticle from 120x67 to 68x67.

The alternative is to resample the other way — stretch the art *vertically* by 1.79 and
scale `H` / `@referenceHeight` with it. That preserves every original texel and is
equally square, and it was rejected on size alone: it leaves a **138x138** cursor, which
is enormous at 1440p. The density is identical either way; only the size differs.

#### Two things this got wrong first, both caught by measuring

- **The filter has to be interpolating, not approximating.** ImageMagick resizes each
  axis in turn, and a cell's *height* is unchanged here — so the vertical pass runs at
  scale 1.0 and must be an identity. Mitchell is not: it blurred, and the art's bbox
  grew one row in each direction, which on a colour-keyed sprite is a dark fringe on
  the key. Catrom and Lanczos pass through their sample points and are exact. With
  Catrom the vertical extent and y-offset are **identical to stock in all 20 textures**.
- **`stock` must be the backup, not the file being overwritten.** The first run copied
  `src` to the backup, rebuilt, overwrote `src`, and then compared `stock` against
  `src` — by then the same file. Every before/after number came out identical, which is
  what that bug looks like: not a wrong number, the *same* number twice.

Black is the colour key (`@skip=(0,0,0)` in `cursor.spr`), so the guard is that the key
must **grow** — the cell is narrower now — and the art must not gain rows. Both are
checked per texture and reported; the key grew in all 20 (+103 to +1878 px).

Stock cursors are uniform and the output matches byte-for-byte in the header: TGA image
type 2, 24-bit, no ID field, no colour map, descriptor `0x00` (bottom-up), via
`bottomup.py --like`. Geometry is exact by construction rather than by eye — for
`Curs_Move` the art lands at `144x28+8+2`, exactly the predicted `7 + 2*0.5625 = 8` to
`7 + 29*0.5625 = 23` per cell. (A thresholded centroid reads the on-screen ratio as 1.04
rather than 1.006, because Catrom softens the horizontal edges and a thresholded blob
widens; the vertical pass is an identity, so the bias is one-sided. Don't re-tune off
that number.)

`CursorA.tga` is left stock: no sprite file references it and the exe has no string for
it — an unused leftover. `--revert` filters by the 20 stems `cursor.spr` actually names,
so it will not touch the `fi**ncurs**ion*` hull backups that share the glob.

**Note that `./a2tex revert all` also restores these**, since it sweeps every
`*.a2neb-backup` in `Textures/RGB`. That is the right behaviour — revert all means
stock — but it means the cursor fix has to be re-applied afterwards.

**Confirmed in game** — no longer stretched.

#### An AI upscale cannot help these, and the binary says exactly why

Asked for, and the answer is no — not as an art change. `armada2.map` makes the cursor
path readable rather than guessable, and it is short:

- **`ST3D_DeviceDirectX8::DrawCursor` (`0x623bd0`) is `xor eax,eax; ret`.** It draws
  nothing. The cursor is a real **D3D8 hardware cursor**, set via
  `SetCursorProperties`, not a quad the renderer composites.
- **`ST3D_DeviceDirectX8::SetCursor` (`0x625c90`) allocates the cursor texture
  pre-scaled.** With the synchronous flag at `[device+0xe0]` clear — the live path — it
  computes

      width  = round( texW * [device+0x18] )
      height = round( texH * [device+0x1c] )

  and creates the texture at that size. **Those two floats are the 4.30 and the 2.40.**
  (The other branch, flag set, creates it at the texture's own `[tex+0x1c]`/`[tex+0x20]`
  — unscaled — and `UpdateCursor` early-returns without blitting, so it is not this.)
- **`ST3D_DeviceDirectX8::UpdateCursor` (`0x625b00`) then does `CopyRects`** — a 1:1
  pixel copy, no filtering — of the sub-rect `[a, b, a+c, b+d]` where
  `c = texW_scaled * sprite.w` and `d = texH_scaled * sprite.h`, into the cursor surface.

So the on-screen cursor is `source texels x [device+0x18]` wide. **The engine does the
magnification itself, into a real texture, and the scale factor is a constant.** Double
the source texels and the cursor comes out twice as big at *identical* density — 4.30
screen px per source texel either way. There is no art change that buys sharpness,
which is why the squash is framed as choosing a size rather than trading quality.

**The lever for sharpness is `[device+0x18]` / `[device+0x1c]`, not the textures.** Set
them to 1.0 and upscale the art 4x and the cursor would be a genuinely crisp 128px
drawn from real texels instead of a 4.3x blow-up; set them *equal* to each other and the
aspect is fixed properly, at the source, with stock art and no squash at all. Both are
code, not data — an ASI hook of the same shape as `menus/`, which is why this
is now worth considering rather than impossible. Not attempted.

Until then, `cursor-aspect.py`'s squash is the whole of what art can do.
