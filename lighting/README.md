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
- **`Planets=1`**: planets and their cloud shells get a night side. Stock gives their
  material a constant half-white term that lights them all round (below).
- **Light sources** ("Light sources", below): point lights reach the GPU-drawn ships
  (`PointLights`), so torpedoes and pulses light the hulls they pass as they always did
  on the CPU path; nebulae glow in their colour (`Nebulae`); a planet's day side lights
  what is near it in the colour of its ground (`PlanetGlows`); the skybox adds a faint
  third light in its own colour (`SkyLight`); and a ship's or station's explosion
  lights its surroundings (`Explosions`).

`./install` runs `install.sh`; `install.sh --remove` takes the three files out again
(`Lighting.asi`, `Lighting.ini`, `Lighting.log`). The exe is patched in memory only, and
`a2mod` switches the plugin as the `lighting` layer. Each launch writes `Lighting.log`:
`call sites patched 11` means every hook took, and the lines after it name every model
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
| `PlanetAmbient` | `0.02 0.02 0.03` (left out: `Ambient`) | the planet material's constant term, added whatever the direction; stock is `0.5 0.5 0.5` |
| `PlanetDiffuse` | `1.00 1.00 1.00` | the planet material's diffuse colour; stock is `0.75 0.75 0.75` |
| `Shaders` | `1` | under crosire's d3d8to9 (`platform/d3d8-chain.py --use d3d8to9`), light the GPU-drawn hulls per pixel in shaders (below); with any other d3d8, or `0`, per vertex as before |
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
| `Log` | `1` | write `Lighting.log` |

The key comes in about 60° off vertical (1.0.0 had about 37°), so from the usual
camera, which looks down from above, the light grazes the hulls and planets instead of
falling straight onto them. The fill, `Ambient` and `PlanetAmbient` are kept low so
that light sources added later stand out against the base lighting. The lighting is
per vertex (per pixel with `Shaders=1` under d3d8to9) and casts no shadows: a flat face such as a saucer's top takes one tone
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
| call at 0x598193 | wraps `GameObject_PreRenderAll` | counts the frame for the hook above |
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

Torpedoes and pulses need nothing more: `Ordnance::PreRenderAll` (call at 0x58e55c)
registers each one's ODF light while the engine's detail level is above 2, and the
point-light path hands it to the GPU draws. A weapon's impact has no explosion object
(it is a sprite effect), so it has no light of its own.

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

**Next, and it changes the look:** the hull texture's alpha, the self-illumination map:
in the traced frame no draw on this path used it (blend `ONE/ZERO`, and no second pass).

## Not covered yet

- Planets stay on the CPU path (their meshes are rebuilt as the camera moves). The
  clouds at a planet's poles pinch into a bright starburst where the cloud texture's
  UVs converge; that is stock.
- The Borg and any hull
  `models/hull-bump.py` patched keep the dot3 path, which takes precedence over the
  vertex buffers.
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
- Weapon impacts flash no light (above). `ShockwaveExplosion` (the big special weapons'
  ring) has its own `AdjustLighting` and is left alone.
