# nebulae — `Nebulae.asi`

A stock nebula is six to eight flat billboards (its SOD's sprite nodes,
`Sprites/nebula.spr`) that turn with the camera and overlap as cards. `Nebulae.asi`
draws a nebula class's gas instead as a volume: slices perpendicular to the view through
the box the class's nebulae fill, depth-tested against the scene, the density computed
from where the nebulae are and a tiling 3D noise. It changes what is drawn and nothing
else: where a nebula is, what it does to ships, the minimap and the fog of war stay the
engine's.

A class is drawn as gas when it has a recipe, `Nebulae\<odf>.ini` in the game
directory (`recipes/` here), and a Direct3D 9 device is behind d3d8 (the d3d8to9 chain,
`platform/README.md`). Anything else keeps the stock billboards.

```
nebulae/install.sh            build, then install Nebulae.asi, Nebulae.ini and the recipes
nebulae/install.sh --timing   ... and log the gas's GPU time (Timing=1)
nebulae/install.sh --remove   take it all out again
```

`./install` runs it; `./a2mod` switches it as the `nebulae` layer. It is not in the
release package (`publish/`) yet.

## The hooks

Addresses from `armada2.map`, GOG patch 1.1. Every site is checked before it is
written; a different `Armada2.exe` leaves the plugin inert and says so in `Nebulae.log`.

- **`Nebula::Render` (0x4a4a00), slot 11 of the Nebula vtable (0x6b148c).** For a class
  with a recipe it returns at once, so the billboards are not drawn. It still marks the
  nebula on screen, which `Lighting.asi`'s nebula lights depend on.
- **Slot 11 of the LatinumNebula vtable (0x6b132c), `GameObject::Render` (0x4d54e0).**
  Latinum is not a `Nebula` but a `TerrainObject` drawn as a model (`latinum.SOD` and
  its `Mlatinum` sprites). Only that vtable's slot is patched, so no other object's
  `Render` changes.
- **The call to `ST3D_GraphicsEngine::RenderParticleList` (0x62d420) at 0x598455 in
  `Armada_RenderAllOurStuff`**: the last draw before the device's `Flush`. Every opaque
  hull is in the depth buffer by then. The plugin calls it, then draws the gas.
- **Ghosts.** A nebula the fog has covered since it was seen is drawn by the engine as
  a ghost: `Ghost::ghostList` (0x736c90) holds a Spirit of it (the object's handle at
  +0x8, a copy of its instance at +0x10), drawn through slot 3 of its vtable
  (`GameObjectSpirit::Render`, 0x4d5f70; vtable 0x6b1ca4). The plugin keeps a ghost's
  gas and skips its billboards.
- **Device `Reset`** (slot 14 of the d3d8 device, chained like `Lighting.asi`'s):
  everything in the default pool is released first.

What it reads: `Nebula::nebulaList` (0x73b1bc), a `std::vector<Nebula *>`; a nebula's
position (+0xac), its class (+0x40) and the class's effect radius (+0x1e0); the ODF
name through `GameObject::GetOdfName` (0x4d5620); and the fog through
`GameObject::CanUserSee` (slot 2), as `Nebula::sCullOccludedNebula` asks it. Latinum
nebulae are in no list of their own: `GameObject::objectList` (0x761084, a pointer to a
`std::list<GameObject *>`) holds every object, and they are the ones with the
LatinumNebula vtable. Their class has no effect radius, so the recipe's `Radius=` sets
the size (100 if absent). A map with no nebula has no `nebulaList`; latinum is still
gathered. The camera is the device's own `VIEW` and `PROJECTION`.

## The gas

Per class, per field of its nebulae:

- **The envelope**: a top-down texture over the field. Alpha, each nebula a soft disc
  out to its reach (`Extent` times the radius); red, how high the dome rises there;
  green, the filaments' wider disc; blue, the height of the nebulae there. One height for
  the whole class put every nebula at the mean of their heights: a latinum nebula raised
  above the plane drew its gas hundreds of units from the object. Now each dome stands
  at its own nebula's height, blended where discs overlap, and the volume's box grows by
  the spread.
- **The density**: a domain-warped fractal noise thresholded by the envelope's cover
  (`Coverage`, `Softness`, `Gamma`), with domes whose lids are lumped by the warp, a
  self-shading step towards a light (`Light`, `Shading`), knots and lanes, optional
  filaments, a core and its ring.
- **The bake**: the gas computed once per field into a volume texture (`VolumeSize`,
  `VolumeHeight`), one layer per draw, when the field's nebulae change. A per-column
  record of where gas is lets a slice pixel over empty space stop after one read. Each
  texel's point is jittered within it: the noise's mip level is about a texel wide, so
  its grid and the bake's beat against each other and drew parallel lines through the
  gas.
- **The draw**: slices on a geometric ladder fixed in distance from the eye (rung j
  between 2^(j/m) and 2^((j+1)/m)). Spread over the box instead, every slice slid
  through the gas as the camera turned, and the gas shimmered in camera glides. The
  rungs per doubling (m) are chosen from the box's distances, with hysteresis, **per
  view**: the HUD's action camera draws the scene, and the gas, a second time each
  frame, and two views sharing one ladder re-chose it every frame. Slices round to the
  8-bit target against a blue-noise threshold that stays put from frame to frame (moved
  per frame, the gas flickered in blotches).
- **Flow**: the volume read through a drifting warp; the drift wraps at 100, where its
  three axes are all whole tiles (at 1 the whole cloud rearranged itself every
  1 / `FlowSpeed` seconds).

### The noise has no lattice

The noise volume (`NoiseSize`, 128³) is built by spectral synthesis: random phases over
a 1/f-like spectrum, through an inverse FFT, so it tiles and has no grid. It was value
noise, random values at the corners of a grid, which varies eight times as much at a
corner as at a cell's centre, with every octave sharing the coarsest grid's planes. The
coverage threshold turned those planes into walls: horizontal and vertical streaks
through every field, a starburst seen from above (the cerulean worst, its tile the
smallest). The shader also reads the tile turned off the world's axes, twice, at scales
that do not divide each other: a single read repeats with the tile, and through a long
path a repeat adds up along its lattice's directions into sheets.

### As bright from every angle

Gas that only adds light is as bright as its path through it is long: a flat field seen
edge-on was up to 1.8 times as bright as from above (tachyon, 133 against 74 on the
bench). `EdgeOnDim` dimmed by the camera's pitch, one factor for the whole frame. Gas
that also absorbs levels off, and that is `Opacity=`: the half-size pass adds up the
optical depth t beside the light C, and the composite draws C (1 − e^−t) / t, which is C
where the gas is thin and the gas's own colour where it is thick. Exact for gas of one
colour, and independent of order, so the slices need no sorting. What is behind loses
e^−t of itself, times `Obscure=`. With it, gas brightness over five pitches from top-down
to edge-on stays within 76–79 (cerulean, `Opacity=3`) and 74–79 (metreon, `Opacity=2`).
Where a class has `Opacity=`, the half-size path ignores `EdgeOnDim`. Latinum has none:
it is a ball, and its core should stay hot.

## Half size (`Resolution=2`)

Slicing at full size costs most where it is needed least. At half size it is a quarter
of that. The slices need the scene's depth to stop at hulls, and the game's depth buffer
is not a texture: the RESZ convention (a bound INTZ texture and a magic `POINTSIZE`,
which DXVK honours) copies it into one. The half-size depth is the nearest of each 2x2,
as view depth in half floats. The gas accumulates in floating point (rgb the light, alpha
the optical depth). The composite takes a 5x5 tent: where the inner 3x3's depths match
the pixel's own (nearly everywhere), as 9 bilinear reads; at a depth edge, all 25 taps
weighted by depth, so a hull's edge stays sharp and gas never shows through it. It is
scissored to the screen rectangle of the gas boxes drawn.

The composite is bound by its reads: a depth read for each of 25 taps cost 0.57 ms
more at 3440x1440, more than all the opacity work. A 3x3 tent (the first) was grainy
from outside the gas.

## Cost

Bench, 3440x1440, the six-class scene at its default view: 1.52 ms of GPU
time for the gas (`Timing=1`). Latinum alone from RTS distance 0.41 ms, close up about
1.0 ms. The noise volume takes about 260 ms to build at start; a field's bake about
100 ms when its nebulae change.

## Recipes

`recipes/<odf>.ini`, section `[Nebula]`; `Reload=1` re-reads them within half a second
of an edit. Shape: `Extent`, `Height`, `Lumps`, `EdgeWarp`, `Radius`. Noise: `Seed`,
`Tile`, `Warp`, `WarpScale`, `Detail`, `FineDetail`, `Coverage`, `Softness`, `Gamma`,
`Ridge`. Colour: `Brightness`, `GasA`, `GasB`, `Glow`, `HueScale`, `HueMix`,
`HueCycle`, `HueSwing`. Light: `Light`, `ShadeStep`, `Shading`, `Knots`, `KnotScale`,
`Lanes`, `LaneScale`. Motion: `Flow`, `FlowScale`, `FlowSpeed`. Filaments: `Filaments`,
`FilamentScale`, `FilamentSharpness`, `FilamentReach`. Viewing: `Opacity`, `EdgeOnDim`,
`NearFade`. Lightning: `Lightning` (flashes a minute, a nebula), `LightningSize`,
`LightningBrightness`, `LightningColour`. The core: `Core`, `CoreSharpness`, `Pulse`,
`PulseSpeed`, `PulseWander`, and its ring of colours `Ring`, `RingAt`, `RingWidth`,
`RingScale`, `RingPatch`, `RingSoftness`, `RingSpread`, `RingCycle`, `RingA`..`RingD`.
Each recipe's header says what it is aiming for.

## Not covered yet

- The release package (`publish/`, the GUI installer) does not carry the layer.
- Lightning lights the gas, not ships.
- Full size (`Resolution=1`) has no `Opacity=`; it keeps `EdgeOnDim`.
