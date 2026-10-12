# fog — `Fog.asi`

In the 3D view, stock draws the fog of war and the shroud with edges that follow the
scanner's grid cell by cell: the shroud's edge is a hard staircase, the fog's a blocky
gradient. `Fog.asi` draws the same two layers, from the same data, with smooth edges: a
circle a ship's sensors clear reads as a circle, and the edge stays clean at every
zoom, however large a cell is on screen.

It changes how the layers are drawn and nothing else. What is hidden is decided by the
engine's scanner as in stock, and nothing the simulation reads is touched, so a player
with it can play a player without it.

```
fog/install.sh            build, then install Fog.asi and Fog.ini
fog/install.sh --remove   take them out again
```

`./install` runs it; `./a2mod` switches it as the `fog` layer. It needs nothing from
`A2_DATA`: it draws the engine's own textures. Without a Direct3D 9 device behind d3d8
(DXVK's own d3d8, `platform/D3D9.md`) or with `Enable=0`, the game draws its own fog.

## How stock draws the fog and the shroud

Addresses from `armada2.map`, GOG patch 1.1. Established by reading `Armada2.exe` for
the hooks below, and on the bench.

- **The data is the scanner's grids.** `Scanner::m_fog` and `Scanner::m_shroud`
  (0x738a54, 0x738a58) are `ScanGrid`s: one 32-bit mask per cell, the width at +0xc and
  the height at +0x10.
- **Each grid becomes a 128x128 texture, one texel per cell.**
  `RadarComponent::InitializeDynamicSprites` makes the minimap's layers, among them
  `fog` and `shroud`, each with a 128x128 texture named `fogTexture` / `shroudTexture`.
  `RadarComponent::UpdateScanGridSprite` fills one from its grid: for each cell, the
  texel at the same place in the layer's art (`Gfog.tga`, a dark cloud; `Gshroud.tga`,
  flat blue-grey) where the cell is hidden, and zero, alpha 0, where it is seen. The grid
  is centred in the texture, at ((128 - w) / 2, (128 - h) / 2).
- **The 3D view draws those textures over the map.** `Terrain_Geometry::Render`
  (0x593200) draws one mesh over the whole map, first with `fogTexture` and the texture
  material `fog` (the call to `ST3D_Node::Render` at 0x5933f5), then with
  `shroudTexture` and `alphathreshold` (the call at 0x593442). Each is one
  `DrawIndexedPrimitive`, made inside that call.
- **The states, measured on the bench** (`SCENE=fog`, crosire's d3d8to9): FVF
  `XYZ|DIFFUSE|TEX1`; stage 0 modulates the texture by the diffuse colour, colour and
  alpha; the texture `A8R8G8B8`, managed, filtered bilinearly, wrapping. The fog blends
  `SRCALPHA`/`INVSRCALPHA` with no alpha test. The shroud is opaque (`ONE`/`ZERO`) with
  an **alpha test at 128**, `GREATEREQUAL`.

So the fog's edge is a bilinear ramp between texel centres with a corner at every cell,
and the shroud's is that ramp cut at one half by the alpha test: a contour that runs
straight across each cell and turns at its corners. Cells are large: on `a2_borg01` the
grid is 128 cells across the map, and a cell covers tens of pixels at the RTS camera's
usual height. That is the staircase.

A bigger texture is not the answer. The texture is the grid, so there is nothing finer
to put in it, and changing the size of the fog textures froze the game when it was
tried (`textures/REMASTERING.md`, `UImid`).

## What the plugin does

Two hooks, both chained (the plugin calls whatever the call or slot went to before):

| Hook | What |
|---|---|
| the two calls to `ST3D_Node::Render` in `Terrain_Geometry::Render` (0x5933f5 fog, 0x593442 shroud) | note which layer is drawing |
| the d3d8 device's `DrawIndexedPrimitive` (slot 71), as `Lighting.asi` chains it | the one draw inside each of those calls goes through the shader; every other draw passes straight on |

`looks_right()` checks that both sites are calls and that each loads its layer's texture
from `Terrain_Geometry` (+0x34 the fog's, +0x38 the shroud's) just before, so another
`Armada2.exe` leaves the plugin inert, and `Fog.log` says so.

For the layer's draw the plugin binds a `ps_2_a` pixel shader (`fog.hlsl`) and leaves
everything else to the engine: its quad, its texture, its tint and opacity (the diffuse
colour the fixed-function vertex stage computes, which the shader multiplies in as
stage 0 did). The shader:

1. **Reconstructs the mask with a cubic filter** through the hardware's bilinear taps:
   the cubic B-spline in four taps (default), or Catmull-Rom in nine. The texture holds
   the art times the mask, and the mask in alpha, so one filtered sample gives both the
   smoothed mask *A* and the art weighted by it; their ratio is the art over hidden cells
   only, so the edge does not darken towards black.
2. **Draws the edge as a smoothstep of A** about `Threshold` (0.5 is where stock's edge
   is), `Softness` wide, and never narrower than `Antialias` pixels (`fwidth`), so it
   neither steps nor aliases when zoomed out.
3. **Holds every tap inside the grid**, so the shroud does not thin along the map's own
   border: outside the grid the texture holds nothing the engine wrote.

The shroud is drawn blended (`SRCALPHA`/`INVSRCALPHA`, alpha test off) instead of
alpha-tested, so its edge can be soft; inside, it is as opaque as stock. Every state
the plugin changes is put back after the draw.

**Why B-spline is the default.** On the bench (`SCENE=fog`, 1920x1080) Catmull-Rom,
which passes through every texel, keeps a scallop at every cell along a curved edge;
the B-spline's contour reads as a smooth curve. The B-spline's price is that a patch
or a gap one cell wide closes up; a scanner's circles are many cells across, so that
does not come up in play. What remains of the grid in the B-spline is a gentle
waviness at cell scale, visible only magnified.

## Settings: `Fog.ini`

| Key | Default | |
|---|---|---|
| `Enable` | 1 | 0: the stock fog and shroud |
| `Log` | 1 | `Fog.log` beside the game |
| `Reload` | 1 | re-read the file when it changes while the game runs (checked every 30 frames) |
| `Fog`, `Shroud` | 1 | 0 leaves that layer stock |
| `Filter` | `bspline` | `bspline` or `catmull-rom` (above) |
| `FogThreshold`, `ShroudThreshold` | 0.5 | where the edge sits, from seen (0) to hidden (1); lower grows the layer |
| `FogSoftness`, `ShroudSoftness` | 0.2, 0.1 | how soft the edge is, in units of the mask: 0 as sharp as a pixel allows, 0.5 a gradient about two cells wide |
| `Antialias` | 1.0 | the least width of an edge, in multiples of the mask's change per pixel |

## Not covered

- **The minimap** draws the same layers as 2D sprites, one texel per cell at about a
  pixel or two each; it is left stock.
- **`Terrain_Geometry::Render`'s other shroud**: with `View_Record` 1 and a user option
  set it draws a `TreeGrid` (`CreateQuadGrid`, "Shroud") instead of the shroud's quad.
  That path was not seen on the bench and is left stock.

## On the bench

`./a2test run fog-edges --install .` (`testbench/scenarios/fog-edges.md`) checks the
hooks and both layers' first smooth draw, and shoots the edges near and from far off.
By hand, `SCENE=fog` (`testbench/scene/scenes/fog.ini`), then edit the clone's `Fog.ini`
(`Enable=0` for stock at the same frame; `Reload=1` picks it up).

Seen on the bench 2026-10-12 at 1920x1080 against stock in the same scene: the shroud's
staircase and the fog's blocky steps become smooth curves; `Enable=0` gives back stock's
edges in the running game; from 7000 units up, the map's border stays straight and the
explored area's edge antialiased.
