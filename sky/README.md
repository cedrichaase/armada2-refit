# Sky — the sky as a function of direction

`Sky.asi` draws a map's sky from a recipe, in a shader, in place of the six painted
faces of the stock skybox. The sky is computed from the view direction, so it has no
faces and no seams, and nothing in it comes from the game's art: a recipe is a handful
of numbers (seed, palette, density), and the look is this folder's code. It can be
redistributed and licensed like the rest of the repository.

    sky/install.sh             build, then install Sky.asi, Sky.ini and Sky\*.ini
    sky/install.sh --timing    ... with Timing=1
    sky/install.sh --remove

A map whose sky has no recipe keeps the stock sky, and so does a chain with no Direct3D
9 device behind d3d8 (DXVK's own d3d8, GOG's): the plugin says so in `Sky.log` and
calls the engine's draw as before.

**Recipes: every sky a stock map uses.** `mbgaqu`, `mbgkling`, `mbgpur`, `mbgred`,
`mbgrg`, `mbgdk`, `mbgkl`, `mbgflu`, `mbggb`, `mbg02` and `mbgblue` (the same model and
art, so the same recipe), `mbgborg`, `mbgdom1` and `mbgbaku`. Left stock on purpose:
`Mbgstars`, a sparse starfield. Also left stock: the eleven sets no map uses (`MbgCard`,
`MbgDom2`, `MbgIkol`, `MbgKlin2`–`4`, `MbgOmega`, `MbgRom1`–`3`). Their faces are named
`.1`–`.6` and no cube model names them, so no map can reach them. The same goes for
`Mbg01.SOD` and `MbgX.SOD`, which carry a planet inside the sky model: a recipe would
drop the planet with the cube.

## Why the faces had to go

Every image route leaves the cube's edges visible: the stock faces are separate
pictures whose edge texels never match (`textures/README.md`, "Skybox": a best fit of
0.077 RMSE across edges), and upscaling or regenerating a face keeps it a separate
picture. A function of direction matches across any edge by construction.

## The hook

`Background_Render` (0x590d90, from `armada2.map`) draws the sky each frame, from its
one caller in the frame loop. It sets the far plane, turns depth writes off, moves
`g_background_instance` (0x772068) to the camera and draws it with one call to
`ST3D_Instance::Render` (0x62e750) at **0x590e17**. Then `Starfield::Render` draws the
point stars, the map's background objects are drawn, and depth is cleared.

The plugin re-points that one call (an `E8`, checked against its expected target
before it is written) at `hook_bg_render`. When the sky has a recipe and the d3d9
device is there, the hook draws the sky and returns; the cube is not drawn. Otherwise
it calls `ST3D_Instance::Render`, and the stock sky draws exactly as before. The point
stars, the background objects and the depth clear stay the engine's.

**Why not a shader at the cube's own draw.** The cube goes the engine's CPU path, which
hands Direct3D pre-transformed, screen-space triangles (`testbench/d3dtrace/README.md`).
No direction reaches the device, so a shader there has nothing to compute the sky
from. **Why not black faces with the sky drawn underneath.** It would need the faces
installed through `a2tex`, and without a d3d9 device the player would get a black sky
rather than the stock one. Not drawing the cube needs no game file at all.

**The view rays.** `ST3D_Camera::ProjectScreenPointToWorldRay` (0x619140) gives the
world-space ray through a point of the camera's viewport. The viewport is the
`ST3D_Rect` (x, y, w, h, floats) at camera +0x1b0. The plugin asks for the four
corners and the centre, then scales each corner ray so its component along the centre
ray is 1, which puts it on the image plane. It draws one quad over the viewport
carrying those rays. A pinhole camera's ray is affine in screen position, so the
interpolated ray is exact at every pixel. On the bench the camera's viewport and the
device's agreed at every resolution tried (3440x1440, 3840x2160, 1600x1200).

**The device.** Storm3D's current device: `ST3D_GraphicsEngine` (0x7ad508) +0xc0 is
its index into the table at +0xcc. The entry is an `ST3D_DeviceDirectX8` (vtable
0x6bc6ac, checked), whose `IDirect3DDevice8` is at +0x90 (as `DrawRectangle` uses it).
`d9_device()` (`platform/d3d9/d3d9dev.h`) turns that into the d3d9 device behind
crosire's d3d8to9.

**The name.** `Starfield_Load_Background_Geometry` (0x590c30) keeps the map's
background name, lower-cased, at 0x738538, as `lighting/README.md` found. It is
`<prefix>` or `<name>.sod`; the recipe is `Sky\<name up to the dot>.ini`, so
`mbgkling.sod` reads `Sky\mbgkling.ini`.

## The draw

Everything the draw touches is inside a `D3DSBT_ALL` state block, captured before and
applied after, so the engine finds the device as it left it. Render targets are not in
a state block; the bake saves and restores them itself.

**Baked once, sampled every frame.** Computing the sky at every pixel cost **6.25 ms
of GPU time a frame at 3440x1440** (Navi 10, measured with timestamp queries). The sky
does not change while a map runs, so the plugin bakes it into a cube texture when the
map's sky (or its recipe file) changes. It renders each face with `bake_ps` into a
render target, reads it back, and copies it into a **managed-pool** cube, which
survives a device `Reset` with no hook on it. Each frame `draw_ps` samples the cube in
the pixel's direction and dithers. `Face=0` keeps the per-pixel path (`sky_ps`) as the
reference.

**No seam from the bake either.** The cube is sampled with or without DXVK's seamless
cube filtering (`d3d9.seamlessCubes`, which the renderer layer sets but a player may
not have). So texel *i* of an S-texel face holds the direction at face coordinate
2*i*/(S−1) − 1: the edge texels hold the edge itself, and neighbouring faces' edge
texels hold the same directions. The step across an edge is then nothing, whichever
filtering is on. The bake reads its texel position from `VPOS`, so Direct3D 9's
half-pixel convention cannot shift it. 10-bit faces (`A2B10G10R10`, falling back to
8-bit) keep the dim gradients of a sky like Aqua's from banding.

Measured on the bench against the per-pixel reference, same camera: aimed at a cube
edge, the largest difference anywhere was **2/255** (mean 0.07, the dither); aimed at a
corner, **2.4/255** (mean 0.08). Neither difference image shows a line.

## The look

`sky.hlsl`, `sky_colour()`:

- **Value noise** on the integer lattice with a quintic fade (C2, so no creases), from a
  lattice hash of our own with no sine in it. Over a 512x512 plane of lattice points
  it measured mean 0.4995, variance 0.0833 (a uniform's is 1/12), neighbour
  correlation 0.0007, and no visible pattern.
- **Fractal sums** with each octave turned by a fixed rotation, so the lattice's axes
  never line up.
- **Domain warp**: three soft three-octave fields displace the lookup of the
  six-octave main field. That turns blobs into billows. A strong warp drags thin
  marbled streaks through the gas, the "electric" look the user rejected, so the
  recipes keep it low (0.35–0.4).
- **Units of its own spread.** The warped sum has a standard deviation near 0.125
  about 0.5, so `n = (f − 0.5) × 8` is about a z-score. `Coverage`, `Softness` and a
  core's gather are in those units, whatever the other settings. A wide `Softness`
  (2–3) keeps the density tracking the field inside the gas, which is what reads as
  soft cloud. A narrow one gives flat masses with hard edges.
- **Colour by density**: empty space (`Deep`), up to the gas's hue by density 0.6, and
  `Glow` added over the densest part. The gas hue mixes `GasA` and `GasB` by a warp
  channel (large soft regions of each) and, by `HueMix`, by the field itself.
- **Cores**: up to two soft lobes about a direction, which gather gas and glow.
  Small and strong (under ~20°, gather over ~1) they saturate into a flat disc; keep
  them wide and gentle.
- **Band**: gas gathered about a great circle, for the skies that run in a band
  (Ba'ku's orange band along the horizon, Dominion's pink one).
- **Stretch** along an axis, for aurora-like curtains, and **Ridge**, a wide bell about
  the field's middle that makes soft bands. Squared, not creased, so it stays gas.

## Recipes

`sky/skies/<name>.ini`, section `[Sky]`. Colours are 0–255, directions are yaw
(degrees about +y; 0 looks along +z, 90 along +x, as `Scene.asi`'s `orbit`) and pitch
(up from the horizon).

| Key | Default | |
|---|---|---|
| `Seed` | 1 | moves the noise domain: a different sky with the same character |
| `Scale` | 2 | the largest structure's frequency over the sphere. 2–3 gives a few billows per cube face |
| `Warp` | 0.4 | domain warp. Over ~0.5 drags streaks |
| `Detail` | 0.5 | amplitude ratio between octaves: lower is softer |
| `Coverage` | 1.0 | where the gas begins, in units of the field's spread: higher is emptier |
| `Softness` | 2.0 | the width of the gas's edge in the same units. 2–3 is cloud |
| `Patchiness` | 0.5 | a one-octave field at a third of `Scale` that gathers the gas in parts of the sky |
| `Ridge` | 0 | 0–1: towards soft bands |
| `Gamma` | 1 | on the density: above 1 thins the faint gas |
| `HueScale` | 2 | how sharply the two gas hues separate |
| `HueMix` | 0.5 | 0: hue by region; 1: hue by density |
| `Brightness` | 1 | the whole sky |
| `Dither` | 1 | the per-pixel dither, in steps of the 8-bit back buffer |
| `Deep`, `GasA`, `GasB`, `Glow` | 0 | r, g, b |
| `Core1`, `Core2` | 0,0,10,0 | yaw, pitch, size (degrees), gather (field units) |
| `Core1Colour`, `Core2Colour` | 0 | r, g, b of the core's own glow |
| `Band` | 0,90,20,0 | yaw, pitch of the band's pole (0,90: the band is the horizon); half-width (degrees); gather (field units; 0 = no band) |
| `Stretch` | 0,90,1 | yaw, pitch of the axis; how far structure is drawn out along it (1 = none) |

`Sky.ini` (`[Sky]`): `Enable` (default 1), `Face` (1536, the cube's edge; 0 = per
pixel), `Reload` (1: `Sky.ini` and the recipe on screen are re-read within half a
second of an edit, and a changed recipe re-bakes), `Timing` (0; 1 logs the frame time
and the sky's GPU time every 600 frames), `Log`.

**Tuning a recipe.** Edit the installed `Sky\<name>.ini` with the game running; the
sky re-bakes within half a second. The recipes aim at the stock set's channel means,
written into each file's header as numbers. Each `Brightness` was scaled so that the
mean over six cube faces, in an offline mirror of `sky_colour()`, lands at about 0.92 of
stock's luminance: a little under, never over. Where the gas saturates into flat
plateaus (dense skies like Borg's), a wider `Softness` (3) restores the depth. The previews behind the shipped recipes
measured Klingon 75/23/5 against stock's 66/17/5, and Aqua 3/14/15 against 4/12/12.
Aqua reads dimmer and softer than the stock faces in game, and the user preferred it
that way (2026-10-09), so don't lift it to match stock's contrast.

## Cost

GPU time of the sky's draw, timestamp queries, Navi 10, `Timing=1`:

| | 3440x1440 | 3840x2160 |
|---|---|---|
| per pixel (`Face=0`) | 6.25 ms | 6.7 ms |
| baked cube (`Face=1536`), vsync on (GPU clocks down) | 0.18 ms | 0.29 ms |
| baked cube, vsync off | | 0.12 ms |

The bake costs 35–280 ms once, when a map's sky first draws. **Frame time at 4K with
vsync off**, alternating procedural and stock in one session (600-frame means):
1.31–1.34 ms with the procedural sky, 1.24–1.26 ms with the stock cube, so about
+0.07 ms a frame. The cube holds 6 × 1536² texels at 4 bytes, 57 MB of video memory.
The noise's finest octave spans about 9 texels a cycle at 1536, so a larger cube adds
memory and no detail.

## Not covered yet

- **Which sky a map shows.** On the bench, `-nointro a2_kling09` and
  `-nointro mp08colplan` both drew `mbgaqu`, though their `.bzn` fields name
  `mbgkling.sod`. A script can change the background at run time
  (`Starfield_Load_New_Background_Geometry`), or the bench may not load the map named.
  Not looked into. The Klingon recipe was judged on the bench by loading it under
  `mbgaqu`'s name, so it has not been compared with stock Klingon on the same view.
- **Recipes seen only on the bench.** Every recipe was drawn on the bench under
  `mbgaqu`'s name (2026-10-09), not on its own maps, and not compared with stock on the
  same view.
- **Parallax layers and 3D nebulae** under the map. If built, they stay render-only in
  this plugin, never map entities, so multiplayer cannot desync.
- **A `Reset` mid-map** is covered by the managed cube in principle; not exercised on
  the bench.
