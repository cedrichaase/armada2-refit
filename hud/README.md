# HUD — the in-game UI, its font and its cursors at any aspect

The in-game HUD has three things that are drawn against a fixed 4:3 (or 5:4) reference
and come out stretched on a wide screen: the **layout canvas** (1600x1200), the **font**
(1280x1024) and the **cursors** (800x600). `HUD.asi` corrects all three inside the engine,
at run time, from the display mode that is actually set. The menus are a different
system: `menus/`.

    hud/install.sh             build HUD.asi, revert the file-based fixes, install
    hud/install.sh --remove    take it out (the HUD is then stock: stretched)

Until hud 2.0.0 the same three corrections were made by rewriting game files for one
resolution — `ui-widescreen.py`, `ui-font-condense.py` and `cursor-aspect.py`, below.
Each baked in the aspect `ARMADA.PRF` had when it ran, so a different resolution drew
the HUD, text and cursors too narrow until someone ran them again. They stay here as
the derivation of what the plugin does and as the `--revert` that `install.sh` runs; do
not install them alongside it — every correction would be applied twice. `HUD.asi`
stands down part by part if it finds them (see `HUD.log`).

## HUD.asi — the same three corrections, in the engine

Addresses are this build's `Armada2.exe` (1.1 + Patch Project 1.2.5), read off
`armada2.map`; the plugin checks each site's bytes before writing and leaves that part
inert, and says so in `HUD.log`, if they differ. The source comment in `hud.c` carries the
detail; this is the map.

| Part | Hook | What it does |
|---|---|---|
| **Canvas and palette** | the two `ParameterDB` constructors' calls to `mLoad` (`0x53414c`, `0x5341b4`) | After a DB loads, if it holds `infoPanelArea` it is the GUI one (`gui_<race>.cfg` includes `gui_interface.cfg`, which includes `gui_glob16x12.cfg`). Its declared canvas width at `+0x2c` becomes `round(1200·W/H)`, and the anchored panel and palette values get exactly `ui-widescreen.py`'s edit, in memory. |
| **Font** | `FontNewScreenWidth`'s one caller, `SetActiveDisplay_Internal` (`0x62c5d7`), and `FontInit` (`0x48440c`) | `ST3D_Font+0x28`, the horizontal scale, becomes `+0x2c · 1.25·H/W` for the three MetaFonts. Glyph quad, pen advance, line width and word-wrap all multiply by `+0x28`, so this condenses glyphs and spacing together — what the file condense did by editing every `.spr` width. |
| **Seams** | `ST3D_Sprite::DrawScaled2D`'s snap block (`0x63aeca`), and `ParameterDB::Get(DBRectangle)`'s entry (`0x5358f0`) | Both edges of a snapped sprite go to `floor(v) + 0.5`, a pixel boundary, so tiles meet and MSAA draws no line outside them; a rect's width and height are taken from its converted far edge. See "Seams between tiles". |
| **Cursors** | `RefreshDisplay`'s `DrawScaled2D` call (`0x6246fa`), and `SetCursor` (`0x625dd9`) | Under DXVK the cursor is the *synchronous* one: the engine draws the sprite itself each frame, after `SetScaleFactor2D(&device+0x18)` = W/800 by H/600. For that one draw the x scale becomes the y scale and the position is re-expressed so the hotspot stays on the pointer; the scale is put back after. The hardware path, if a setup ever takes it, gets `fmuls 0x18(%esi)` → `fmuls 0x1c(%esi)` in `SetCursor`. |

What was established to get there, so it need not be re-derived:

- **`Get(DBRectangle)` converts every rect at read time** from the DB's declared canvas
  (`+0x2c`/`+0x30`) into `cfgSCREEN_WIDTH x cfgSCREEN_HEIGHT` — `RTS_CFG.h`'s 1600x1200
  — which is then scaled to the back buffer per axis (`mConvertRectangle`, `0x535ad0`).
  That fixed 1600 is the "hard-coded 1600" the palette reads against. It has 33 readers
  in the exe, the font's draw path among them; **do not change `cfgSCREEN_WIDTH`** to
  fix the canvas.
- A DB's value strings are parsed at `Get` time. The plugin points an edited entry at a
  buffer of its own; the destructor frees the line buffers (`+0x20`) and bucket table
  (`+0x28`) as blocks, never an entry's value, so that is safe.
- **`FontNewScreenWidth` runs on every mode change**, with the `ST3D_DisplayMode`
  `{W, H, bpp}` in `ecx` at the call. The font therefore follows a resolution change
  mid-session. The GUI DB is loaded when a mission starts, so **the layout follows at
  the next mission**.
- `ST3D_Font`s are constructed in exactly one place (`cFontSet::Load`), and only the
  three MetaFonts' current fonts draw text, so those three are all there is to fix.
- **There are two cursor paths, and the one in use is not the hardware cursor.**
  `[device+0xe0]` (`SetSynchronousCursor`) chooses. Clear: `SetCursor` builds a D3D
  hardware cursor texture at `texW·[dev+0x18]` by `texH·[dev+0x1c]`. Set: `SetCursor`
  makes no hardware cursor and `RefreshDisplay` (`0x624630`) draws the sprite as a 2D
  quad under the global 2D scale (`0x7ad6e8`), which it sets to `&device+0x18` and does
  not restore. hud 2.0.0 patched only the hardware path, and the cursor stayed 1.79x wide
  in game — that is how the second path was found. The `.spr` experiment's "map-plane"
  cursor below is this path: it draws the sprite's `W H`.
- `[device+0x18]`/`[device+0x1c]` are W/800 and H/600 and also map cursor positions
  (`SetCursorPosition` divides by them); the plugin changes neither, only what one
  multiply and one draw read.
- The 2D scale is also set by `ST3D_Camera::SetViewport` and `ResizeViewportToExtents`,
  and read by every `ST3D_Sprite` 2D draw and by the font, so the cursor hook restores it.

What it does differently from the files, on purpose: the font's glyph art stays stock
and is drawn narrower, instead of being resampled to fewer texels and point-sampled back
up. At 3440x1440 that is 1.41 screen px per texel on both axes rather than 2.69 across,
so the text keeps every stock texel. Judge it in game (see "The font").

## Seams between tiles

Faint lines ran through the briefing panel (a 5x4 grid of 256x256 `uiObjectives`
sprites, drawn by `StandardBackground`) at every aspect but 4:3, and along the joins
of HUD panels built from pieces: the 3D view showing through a 1–2 px gap between
two quads. The textures were not it: stock and remastered tiles match their
neighbours across every edge to within 2–3 levels, the same as any two interior
columns. Measured by replacing the tiles in a bench clone with a flat red/green
checkerboard with each edge texel marked, MSAA off: at 16:10 every tile drew 255 px
wide, starting at 352, 608, 865, 1120 and 1376 — gaps at 607, 863–864 and 1375.

Two roundings in the engine, each harmless alone at a whole-number scale:

- **`DrawScaled2D` snaps the position but not the size.** With sprite flag `0x80`
  (set on UI sprites) it moves x and y to `floor(v) + 0.25` screen px and keeps the
  unsnapped width. D3D9 pixel centres are at integers, so a quad covers pixels
  `floor(x) + 1` through `floor(x + w)`, and one ending at `x + w` short of the next
  tile's snapped start leaves a column uncovered. At 16:10 a 213-unit rect is 255.6 px:
  tile 0 at 351.6 covers 352–606, tile 1 starts at 608. This is stock behaviour at any
  mode where W/1600 or H/1200 is fractional, 1024x768 included; 800x600 (0.5) and
  1600x1200 (1.0) never show it.
- **`Get(DBRectangle)` rounds x, y, w and h separately** when it converts a rect from
  the declared canvas into the 1600x1200 space (the conversion is `mConvertRectangle`
  inlined, `floor(v · (1600/canvas) + 0.5)` each). At a 1920 canvas, 256 wide at 512
  converts to x 427 and w 213, ending at 640 where the next tile starts at 640 — but
  256 at 256 gives 213 + 213 = 426 against the next tile's 427. That unit is the second
  pixel of the 863–864 gap. Only a canvas other than 1600x1200 converts, so this one
  exists only because `HUD.asi` re-declares the canvas.

`HUD.asi` snaps the far edge the same way as the near one and draws the quad between
them (at `+ 0.5`, not stock's `+ 0.25`: below), and takes a rect's w and h as the converted far edge minus the converted near one.
x and y are converted exactly as the engine does (the original function runs with the
canvas set to stock, and the conversion is reproduced, scale rounded to float first),
so they are bit-identical to stock and w and h move by at most one unit. After it, the
test pattern's edge texels abut at every join at 16:10 and 16:9, both axes; the bench's
count of 1-px vertical lines in the briefing screenshot fell by 24–42 at each wide aspect (214 → 177 at 16:9), and 4:3 is
unchanged. `Seams=0` in `HUD.ini` turns it off.

**A second line, with MSAA: stock snaps to `+ 0.25`, a quarter-pixel short of a pixel
boundary.** With the gaps closed, the user still saw faint lines in game, over the
flat grey of unexplored space in the Federation mission (`testbench/scenarios/hud.md`
now runs there): along the briefing's outer edge, around the minimap frame, at the
command-bar joints. Each was one pixel *outside* a sprite — at 21:9 column 989 and
row 96, where the briefing's first pixels are 990 and 97 — about 7 levels darker than
the fog. Not the art: the installed tiles' alpha is byte-identical to stock at every
edge, and 0 in those columns. Not texture wrap at the quad edge either: tiles made
transparent but for an opaque right column and bottom row put nothing on their left or
top edges. It was MSAA: the same art with `MSAA.ini` `Samples=0` has no line, and with 8
it does. A quad from `n + 0.25` covers a quarter of pixel `n`, so some of its samples
count, and MSAA shades that pixel once, at its centre — outside the quad, where the
texture coordinate has run past 0 and wrapped to the sprite's far edge. The snap now
puts both edges at `floor(v) + 0.5`, the boundary between two pixels: no sample is
half-way, the pixels covered without MSAA are the same, and at 1:1 each pixel centre
lands on a texel centre instead of a quarter off it. With MSAA 8 the column and row
read exactly the fog colour after it.

The 3D view stops one row short of the screen at 3440x1440 (row 1439 is black under the
fog); the minimap panel's base looks dark there because nothing is drawn behind it. That
is the viewport, not the HUD, and is not handled here.

Not seams, and left alone: the faint olive grid behind the briefing text in stock is the
map grid showing through the panel, whose alpha is 217; the white bracket and lines in
the minimap are the camera's view outline.

## The layout canvas: where the UI stretch comes from

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

## The font

Until hud 2.0.0 this was its own `font` layer; `HUD.asi` now applies the same factor
at run time (above). What follows is the derivation, and the file-based condense that
`install.sh` reverts.

`hud/ui-font-condense.py` condenses the in-game bitmap font so it is not drawn 1.9x too
wide at 21:9. Its backups are `.a2font-backup`, not `.a2neb-backup`, so that `a2tex`
can never revert them (the end of this section). It follows from the canvas change
above.

### The font does not ride on the canvas either, and that is the canvas fix's bill

Re-declaring the canvas un-stretches every panel, icon and rect. **It does not touch the
text**, which comes out huge and visibly wide — and that is not a leftover of the old
bug, it is a new mismatch the fix created.

The in-game font is a bitmap sprite, not a system font. `Armada2.exe` builds the name

    Font%s%d.spr        %s = "Final4_", %d = a point size

and `Sprites/` ships eight of them — 10, 12, 13, 15, 16, 19, 20, 24. Each is an atlas:
`Textures/RGB/FontFinal4_<size><page>.tga` is **solid white RGB with the glyphs entirely
in alpha** (measured: mean R=G=B=255, mean A=35.5), and the `.spr` carries a per-glyph
`(u,v)` offset table and a per-glyph advance-width table. The `*Color` keys in
`gui_glob16x12.cfg` tint the white.

Those atlases are authored for a **1280x1024** tier, and the font path scales glyph quads
by back-buffer / 1280x1024 — **2.6875x across against 1.40625x down** at 3440x1440,
whatever `screenWidth` says. Measured against the advance tables, off a 3440x1440
screenshot:

| element | font | predicted | measured |
|---|---|---|---|
| `OBJECTIVES:` (header) | 24 | 620.9 x 32.3 px | 619.5 x 32.7 |
| `BRIEFING SUMMARY:` | 24 | 962.2 px wide | 960.2 |
| `The Borg Queen and a ...` | 16 | 1355 px wide | 1368 |
| `4000` (resource bar) | 16 | 137.1 x 21.1 px | 135.9 x 22.4 |

So every glyph is drawn 2.6875/1.40625 = **1.911x too wide**, and against a canvas that
now scales at 1.20 it is also 2.24x too wide for the panel around it.

**The vertical axis is already correct and is not touched.** 1.40625 against the canvas's
1.20 is a ratio of 1.171875, and that ratio is stock: at 4:3 the font scales by H/1024
against a canvas of H/1200, the same 1.171875. The game has always drawn text 17% taller
than its layout coordinates imply.

**Why 1280x1024 and not 1600x1200.** A rival fit — glyphs scaled by back-buffer /
1600x1200, the stock canvas — matches the body text and the line pitch just as well,
because `sz16 x 2.6875` and `sz20 x 2.15` differ by 1.3% and nothing measurable here
separates them. Word-wrap does not separate them either; both reproduce all five
paragraph breaks. **The headers do.** At 2.15x across, `OBJECTIVES:` at 619.5 screen px
needs an atlas 288 texels wide with a 27-texel cap — a ~30pt font. `FontFinal4_30` does
not exist. At 2.6875x it is `sz24` (231 x 23) to 0.2%, the largest atlas shipped, which
is what the largest tier should reach for.

**`hud/ui-font-condense.py`** squeezes each glyph's art and its advance width by
`(H/1024)/(W/1280)` = `1.25 * H / W` — 0.5233 here — so the engine's own 1.911x stretch
lands the glyph back at its authored proportions. Cell height, atlas size, page layout,
row assignment, frame counts and the white RGB plane are untouched; only the alpha plane
is rebuilt and only the `u`/width numbers move. 1792 glyphs across 8 sizes.

    hud/ui-font-condense.py --preview /tmp/p.png   # stock vs condensed, at game scale
    hud/ui-font-condense.py --dry-run
    hud/ui-font-condense.py
    hud/ui-font-condense.py --check                # do .spr and .tga still agree?
    hud/ui-font-condense.py --revert

**The cost is horizontal sampling, and it is unavoidable from data alone.** The
destination quad is `texels x scale`, so the only lever on width is texels: a glyph that
was 28 texels wide is now 15, and `@tmaterial=font #No filtering, ever.` means the engine
point-samples it back up 2.6875x. Text is correctly proportioned and horizontally
chunkier.

Lanczos held the stems best of the three filters tried (`--filter`; Triangle is softer,
Box loses sub-texel stem placement).

**A crisper alternative was built, measured, and rejected in game — `--method runs`.**
Keep it and keep this note: every static metric favoured it and it still looked worse on
screen, so the next person who finds the text soft can learn that the experiment has
already been run.

The argument was that horizontal antialiasing buys nothing here. The axes magnify very
differently — 2.6875x across against 1.40625x down — so downward a part-covered texel
spans about a screen pixel and reads as a real soft edge, while across it is painted as a
flat 2.7px block that softens nothing and merely puts a grey slab where a stroke edge
should be. Stock's stems are *one texel* wide, so a 0.5233 resize asks for half a texel
and gets a slab every time: on `FontFinal4_24a` the fully-opaque texel count goes
6000 → 1572, with hundreds more picking up an alpha 1–4 ringing halo. Thresholding is no
answer either — at 45% it erases `!` `"` `I` `i` `l` and at 50% it erases 31 glyphs, all
of them the ones already one texel wide. So `runs` works a scanline at a time: map the
stock line's ink runs by the factor, give every run **at least one texel**, keep stock's
gaps, and fill each run flat at that line's own peak alpha — crisp across, stock's shading
kept down, no dropouts, and an invariant stroke count per line. Widths are floored rather
than rounded because rounding gives `1` a foot twice its stem width and `1187` renders as
`[187`.

| method | ink/expected | erased | thinned | fattened |
|---|---|---|---|---|
| **Lanczos resample** — ships | **1.028** | **0** | **0** | **0** |
| Box + threshold 35% | 1.224 | 0 | 2 | 227 |
| Box + threshold 45% | 1.074 | 6 | 29 | 64 |
| Box + threshold 50% | 0.904 | 31 | 120 | 4 |
| runs, width rounded | 1.007 | 0 | 1 | 25 |
| runs, width floored — rejected | 0.900 | 0 | 9 | 16 |

`runs` also takes `24a` from 1572 opaque texels back to 3948 against stock's 6000, and the
alpha plane back to 14 discrete levels from 256.

**And none of that settled it.** Judged in the actual game the crisp variant looked worse
than the soft one. The likely reason is that the reasoning models the engine as a bare
point-sampled blit, while the real text is tinted, drawn over lit panel art and read at a
normal viewing distance — conditions under which a grey slab reads as a soft edge after
all and hard 2.7px blocks read as jagged. The offline renders reproduced the sampling but
not the context.

**The lesson is the part worth keeping:** ink ratio, opaque-texel count, dropout count and
alpha-level count all favoured the variant that lost. They measure weight and structure,
not legibility. Do not change the font's appearance on the strength of that table — put it
in the game and look at it.

Per-glyph integer rounding costs a little accuracy: measured over the whole charset the
realised factor is 0.514–0.558 against the 0.5233 target, biased slightly narrow. The two
sizes actually drawn at this resolution land at 0.5235 (24) and 0.5281 (16). `sz10` is the
worst at 0.5584 because its glyphs are 3–8 texels wide, and it belongs to the 640x480
tier, so it is not drawn here.

**Confirmed in game.** `OBJECTIVES:` measures 323.5 screen px wide against 325.2
predicted — 0.5% — with the cap height unchanged at 32.7 px, exactly as intended, and the
glyph aspect back to 9.89 against the atlas's authored 10.04. The briefing paragraph
reflowed from five lines to three, which is the same prediction seen from the other side.

One thing the model does not capture, and it is **stock behaviour, not a condensing
artefact**: measured line widths run a few percent over the sum of the advance widths,
because **the engine rounds each glyph's advance up to a whole screen pixel**. The drift
is 0.36 screen px per glyph condensed against 0.28 stock — the same effect at the same
per-glyph rate, just accumulated over the longer lines a condensed font fits. Do not
"correct" it in the `.spr`; the widths are right.

**The backups are `.a2font-backup`, not `.a2neb-backup`, deliberately.** `a2tex revert
all` restores every `Textures/RGB/*.a2neb-backup`; if the atlases went back to stock while
the condensed `.spr` files stayed, every `u` and width would point into the wrong place in
a wider glyph — garbled text, out of the command that is supposed to be the safe way out.
A distinct suffix keeps a2tex out of it, the same division ui-widescreen.py already has
with `misc/`. `--check` is what catches a mismatch if one ever happens.
