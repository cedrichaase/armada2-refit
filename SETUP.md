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

### Anisotropic filtering — untried, and probably the cheapest visual win left

`dxcfg.ini` sets `anisotropic=application`, i.e. whatever a 2001 renderer asks for, which
is likely none. Everything drawn at a steep angle to the camera — which in a top-down RTS
means every ship hull and every planet surface — is therefore sampled with plain
trilinear filtering and blurs along the axis of foreshortening.

DXVK can force it regardless of what the application asks. A `dxvk.conf` beside
`Armada2.exe`:

    d3d9.samplerAnisotropy = 16

DXVK's d3d8 runs on its d3d9 backend, so the `d3d9.*` key is the right one under the
chain that is actually live above. It costs nothing, applies to every texture in the
game at once, and is reversible by deleting the file.

**Not done.** It is the one renderer-side change with an obvious upside, but it touches
the graphics stack while the regression above is open, so it should be tried on its own,
with nothing else changing, so that a bad result is attributable.

### Also outstanding

`d3d9=n,b` has never been added to `WINEDLLOVERRIDES`. Without it the translator may
resolve to Wine's builtin d3d9 rather than DXVK's. Target value:

    winmm=n,b;d3d8=n,b;d3d9=n,b

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
