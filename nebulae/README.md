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
- **The two calls to `GameObjectClass_Clean` (0x4cd4f0)**, at 0x485c06 in
  `Program::GameClosePostDebriefing` and at 0x4873ea in `Program::GameDoOne`. It frees
  every object class, the nebula classes among them, when a game ends and before the
  next is set up. The plugin keeps what it knows of a class (its recipe, envelope and
  baked volume) by the engine's class pointer, so it lets all of it go there first, then
  calls through. Kept, the next game's classes came back at recycled addresses and took
  over the old entries (another class's recipe and radius), each stale entry held its
  textures, and after sixteen classes every nebula fell back to billboards.

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
  A bake (or an envelope) that cannot be made leaves that class on its stock billboards
  until its recipe, `Nebulae.ini` or the device changes; so does the noise for every
  class. Neither is retried every frame: the noise alone takes a quarter of a second.
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
  Two reads of the warp, turned apart and drifting opposite ways, summed: with one, the
  displacement field only slid past, and the gas wobbled in place while keeping its
  shape; two change shape as they pass each other. The sum is clamped to one `Flow`, the
  reach the occupancy record is widened by.
- **Soft edges** (`Fringe=`, `Fuzz=`). Without them a field with no filaments stopped at
  its outline: the Radioactive and the Metreon read as compact bodies with hard cutoffs
  against space, where stock's billboards fade out ragged and the Mutara's filaments,
  reaching 1.8 times past the body, gave it a halo. `Fringe=` bakes a thin haze about the
  body, out to `FringeReach` times the reach (the envelope's green disc, the filaments'
  when wider), from a soft noise that is not thresholded as the body's is, fading with the
  disc and the height. `Fuzz=` acts at the draw: the fine octave's two reads drift
  opposite ways (`FuzzSpeed=`, noise tiles a second), and where the gas is thin (below
  0.45) it raises or eats it by up to `Fuzz`, so the outline is wispy and its wisps come
  and go. Bench, 1920x1080, the Radioactive at RTS distance: +0.1 ms of gas
  (1.36 → 1.47 ms); the frame changes by a mean of 8.6/255 in 30 s, the old flow 3.4 in
  20 s. `Bake=0` (the reference) has the fringe but not the fuzz.

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

## Hull edges (`EdgeRefine=1`)

Half size drew every hull in or in front of the gas with a stepped border: background
gas leaked one to three pixels onto the hull's edge in two-pixel steps, and any sharp
change of the gas along a silhouette (a ship's shadow, "Ships' shadows in the gas") came
out on the half-size grid. The half-size depth is the nearest of each 2x2, so a half
pixel straddling a silhouette carries only the gas in front of the hull, and the
composite's tent took the outer ring's depth from its inner neighbour.

**`EdgeRefine=1` draws the gas a second time at full size, but only at depth edges.**
Half size everywhere else, so the cost follows the length of the silhouettes, not the
screen:

- **Where:** two passes at quarter size take how far the half-size depths spread over the
  7x7 half pixels about each 2x2 cell (the tent's 5x5 and one more). Above a spread of
  0.06 (fully at 0.15) the composite takes the full-size gas, filtered over the 3x3 full
  pixels at its own depth (within 1%), since a full-size pixel alone is grainier than the
  tent. The window has the extra ring because the half pixel beside a straddling one can
  be wrong too: along one cube's edge the raw half-size gas was about 40% too bright
  (152 against 117) although its depth was right, and `Shadows` and `Obscure` made no
  difference; why is not known. With the tent's 5x5 alone, a step was left at the band's
  edge.
- **The mask:** those pixels and their 3x3 neighbours are opened in a D24X8 buffer of
  our own (depth from the viewport, MinZ = MaxZ, not from the shader), and the slices are
  drawn again into a full-size A16B16G16R16F target, tested against the INTZ copy in the
  shader as at half size.
- **Only over the 16x16 tiles that hold an edge.** Any draw over a tile spoils its fast
  clear, and then hierarchical Z no longer rejects whole slices there: a full-screen
  mask that discarded every pixel cost 0.2 ms with nothing refined (binding the INTZ
  texture as the depth-stencil and stenciling, or writing depth from the shader, the
  same). One pass at a pixel per tile flags the tiles; a fixed grid of tile quads is
  culled in the vertex shader by a vertex texture read (vs_3_0 `texldl`, which vkd3d
  compiles).
- `EdgeRefine=2` draws everything at full size this way: the reference the band was
  measured against. `Timing=1` logs the pixels refined a view.

Measured (bench, 3440x1440, vsync off, mnebula7, 8 and 9 loaded, two Borg cubes and two
Klingon battleships in the middle one, the Key behind the camera; gas GPU time, `Timing=1`):

| View | Half size | `EdgeRefine=1` | `EdgeRefine=2` |
|---|---|---|---|
| RTS distance | 3.92 ms (202 fps) | 4.41 ms (186 fps) | 17.9 ms (53 fps) |
| Close-up, the gas fills the screen | 5.15 ms (154 fps) | 5.88 ms (140 fps) | 23.6 ms (40 fps) |
| Wide, three classes | 2.91 ms (258 fps) | 3.22 ms (241 fps) | 13.0 ms (72 fps) |

Against `EdgeRefine=2`, blurred by 1.5 px so grain does not count, the pixels about the
hulls off by more than 4/255 fall from 0.5–0.8% to 0.04–0.11%. At 600% the refined band
is a little more mottled than the tent beside it.

### Other ways tried

Each was built, behind its own key, and measured in the same scene and session. Their
code is on the branch `worktree-nebula-hull-edges`.

- **The whole gas at full size** (`Resolution=1` taking this float path at a scale of
  1, a 9-pixel tent so the grain is as fine as half size's). Every artefact of half size
  goes, but the gas costs 3.5 to 3.6 times as much: 14.2, 18.6 and 10.2 ms in the three
  views, **50 fps** in the close-up. The reads of the baked gas and their maths are most of
  it (with every slice pixel returning a constant after its depth test, 4.0 ms); skipping
  the depth read saved nothing. Every other slice in alternate 2x2 blocks (by a stencil)
  took 8.1–10.1 ms but brightened the gas and drew a dotted line along hulls.
- **A better upsampling** (`Upsample=`): the edge test read through a filtered half-size
  depth at the tent's 9 points (the inner 3x3 alone missed the outer ring, whose gas
  leaked across silhouettes), every tap weighed by its own depth, and on a full-size
  silhouette the gas for the nearest and the farthest depth about it mixed by coverage,
  so the gas's edge ramps over a pixel as the multisampled hull's does. +0.01–0.03 ms. It
  removes the leak but not the steps of a shadow's edge. A checkerboard of nearest and
  farthest half-size depths, as half-size fog and SSAO use, added nothing once every tap
  had its own depth.
- **Skipping thin gas** (`ThinGas=0.05`): below that baked density no fine octave and no
  shadow read. 0.3–0.5 ms less at half size, no visible change (blurred difference at
  most 1.2/255).
- **Shadows that fill in** (`ShadowFade=`): "Ships' shadows in the gas", "Seen down a
  shaft".

## Lit by the Key

The gas's light is two parts: its own glow, and the Key's, which the gas scatters. The
Key's share (`KeyShare=`, 0.8; 0 is the gas as before, all its own light) is what a
ship's shadow takes away, and the reason there are shadows to see at all. While the gas
was all its own light, a shadow could only dim it by a fixed fraction (the first try,
`ShadowStrength=0.75`): seen side on, a pixel's line of sight crosses a shaft only for
a ship's width out of hundreds of units of gas, and the shafts showed only in a
difference image, amplified.

**The Key's share is scattered forward** (Henyey-Greenstein, `Scatter=`, 0.2), taken
over its value side on to the Key so that a side view keeps the recipe's brightness:
looking towards the Key the gas is brighter (x1.9 at 0.2), with the Key behind the eye
dimmer (x0.6). That is the silver lining a cloud has against the sun, and it is where
shafts show: looking towards the Key, the lines of sight run down the shafts, not across
them. At 0.3 the whole field was 2.3 times as bright looking towards the Key; that was
too much. **The self-shading steps towards the Key** (the recipe's `Light=` is used only
without one), so the billows' lit sides face the way the hulls' do; a new Key re-bakes
the gas.

The Key's direction comes from `Lighting.asi`'s export `LightingKey`, read off the
engine's lights at each draw, so it is there with no hull on screen and with its
`Shadows=0`. Without `Lighting.asi`, the gas is its own light only.

The other way round, the light a nebula casts on ships is `Lighting.asi`'s, and takes
its hue from the recipe through this plugin's export `NebulaLightColour`: the mean of
`GasA`, `GasB` and `Glow`, brightest channel 1, while the class is drawn as gas, and 0
otherwise (`lighting/README.md`, "Nebulae").

Measured on the bench (2026-10-12, 1920x1080, `mnebula8` with the ships below,
`FlowSpeed=0`), the frame's mean against before: side on 54 → 58, towards the Key
49 → 81, the Key behind the camera 55 → 50, from above 57 → 52.

## Ships' shadows in the gas

A ship between the Key and the gas takes the Key's share of the gas's light away behind
it: a dark shaft down through the field along the Key's direction (`Shadows=1`,
`ShadowSoftness=` in `Nebulae.ini`). Lightning is the gas's own light and is not
shadowed.

**The shadow map is `Lighting.asi`'s**, the one its hulls cast for the Key
(`lighting/README.md`, "Shadows"). `Lighting.asi` exports `LightingShadowMap`, which
hands over the texture, its world-to-map columns, its bias and its size only while the
map belongs to the view being drawn and to this frame; `Nebulae.asi` finds it with
`GetModuleHandleA("Lighting.asi")` and `GetProcAddress`, binds it to sampler 8 for the
gas and nothing else, and keeps no reference. Without `Lighting.asi`, with its
`Shadows=0`, or with no hull on screen, there is no map and the gas is unshadowed. The
map covers the hulls on screen, so a hull off screen casts nothing into the gas.

**Each sample of the gas reads the map twice**, as PCSS does with many reads: the first,
anywhere in the widest penumbra possible at the sample's depth, finds a hull and how far
in front of the sample it is; the second, within the penumbra that distance gives
(`ShadowSoftness` per unit), is the answer, lit or not. The light is a disc, as the sun
is, so a shadow is sharp beside the hull and softens down the shaft. Each pixel's answer
is 0 or 1; the slices and the composite's tent add them up into a soft edge. The read
points come from the pixel's blue-noise thresholds, walked from slice to slice, so they
stay put from frame to frame like the slices' own.

**The shadow is taken at both of the gas's points** (`Samples=2`): a quarter of the gap
either side of the jittered point, which itself moves over only half the gap, so the
two together cover all of it. Taken at the jittered point alone (1.1.0 to 1.2.0), half
of every gap went unsampled, and a shaft seen along its length (looking towards the
Key, a Borg assimilator in `mnebula8`) drew as one copy of the hull's silhouette per
slice with gaps between them; in a `Shadows=0` difference image they stood as a stack.
`Samples=1` joined them but was grainier. The cost was within the bench's run-to-run
spread (gas 2.6–2.9 ms against 3.0–3.2 ms, 1920x1080, two sessions), since a read
outside the map's square returns at once.

**The map is cleared to 1, and 1 means no hull**, not a hull at its far end: its hulls
lie well inside 0..1 (it is fitted round them with a margin). Compared as a depth, gas
beyond every hull along the Key was in shadow wherever there was no hull at all, a
wedge with a straight edge through the field (bench, the first try).

Measured on the bench (2026-10-12, 1920x1080, `mnebula8` with a Galaxy, a Sovereign and a
Borg cube in it; the recipe's `FlowSpeed=0` so that only the shadow differs): `Shadows=0`
against `Shadows=1` differ only in shafts from the hulls down and away along the Key.
The frame's mean difference is 2.5 of 255 looking towards the Key, where the shafts are
plain at 1x; 0.36 side on and 0.2 from above, where they run across or away from the
lines of sight.

**Cost** (`Timing=1`, bench, 3440x1440, the same scene, the camera inside the field so
the gas fills the screen; 6.1–6.3 ms of gas before): lit by the Key, +0.04–0.07 ms;
the shadows, +1.25 ms looking towards the Key (+20%) and +0.6 ms from above. That is
two dependent reads of the map per slice pixel with gas in it (none over empty space),
at half size.

**Seen down a shaft, a shadow is a dark ring about the hull.** With the Key behind the
camera, as from the ordinary RTS view, each hull's shaft runs straight away behind it,
and a line of sight just outside the silhouette meets the shaft some way behind the hull
and runs inside it, through the penumbra that widens with depth (`ShadowSoftness`). The
gas there loses the Key's share of its light, 80%: a dark outline hugging every ship. It
is what the shadows should look like from there, and it was half the stepped border hulls
drew in the gas; the other half, its edge on the half-size grid, is "Hull edges". A
shadow that fills in behind its caster, as light the gas about it scatters would fill it
(`ShadowFade=`, world units, built and measured; branch `worktree-nebula-hull-edges`),
weakens the ring: about a Klingon battleship the shadow's mean darkening fell from 3.3
to 1.2 of 255 at 150 and to 0.5 at 60, at no measurable cost. It also shortens the
shafts looking towards the Key, where they are meant to show (at 60 little shadow is
left), so it is not in.

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
`Lanes`, `LaneScale`. Motion: `Flow`, `FlowScale`, `FlowSpeed`. Edges: `Fringe`, `FringeReach`, `FringeScale`, `Fuzz`, `FuzzSpeed`. Filaments: `Filaments`,
`FilamentScale`, `FilamentSharpness`, `FilamentReach`. Viewing: `Opacity`, `EdgeOnDim`, `KeyShare`, `Scatter`,
`NearFade`. Lightning: `Lightning` (flashes a minute, a nebula), `LightningSize`,
`LightningBrightness`, `LightningColour`. The core: `Core`, `CoreSharpness`, `Pulse`,
`PulseSpeed`, `PulseWander`, and its ring of colours `Ring`, `RingAt`, `RingWidth`,
`RingScale`, `RingPatch`, `RingSoftness`, `RingSpread`, `RingCycle`, `RingA`..`RingD`.
Each recipe's header says what it is aiming for.

## Not covered yet

- The release package (`publish/`, the GUI installer) does not carry the layer.
- Lightning lights the gas, not ships.
- Full size (`Resolution=1`) has no `Opacity=`; it keeps `EdgeOnDim`. A full size on the
  float path was built and measured ("Hull edges", "Other ways tried"), and is too slow
  where the gas fills the screen.
- `EdgeRefine=1`'s band is a little grainier than the tent beside it, and the half-size
  gas beside a straddling half pixel can be too bright ("Hull edges"); neither is
  understood yet.
