# Lighting — ships on the GPU, scene lights, planets with a night side, light sources

`Lighting.asi` changes how Armada II lights its ships, stations and planets, in parts
that `Lighting.ini` switches separately:

- **`GPU=1`**: ships and stations are drawn through the engine's own static vertex
  buffers. That is fixed-function Direct3D: the GPU transforms and lights them, where stock
  does both on the CPU.
- **`Lights=1`**: each map's own lights are replaced by two of the plugin's: a warm key
  (`KeyColour`, `KeyAxis`) and a dim blue fill from the opposite side (`FillColour`,
  `FillAxis`). Every renderer lights from that one list, so the CPU, GPU and bump-mapped
  paths agree.
- **`BumpShaders=1`** (with `Shaders=1`): bump-mapped hulls, the Borg and any hull
  `models/hull-bump.py` patched, go through the same shaders, lit by the engine's own
  normal map ("Bump-mapped hulls", below).
- **`Planets=1`**: planets and their cloud shells get a night side. Stock gives their
  material a constant half-white term that lights them all round (below).
- **Light sources** ("Light sources", below): point lights reach the GPU-drawn ships
  (`PointLights`), so torpedoes and pulses light the hulls they pass as they always did
  on the CPU path; nebulae glow in their colour (`Nebulae`); a planet's day side lights
  what is near it in the colour of its ground (`PlanetGlows`); the skybox adds a faint
  third light in its own colour (`SkyLight`); and a ship's or station's explosion
  lights its surroundings (`Explosions`); a phaser lights the firing ship around
  its emitter and the target where it strikes (`Phasers`).
- **`Shadows=1`** (with `Shaders=1`): ships and stations cast the Key's shadow on
  themselves and on each other, from a depth map of the hulls drawn; **`PlanetShadows=1`**:
  a planet shadows what is behind it from the Key ("Shadows", below).

`./install` runs `install.sh`; `install.sh --remove` takes the three files out again
(`Lighting.asi`, `Lighting.ini`, `Lighting.log`). The exe is patched in memory only, and
`a2mod` switches the plugin as the `lighting` layer. Each launch writes `Lighting.log`:
`call sites patched 13` means every hook took, and the lines after it name every model
switched to vertex buffers, the map's own lights, the sky light (`sky:`) and each
planet's ground colour (`planet glow:`).

## Settings

| Key | Default | Meaning |
|---|---|---|
| `GPU` | `1` | static vertex buffers for every game object type's model |
| `Lights` | `1` | replace the map's lights with Key and Fill |
| `KeyColour`, `FillColour` | `1.00 0.96 0.90`, `0.06 0.08 0.18` | linear RGB, 1 = full |
| `KeyAxis`, `FillAxis` | `0.50 -0.50 0.71`, the negation | the light matrix's third axis, in the engine's own convention: the stock key on the first Federation map is `0 -0.707 0.707` and lights the hulls from above |
| `Ambient` | `0.05 0.05 0.07` | light every GPU-drawn surface gets, whatever its direction |
| `Planets` | `1` | planets lit by Key and Fill with a night side (below) |
| `PlanetAmbient` | `0.018 0.018 0.027` (left out: `Ambient`) | the planet material's constant term, added whatever the direction; stock is `0.5 0.5 0.5` |
| `PlanetDiffuse` | `1.00 1.00 1.00` | the planet material's diffuse colour; stock is `0.75 0.75 0.75` |
| `PlanetShaders` | `1` | under d3d8to9, draw planets and their cloud shells in shaders, lit per pixel ("Planets on the GPU", below); with any other d3d8, or `0`, the CPU path as before |
| `PlanetFill` | `0.315` | with `PlanetShaders`, every directional light but the Key, times this, on planets |
| `PlanetSun` | `1.2` | with `PlanetShaders`, the Key on planets times this; the light on the texture is clamped at this rather than 1, so the lit side can be brighter than the CPU path's |
| `PlanetWrap`, `PlanetDusk` | `0.25`, `1.00 0.55 0.35` | how far past the terminator the light wraps, and its colour where it grazes |
| `PlanetHaze`, `PlanetHazeColour`, `PlanetHazePower` | `0.45`, from the ground, `3.0` | the atmosphere seen edge-on at the limb: strength, colour (left out: half the ground's hue, half a sky blue), how closely it hugs the edge |
| `PlanetGlint`, `PlanetGlintPower` | `0.30`, `40` | a highlight off water (ground bluer than red or green); `0` none |
| `CityLights`, `CityLightColour` | `0.8`, `1.00 0.72 0.38` | a developed planet's cities glowing on its night side; `0` none |
| `Shaders` | `1` | under crosire's d3d8to9 (`platform/d3d8-chain.py --use d3d8to9`), light the GPU-drawn hulls per pixel in shaders (below); with any other d3d8, or `0`, per vertex as before |
| `NearFade` | `1` | with `Shaders`, keep a ship the engine fades for filling more than half the view in the shaders, the fade drawn as a screen door ("The near fade", below); `0` leaves it to the CPU path, in stock's lighting |
| `BumpShaders` | `1` | with `Shaders`, draw bump-mapped hulls (the Borg; `hull-bump.py`'s) in the hull shaders, their normal from the normal map, instead of the engine's dot3 passes ("Bump-mapped hulls", below); `0` leaves them to the dot3 passes |
| `SelfIllumination` | `1.8` | with `Shaders`, how strongly a self-illuminating hull's night lights show (below); `1` is stock's second pass, `0` none, above 1 brighter than their texture ("Dynamic range") |
| `HullSun` | `1.1` | with `Shaders`, the Key on GPU-drawn hulls times this, the light no longer clamped at 1 ("Dynamic range") |
| `HighlightKnee` | `0.8` | with `Shaders` or `PlanetShaders`, colour above this rolls off towards white instead of clipping; `1` clips as before ("Dynamic range") |
| `Specular`, `SpecularPower` | `0.7`, `24` | with `Shaders`, a highlight from every light, times the texture's brightness; strength (`0`: none) and exponent (below) |
| `RimLight`, `RimPower` | `0.12 0.14 0.20`, `3.0` | with `Shaders`, light on the faces turned edge-on to the camera; colour (`0 0 0`: none) and how closely it hugs the edge |
| `FixMirrored` | `1` | light meshes that a model mirrors back with its node matrix the right way round (below) |
| `PointLights` | `12` | point lights per GPU draw, the strongest first: up to 16 with `Shaders`, 6 in Direct3D's slots; `0` gives the GPU path none, as stock |
| `Nebulae` | `1` | nebulae light their surroundings in their glow colour |
| `NebulaBrightness`, `NebulaRange` | `2.0`, `8.0` | the glow colour's multiplier; the falloff's (stock 60 + 60 units) |
| `NebulaCull` | `3.0` | a nebula counts as on screen, drawn and lighting, while its bounding sphere times this is in view; `1` is stock |
| `PlanetGlows` | `1` | a planet's day side lights what is near it |
| `PlanetGlow`, `PlanetGlowRange` | `1.5`, `6.0` | Key x ground colour x this, from the planet's centre; full at its surface, gone at this many radii; times the share of the day side facing the object |
| `SkyLight` | `0.35` | the sky light's strongest channel; `0` leaves it dark |
| `Explosions` | `1` | ship and station explosions light their surroundings |
| `ExplosionColour`, `ExplosionBrightness`, `ExplosionRange` | `1.00 0.62 0.28`, `4.0`, `10.0` | the flash's colour and peak; full over the explosion's radius (at least 40 units), gone at this many radii |
| `Phasers` | `1` | a phaser lights the firing ship around its emitter, and the target where it strikes, while the beam is drawn ("Phasers", below) |
| `PhaserBrightness`, `PhaserStart`, `PhaserRange`, `PhaserLift` | `4.5`, `0`, `24`, `3` | the emitter's light: the beam's colour times this (`0` none); full to `PhaserStart` units from the light, gone at `PhaserRange`; it sits `PhaserLift` units out along the beam |
| `PhaserWrap`, `PhaserFalloff` | `0.6`, `2` | the emitter's light in the hull shaders: how far its diffuse term wraps round past the faces turned to it (`0`: Lambert), and the power its falloff is raised to (a hot spot) |
| `PhaserImpact`, `PhaserImpactStart`, `PhaserImpactRange`, `PhaserImpactLift` | `2.0`, `6`, `70`, `8` | the light at the beam's end: the beam's colour times this (`0` none), and its falloff and lift as for the emitter |
| `OrdnanceColours` | `1` | a torpedo's or pulse's light takes the colour of its own sprite, at the ODF's brightness ("Torpedoes and pulses", below); `0` keeps the ODF's `lightColor` |
| `Shadows` | `1` | with `Shaders`, the hulls in the shaders cast the Key's shadow on themselves and each other ("Shadows", below); `0` none |
| `ShadowSize` | `2048` | the shadow map's width and height in texels (512 to 8192); it is fitted round the hulls on screen, so zoomed in a texel is a fraction of a unit |
| `ShadowStrength` | `1.0` | how much of the Key a shadow takes away, from the map and from planets; `1` all of it, so a shadowed face keeps the fill, sky light, ambient and point lights |
| `PlanetShadows` | `1` | with `Shaders`, a planet between a hull and the Key shadows it; `0` none |
| `Log` | `1` | write `Lighting.log` |

The key comes in about 60° off vertical (1.0.0 had about 37°), so from the usual
camera, which looks down from above, the light grazes the hulls and planets instead of
falling straight onto them. The fill, `Ambient` and `PlanetAmbient` are kept low so
that light sources added later stand out against the base lighting. The lighting is
per vertex (per pixel with `Shaders=1` under d3d8to9); only the Key casts shadows
(`Shadows=1`, shaders only). A flat face such as a saucer's top takes one tone
whatever the angle. On the bench (`SCENE=planet`, the view 65° down) the Galaxy's saucer
went from a mean grey of 155 to 111.

Requires *Hardware Vertex Processing* on (Graphics Options; the default). With it off
the engine keeps every mesh on the CPU and only the lights part applies.

## How the engine draws a mesh

Traced with `testbench/d3dtrace` and read from `armada2.map`. `ST3D_Mesh::Update`
(0x631c10) picks one of three renderers from the mesh's flag word: the dot3 bump path
(`ST3D_Dot3_MeshVB`, bit 0x2, which stock gives only the Borg), a plain vertex-buffer
path (`ST3D_Standard_MeshVB`, bit 0x1), or neither, the CPU path. The CPU path lights,
transforms, clips and sorts every frame and hands Direct3D screen-space triangles.

Bit 0x1 is set only by `ST3D_Database::EnableStaticVertexBuffers(bool)` (0x6207a0),
which flags every mesh of a model and rebuilds it. Stock calls it from the
`AsteroidFieldClass` constructor and from the debug command `ToggleAsteroidVertexBuffers`,
so only asteroids ever used the GPU path. `ST3D_Standard_MeshVB::DeviceIsSupported`
(0x63e310) requires the *Hardware Vertex Processing* option and a device with hardware
T&L. `ST3D_Mesh::RenderInternal` (0x6325d0) still sends a draw to the CPU path while a
per-render effect is attached (cloak, warp-in), as stock does for asteroids.

## The hooks

| Where | What | Why |
|---|---|---|
| call at 0x4ccd66 and 0x4ccd7e, in the `GameObjectClass` constructor (once per ODF) | wraps `FindLogicalDatabase` / `FindVisibleDatabase`; every plain `ST3D_Database` returned gets `EnableStaticVertexBuffers(true)` | what the asteroid field does, for every object type. A `Planet_Database` is skipped: it re-tessellates itself every frame |
| call at 0x63e4e4, `ST3D_Standard_MeshVB::Render` | replaces `SetMaterial`: diffuse white, ambient zero, emissive `Ambient=`; and turns `NORMALIZENORMALS` on | below |
| slot 3 of the `ST3D_Standard_MeshVB` vtable (0x6bcbdc), `Render` | reverses the enabled lights for a draw whose object matrix (0x7ad640) is mirrored | below |
| call at 0x597f83, `GameObject_PreRenderAll` | wraps `ST3D_GraphicsEngine::RegisterLight`: the frame's first directional light is replaced by Key and Fill, the map's others are dropped | below |
| call at 0x598193 | wraps `GameObject_PreRenderAll` | counts the frame for the hook above; afterwards adds the planet and explosion lights and recolours the torpedoes' (`Ordnance::PreRenderAll` is the next call, at 0x598199) |
| slot 3 of the `ST3D_Dot3_MeshVB` vtable (0x6bc7d4), `Render` | draws a bump-mapped group once, in shaders | "Bump-mapped hulls", below |
| six `fmuls` in the `GroundMesh` constructor (0x595c22, 0x595c3d, 0x595c57; 0x595c70, 0x595c7f, 0x595c8e) | their operand, the stock 0.75 and 0.5, now points at `PlanetDiffuse` and `PlanetAmbient`, one channel each | "Planets", below |

Each call site is checked (an `E8` to the expected function, the `SetMaterial` call's
bytes, the vtable slot's address) before any is written. A different `Armada2.exe`
logs `NOT PATCHED` and is left alone.

### The material

`Render` (0x63e450) hands Direct3D the SOD's lighting material as it stands: its
diffuse colour, and its ambient as both ambient and emissive. The CPU path
(`LightVertices_Lambert`, 0x646bd0) never tints the texture by that diffuse colour,
and many SODs carry a leftover one. On the bench the GPU path showed it at once: pure
red (1, 0, 0) on the freighter's cockpit, brick (0.745, 0.227, 0.157), a purple seam
along the Galaxy's back. Some materials also have a reddish ambient that turned a whole
hull pink as emissive. So the plugin's material is diffuse white with its own neutral
emissive. Feeding in the engine's colour at +0x28, which the CPU path also adds, was
tried: on the first Federation map it is (0.66, 0.34, 0.34), and every hull went pink.

`NORMALIZENORMALS` is off in stock, which on this path made lighting depend on a
model's scale.

### Mirrored meshes

Some models carry a mesh built mirrored and mirrored back by its node matrix. The
Akira's distant mesh (`Fcruise1.sod`, the 258-face one the engine switches to when
zoomed out) is drawn with its near mesh's matrix with the X axis negated, determinant
−1. On the vertex-buffer path it lit as if its normals pointed the other way: dark and
blue from above, bright from below, flipping at the zoom where the engine changes mesh.
Its normals agree with its winding, and its world matrix is a clean rotation apart
from the mirror; negating that mesh's normals put it right on the bench. The plugin
does the equivalent without touching the mesh: for a draw under a mirrored matrix it
reverses the enabled lights' directions and restores them after (N·−L = −(N·L)).

Two approaches that failed: flipping every mesh whose normals point "inward" from its
centre flipped the correct ones too (by that measure most stock meshes point inward),
and turned the near hulls dark.

### The lights

A map's lights are game objects of class `dlight` that the mission editor placed: 64
of the 72 maps have two, 6 have three, 2 have one, each with its own colours. On the
first Federation map they are a white key and a fairly strong blue fill (0.05, 0.29,
0.53); with the GPU path that blue lit the Akira's top. `GameObject_PreRenderAll`
(0x597f30) registers them every frame, and the CPU path, `PreRender` of both
vertex-buffer classes and the dot3 shader's constants all read the one list. A
registered light is a colour and a `Matrix34`, and a directional light shines along
the matrix's third axis (`ST3D_Standard_MeshVB::PreRender`, 0x63e340, reads floats
6–8). The plugin builds a symmetric matrix (a reflection taking z to the wanted axis),
so the third row and column agree however a reader indexes it.

## Planets

A planet is not an ordinary model. Its visible database is a `Planet_Database`
(`models/README.md`, "Planets.asi"), whose two `GroundMesh` hemispheres and `Atmosphere`
cloud shell the engine re-tessellates as the camera moves. They stay on the CPU path,
and `GPU=` leaves them there. The CPU path does take the scene's lights. On the bench,
a wrapper around the device's per-light-model table (`ST3D_DeviceDirectX8`+0x50, filled
at 0x62314a, called from `ST3D_Mesh::RenderInternal` at 0x632524 with the material's
model at +0x40) logged all three planet meshes lit by `LightVertices_Lambert` (0x646bd0),
with Key and Fill in the engine's list and no override material.

What kept stock planets from showing a night side is their material. The `GroundMesh`
constructor (0x595ba0) builds one per mesh, `ShroudLightingMaterial`, light model 1
(Lambert), from `ST3D_Colour_White`: White x 0.5 as the first colour and White x 0.75 as
the diffuse. Lambert starts every vertex from the engine's ambient (0 on the bench map)
plus that first colour (the workspace's +0x58 once `SetLightingMaterial` has run, logged
as 0.5), adds each light, and clamps to 1. So half white reaches every vertex whatever
its direction: the day side clamps and the night side stays half lit. The Key comes
from behind the usual camera, which hid this further: from the default view a planet
is seen almost straight down its lit hemisphere and shows no terminator at all.

Each channel of both colours is one `fmuls` with the stock constant (0.75 at 0x6ae70c,
0.5 at 0x6ae220) as its operand. The plugin checks the six instructions (`D8 0D` and the
constant's address) and points them at `PlanetDiffuse` and `PlanetAmbient`. White is
1, so the products are those values. The material is built once per planet mesh, so
nothing runs per frame.

On the bench (`testbench/scene`, `SCENE=planet`, 1920x1080, mean grey over the disc,
stock material -> `Planets=1`): from above, on the lit hemisphere, 181 -> 164 and no
longer clamped; from below, the night side, 107 -> 72, blue from the Fill; side on, the
lit and dark halves 154/93 -> 111/64. With Key and Fill both set to black the stock
planet kept a mean of 93, the constant term alone. The user judged the bench result
"more like it".

## Light sources

The engine has point lights. A nebula carries one, a torpedo or pulse whose ODF sets
`lightColor` carries one, and the CPU path lights a mesh with every point light in the
list (`ST3D_Point_Light::LightVerticesLambert`, 0x62f640): full colour out to the
falloff start (+0x100 of the light), then linearly down to nothing at start + range
(+0x104). The vertex-buffer path never sees them. `ST3D_Standard_MeshVB::PreRender`
(0x63e340) copies only the directional lights into Direct3D, those whose type at +0xf0
of the class data is 1. So with `GPU=1` the ships had lost the torpedo lights stock
showed, and nothing a plugin added as a point light would reach them.

### Strength

1.3.0 set every source low: nebula glows at their ODF colour (a half-strength primary),
`PlanetGlow=0.6`, `SkyLight=0.15`, `ExplosionBrightness=2` over 5 radii. In game the
user found nebulae, planets, the sky and explosions all too subtle. Part of that is
structural: Direct3D sums ambient and every light and clamps at 1 before the texture,
and the Key already takes the side it lights most of the way there, so a source shows
mostly on a hull's shadowed side and as a shift in hue on its lit one. 1.3.1 doubles
each source's colour (`SkyLight` a little more, 0.35) and widens each reach: planets to
5 radii, explosions to 10 radii (a frigate's gone at 400 units). Nebulae went to 8x their
ODF falloff (full to 480 units), back to 5x in 1.3.2, when a Radioactive nebula's full
yellow outreached a planet right beside the ship, and to 8x again in 1.3.3 once the planet
glow showed. 1.3.3 also raised the planet glow by a quarter (`PlanetGlow=1.5`) and its
reach to 6 radii. A glow of 2 saturates its
channels on the side facing it; that is the intended look, and the keys take it back down.

### Point lights on the GPU path

The plugin already wraps `ST3D_Standard_MeshVB::Render` (vtable slot 3) for mirrored
meshes. For every other draw it walks the engine's light list (engine 0x7ad508 +0x60, a
`std::list` whose node holds the light at +0, its colour at +4 and its `Matrix34` at
+0x10), keeps the point lights (vtable 0x6bc8ac) that reach the draw's position (the
translation of the current matrix, 0x7ad640), and sets the strongest `PointLights` of
them as Direct3D point lights in the slots `PreRender` left free, switched off again
after the draw. Two kinds are handled differently:

- **A soft light** fades over a distance like its reach (nebula, planet, explosion).
  It is weighed by the engine's falloff at the object's position, which is folded into
  its colour, and given an unlimited range. Across a ship, which is small against such
  a light, that matches the per-vertex falloff.
- **A hard light** fades over less than a quarter of its start: the torpedoes and
  pulses (the Galaxy's photon: full to 50, gone at 55). Weighed at the centre, one lit
  a whole saucer green from its rim, or missed a ship it was touching (bench, Borg
  torpedoes on the Galaxy). It goes to Direct3D at full colour with `Range` = start +
  range, so each vertex is in it or not, as on the CPU path. It is picked when it is
  within that reach plus 150 units (`HARD_REACH`) of the draw.

Lights of one colour count once, by the strongest. A nebula field is many nebula
objects of one type, each with its own light, and summed they would paint a hull in
its colour at full saturation wherever two or three reached it.

### Which side a point light lights

The stock meshes' normals point inward. The engine compensates for its directional
lights: in the device, the light in slot 0 had the direction (0.836, 0.487, 0.251) where
the sky light's axis was (−0.836, −0.487, −0.251), and the world matrix of the draw was
the engine's object matrix, so nothing else stands between the two. A point light has no
direction to negate: Direct3D takes it from the light's position, and with inward
normals it lit the side of a hull turned *away* from the source. In game a Bird of Prey
beside a Metaphasic nebula glowed green on the far wing; on the bench (`SCENE=planet`)
the planet's glow, picked at full strength, moved the ship's planet-facing side by one
level in 255.

A soft light now goes to Direct3D mirrored through the draw's origin (2 x origin −
position). That reverses its direction exactly at the origin and, for a light far away
against the size of a ship, nearly everywhere on it. On the bench, the side of the
Galaxy facing the planet: mean RGB 19/31/40 with `PlanetGlows=0`, 20/32/41 with the glow
unmirrored, 25/39/47 mirrored; the side facing away stays dark against the planet.

A hard light (torpedo, pulse) cannot be mirrored: its sphere would land on the far end
of the ship. On the fixed-function path it still lights the faces turned away from it,
which from above are mostly hidden. With `Shaders=1` it lights the right side: the
shaders take every point light where it is and turn the normal round instead (below).

### Nebulae

Every nebula builds a point light in `Nebula::InitializeGeometry` (at +0x1ac, falloff
300 + 300), and `Nebula::Simulate` registers it through `Simulate_Nebula_Lights`
(0x4a5160, called at 0x4a4a7a) while the nebula is on screen. Stock sets the light to
the class's glow colour (`red_glow` .. at NebulaClass +0x22c) swung by noise, with the
class's falloff (`glow_falloff_start`/`_range`, +0x244/+0x248: 60 and 60 on every stock
nebula). On the bench it came out black frame after frame. The plugin replaces that
call: the glow colour times `NebulaBrightness`, steady, and the falloff times
`NebulaRange`. The glows are a half-strength primary or two: Mutara `(0.5, 0, 0.5)`,
Metreon `(0.5, 0, 0)`, Metaphasic `(0, 1, 0)`, Cerulean `(0, 0, 0.5)`. On the bench
(`SCENE=nebula`) the Galaxy at the Mutara's edge turned violet on the side facing it and
stayed grey on the far side.

### Nebula culling

A nebula is drawn, and registers its light, only while the engine counts it as on
screen. `Nebula::Simulate` calls `Simulate_Nebula_Lights` only while the object's
on-screen flag (+0x25) is set, and `Nebula::Render` draws nothing while the nebula's
own culled flag (+0x1b0) is set. `Nebula::sCullOccludedNebula` (0x4a52c0) sets that
flag on every nebula each frame, then clears it for those that pass the fog check
(`GameObject::CanUserSee`) and the camera test, slot 3 of the `NebulaInstance` vtable
(0x6b1448). That slot is the generic `ST3D_Instance::FrustumTest` (0x62e990): the
instance's bounding sphere (the node's at +0x1c when there is one at +0x80, else its
own at +0x34, radius at +0xc) against the camera frustum. The survivors are then sorted
by distance and thinned where one hides another.

The sphere is smaller than what the nebula does. The Big Mutara's is 312 units, and its
light reaches 960 at `NebulaRange=8`. Zoomed in on a ship 566 units from the Mutara's
centre (bench, `SCENE=nebula`, a second Galaxy spawned at 1800,0,1800), the nebula's
light went from registered to not between a camera distance of 600 and 120, and the
ship turned from violet-edged to plain grey. Its cloud goes the same way.

The plugin puts its own function in that slot. It multiplies the sphere's radius by
`NebulaCull`, calls `FrustumTest`, and puts the radius back. Every caller that goes
through the nebula's vtable gets the margin, and nothing else is touched. At 2 the
light stayed registered at 600, 120 and 70, and the hull kept its violet edge. The
default 3 (935 for the Mutara) covers the light's whole reach. A nebula just outside
the view costs a few draws, which the GPU clips.

### Planets: light from the day side

Each frame, after `GameObject_PreRenderAll`, the plugin walks the same object list
(0x761084) for planets (vtable 0x6b2b3c). For each it registers a point light at its
centre: position from the Entity's transform (+0x44), radius from its bounding sphere
(+0x34), whose centre is in object space. The light is full at the `Planet_Database`
radius the engine itself uses (262 for class M) and gone at `PlanetGlowRange` radii.
`pick_points` scales it per draw by the share of the day side that faces the draw,
½ − ½ (direction from the centre · `KeyAxis`): 1 under the sun, ½ over the terminator,
0 over the night side. Up to 1.3.1 the light sat on the surface under the sun; from
there it came from where the Key does, onto faces the Key already lights to the clamp,
and showed nowhere. Its colour is Key x `PlanetGlow` x the
mean colour of the planet's ground texture, `groundTextureName` (PlanetClass +0x4bc)
with `1` and `2` appended, one file per hemisphere, else the name alone, else
`atmosphereTextureName` (+0x4ac). Only lit texels count, because the class planets'
gore unwrap is black between the lobes. The texture is read from `Textures/RGB` once
per name, so the colour follows whatever art is installed (Class M: `0.38 0.59 0.61`).

The light is built as the nebula builds its own (operator new 0x652710, then
`ST3D_Point_Light`'s constructor 0x62f240), but with a copy of the class's vtable whose
four `LightVertices` slots (13 to 16, the CPU path's) do nothing. It stays in the
engine's list, so `pick_points` hands it to the GPU draws, but the CPU-lit planets and
moons ignore it. The planet itself is among them, and would otherwise be lit from just
above its own surface.

On the bench (`SCENE=planet`, the view from below the saucer, which faces the planet
and is turned away from the Key): mean RGB over the ship 11/15/22 without the glow,
18/27/34 with it.

### The skybox

`Starfield_Load_Background_Geometry` (0x590c30) copies the map's background name,
lower-cased, into a buffer at 0x738538 that holds it for as long as the map runs. Each
frame, before `GameObject_PreRenderAll`, the plugin compares that buffer with the name
it last saw, and on a change reads the faces once. The name is one of two forms
(`textures/README.md`):

- **A prefix**: the faces are `<prefix>0..5.tga`, and `CreateBackgroundFace` (0x590f10)
  builds face *i* of a ±100 cube facing +z, +x, −z, −x, +y, −y for *i* = 0..5.
- **A cube SOD** (`<name>.sod`) that names its textures. The plugin reads every
  length-prefixed string in the file that opens as a TGA.

The colour is the faces' mean with each texel weighted by its chroma (max − min). That
is the hue of the sky's clouds and not the black between them, scaled to a peak of
`SkyLight`. The light comes from the sum of the face directions, weighted the same way.
A SOD sky's faces carry no direction the plugin knows, and a sky can have no side to
speak of; either way the light falls along `FillAxis`. It is a third directional light,
registered with Key and Fill. On the bench map (`mbgaqu`, a prefix sky) it came out
teal `(0.31, 1, 0.98)` x `SkyLight`, from the upper +x side. At `SkyLight=1` the top of
the Galaxy went from 40/45/47 to 81/104/101. 1.3.0 shipped 0.15, of the order of the
fill; 1.3.1 raised it to 0.35 ("Strength", above). The skybox itself does not take scene lights.

### Explosions

A ship or station that dies goes up in a `FireballExplosion` (the `xfireb*` ODFs,
`classLabel = "fireballexplode"`): a model played for `length` seconds (ExplosionClass
+0x4c, 2.5 for all of them), its time left counted down at +0xa8 by
`FireballExplosion::Simulate` (0x465510), which deletes it at zero. Stock gives it no
light. The plugin wraps `Simulate` and the scalar deleting destructor (0x465910),
slots 14 and 0 of the vtable (0x6afbd4), to keep a list of the live ones. Each frame,
after `GameObject_PreRenderAll`, it registers a point light at each: `ExplosionColour`
x `ExplosionBrightness`, up to full in the first 0.15 s, then down with the square of
the time left. The light is full out to the explosion's bounding radius (34 for a
frigate) and gone at `ExplosionRange` radii, no less than 40 units each. It is an
ordinary point light, so the CPU path (planets, cloaking ships) takes it too.

On the bench (a frigate dying 110 units from the Galaxy, paused at 0.3 s): the hull
under it went from 20/32/38 to 30/39/41 mean RGB, orange-brown where it was dark blue.

### Torpedoes and pulses

`Ordnance::PreRenderAll` (call at 0x58e55c) registers each live torpedo's or pulse's
light while the engine's detail level is above 2, and the point-light path hands it to
the GPU draws. A weapon's impact has no explosion object (it is a sprite effect), so
it has no light of its own.

**The ODF colours ignore the projectile.** Every ODF that sets `lightColor` gives its
`OrdnanceClass` one `ST3D_Point_Light` (class +0x10, built in the class constructor
as `ord_light`), colour at +0xf4, and every ordnance of the class registers that one
light where it is. The colours stock gives them: every Federation photon `0 1 1`
(cyan) though its sprite is orange, nearly every other torpedo and pulse `0 1 0`
(green), the Klingons' red torpedoes and the Cardassians' yellow plasma included.
Measured from the stock art, the mean colour of each sprite's first frame, scaled to
a peak of 1:

| Faction | Sprite texture | ODF `lightColor` | Sprite |
|---|---|---|---|
| Federation (photon) | `Wftorp` | `0 1 1` | `1.00 0.69 0.10` |
| Federation (quantum, `fbattlephotono`) | `Wfbluetorp` | `0 1 1` | `0.14 0.56 1.00` |
| Klingon | `Wktorp` | `0 1 0` | `1.00 0.17 0.17` |
| Borg | `Wbtorp` | `0 1 0` | `0.08 1.00 0.92` |
| Cardassian (plasma) | `Wctorp` | `0 1 0` | `1.00 0.97 0.08` |
| Romulan | `Wrtorp` | `0 1 0` | `0.11 1.00 0.42` |
| Species 8472 (pulse, `spmpulseo`) | `Wpulse`, second strip | `0 1 0` | `0.78 1.00 0.43` |

**The fix takes the colour from the sprite.** The class keeps its sprite at +0x12c
(the ODF's `Sprite`, looked up in the sprite table by name); an `ST3D_Sprite` holds its
first frame as fractions of its texture, U V at +0x38 and W H at +0x40, and its texture
at +0x58, an `ST3D_DatabaseElement` whose name (+0x8) is the file name. The plugin
reads that rectangle of the installed TGA (`Textures/RGB`, so a remastered sprite is
measured as installed) and takes its mean colour: the sprites draw additively, so the
mean is what they add. That hue, scaled to the ODF colour's peak, replaces the light's
colour, so a weapon keeps its brightness. Why the first frame: a flipbook keeps one hue
across its frames (every stock torpedo's first frame is within 0.03 of its whole sheet),
while the pulses share one sheet, `Wpulse`, a colour per strip, and only the sprite's
own rectangle tells them apart (the whole sheet reads `1.00 0.93 0.78` for all of them).

`Ordnance::PreRenderAll` is called straight after `GameObject_PreRenderAll`, so the
plugin's wrapper of the latter walks the live ordnance (the list at 0x771fac, the class
at +0x34) and recolours each class's light before it is registered. Each class is
measured once and logged (`ordnance light: Wftorp odf (0, 1, 1) sprite mean ... ->`):
a light counts as done while it holds the colour written to it for that sprite, so a
class rebuilt for the next mission, with its ODF colour back, is measured again. The
ODFs stay stock: only the light's colour in memory changes, which is drawing alone,
so nothing another player sees or simulates differs.

On the bench (2026-10-07, `SCENE=factions`: one torpedo or pulse ship of each playable
faction firing at a Borg cube), `Lighting.log` recoloured all six (the table above, to
the second decimal). Under Federation fire the cube's plating took a warm orange pool
where a torpedo arrived; the same burst on `main` washed whole faces cyan. The mean
(G+B)/2 − R of the cube's centre over the burst: worst frame 29.5 before, 8.3 after
(the cube's own blue shield flashes count in both). The Klingon torpedo lit it red.
The Federation quantum torpedo (`fbattle`) was not in the scene.

### Phasers

A phaser shot is an ordnance object of class `Phaser` (vtable 0x6b8ee4) that lives as long
as its beam is drawn. `Beam::Simulate` (0x58b850) puts the beam's start (+0xbc) on the
firing ship's hardpoint every frame and its end (+0xc8) on the target, and counts its
time left (+0xac) down from the class's `lifeSpan` (OrdnanceClass +0x1c); at zero it
folds the beam up. Stock gives a phaser no light: no phaser ODF sets `lightColor`.

Each frame, after `GameObject_PreRenderAll`, the plugin walks the live ordnance, the
list at [0x771fac] that `Ordnance::PreRenderAll` (0x58e4f0) walks, with the object at
node +8 and +0x27 set once it has expired. For each live phaser that is visible (+0x24,
which `PreRenderAll` asks before it registers a torpedo's light) it registers two point
lights, at the beam's start (the emitter) and at its end (the impact), up over 0.06 s
and down over the last 0.25 s. The end is where `Beam::Simulate` lands the beam: on the
target's shield sphere while its shields hold, else on the hardpoint it aims at. Each
end has its own brightness, falloff and lift (`PhaserBrightness`…, `PhaserImpact`…).
No hook is
patched for this; before it reads any ordnance the plugin checks that slot 0 of the
`Phaser` vtable is its deleting destructor (0x57ee70), and logs `NOT PATCHED` if not.
A list read each frame never holds a pointer to an object that has gone, which tracking
lifetimes through `Simulate` and the destructor, as the explosions do, would.

- **Where.** Both ends are on a surface, and a light on a surface only grazes it: N·L
  is near 0 for the faces around it. Each light sits a few units along the beam from
  its end, towards the other: off the shooter's hull, which the beam leaves by
  construction, and off the target's hull or shield on the side the shot came from.
  That is enough at the impact, where the struck hull faces the shooter. It is not at
  the emitter: a beam mostly leaves sideways, so the lifted light is still level with
  the deck around the hardpoint. The first version (one light for both ends, the
  impact's falloff, lifted 8 units) moved that deck by +3/255 and was not noticeable in
  play.
- **The emitter's hot spot.** So the emitter's light gets its own light object, a tight
  falloff (gone at 24 units, squared: `PhaserFalloff`) and a wrapped diffuse term in
  `hull.hlsl`, `(N·L + w) / (1 + w)` with `w = PhaserWrap`: plating the light grazes
  takes about a third of it, and faces turned away from it (the far side of the saucer)
  still none. The wrap and power go to the shader in `pfall.z`/`.w`, 0 and 1 for every
  other light, which therefore draws as before. The fixed-function path (no d3d8to9)
  has neither, and lights the emitter as Direct3D's own point light does.
- **Colour.** The beam's own art. The class's sprite (OrdnanceClass +0x12c, an
  `ST3D_Sprite`) holds its texture at +0x58, whose name, as for every
  `ST3D_DatabaseElement`, is at +0x8. The lit texels' mean, scaled to a peak of 1 and
  read once per texture, is logged as `phaser:` (Federation `Wfedphaser`:
  `1.000 0.655 0.403`). It is times the beam's tint (+0xf0: white, or the owner's
  colour with `NORMAL_WEAPON_TEAM_COLOR`) and `PhaserBrightness`. `rdphaser`,
  `blphaser` and `mphaser` are rows of the one `Wphaser` sheet and all take its mean.
- **Two light objects**, one for every emitter and one for every impact:
  `RegisterLight` keeps its own copy of each colour and matrix, and the falloff is the
  light object's.
- **Picking.** `pick_points` takes it as it takes a torpedo's light: by its reach plus
  `HARD_REACH`, at its real position, never mirrored. The shaders still fade it per
  pixel, so it reads as a glow rather than a sphere. It is never merged with another
  light of its colour, because a Galaxy's banks fire together.

On the bench (`SCENE=firing`, `orbit shooter 200 25 220`, 1920x1080, a 200x70 box of
the saucer's deck beside the dorsal emitter, mean RGB): the first version, no beam
129/153/155; beam with `Phasers=0` 132.5/154.6/156.4, the flare sprite alone; beam with
`Phasers=1` 135.8/157.1/158.3. The hot spot (2026-10-07, the box at 800,420): no beam
93.6/114.2/116.9; beam 124.3/134.9/129.4, a warm patch on the deck centred on the
hardpoint, the rest of the saucer unchanged. At `PhaserBrightness=6`, `PhaserRange=35` it
was 148/152/142 and spread over a third of the saucer, clipping white at the core. From
below (`orbit shooter 200 -30 220`) a ventral emitter lights the saucer's underside
round its hardpoint and the neck beside it; nothing shows through on the far side.

The impact, on the bench (a Galaxy firing at a second Galaxy with its shields up,
`orbit target 215 25 130`, a 100x30 box of the target's engineering hull below the
hit, mean RGB): no beam 146.7/166.7/166.9; with a beam and `PhaserImpact=0` the same in
all five such frames; with `PhaserImpact=1` about 201/214/209 in four, a warm cream
wash over the hull and neck around the hit. The shield's own flashes (cyan) come and go
in both.

## Shaders

Phase 2 of `platform/D3D9.md`, first step. With crosire's d3d8to9 in the d3d8 slot the
game's device answers `QueryInterface(IDirect3DDevice9)` with the Direct3D 9 device behind
it (`platform/d3d9/d3d9dev.h`), and the hull draws of `ST3D_Standard_MeshVB::Render`
run with `hull.hlsl`'s `vs_3_0`/`ps_3_0` pair in place of fixed-function lighting.

**What the pair reproduces.** A `testbench/d3dtrace` frame of the planet scene with this
plugin (2026-10-05): the hull draws are `XYZ|NORMAL|TEX1` indexed triangle lists, lit by
Direct3D (`LIGHTING` at its default, on), stage 0 `MODULATE(TEXTURE, DIFFUSE)` for colour
and alpha, stage 1 off, blend `ONE/ZERO` (opaque, so the texture's alpha goes nowhere),
no fog, no specular; lights 0–2 are the sky light, Fill and Key as directionals and slot
3 a point light (the planet's glow, mirrored). The shaders compute Direct3D's lighting
equation for exactly that, per pixel: the material's emissive plus ambient, plus each
enabled light's diffuse x material diffuse x max(0, N·L) x its attenuation within its
range, clamped, times the texture. The lights, material and transforms are read back from
the device at the draw, so everything above carries over unchanged: the Key, Fill and sky
light, the picked point lights with their mirroring, `FixMirrored`'s reversal. A draw that
is anything else (another vertex format, fog, specular, another stage setup, a spot
light) goes fixed-function, and `Lighting.log` names the first of each reason.

**Where the shaders are bound.** In a hook on the d3d8 device's `DrawIndexedPrimitive`
(slot 71), and only while `VBRender` is running. The engine sets its vertex format with
d3d8 `SetVertexShader(FVF)`, which d3d8to9 turns into d3d9 `SetFVF` +
`SetVertexShader(NULL)`, so shaders bound before the engine's own setup would be unbound
by it. The FVF stays the input layout. The previous shaders are put back right after the
draw. The hook is patched in only when the device has a Direct3D 9 device behind it:
under DXVK's d3d8 the log says so and nothing is patched.

**Measured on the bench** (`SCENE=planet`, `orbit ship 200 20 150`, 1920x1080, d3d8to9):
`Shaders=1` against `Shaders=0`, the Galaxy's box has mean 0.323 against 0.320 and RMSE
0.035 (part of it the planet behind, which moves between runs). Light direction, colours
and levels match; shading runs smoothly across large triangles (the nacelles, the
engineering hull's flank) where per-vertex lighting interpolated it. Under DXVK's d3d8
with `Shaders=1` the log reads `no Direct3D 9 device` and the ship renders as before.

**Point lights in the shaders (1.6.0).** The directional lights still come from the
device: the engine reverses them to match the inward normals, which is right as it is.
The point lights do not: `hook_vb_render` picks up to 16 (`PointLights`, default 12) and
hands them to the pixel shader at their real positions, with the engine's own falloff
(full to the light's start, linear to nothing over its fade) applied per pixel rather
than once at the draw's origin, against the normal turned round (`misc.x`, -1). So a
torpedo's or a pulse's hard sphere lights the side of the hull facing it, the open item
above, and a large ship near a soft light falls off across its length. A mirrored draw
(`FixMirrored`), whose normals end up pointing outward, takes them with the sign +1, so
mirrored meshes get point lights for the first time. Direct3D's slots still get the
strongest 6, mirrored as before, for any draw that falls back.

On the bench (2026-10-05): in the `firing` scene a draw took the Galaxy's own torpedo as
a hard light (`Lighting.log`: `a draw takes 1 point lights, 1 of them hard`); in the
`planet` close-up the planet's glow at its real position lit the ship as the mirrored one
had, as it should for a light far away against the ship. The pixel shader is about 716
instructions as vkd3d emits it (unoptimised): over shader model 3.0's guaranteed 512,
which DXVK does not enforce and current Windows hardware exceeds.

**Draws left fixed-function on purpose.** In the `firing` scene some `VBRender` draws use
stage 0 `SELECTARG1(TEXTURE)`: the texture as it is, unlit. Lighting has nothing to add to
them; the log names it once.

**Night lights (1.7.0).** A hull texture's alpha is a night-lights map: windows, the
deflector, nacelle grilles, bussard collectors (`textures/README.md`). A `testbench/d3dtrace`
frame of the CPU path (`GPU=0`, planet scene) draws the Galaxy twice: texture x lit colour,
opaque, then stage 0 `SELECTARG1(TEXTURE)` with alpha `MODULATE`, blended
`SRCALPHA/INVSRCALPHA`, so the result is the lit texture with the bare texture laid over
it by its alpha. That second pass belongs to the material class: `armada2.map` names
`ST3D_SelfIlluminatingMaterial` (vtable 0x6bc854) with its own `NumPasses` (slot 3) and
`SetPassRenderState` (slot 4). `ST3D_Standard_MeshVB::Render` (0x63e450) calls the
material's slot 4 for pass 0 only, so on the GPU path the night lights never drew; the
vertex-buffer trace has one opaque draw per mesh. The material is `Render`'s third
argument, which `hook_vb_render` already receives: when its vtable is that class's, the
shader folds the second pass in, `lerp(texture x light, texture, alpha x SelfIllumination)`,
exactly the CPU path's result at `1`. Other materials are untouched, so a texture whose
alpha means something else, or has none (it samples as 1), is never lit up by it.

On the bench (`SCENE=planet`, `orbit ship 200 20 150`): the CPU path, the shaders before
and the shaders with the night lights side by side. The blue nacelle grilles, the red
bussard collectors, the saucer windows and the running lights are back as the CPU path
draws them, now over the per-pixel hull. `Lighting.log`: `a self-illuminating material,
its night lights folded in`.

Not reproduced: the CPU path's first pass also has `SPECULARENABLE` on, with the
specular colour computed on the CPU; the GPU path has had none since 1.0.0. 1.8.0 adds a
highlight of its own instead (next).

**Specular and rim (1.8.0).** Two terms the fixed-function path could not give a hull
reading as metal against black space:

- A Blinn-Phong highlight from every light, directional and point, times `Specular` and
  a gloss mask: the texture's luminance, so pale plating shines and dark seams and
  panel lines do not. It is added after the texture (white light on metal) and before
  the night lights are laid over, so a lit window is not glossed. The outward normal
  and each light's true direction are the inward normal and the device's direction both
  turned round (`misc.x`), as for the point lights. The camera is where the view matrix
  takes to the origin, `-t R^T`.
- A rim: `RimLight` x (1 − N·V)^`RimPower`, added to the light before the texture, so it
  takes the hull's own colour, on the faces turned edge-on to the camera. A hull's dark
  side keeps its outline.

Measured on the bench (`SCENE=planet`, 2026-10-05). From the usual angle (`orbit ship
200 20 150`), the Galaxy's box: mean 0.321 without, 0.338 with; the saucer and the
nacelle tops take a broad sheen. Side on (`orbit ship 20 10 150`), 0.194 → 0.228: the
saucer's edge and underside lift and keep a cool outline. `SpecularPower=64` with
`Specular=0.6` read as nothing at all from the usual angle (mean 0.320, as without): the
hulls are low-poly, and a large flat face such as the saucer's top has one normal, so a
tight highlight lands only at the one angle that reflects the key into the camera and
then lights the whole face at once. The broad default is the one that reads; how it reads
as ships turn is for the game to show. The pixel shader is about 1370 instructions as
vkd3d emits it; with vsync off in the `firing` scene the frame stayed on its 1.0 ms
floor, as before.

## Dynamic range

The frame is 8-bit and bloom (`postfx/`) works on it afterwards, as `pow(colour, 6)`, so
only what reaches nearly white blooms. Before 1.10.0 nothing lit on a hull could reach
it: `hull.hlsl` clamped the summed light at 1 *before* the texture, so a fully lit plate
was at most its texture's own grey, and an explosion beside a hull
(`ExplosionBrightness=4`) lit it no brighter than the Key alone. The night lights were
the texture's own colour, and ended up no brighter than the plating around them. With
the darker scene lights since 1.2.0, the frame had almost nothing for bloom to take.

- **The light is no longer clamped.** `t × light + specular` can pass 1: under an
  explosion, a torpedo, or the Key times `HullSun`.
- **A shoulder instead of a clip** (`HighlightKnee`): each channel as it is up to the
  knee, then `k + (1−k)(1 − e^(−(x−k)/(1−k)))`, which meets it with the same slope and
  approaches 1. Per channel, so what is hotter than white turns white, as a bright light
  does, rather than staying a flat saturated colour. The planet shaders end in the same
  shoulder (their light was already allowed past 1 by `PlanetSun`, and was clipped).
- **The highlights are what is pushed, not the hull.** `SelfIllumination` above 1 draws
  the night lights at the texture times that, and `Specular` is doubled.

Measured on the bench (`SCENE=planet`, `orbit ship 200 20 150`, a 720x330 box round the
Galaxy, one session relaunched per variant, before bloom):

| | `HullSun` | knee | `SelfIllumination` | `Specular` | mean | share > 0.85 | mean of c⁶ |
|---|---|---|---|---|---|---|---|
| A (as 1.9.0) | 1 | 1 | 1 | 0.35 | 0.380 | 0.4% | 0.037 |
| B | 1.6 | 0.8 | 1 | 0.35 | 0.426 | 9.6% | 0.098 |
| D | 1.25 | 0.8 | 2 | 0.6 | 0.419 | 3.2% | 0.073 |
| E | 1 | 0.8 | 3 | 0.8 | 0.369 | 3.7% | 0.053 |

B (and 2.2, worse) brightens the whole lit side: the plating rises to the windows'
level and they vanish into it, so the hull loses contrast while bloom rises. E keeps the
plating where it was and puts the near-white pixels where the eye expects light:
windows, nacelle grilles, bussards, the glint on the saucer. The defaults sit between D
and E (`HullSun=1.1`, `Specular=0.7`); `SelfIllumination` shipped at 2.5 on the bench and
came down to 1.8 after a look in game. A knee of 0.6 was tried
first and flattened planets: their ground is bright art, and compressing everything over
0.6 hazed it. In the `firing` scene the Galaxy's own torpedo now lights its engineering
hull orange as it leaves (the Borg cube was on the dot3 path and took none of this until
1.11.0: "Bump-mapped hulls").

## Bump-mapped hulls

`BumpShaders=1`. A mesh whose material names a bump map (`models/README.md`, "How a SOD
asks for it": the Borg in stock, and every hull `hull-bump.py` patches) does not take the
vertex-buffer path above: `ST3D_Mesh::Update` gives it `ST3D_Dot3_MeshVB` first. Before
1.11.0 none of this layer's shading reached such a hull: no `Ambient`, no point lights at
their positions, no specular, rim or shoulder, and its night lights as the dot3 path drew
them.

**What the dot3 path does.** Read from `ST3D_Dot3_MeshVB::Render` (0x6275a0),
`DrawLight` (0x627370) and `CreateShader` (0x627200). The vertex buffer has a 68-byte
vertex: position, normal, UV, then three object-space vectors S, T and S x T, which
`ST3D_CreateBasisVectors` (0x627aa0) sums per vertex from each triangle's UV slopes. The
bump map is turned into a normal map at load (`ST3D_CreateNormalMap`) and is the
group's second texture. `Render` draws every group once per light in the engine's list,
added together. Stage 0 is `DOTPRODUCT3` of the normal map against the light vector, which
the game's own vertex shader (`Shaders\dot3_directional.nvv`) takes into each vertex's S,
T, S x T basis. The light's colour times the material's diffuse goes in `TEXTUREFACTOR`; a
point light is reduced to a direction and a falloff at the mesh's centre. One more draw
multiplies the frame by the diffuse texture, and for a self-illuminating mesh (the mesh's
+0x12c, bit 4) a third draws the night lights over it. There is no ambient term
(`models/README.md`, "Measured": 18–21% darker than the CPU path).

**What the plugin does instead.** It takes slot 3 of the vtable and, under d3d8to9, draws
the group once itself, from the same vertex and index buffers (the engine's handles at
+0 and +8 of the group's 24-byte record, its vertex and triangle counts at +0x10 and
+0x14). The texture material's first pass sets stage 0 and the blend as for a plain hull.
The normal map goes to stage 1 through the engine's own `SetTexture`, which caches what
each stage holds. A Direct3D 9 vertex declaration of the 68-byte layout replaces the
engine's shader for the draw, and the d3d8 vertex shader is set back after it. `bump_vs`
and `bump_ps` (`hull.hlsl`) then give the hull everything `hull_ps` gives: the same
`shade()`, constants and point lights. The material is `hook_set_material`'s: diffuse
white, `Ambient` as emissive. The dot3 path sets no Direct3D lights, so the directional
lights come from the engine's list, as `ST3D_Standard_MeshVB::PreRender` would set them.

**The normal.** The dot3 shader projects the light onto S, T and S x T and dots that with
the map, so the normal it lights with is S·n.x + T·n.y + (S x T)·n.z. It is outward and
faces the light, where a stock mesh's normal points inward. The first build used it as it
stands. On a Galaxy that `hull-bump.py` had patched, whose map is flat (n = (0, 0, 1)), it
drew dark streaks down the saucer and the neck that the hull shaders do not have. S x T
is summed from UV slopes, and it turns away from the surface wherever the UVs are
mirrored or meet at a seam. The stock dot3 path lights with that same vector. So
`bump_ps` takes the surface from the SOD's own vertex normal (inward, turned round) and
uses S and T only for the map's slope, each made square to it. A flat map then gives
exactly a plain hull's shading. A draw under a mirrored matrix needs no `FixMirrored`
reversal: normal and light go to world space through the same matrix.

**Left to the dot3 passes:** any draw with no Direct3D 9 device behind d3d8 (DXVK's
d3d8, or `Shaders=0`), with fog on, while the engine's debug triangle cap (0x72c3f4) is
set, or for a group without both textures. `Lighting.log` names the first
(`shaders: bump-mapped hull draws in shaders N, stock dot3 M`).

Measured on the bench (1920x1080, d3d8to9, 2026-10-06):

| | stock dot3 | `BumpShaders=1` |
|---|---|---|
| Borg cube (`SCENE=firing`, `orbit target 60 25 260`), mean grey over the cube | 24.0 | 28.6 |

The same faces are lit on both, and the relief from the Borg's height maps is kept; the
lift is `Ambient`, the specular and the rim, and the night lights show as on the plain
hulls. A Galaxy that `hull-bump.py` had patched, against the same Galaxy on the hull
shaders (`SCENE=planet`, the 720x330 box): side on (`orbit ship 20 10 150`) mean 0.1414
both, RMSE 0.023; from above (`orbit ship 200 20 150`) 0.393 against 0.396, RMSE 0.049,
part of it the planet's clouds, which move between runs. With S x T as the normal, the
first build measured 0.392 and RMSE 0.080 from above, the streaks.

## Planets on the GPU

`PlanetShaders=1`, phase 3 of `platform/D3D9.md`. A planet is two `GroundMesh`
hemispheres and a cloud shell (`Planet_Database` +0xa8, +0xac, +0xb0), which
`GroundMesh::Recompute` (0x596200) rebuilds whenever the camera's distance changes the
facet size, and re-UVs every frame for a shell with turbulence. That is why they never
had a static vertex buffer, and why turning on the engine's own vertex-buffer path for
them would not do: `ST3D_Mesh::Update` builds that buffer once, and `Recompute` does not
call it. On the CPU path they reached Direct3D as screen-space triangles with a lit
colour (`testbench/d3dtrace`), with nothing for a shader to light.

`Recompute` does leave the object-space arrays every `ST3D_Mesh` has: positions at
+0xc0 (count +0xc8), normals at +0xc4, UVs at +0x124, and groups at +0x104 (count
+0x100, 0x1c bytes each) whose faces (+0x8, count +0xc, 0x28 bytes each) hold three
position indices and three UV indices. It is the layout `ST3D_MeshVB_Imp::CreateBuffers`
(0x637e40) reads to build a hull's buffer.

| Where | What |
|---|---|
| slot 11 of the `GroundMesh` vtable (0x6bab88), `ST3D_Mesh::RenderInternal` (0x6325d0) | the plugin's own: the camera's sphere test (`CheckSphereVisibility`), the texture material's `SetRenderState`, `SetTextureWrap`, `SetCulling` and `SetWorldTransform` as the vertex-buffer path sets them; then per group, for each of the material's passes (`NumPasses`, `SetPassRenderState`, `PassCleanup`, slots 3–5), the group's triangles from those arrays through `DrawPrimitiveUP` with `hull_vs` and a pixel shader of `planet.hlsl` |

The engine still sets every texture, blend and stage state, so only the colour is the
plugin's. The ground's material (`ST3D_PlanetaryMaterial`) has one pass, or two when
the planet has a development texture (`PD_<class><hemisphere>`): its cities, alpha
blended, times the planet's own population map in stage 1 (`cityAllocTexture`, painted
from `CityAllocArray`). The cloud shell is the class's `atmosphereTextureName`, blended
by its alpha. The pixel shaders are `ground_ps`, `city_ps` and `cloud_ps`:

- **The normal is the sphere's**, the direction from the world matrix's translation, per
  pixel. The terminator is round whatever the tessellation.
- **The lights** are read from the engine's list: the directional lights (the brightest
  is the Key; the others times `PlanetFill`, since on a sphere each lights a whole
  hemisphere: at 1 the sky light lit the night side teal) and up to 8 point lights that
  reach the surface, without the planet glows, which the CPU path never lit a planet
  with either. The material's constant term and diffuse are the ones `Planets=` sets.
  The cloud shell's are not: `PlanetInstance::mSetupAtmosphere` calls
  `SetAtmosphereTint` every frame, which writes 0.5 and 0.75 x the tint over them, so
  stock clouds stayed half lit on the night side even with `Planets=1`. The shader takes
  the tint back out and uses `PlanetAmbient` x tint.
- **Dusk:** the light wraps `PlanetWrap` past the terminator and takes `PlanetDusk`'s
  colour where it grazes.
- **The limb:** `PlanetHaze` x (1 − N·V)^`PlanetHazePower`, lit where the light
  reaches, added to the ground and the clouds; the cloud shell is larger than the ground,
  so at the edge it shows as a thin halo.
- **Water:** a Blinn-Phong glint where the ground is bluer than it is red or green and
  not bright.
- **Cities:** by day lit as the ground, as stock. On the night side they are lit from
  within, `CityLightColour` x `CityLights`, by a pattern the shader makes
  (`city_night`). Up to 1.12.0 it lit the whole development texture one colour, which
  showed each town as a flat cream blotch: the texture's alpha is one solid blob per
  town, with nothing inside it (`PD_ECNA`, `PD_ECFR`, `PD_BORG`, measured), and its
  colour is day-side ground. So the blobs say only *where* people live:
  - **Density:** the alpha, times the population map in stage 1 (alpha times its
    brightest channel, whichever the engine paints), averaged over 8 taps at 5 texels
    and 8 at 14. Explicit taps rather than a mip bias, since nothing says these textures
    have mips. A noise breaks each town into districts.
  - **Streets:** the borders of a Voronoi net, 120 cells across the texture, inside
    towns, beaded with lights along them, each border its own brightness; **lanes:** a
    finer net of 300 in the denser parts; **highways:** a net of 20 cells, half its
    borders dropped, reaching out between neighbouring towns. All are warped by a slow
    noise so the roads curve. One net of 120 alone read as cracked glass close up.
  - **Lights:** single points, thick in towns and thinning out past them; **glow:** a
    faint warmth over each town; **downtown:** brighter in a few hot spots where a town
    is densest. Light past 0.9 whitens.
  - **No shimmer:** a line or point narrower than a pixel (`fwidth`) spreads the same
    light over the pixel instead, so a distant planet shows an even glow.
  The pattern was tuned on an offline preview over the real textures, then on the bench
  with a colony ship ordered onto a class M planet (select it, D, click the planet; the
  script call `ScriptInterfaceImp::Colonize` did nothing from the scene plugin). The
  shader is about 1800 instructions, past ps_3_0's guaranteed 512; DXVK runs it, as it
  runs `bump_ps` at 775.

**One trap, met on the bench:** `ST3D_DeviceDirectX8::PolygonSortRequired` (0x625510)
is not a property of the device. It reads the material last set on it, so asked before
the planet's own material it answered for whatever was drawn before, and the cloud shell
went to the stock path on some frames and the plugin's on others: it flickered. The
plugin does not ask. A blended shell is drawn at once over its own ground instead of
being deferred to the engine's sort; nothing else sits between the two.

On the bench (`SCENE=planet`, 1920x1080, d3d8to9): five frames of a paused view were
byte-identical, and every planet mesh went through the shaders (`Lighting.log`:
`planet shaders: meshes drawn`, `left to stock 0`). Side on (`orbit planet 305 15
1000`) the terminator is a clean curve with a narrow warm band; from the night side
(`orbit planet 35 10 1000`) the planet is dark with its clouds faintly visible, where
the CPU path lit both its ground and its clouds evenly.

## Shadows

`Shadows=1` and `PlanetShadows=1`, with `Shaders=1` under d3d8to9; phase 3 of
`platform/D3D9.md`. Only the Key casts a shadow: it is the one light strong and
directional enough for one to read, and the fill, sky light, ambient and point lights
still reach a shadowed face, so it stays dark blue rather than black.

**The engine has no pass for a shadow to come from.** It draws one object at a time,
each straight into the frame. So the plugin keeps a list of every hull draw it puts
through the shaders, the plain ones (`hook_dip`) and the bump-mapped ones
(`bump_draw`): the Direct3D 9 vertex and index buffers (a reference to each), the
draw's arguments, the base vertex index (asked of the d3d8 device, since d3d8to9 keeps
it), `WORLD`, and the mesh's bounding sphere. `ST3D_MeshVB_Imp::Init` (0x637d20) keeps
the `ST3D_Mesh` at +0xc of the vertex-buffer object, and the sphere is the mesh's own
centre (+0xd4) and radius (+0xe0), the pair `RenderInternal`'s camera test reads. At the
first hull draw of the next frame (`sm_frame`; a frame is a call of
`GameObject_PreRenderAll`, which the plugin already wraps) it draws that list again into
a depth map from the Key, and every hull draw of the frame looks itself up in it.

**The map.** R32F, `ShadowSize` square, with its own D24X8 depth surface, orthographic
along the Key (the brightest directional light in the engine's list). It is fitted round
the spheres of the hulls drawn, so it covers what is on screen: zoomed in on one Galaxy
a texel is 0.18 units, over a battle a few units. Its width goes in steps of 2^¼ and
its centre is snapped to whole texels, so from one frame to the next its texels stay
where they were and a shadow's edge does not crawl as the camera moves. `depth_vs`/
`depth_ps` (`hull.hlsl`) write depth 0..1 across the hulls' range; both faces cast.
Everything the pass changes is captured beforehand in a `D3DSBT_ALL` state block and
applied again after it, with the render target and depth surface, so the engine's next
draw finds the device as it left it. The target and depth surface are in the default
pool, so the plugin wraps the d3d8 device's `Reset` (slot 14) and lets them go first,
with every buffer reference it holds.

**The lag, taken out.** The map is a frame old. A hull that moved since would find its
own shadow a frame behind it, and its lit faces would cross their own shadow. So a draw
looks itself up where it was when the map was drawn: the record in the map's list with
the same buffers and arguments nearest to it (within 300 units) gives its `WORLD` then,
and the vertex shader takes the shadow coordinate with that matrix. A hull's shadow on
itself is then exact however it moves and turns; only one hull's shadow on another is a
frame late. A draw with no record (new on screen, or switched to another level of detail)
uses its own matrix. Two ships of one class share buffers, and are told apart by where
they are.

**Lookup.** Each vertex is moved out along its normal before the lookup, by half a texel
where the Key falls straight on, up to two where it grazes, with a depth bias of half a
texel more: the low-poly hulls' smooth normals otherwise shadowed their own faces near
the terminator. The pixel shader takes 3x3 texels, weighted by where the point falls
between them (16 taps), and scales the Key's diffuse and highlight by what passes
(`dcol[k].a` marks the Key among the directional lights).

**Planets** cast no shadow into the map. A planet is a sphere, and the plugin knows each
one's centre and radius (as for its glow, `sm_planets`: 262 for class M, the engine's own
`Planet_Database` radius), so the pixel shader tests up to 8 directly: a point the Key
reaches only through a planet is in its shadow, softly over an edge that widens with the
distance behind the planet.

Measured on the bench (2026-10-07, 1920x1080, a scene of a Galaxy, a destroyer behind
the planet, a shipyard with a Galaxy above it and a Borg cube; each view taken with
shadows on, then the game relaunched with `Shadows=0 PlanetShadows=0`):

- The Galaxy from below and behind (`orbit ship 225 45 160`): the engineering hull throws
  a hard shadow across the right nacelle's pylon and inner face; the difference is black
  everywhere else on the hull, so no acne on the saucer or the flat faces.
- The destroyer behind the planet's night side: fully sunlit before, dark in the
  planet's shadow after.
- The shipyard: the docked ships and the two towers shade the lower rail.
- The Borg cube (bump-mapped, convex): only its pipes' hairline shadows.
- A destroyer ordered across the map, at about 100 units a second: its nacelles shade
  its flanks with no offset bands or streaks.
- The `firing` scene after merging the phasers: combat draws as before;
  `Lighting.log` reads `shadows: map 1000 of 5 hull draws`.

Not shadowed: anything not drawn through the hull shaders (the CPU path's cloaking or
warping ships, translucent meshes) casts and takes no shadow, and planets take none from
ships. A hull outside the view casts none into it: the map holds only what the engine
drew, and it culls what is off screen. The map is one, not cascaded: on a wide view
reaching far towards the horizon, its texels grow with everything it must cover. Up to
2048 hull draws a frame cast; the rest are logged once and cast none.

## The near fade

A ship or station that fills more than half the view fades out as it fills more:
`CraftInstance::ComputeFadeOut` (0x4caff0) takes the share of the view it fills,
from `cfgFADE_OUT_MIN_FOV` (0.5) to `cfgFADE_OUT_MAX_FOV` (2.0), down to
`1 - cfgFADE_OUT_MAX` (0.125). `ST3D_Instance::RenderInternal` (0x62e780) keeps the
fade at the instance's +0x78, sets +0x77 when it is below exactly 1, and makes the
instance the engine's +0x100 while its meshes draw. `PolygonSortRequired` (0x625510,
slot 29 of `ST3D_DeviceDirectX8`) then answers "sort", and `ST3D_Mesh::RenderInternal`
(0x6325d0) sends every mesh to the CPU path, the one that sorts. So a close ship was
drawn in stock's lighting, and switched to the shaders the moment the camera drew far
enough back for its fade to reach 1. A fade of 0.95 cannot be seen against space; the
switch of lighting can (seen in a showcase take: the Martok at orbit distance 150,
lit from 5.7 s as the camera glided to 200).

Measured on the bench with the Martok framed at 150: at the call that picks the path,
every "sort" answer, 760 of 760, came from a faded `CraftInstance`; at ordinary
distance none.

The plugin wraps that slot and answers "no sort" when the fade is the only reason (an
opaque material, the instance's fade slot is `CraftInstance`'s, no cloak callback at
the engine's +0x110, no per-render effect at +0xf8), asked from the mesh's
`RenderInternal` (the two calls returning to 0x63275b and 0x6327ea) or from
`ST3D_TextureMaterial::SetRenderState` (returning to 0x64472a), which leaves the
material's blend states unset when the answer is "sort". Every other caller, a cloak
and a translucent material keep stock's answer. `hull.hlsl` draws the fade as a
screen door: a pixel is kept where the fade is above interleaved gradient noise over
the screen. It stays opaque and writes depth, so it needs no sorting and whatever is
behind shows through the gaps in any draw order; blending in place would hide a ship
drawn later behind it.

## Not covered yet

- With `PlanetShaders=0` or without d3d8to9, planets stay on the CPU path. Either way
  the clouds at a planet's poles pinch into a bright starburst where the cloud
  texture's UVs converge; that is stock.
- The city pattern is cut in texture space, so it stretches where a hemisphere's UVs
  do, at the limb of each hemisphere: a highway there can draw as a long straight streak.
- Without d3d8to9, the Borg and any hull `models/hull-bump.py` patched keep the dot3
  passes ("Bump-mapped hulls"). The fallback under DXVK's d3d8 has not been run on the
  bench since 1.11.0; it is the stock function, called whenever there is no Direct3D 9
  device.
- Translucent materials still go through the CPU path for sorting (see the moons in
  `models/README.md`). Under DXVK that path was slow for another reason: it reads back
  a dynamic vertex buffer kept in GPU memory. The selection bubbles cost 70 ms a frame
  with 30 ships selected. `dxvk.conf` fixes that, not this plugin (`postfx/README.md`,
  "Reading back a dynamic vertex buffer").
- Seen on the first Federation campaign map only. Other races, combat effects and frame
  rate have not been measured.
- A mirrored mesh (`FixMirrored`) gets no point lights on the fixed-function path:
  reversing a directional light fixes its normals, and a point light has no such
  reversal. With `Shaders=1` it gets them.
- A hard light can tint a whole face of a large flat hull (a Borg cube) when the draw
  goes the fixed-function way: per vertex, one vertex inside the torpedo's sphere
  colours its triangle. Seen on the bench with Borg torpedoes on a Borg cube before
  `BumpShaders` (1.11.0) drew the Borg in shaders; not re-checked since. Not caused by
  `OrdnanceColours`, which changes only the colour.
- Weapon impacts other than a phaser's flash no light (above). `ShockwaveExplosion` (the big special weapons'
  ring) has its own `AdjustLighting` and is left alone.
