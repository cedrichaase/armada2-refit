# Star Trek: Armada II — Heroic / Proton setup notes

Everything learned getting the GOG release running well on Arch + Hyprland. Separate
from the texture work in `README.md` / `CLAUDE.md`, though the d3d8 section matters to
anything touching the renderer.

## The install

| | |
|---|---|
| Game | `/home/cedric/Games/Heroic/Star Trek Armada II` |
| Prefix | `/home/cedric/Games/Heroic/Prefixes/Star Trek Armada II` |
| Heroic config | `~/.config/heroic/GamesConfig/1174788223.json` |
| Runner | Proton-CachyOS-latest |
| Base | GOG release = Armada II + patch 1.1, plus Patch Project 1.2.5 |
| GPU | RX 5700 XT (Navi 10 / gfx1010), 8 GB — Vulkan fine, **ROCm effectively unsupported** |

## Heroic configuration

**Heroic rewrites `1174788223.json` when it exits.** Only edit it while Heroic is fully
closed, or the change is silently lost. Backup: `1174788223.json.bak-20260920`.

Current environment:

    WINEDLLOVERRIDES = winmm=n,b;d3d8=n,b

`n,b` = native first, then builtin. Wine searches the application directory before the
system directory, so this is what makes Wine load the game-directory `winmm.dll`
(the ASI loader) and `d3d8.dll` (the Patch Project proxy) instead of its own.

## Widescreen

`STA2WidescreenPatch` v1.0 ships two files into the game directory:

    STA2WidescreenPatch.asi      9216      the patch itself
    winmm.dll                    2169856   Ultimate ASI Loader (ThirteenAG)

It only loads because of the `winmm=n,b` override above. Without it Wine uses its
builtin winmm, the loader never runs, and the patch is inert with no error.

**Setting the resolution:** the in-game graphics menu was unusable (see Hyprland,
below), so it was written directly into `ARMADA.PRF`, line 5:

    0.5 0.5 5 5 5 4 3440 1440 32 1 <NUL> 0 1 0.625 <CR>
                    ^^^^ ^^^^ ^^
                    w    h    bpp

    perl -0777 -pi -e 'binmode STDOUT; s/ 1024 768 32 / 3440 1440 32 /' ARMADA.PRF

**The file contains an embedded NUL byte** — use binary-safe tooling, not `sed`.
Backup at `ARMADA.PRF.bak` (151 bytes). The game rewrites the file on exit (now 154
bytes) and the resolution persists.

### The UI stretch, and where it actually comes from

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

`tools/ui-widescreen.py` re-declares the canvas as `1200 x display aspect` — 2867x1200
here — so both scale factors come out at 1.20 and the UI keeps the size it has today
instead of shrinking. Re-declaring alone is not enough: the extra 1267px all appear on
the right, so it also moves the right-anchored and centred panels. 20 values in 8 files,
nothing else touched, CRLF and tab alignment preserved byte-for-byte.

    tools/ui-widescreen.py --dry-run     # show the 20 changes
    tools/ui-widescreen.py               # apply, reading the resolution from ARMADA.PRF
    tools/ui-widescreen.py --revert      # restore from the *.a2neb-backup beside each

The format keeps screen placement in a handful of `<name>PanelArea` keys and makes
everything else relative to its panel, which is why the change is so small — and why
the sub-element coordinates must **not** be touched.

#### The popup palette is not on the canvas

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

#### The cursors are not on the canvas either

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

##### There are two cursor draw paths, and only one of them reads cursor.spr

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

##### So the correction goes in the art, and cursor.spr goes back to stock

`tools/cursor-aspect.py` squashes the **art** horizontally by `1/1.79` inside its
unchanged 32x32 cell. That fixes *both* paths at once — the map-plane path draws those
same texels into the same 1.79-stretched rect — which is why the `.spr` rewrite was
reverted rather than kept. Keeping both would square the art twice and leave the
map-plane cursor too narrow.

    tools/cursor-aspect.py --dry-run     # 20 textures, frame grids and hotspots
    tools/cursor-aspect.py               # rebuild, reading the resolution from ARMADA.PRF
    tools/cursor-aspect.py --revert      # restore from the *.a2neb-backup beside each

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

##### Two things this got wrong first, both caught by measuring

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

##### An AI upscale cannot help these, and the binary says exactly why

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
code, not data — an ASI hook of the same shape as `tools/menuscale/`, which is why this
is now worth considering rather than impossible. Not attempted.

Until then, `cursor-aspect.py`'s squash is the whole of what art can do.

## The menus are a different UI, and a different problem

**`gui_*.cfg` is the in-game HUD. It has nothing to do with the menus.** The main menu,
options, load/save, campaign select and multiplayer screens are a separate system — the
*shell* — and the widescreen work above does not touch them. They rendered in the
top-left 800x600 of the 3440x1440 screen.

Full write-up, build and usage: **`tools/menuscale/README.md`**. The two facts worth
having here, because both are counter-intuitive and both were measured:

**1. Nothing in any config file can scale the shell.** Every menu is a Win32 dialog
drawn with GDI at hard-coded pixel coordinates, from 800x600 8-bit BMPs in `bitmaps/`.
`Armada2.exe` imports `GDI32!BitBlt` and **no `StretchBlt`, and no
`SetWorldTransform`** — there is no scale factor in the shell to change. It also never
goes near Direct3D, so the d3d8 proxy, DXVK and `dxcfg.ini` cannot reach it either.

**2. The menus were not sitting in a big screen — they were filling a small one.** This
is the part that looks like something else entirely. `GetSystemMetrics(SM_CXSCREEN)`
returns **800** at the main menu, and the game's own window is 800x600 at 0,0. The
engine asks for an 800x600x16 display mode for the front end, hard-coded in two places
in `ST3D_GraphicsEngine::SetActiveDisplay_Internal`:

    push 0x10 ; push 600 ; push 800
    call ST3D_DisplayDevice::FindDisplayMode(int w, int h, int bpp)

Wine then parks that small screen in the corner of the virtual desktop, because a
tiling compositor will not let the desktop window shrink to match it. So there was
nowhere to scale *into*, and raising that mode alone changes nothing either — the shell
would still draw its 800x600 in the corner. Both halves are needed.

`MenuScale.asi` does both: it rewrites that mode to the desktop size *in memory*
(`Armada2.exe` on disk is never touched) and scales the shell into the room it creates.
It loads through the Ultimate ASI Loader, the same `winmm=n,b` override that already
carries `STA2WidescreenPatch.asi`, so it needs no setup of its own.

    tools/menuscale/install.sh            # install
    tools/menuscale/install.sh --mode 0   # install, observe and log only
    tools/menuscale/install.sh --remove   # complete uninstall

**`--remove` is a complete uninstall.** The plugin only ever *adds* `MenuScale.asi`,
`MenuScale.ini` and `MenuScale.log` to the game directory — there is no backup to keep
and nothing to revert, unlike `ui-widescreen.py`.

**Confirmed in game at 3440x1440:** main menu and single-player/campaign screen, both
at 1920x1440, centred, pillarboxed — upscaled, never stretched. Input follows the
picture. Open items are listed at the end of `README.md`.

### Two command-line switches worth knowing

`Armada2.exe` carries its own switch table, recovered from the binary — `nointro`,
`window`, `res`, `resolution`, `bpp`, `fullscreen`, `shelltest` and ~40 more; the full
list is in `tools/menuscale/README.md`.

**`-nointro` skips `Intro.bik` (35 MB) and the three logo reels.** Worth adding to
Heroic's launch arguments; it front-loads every launch otherwise.

**The leading `-` or `/` is not optional, and getting it wrong is not an error.** The
parser checks the first character of each token and only then matches the switch table;
anything else is stored as the **mission name**. A bare `nointro` therefore starts a
match on a mission that does not exist, which presents as a game with a HUD and no map
— not as a rejected argument. That cost a round of confused debugging here.

## Map scrolling

The stock scroll feel is slow because **two of the three knobs ship at or near their
minimum**, and the one that matters most is a per-user setting the game already exposes
in its own options screen. Derived from `Armada2.exe` + `armada2.map`, not guessed:
`cOverViewImp::mCheckCameraPan` (`0x5244f0`) and `ParabolicCamera::Pan` (`0x4dfa50`).

The whole chain, per frame:

    pan = edgeOrKeyFactor
        * UserProfile.<mouse|keyboard>_scroll_speed     // ARMADA.PRF, set by the slider
        * SCROLL_ACCELERATION                           // RTS_CFG.h, 3
        * ramp                                          // INITIAL_SCROLL_SPEED -> MAX_SCROLL_SPEED
        * dt
        * SCROLL_COEFFICIENT / (OVERVIEW_INIT_HEIGHT + |OVERVIEW_INIT_HEIGHT - camHeight|)

`ramp` starts at `INITIAL_SCROLL_SPEED`, grows by `dt` each frame, clamps at
`MAX_SCROLL_SPEED` and **resets the moment you stop**.

**The ramp gains exactly +1.0 per second, and that rate is hardcoded** — `g_scrollSpeed
+= dt`, with no config name over it. It is the constraint that shapes every other choice
here, because it fixes how long the ramp takes to traverse whatever range you give it.
Stock's 1 → 2 therefore takes one second to double and stops there.

**Do not set `INITIAL_SCROLL_SPEED = MAX_SCROLL_SPEED`.** It looks like a clean way to
kill the wind-up on short nudges, and it is — the clamp fires on frame one. But it also
flattens the ramp to a constant, which **removes acceleration entirely**, and that is
worse: crossing the map is exactly the case that wants to speed up as you hold. Tried
here, and it read in game as "very linear — reasonable around the base, cumbersome
between areas, holding at the edge doesn't do anything". Both halves of the ramp matter,
and they want *separating*, not collapsing.

The shape that works is a **low floor and a high ceiling**, with the base speed made up
by `SCROLL_COEFFICIENT` instead:

- `INITIAL_SCROLL_SPEED` sets what a quick corrective nudge gets. Keep it at 1.
- `MAX_SCROLL_SPEED` sets what a held scroll builds to. 8 means a 8x spread, reached
  after 7 seconds of holding — but 2x at one second and 3x at two, which is where the
  feel actually lives. Much above 8 and the top of the range is unreachable in practice.
- `SCROLL_COEFFICIENT` scales both ends together, so it is the knob for "everything is
  too slow", and the one to raise when lowering the floor would otherwise cost you.

### Every navigation path funnels through Pan

Worth knowing before tuning anything, because it is not obvious from the names:
`ParabolicCamera::Scroll` (`0x4dfbb0`) is nine instructions that forward straight to
`Pan` via vtable slot `+0x38`. So edge-scroll (slot `+0x38` directly), the arrow keys and
**right-mouse-drag** (`cOverViewImp::mMouseRightDrag`, slot `+0x3c`) all end up in the
same `Pan`, and **`SCROLL_COEFFICIENT` is the one multiplier that scales all three**. It
is the knob to reach for when the per-device sliders are not enough — and the only one
that also touches right-drag, which bypasses both profile speeds entirely.

`FASTSCROLL_COEFFICIENT` (0.005) is right-drag's own scale factor, applied before `Pan`.
Left alone here: `SCROLL_COEFFICIENT` already lifts that path, and right-drag is direct
manipulation where a 1:1 feel against the cursor is the point.

### The two speeds live in ARMADA.PRF, not in any config file

`GameConfiguration::LoadProfile` (`0x53dc00`) reads **line 2 of `ARMADA.PRF`** as eleven
whitespace-separated values. Fields 3 and 4 are the scroll speeds:

    2 3 1 2 0 0 1 0.28 1 1 0
        ^ ^
        | keyboard_scroll_speed
        mouse_scroll_speed

(Confirmed three ways: fields 1/2/7/9/10 match the constructor's defaults at `0x53da80`,
field 4 matches `KEYBOARD_SCROLL_RATE = 2.0` from `RTS_CFG.h`, and field 8 matches
`cfgMOUSE_HOLD_LEVEL = 0.28` from the same file.)

Both are also **sliders in the game's own Options → Game Settings screen**, and the
slider is the supported way to tune them. The mappings, from `GameSettings.obj`:

| Setting | Slider range | Stored value | Stock | Headroom |
|---|---|---|---|---|
| `mouse_scroll_speed` | 1–50 | `slider / 10` | 1.0 (slider 10) | up to **5.0**, a 5x lift |
| `keyboard_scroll_speed` | 1–20 | `slider + 1` | 2.0 (**slider 1 — the minimum**) | up to **21.0**, a 10.5x lift |

So no binary patching and no config surgery is needed for speed: the keyboard slider
ships pinned to its lowest setting and the mouse slider to a fifth of its range.

`LoadProfile` clamps only the difficulty field (0–2); the two scroll floats are read
unclamped, so `ARMADA.PRF` can hold values above the slider maxima — but opening the
options screen rewrites them back into range, so don't rely on it.

**`MOUSE_SCROLL_RATE` is not settable from `RTS_CFG.h`.** The name is absent from the
EXE's lookup table (`KEYBOARD_SCROLL_RATE` is present), so the global keeps its compiled
1.0 and is only ever used as the seed for a *fresh* profile. Once `ARMADA.PRF` exists,
the profile wins for both. Editing `KEYBOARD_SCROLL_RATE` in `RTS_CFG.h` likewise does
nothing to an existing profile.

### SCROLL_BORDER_WIDTH is the other half of "unwieldy"

`RTS_CFG.h` ships `SCROLL_BORDER_WIDTH = 2` — the mouse must be within **2 pixels** of a
screen edge for edge-scrolling to engage at all, and the factor ramps linearly across
that band. The EXE's own compiled-in default is **20**, so stock's 2 is the config
file overriding the engine down to a hair's width. At 3440x1440 that is the difference
between a usable edge and one you have to hunt for.

Raised to 20 here.

### What is set

| Where | Key | Stock | Now | Effect |
|---|---|---|---|---|
| `ARMADA.PRF` field 3 | `mouse_scroll_speed` | 1 | **5** (slider 50/50) | 5x, edge-scroll |
| `ARMADA.PRF` field 4 | `keyboard_scroll_speed` | 2 | **10** (slider 9/20) | 5x, arrow keys |
| `RTS_CFG.h:40` | `SCROLL_BORDER_WIDTH` | 2 | **20** | usable edge band |
| `RTS_CFG.h:41` | `SCROLL_COEFFICIENT` | 90000 | **300000** | 3.3x, *every* path |
| `RTS_CFG.h:42` | `FASTSCROLL_COEFFICIENT` | 0.005 | **0.015** | 3x, right-drag only |
| `RTS_CFG.h:49` | `MAX_SCROLL_SPEED` | 2.0 | **8.0** | held scroll ramps 8x, not 2x |
| `RTS_CFG.h:50` | `INITIAL_SCROLL_SPEED` | 1 | **1.0** | unchanged — keeps ramp headroom |

Compounding, against stock's own equivalent at each end: an edge-scroll nudge is about
17x stock's nudge, and a fully ramped held scroll about 67x stock's ramped scroll. More
to the point, the *spread between them* goes from 2x to 8x — holding now accelerates
instead of crawling at one speed. Right-drag is 10x and stays linear: it calls `Pan`
directly and never touches the ramp, which is correct for direct manipulation.

`tools/scrollspeed.py` reads and writes all of these; run it with no arguments to print
the current state, and `--revert` to restore both backups. Use it rather than editing by
hand — it keeps `ARMADA.PRF`'s CRLF and field count intact and tells you the slider
position each value corresponds to.

Both files have a `.a2neb-backup` beside them; `cp X.a2neb-backup X` reverts either.
**Edit `ARMADA.PRF` only while the game is closed** — it is rewritten on exit, so a live
game will clobber the change. Tuning further is best done from the in-game slider, which
writes the same fields.

One caveat on `RTS_CFG.h`: the EXE hashes it across clients — *"EXE / RTS_CFG.h files do
not match node %d"* — so any change there has to be mirrored on every machine in a
multiplayer game. `ARMADA.PRF` is per-user and carries no such constraint.

### Telling whether a change actually landed

Both files are read **at launch**, so nothing applies to a running game. Beyond that the
two behave differently, and confusing them wastes a round trip:

- **`RTS_CFG.h`** is parsed fresh every launch (`0x490d73` opens it by name; a name the
  EXE's table does not know is silently ignored and keeps the compiled default). Edit it
  any time.
- **`ARMADA.PRF` can be rewritten by the game**, from the values held in memory, which
  would clobber an edit made while it runs. In practice it has **not** been observed to
  rewrite on exit under Heroic/Proton across several sessions here — its mtime stayed at
  the hand-edit through a number of launches, and the one time it did change was the
  first run, rewriting the GPU name and resolution. Treat a rewrite as possible but not
  the norm, and edit with the game closed anyway.

The cheapest confirmation is the options screen: **Options → Game Settings**, and read
the slider positions against the table above. Neither `RTS_CFG.h` value shows in any UI —
those you judge by feel. When the menus themselves are not usable, `tools/scrollspeed.py`
with no arguments prints the same information from the files.

**Compare the game's start time against the file mtimes** — that is the one check that
settles "did this session have the change", and it needs no UI:

    ps -o lstart= -p $(pgrep -f '[A]rmada2\.exe$')
    stat -c '%y  %n' RTS_CFG.h ARMADA.PRF

**`pgrep -f Armada2` matches the shell that is running the pgrep**, so it reports the
game as running when it is not — a self-match that produced a confident wrong conclusion
here twice in one session, in both directions. Bracket a character (`'[A]rmada2'`) or
match on the process's own command line. Under Heroic the real process is the last of a
six-deep stack — `umu_run.py`, `srt-bwrap`, `pv-adverb`, `proton`, `umu.exe`, and finally
`X:\Games\...\Armada2.exe` — so "did it actually quit" is a question worth checking
rather than assuming; see also the note that Wine calls the process `Main`.

## Patch Project 1.2.5

**The NSIS installer refuses to run against a GOG install**, with
*"Make sure you have Armada II with Patch 1.1 installed in the target directory."*
This is a known GOG incompatibility, not a broken download
(installer md5 `8216c620fb17331a3d647f550ccfa723`).

**Workaround:** download the ZIP distribution of the same version and copy `install/*`
into the game directory by hand:

    Armada2Hook.dll   1865728
    Armada2Hook.mad    105160     MadExcept crash-reporter data
    d3d8.dll            45056     proxy — see below
    FOmsvc.dll         167936

`Armada2.exe` is **not** modified — 1.2.5 is the "loader-free" release, confirmed by
diffing against the original.

## The d3d8 chain

The fiddliest part of the setup. **Two different things both want to be `d3d8.dll`:**

- **GOG's** `d3d8.dll` (1101824) is a full **d3d8to9 translator** — it implements
  Direct3D 8 on top of Direct3D 9.
- **Patch Project's** `d3d8.dll` (45056) is a **proxy** that exports only
  `Direct3DCreate8`, and loads the real implementation from the **system directory**
  via `GetSystemDirectoryA`.

Because the proxy looks in the system directory, the two can be stacked rather than
chosen between:

    Armada2.exe
      -> <game dir>/d3d8.dll          Patch Project proxy      45056
      -> syswow64/d3d8.dll            GOG d3d8to9 translator   1101824
      -> syswow64/d3d9.dll            DXVK                     7798798
      -> Vulkan

GOG's original was moved aside in the game directory as `d3d8.dll.gog-backup`, and
DXVK's own d3d8 was backed up as `syswow64/d3d8.dll.dxvk-backup`.

### PCGamingWiki's advice is wrong for this build

It suggests renaming the patch's `d3d8.dll` to `dinput.dll` to dodge the conflict.
Verified against this executable:

    objdump -p Armada2.exe | grep -i dinput     # no matches

The exe imports **no** dinput or dinput8 at all, so a `dinput.dll` would never be
loaded and the patch would be silently inert.

### ⚠ CORRECTION: 320548 is Wine's builtin d3d8, not DXVK's

**The section below is wrong about what is in the prefix, and the error inverted the
whole diagnosis.** It records `syswow64/d3d8.dll` at 320548 bytes as "DXVK's exact
size". It is not. Measured with `sha256`, not with size folklore:

| file | bytes | what it actually is |
|---|---:|---|
| `syswow64/d3d8.dll` | 320548 | **byte-identical to Wine's builtin d3d8** |
| `syswow64/d3d8.dll.dxvk-backup` | 320548 | the builtin as well — the backup never held DXVK |
| DXVK d3d8 (Proton's) | 1658894 | ~1.66 MB, and has never been in this prefix |
| DXVK d3d8 (Heroic's 3.1.1) | 1687566 | likewise |
| `syswow64/d3d9.dll` | 7798798 | genuinely DXVK (Proton's), but **bypassed** |

So the chain that has actually been running is:

    Armada2.exe
      -> <game dir>/d3d8.dll      Patch Project proxy   45056
      -> syswow64/d3d8.dll        WINE BUILTIN d3d8     320548
      -> wined3d
      -> OpenGL

**DXVK is not in this game's render chain and never has been.** Wine's d3d8 talks to
`wined3d` directly; it never loads `d3d9.dll`, so the DXVK d3d9 sitting in the prefix is
never reached. That is why no `d3d9.*` key in `dxvk.conf` changed anything, why the DXVK
HUD never appeared, and why `--diagnose` produced a `xalia_dxgi.log` (a different
process, which does use DXVK) but no `Armada2_d3d9.log`.

It also means the entire texture project — 2048 skyboxes, a 4096 atlas, 1024 hulls — has
been rendering through wined3d/OpenGL, not Vulkan. Worth knowing before any of it is
attributed to DXVK.

**And the prefix is not a durable place to fix it.** Proton's `default_pfx` holds
`syswow64/d3d8.dll` and `d3d9.dll` as **symlinks** to its own Wine builtins and restores
them on prefix sync. A DXVK `d3d8` written into `syswow64` was verified by hash, then
was Wine's builtin again after a single launch — with `autoInstallDxvk` already false,
so Heroic was not the cause that time. Proton was.

The durable slot is the **game directory**, which nothing manages and which Wine
searches *before* the system directory — the same mechanism that already makes the
game-directory `winmm.dll` and `d3d8.dll` load at all. So `--use dxvk` puts DXVK's
`d3d8.dll` **and** `d3d9.dll` beside `Armada2.exe`, replacing the Patch Project proxy
(kept as `d3d8.dll.proxy-backup`) and making the prefix irrelevant to the outcome.

`autoInstallDxvk` also turns out to be the whole of the "regression" recorded below:
set to true it redeploys over the slot on every launch, which is it working as designed
rather than misbehaving.

`tools/d3d8-chain.py` exists so this cannot recur: it identifies every link by hashing
it against the candidates actually present on the machine and **names** what it found,
reporting `UNKNOWN` rather than guessing. Never identify one of these by size again.

    tools/d3d8-chain.py --status      identify the live chain
    tools/d3d8-chain.py --use dxvk    DXVK d3d8 -> DXVK d3d9 -> Vulkan
    tools/d3d8-chain.py --use gog     GOG d3d8to9 -> DXVK d3d9 -> Vulkan
    tools/d3d8-chain.py --revert      back to Wine's builtin, Heroic managing it again

`--use` also sets `autoInstallDxvk`, so it needs Heroic closed, and it refuses before
touching anything rather than half-applying.

### The original note, kept for the record

### ⚠ This fix does not currently survive a launch

`autoInstallDxvk` is `true`, and **Heroic redeploys DXVK's DLLs into the prefix on every
launch**, overwriting the GOG translator. Confirmed: `syswow64/d3d8.dll` is back to
320548 bytes (DXVK's exact size) with the same mtime as `d3d9.dll`, `d3d11.dll` and
`dxgi.dll` — a bulk redeploy.

Consequences:

1. The chain above is **not in effect right now**.
2. **The cutscene-crash fix was therefore never actually tested** — it was reverted
   before the next play session.

Re-checked 2026-09-21 and still true. `syswow64/d3d8.dll` is 320548 bytes with the same
mtime as `d3d9.dll`, both rewritten at the last launch; `autoInstallDxvk` is still
`true`; `WINEDLLOVERRIDES` is still `winmm=n,b;d3d8=n,b`.

So the chain actually in effect is **the proxy into DXVK's own d3d8**, not into the GOG
translator:

    Armada2.exe
      -> <game dir>/d3d8.dll          Patch Project proxy      45056
      -> syswow64/d3d8.dll            DXVK d3d8                320548
      -> Vulkan

Worth knowing rather than only regretting: DXVK's native d3d8 is what the whole texture
project has actually been rendering through, and it has handled 2048 skyboxes, a 4096
atlas and 1024 hull textures without complaint.

Remedies, in order of preference:

- Set `autoInstallDxvk` to `false` (Heroic closed), then restore the translator:
  `cp "<game dir>/d3d8.dll.gog-backup" "<prefix>/pfx/drive_c/windows/syswow64/d3d8.dll"`
  DXVK's `d3d9.dll` stays in place; only the d3d8 slot needs to stop being managed.
- Or re-copy the translator after every launch, which is fragile.

### Renderer-side enhancements

Everything in this section is global — it applies to every texture and every model at
once, costs nothing per-asset, and is reversible by deleting one file. All of it is
independent of the texture pipeline. Verified against **DXVK 3.1.1**, the version Heroic
actually deploys (`~/.config/heroic/tools/dxvk/dxvk-3.1.1/`), by reading the config keys
out of the shipped `d3d9.dll` rather than from documentation — the key list below is what
this binary honours, not what some DXVK version honours.

#### This does not have to wait for the d3d8 regression

An earlier revision of this file said the graphics stack should not be touched while the
chain regression above is open. For `dxvk.conf` specifically **that is wrong, and the
reason matters**: both possible chains end in DXVK's `d3d9.dll`.

    proxy -> DXVK d3d8   -> DXVK d3d9 -> Vulkan      (live today)
    proxy -> GOG d3d8to9 -> DXVK d3d9 -> Vulkan      (after the regression fix)

`d3d9.*` keys are read by the d3d9 layer, which is present and is DXVK's in both. So a
`dxvk.conf` change is attributable regardless of which `d3d8.dll` won the last launch,
and does not need the regression closed first. The AA routes below are the ones that do.

#### Tier 1 — `dxvk.conf`, free and reversible

`tools/renderer-config.sh` writes it, in **cumulative stages**, because this project's
method is one change at a time and three keys at once is not attributable.
**All three stages are installed and confirmed in game.** It verifies every key against
the `d3d9.dll` that actually loads — the one in the game directory — before writing,
because a key DXVK does not recognise is silently ignored:

    tools/renderer-config.sh              # stage 1: anisotropic filtering only
    tools/renderer-config.sh --stage 2    # + mip LOD bias
    tools/renderer-config.sh --stage 3    # + seamless cube filtering
    tools/renderer-config.sh --bias -0.25 # milder stage 2 bias (default -0.5)
    tools/renderer-config.sh --show       # what is installed now
    tools/renderer-config.sh --remove     # delete dxvk.conf, full revert

- **`d3d9.samplerAnisotropy = 16`** (stage 1). `dxcfg.ini` asks for `application`, i.e.
  whatever a 2001 renderer requests, which is likely none. In a top-down RTS every hull
  and every planet is drawn at a steep angle to the camera, so all of it is sampled with
  plain trilinear and blurs along the axis of foreshortening. This is the single biggest
  free win and the one to judge on its own.
- **`d3d9.samplerLodBias = -0.5`** (stage 2). Biases mip selection toward the sharper
  level. This matters *because of the texture work*, not independently of it: there is
  now 1024–2048 art under a camera that views the map plane obliquely, and trilinear
  picks a blurrier mip than the art can support. **Only safe with AF already on** —
  without it a negative bias aliases rather than sharpens, which is why it is a separate
  stage and not part of stage 1. `d3d9.clampNegativeLodBias` is the guard if it
  overshoots. **The failure mode is shimmer on movement, not blur when
  still** — and in this game it shows on the billboard layer first, so if the map crawls
  while scrolling, halve the bias with `--bias -0.25` before abandoning the stage.
  Below about -1.0 it aliases faster than AF can clean up.
- **`d3d9.seamlessCubes = True`** (stage 3). Filters across cube-map face edges. Directly
  relevant to the finished skybox class: many maps bind a `.sod` cube model, and a
  face-edge seam gets *more* visible at 2048/face, not less.

`DXVK_HUD=fps,frametimes` is the measuring aid; it is deliberately **not** written into
`dxvk.conf`, so it cannot be left on by accident. Expect the GPU to be near-idle — a 2001
engine against an RX 5700 XT — and all three stages to be free.

#### Telling whether a renderer setting did anything

Asked after stage 1 went in and the answer was not obvious by eye. It is two separate
questions and they need separate tools, because "the setting was ignored" and "the
setting worked and is subtle" look identical in game.

**Is it applied?** A `dxvk.conf` key DXVK does not recognise is *silently ignored* — no
error, no warning. DXVK does print its effective configuration at startup, so ask it:

    tools/dxvk-logging.py --diagnose  # logging + HUD + d3d9=n,b, all at once
    # launch the game once
    tools/dxvk-logging.py --check     # report the effective configuration
    tools/dxvk-logging.py --off       # take all of it back out again

`--diagnose` answers three questions in one launch, ordered so each makes the next
meaningful:

| observation | conclusion |
|---|---|
| no DXVK HUD overlay in game | DXVK is not in the chain; **no** `dxvk.conf` key can work |
| HUD shown, but no log file | DXVK's d3d9 layer never loaded |
| log written, keys absent from `--check` | DXVK ran but never found or read `dxvk.conf` |
| keys present in `--check` | it is applied, and the effect really is that subtle |

**Why `d3d9=n,b` is part of the diagnosis.** DXVK's `d3d8.dll` imports `d3d9.dll` by
name — confirmed with `objdump`, not assumed — so every `d3d9.*` key is read by a layer
that only exists if Wine resolves `d3d9` to DXVK's build rather than its own builtin
WineD3D. This prefix has only ever carried `winmm=n,b;d3d8=n,b`, and setting
`WINEDLLOVERRIDES` at all *replaces* whatever Proton would have set, so the d3d9 slot
may never have been native. That is the leading suspect whenever a `d3d9.*` key appears
to do nothing here, and it is the same gap the "Also outstanding" note below has
recorded, untested, all along. `--off` restores the original override string too.

**Heroic must be closed** for `--on`/`--off`: it rewrites `GamesConfig` on exit and
would discard the edit. The script refuses rather than losing the change silently, and
backs the file up regardless. Note Heroic's key is spelled `enviromentOptions`, missing
an `n` — matching its typo is required.

**Did it change the picture?** `tools/ab-shot.sh` grabs frames and diffs them
numerically, so the answer is a number rather than an impression:

    tools/ab-shot.sh grab before
    # change one thing, relaunch, return to the same save without moving the camera
    tools/ab-shot.sh grab after
    tools/ab-shot.sh diff before after 600 400 1200 300     # W H X Y, region only

Aim at a region, not the whole frame: a whole-frame diff of this game is dominated by
ships drifting and sprites animating between the two grabs, which will swamp the effect
and make any setting look like it did something.

#### Why anisotropic filtering is structurally quiet in THIS game

Worth stating plainly, because the usual "AF transforms an old game" advice assumes
content this game does not have. AF only acts on surfaces **oblique to the camera**, and
it is a correction to *mip selection* — so it can only touch geometry that is both
mipmapped and foreshortened.

- **The dominant visual layer here is immune by construction.** Map nebulae, resource
  clouds, weapons and explosions are camera-facing **billboards**. A billboard is never
  oblique — that is what makes it a billboard — so AF cannot affect any of it. In a
  frame like the one that prompted this, that is most of what the eye is drawn to.
- **What AF *can* reach is small on screen.** Hull flanks, planet limbs and the skybox
  at grazing angles. A Sovereign draws ~340px wide in a top-down RTS, so its
  near-grazing side surfaces are a few dozen pixels tall. Real, and not dramatic.

So a subtle result at stage 1 is the expected result, not evidence of a broken setting —
which is exactly why `--check` exists to separate the two.

**The corollary is the useful part: stage 2 should be the visible one here.** A mip LOD
bias is not conditional on obliquity — it shifts mip selection for *every* mipmapped
surface, camera-facing billboards included. That is precisely the layer AF cannot touch
and precisely where the upscaled art lives. Expect stage 2 to do more for this game's
appearance than stage 1, which inverts the usual ordering. Stage 1 still goes first:
AF is what keeps the sharper mips stage 2 selects from aliasing on the oblique surfaces.

#### RESOLVED: DXVK works, and the Wine virtual desktop was the blocker

**Confirmed in game at 3440x1440, with all of `dxvk.conf` applied — stages 1, 2 and 3.**
The log for the working launch: config found, **no errors at all**,
`last mode set: 3440x1440`, and all four keys under "Effective configuration":

    d3d9.samplerAnisotropy = 16
    d3d9.samplerLodBias = -0.5
    d3d9.clampNegativeLodBias = False
    d3d9.seamlessCubes = True

The working configuration, all four parts required together:

| part | value | why |
|---|---|---|
| `GAME/d3d8.dll` | DXVK d3d8 | the prefix is not durable — Proton restores it from symlinks |
| `GAME/d3d9.dll` | DXVK d3d9 | DXVK's d3d8 imports `d3d9.dll` by name |
| `WINEDLLOVERRIDES` | `winmm=n,b;d3d8=n,b;d3d9=n,b` | without the d3d9 entry Wine resolves it to builtin WineD3D |
| Wine virtual desktop | **off** | inside it DXVK's `ChangeDisplaySettingsEx` fails and the game falls back to 640x480 |

Set it up with `tools/d3d8-chain.py --use dxvk` and
`tools/virtual-desktop.py --off`; reverse with `--revert` and `--on`.

**The `d3d9=n,b` override is load-bearing, not diagnostic.** It arrived as part of
`dxvk-logging.py --diagnose`, so `--off` used to strip it — which would have silently
broken the chain the moment diagnostics were switched off. Ownership now sits with
`d3d8-chain.py` (`--use` adds it, `--revert` removes it) and the logging tool only ever
adds, never removes.

**The two-window focus bug the virtual desktop was added for did not return** under
DXVK. If it ever does, the alternative is Omarchy window rules — see the Hyprland
section.

#### How it was found — kept because the method is the lesson

Settled by measurement, after three wrong explanations for "I can't see a difference".

**The settings were never the problem, and neither was subtlety.** With DXVK's `d3d8.dll`
and `d3d9.dll` in the game directory the HUD appeared and `Armada2_d3d9.log` reported:

    info:  Found config file: dxvk.conf
    info:  Effective configuration:
    info:    d3d9.samplerAnisotropy = 16
    info:    d3d9.samplerLodBias = -0.5
    info:    d3d9.clampNegativeLodBias = False

So `dxvk.conf` is found and every key applies, once DXVK is actually reached.

**But the game then collapses to 640x480.** The same log:

    err:   D3D9: EnterFullscreenMode: Failed to change display mode   (x4)
    err:   D3D9: Failed to set initial fullscreen state               (x4)

It alternates 3440x1440 and 640x480 across 25 mode sets and ends on 640x480;
`MenuScale.log`'s last line agrees, reporting `screen 640x480`. So the engine asks for
exclusive fullscreen, DXVK cannot change the display mode, and the fallback wins.

**The prime suspect is the Wine virtual desktop** (`Software\\Wine\\Explorer`,
`Desktop=Default`, `Default=3440x1440`). wined3d never needed a real mode change inside
it; DXVK calls `ChangeDisplaySettingsEx` and it fails. The Hyprland section below
already records reverting the virtual desktop as the documented alternative, using
Omarchy window rules instead — that is the next experiment, and it is one launch.

**Current state: reverted to the stock chain so the game is playable.**
`tools/d3d8-chain.py --revert` put the Patch Project proxy back; Direct3D 8 is wined3d
on OpenGL again and `dxvk.conf` is inert. The DXVK chain is one command away
(`--use dxvk`) whenever the fullscreen question is worth another launch.

**Note that the renderer question is now separable from the texture question.** Nothing
about the texture work depends on any of this: it has always rendered through
wined3d/OpenGL and continues to.

#### Tier 2 — anti-aliasing: route 2 built, `tools/msaa/`

**Route 2 is built, installed and confirmed in game: `MSAA.asi`, 8x.** The first launch
logged 8x on all three device creations (3440x1440, the 640x480 fallback, 3440x1440
again) and DXVK logged no errors.

    tools/msaa/install.sh                 # build + install (Samples=8)
    tools/msaa/install.sh --samples 4     # or 2; 0 patches nothing
    tools/msaa/install.sh --remove        # three files out, stock behaviour back

It redirects the two `call GetWindowHandle` that sit just before
`IDirect3D8::CreateDevice` in `ST3D_DeviceDirectX8::CreateDevice` (`0x6235d0`). At
that point `edi` is the present-parameters struct. The plugin then sets:
- the highest sample count the device accepts for both the colour and D16 depth
  formats
- `SwapEffect=DISCARD`
- a non-lockable back buffer

`Reset` reuses the same struct, so a device loss keeps MSAA. `MSAA.log` records what
each device creation actually got. The renderer was read end to end first: nothing
renders off-screen, and the one back-buffer read-back is the minimap copy
(`CopyOffscreenToTexture`). That copy works only because DXVK's D3D8 CopyRects
resolves a multisampled source through `StretchRect`. So **check the minimap first**
when judging it. Full reasoning in `tools/msaa/README.md`.

What follows is the analysis that chose the route.

**DXVK 3.x ships no MSAA-forcing key.** Read out of the binary that actually loads, not
from documentation: there is no `forceSwapchainMSAA` or equivalent.
`d3d9.forceSampleRateShading` exists but only does anything once MSAA is already on. So
AA cannot come from `dxvk.conf`, and the three routes are now these — note that the
cheapest one died when the chain changed:

1. **~~`dxcfg.ini`'s own `antialiasing=` key~~ — dead.** It is read by the GOG d3d8to9
   translator, and the working chain replaced the translator with DXVK's own d3d8.
   `dxcfg.ini` is inert and will stay inert unless the chain moves to
   `d3d8-chain.py --use gog`, which is a real option (GOG d3d8to9 → DXVK d3d9 → Vulkan)
   and would make both `anisotropic=` and `antialiasing=` live again. Untested, and its
   accepted values are unknown because `dxcfg.exe`'s strings are packed.
2. **An ASI hook on `IDirect3D8::CreateDevice`**, setting `MultiSampleType`. Squarely
   inside the toolchain `tools/menuscale/` already proves — clang + lld-link + the
   Ultimate ASI Loader that is already carrying two plugins — and `armada2.map` gives
   the call site. **The most likely to work, and now the most attractive**: DXVK
   implements D3D8 multisampling properly on Vulkan, which wined3d's D3D8 path did not
   reliably do, so this became a better bet the moment the chain changed.
3. **Post-process AA via vkBasalt** — see Tier 3. FXAA/SMAA rather than MSAA, so it
   softens edges rather than resolving them, but it costs no code at all.

**Supersampling by rendering above display resolution remains a trap.** It looks free
given the GPU headroom and is not: `ui-widescreen.py`'s canvas arithmetic, the
`popupPaletteXA` correction and `MenuScale.asi`'s desktop-size read are all tuned to
3440x1440 and would need re-deriving.

#### Tier 3 — post-processing: bloom done, the rest untried

**This was impossible until the chain changed, and that is the point.** vkBasalt is a
**Vulkan layer**. Until DXVK went in, Direct3D 8 landed on wined3d/**OpenGL**, so
vkBasalt had nothing to attach to — any attempt would have done nothing, with no error
to explain why. Now that the game renders through Vulkan, the whole post-processing
family is reachable for the first time.

**Done: bloom is installed and confirmed in game** — MagicBloom through vkBasalt,
threshold 6, intensity 0.08, accepted by the user after one step up from 0.05.
**Home toggles it live**, which makes it the one renderer setting in this project with a
same-frame A/B.

    tools/vkbasalt/build.sh          # build + install the layer (per-user, pinned)
    tools/postfx.py --on             # write config, prove it compiles, enable in Heroic
    tools/postfx.py --set --intensity 0.08 --threshold 6    # retune; relaunch to see
    tools/postfx.py --check          # read vkBasalt's log from the last launch
    tools/postfx.py --off            # disable for the game
    tools/vkbasalt/build.sh --remove # uninstall everything

How it is put together, and why each piece is that way:

- **Built from source into `~/.local`, not from the AUR.** `lib32-vkbasalt` wants a
  sudo password and installs system-wide; the build needs only what is already here
  (`g++ -m32`, `lib32-libx11`, `glslang`) plus meson/ninja in a venv and headers fetched
  by commit. Every source is pinned by SHA. vkBasalt 0.3.2.10 (2023, the last tag)
  does not build against current Vulkan-Headers — `vk_layer.h` left that repo — so the
  headers are pinned to SDK 1.3.250.
- **32-bit only**, because a Vulkan layer loads into the process that calls Vulkan, and
  that is the i386 game. This holds because Proton-CachyOS uses *old* WoW64 unless
  `PROTON_USE_WOW64=1` is set — **under new WoW64 the Unix side is 64-bit and this
  layer would silently never load.** If that variable ever appears, build a 64-bit one.
- **The manifest is inert without `ENABLE_VKBASALT=1`**, which `postfx.py` sets in this
  game's Heroic environment only, beside `VKBASALT_CONFIG_FILE` and a log file.
- **Nothing lives in the game directory.** vkBasalt's config parser drops spaces and
  warns against them in shader paths, and the game directory is `Star Trek Armada II`;
  config, wrapper, shaders and log are all under `~/.local/share/a2-vkbasalt/`.
- **Shaders are split across two branches of crosire/reshade-shaders**, which is not
  obvious: the single-file bloom shaders are on `legacy`, and `legacy` ships **no
  `ReShade.fxh`** — the headers every one of them includes are on `master`.
- **`fxcheck`** (`tools/vkbasalt/fxcheck.cpp`) links the layer's own `libreshade.a` and
  compiles an effect with vkBasalt's exact macros and codegen flags, offline.
  `postfx.py` refuses to enable an effect it cannot compile, and refuses any config key
  the compiled effect does not expose. Both failures are otherwise *silent* in game — an
  absent effect, an ignored key — which is the DXVK lesson applied in advance. It caught
  the missing headers on its first run; the output also passes `spirv-val`.
- **`MagicBloom` is wrapped, not used as shipped** (`A2Bloom.fx`, written by
  `postfx.py`): eye adaptation off, because it rescales bloom by average frame
  brightness and would pulse as the camera pans between empty space and a nebula; lens
  dirt off, because there is no lens. Those are preprocessor switches, which
  `vkBasalt.conf` cannot set.

**The numbers were measured before the first launch**, by approximating the shader —
threshold power, 8-level blur pyramid, Hable tonemap at MagicBloom's fixed 100x, screen
blend — in ImageMagick on a real frame:

| threshold / intensity | latinum | fog of war | HUD | whole frame |
|---|---|---|---|---|
| 2 / 1 (MagicBloom's defaults) | — | — | — | mean +123/255, 99.7% of pixels changed |
| 4 / 0.1 | +27.2 | +6.5 | +6.3 | |
| 6 / 0.1 | +22.5 | +0.7 | +3.8 | |
| 6 / 0.03 | +7.7 | +0.2 | +1.2 | |

The threshold is an *exponent* on colour, so it decides **what** blooms and intensity
decides **how much**. At 6 the glow lands on the additive sprites that already clip and
leaves the grey shroud alone (31:1); at 4 the shroud hazes over (4:1). The HUD is in
the frame vkBasalt sees, so it can bloom too; the high threshold is also what keeps
that small. The shader's own defaults would have made the frame milky.

It operates on the final swapchain image, and that decides what is possible: CAS
sharpening, FXAA/SMAA, LUT/tonemapping and depth-independent bloom all work; anything
needing depth does not.

**Bloom is the one worth doing**, and the reason is specific to this game rather than
general taste. Armada II's visual language is almost entirely additive sprites —
nebulae, weapons, engine glows — and the latinum clouds already clip to flat white where
overlapping billboards composite past 255. That clipping is stock-faithful and cannot be
fixed in the texture, because the per-channel mean *is* the light contributed and
lowering it breaks the match against stock. Bloom converts that blowout from "the
texture ran out of range" into "that is a bright object", which is the correct read and
which no amount of texture work can produce.

Judge it the way `tools/measure-invention.sh` judges a blend, not at 1:1 — and
`tools/ab-shot.sh` will diff two launches numerically.

#### Tier 0 — ambient occlusion, which is the wrong tool here

Unchanged by any of the above, and the reasoning is worth keeping because "add AO and
bloom" is the reflex suggestion for any old game and only half of it survives contact
with this one.

The practical objection: depth-buffer access through d3d8 → d3d9 → DXVK is exactly where
ReShade's depth detection is least reliable.

The real objection is about content. **AO darkens contact points and creases, and this
scene has neither.** Ships float in vacuum against a skybox — nothing touches anything,
there is no ground plane, no architecture, no interior corners. The 2001 hull models are
low-poly, so there are barely any geometric creases to occlude either, and what detail
exists is *painted into the diffuse map*, where AO cannot see it. The cost is high and
the return is faint rim-darkening on ship silhouettes.

The same reasoning rules out most lighting-based effects: the game has close to no
lighting model to enhance. That is why the texture work has so much leverage here and
why shader tricks have so little.

### No longer outstanding

`d3d9=n,b` is in `WINEDLLOVERRIDES` — `winmm=n,b;d3d8=n,b;d3d9=n,b`, confirmed in
Heroic's config. It is part of the working DXVK chain above.

## Hyprland / window management

**Symptom:** the game opened a second window; the settings menu drew in one while input
stayed grabbed by the other, so the menu was visible but could not be clicked. Focus
kept snapping back to a black fullscreen frame.

**Fix:** a Wine virtual desktop, so everything renders inside one window. Appended to
`<prefix>/pfx/user.reg` (backup `user.reg.bak-a2`):

    [Software\\Wine\\Explorer]
    "Desktop"="Default"

    [Software\\Wine\\Explorer\\Desktops]
    "Default"="3440x1440"

**Alternative if the virtual desktop ever causes trouble** (it is the prime suspect for
the cutscene crash if the d3d8 chain turns out not to be the cause): revert it with
`cp user.reg.bak-a2 user.reg` and use Omarchy window rules instead — `o.window(...)`
with `fullscreen = true`, pattern at
`/usr/share/omarchy/default/hypr/apps/retroarch.lua`. Note the window class is
`steam_proton`, which is not unique to this game, so scope the rule by title.

## Known issues

- **Cutscene crash.** Finishing Federation mission 1 threw a DirectX-related error and
  crashed when the completion cutscene tried to play. Startup videos (`Intro.bik`) play
  fine, so Bink itself works; the hypothesis is a D3D8 device reset on the transition
  from live 3D to fullscreen video. No diagnostics were produced at all — `Logs/` empty,
  no coredump, no MadExcept report, no Heroic session log. **Unresolved**, and per the
  section above the intended fix is not currently installed.

## Campaign progress format

`save/shell.set`, 78 bytes. Backup at `save/shell.set.bak`.

- Stock: all zeros except offset 11 = `0x01`.
- Setting **all** bytes to `0x01` unlocked the first **two** missions of every campaign.
- So the bytes are **progress counters** (value = missions completed), not booleans.
  `0x09`/`0x0A` should open all ten.
- Offset 11 is *not* the Federation counter — it held `1` while only mission 1 was
  selectable.

`mshell.set` in the game directory is the mission list: 40 entries, six tutorial plus
ten each for Federation, Klingon and Borg.

## Gotchas

- **Neither obvious way of finding the game process works**, and the advice that used to
  stand here — "use `pgrep -x Armada2.exe`" — is **wrong**:
  - `pgrep -f "Armada2.exe"` matches its own command line and reports a false positive.
    Likewise `pkill -f startrekarmada2` killed its own shell (exit 144).
  - `pgrep -x Armada2.exe` matches **nothing, even while the game is running**, because
    **Wine reports the process `comm` as `Main`.** This is not cosmetic. It made a test
    harness report "game DOWN" for a game that was plainly up — producing several
    confident, wrong conclusions about the game exiting on its own — and it made a
    `pkill -x Armada2.exe` cleanup a silent no-op, so **seven orphaned instances
    accumulated over half an hour**, none with a window, each still holding a PipeWire
    stream and audibly playing the menu music.

  Collect PIDs with `ps` and kill them individually:

      ps -eo pid=,args= | awk '/Armada2\.exe/ { print $1 }'

  **`tools/menuscale/stop-game.sh` does this properly** — it also kills the Wine helpers
  for this prefix only (matched by `WINEPREFIX` out of `/proc/<pid>/environ`, so an
  unrelated Wine app cannot be caught in it), and drops stale PipeWire nodes, which
  outlive the process, stay in state `running`, and keep playing. Killing the processes
  is *not* enough on its own.
- **`md5sum` wedges on Wine-backed paths** — it blocked in `unix_stream_read_generic`
  partway through a manifest. For before/after comparison use
  `find -printf '%s\t%TY-%Tm-%Td\t%p\n'` instead; size+mtime is enough and is instant.
- **Heroic overwrites its per-game JSON on exit.** Close it before editing.
- **Web sources:** moddb.com and pcgamingwiki.com return HTTP 403 to automated fetches;
  armadafiles.com has a broken TLS certificate (altnames are `*.kasserver.com`) — reach
  it over plain `http` with `curl`.

## Verification recipes

    # what is actually in the d3d8 slot? 320548 = DXVK, 1101824 = GOG d3d8to9
    stat -c '%s %y' "<prefix>/pfx/drive_c/windows/syswow64/d3d8.dll"

    # did DXVK redeploy? these four sharing an mtime means yes
    cd "<prefix>/pfx/drive_c/windows/syswow64" && stat -c '%n %y' d3d8.dll d3d9.dll d3d11.dll dxgi.dll

    # what does the exe actually import?
    objdump -p Armada2.exe | grep -i 'DLL Name'

    # current overrides, without opening Heroic
    python3 -c "import json;print(json.load(open('$HOME/.config/heroic/GamesConfig/1174788223.json'))['1174788223']['enviromentOptions'])"

    # is the game really running?  NOT `pgrep -x Armada2.exe` -- Wine calls it "Main"
    ps -eo pid=,args= | awk '/Armada2\.exe/ { print $1 }'

    # stop it, its Wine session, and any stale audio node it left behind
    tools/menuscale/stop-game.sh

    # did the menu scaler load, and what did it patch?
    cat "<game dir>/MenuScale.log"
