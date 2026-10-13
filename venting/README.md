# venting — `Venting.asi`

A damaged ship vents: while its engines are down, stock streams flame-coloured smoke
sprites from the nacelle (the venting plasma), and while other systems are, blue-grey
puffs from the hull (the venting gas). `Venting.asi` draws both itself, in shaders:
each of the engine's particles becomes a puff of gas whose billows roll as it flies,
the plasma white-hot where it leaves the vent and cooling through yellow and orange to
red, the gas pale where it leaves and turning blue; at each vent that is spraying, a
glow where the stream leaves the hull and a surging jet core along its first stretch;
on a moving ship, one unbroken trail from each vent where stock leaves a row of puffs;
and each vent lights the hull about it, flickering, in its own colour: subtly, warm
for plasma, a faint blue for gas.

It changes what is drawn and nothing else: the particles are the engine's, made, moved
and timed by its own emitters, and nothing the simulation reads is touched, so a player
with it can play a player without it.

```
venting/install.sh            build, then install Venting.asi and Venting.ini
venting/install.sh --remove   take them out again
```

`./install` runs it; `./a2mod` switches it as the `venting` layer. It needs nothing from
`A2_DATA`: the effect is procedural. Without a Direct3D 9 device behind d3d8 (DXVK's own
d3d8, `platform/D3D9.md`), with `Enable=0`, or for a kind switched off (`Plasma=0`,
`Gas=0`), the game draws its own sprites.

## How stock draws the venting

Addresses from `armada2.map`, GOG patch 1.1. Established by reading `Armada2.exe`
for the hooks below, and on the bench.

- **The damage nodes.** A craft's model carries a node per system under a `Damage`
  group: on the Galaxy (`Fgalaxy.SOD`) `Shield`, `Sensors`, `Target`, `Life` and
  `Engines`, each holding a particle emitter, and `Crew` holding the `crew1`..`crew5` fire
  sprites. `CraftClass::InitializeDamageNodes` (0x4bfc70) finds them by name and keeps
  them at +0x408..+0x420 of the class; `CraftInstance::RenderInternal` (0x4cb0e0) shows
  or hides each one every frame from a byte per system at +0x9c..+0xa0 of the instance,
  which `CraftInstance::Update` sets from the systems' state (`testbench/scene/README.md`,
  "Damaged-system effects", has the rest). A shown node's emitter sprays; a hidden one is
  not traversed and makes nothing.
- **The emitters** are `Sprites/emitter.spr`'s "Damage emitters", all `type=spray`:
  `plasmasml`, `plasmamed`, `plasmalrg`, `plasmaxl` (the sprite node `ventplas`: the
  `xsmoke` flipbook, additive, red-orange smoke art, fading white to black over three
  seconds) and `steamtiny`, `steamsml`, `steammed`, `steamlrg`, `steamxl` (`ventgas`:
  a soft `xparticle` dot fading grey to blue to black over two). Of the game's 453
  models, 166 name one; the engines carry plasma (the Galaxy `plasmalrg`), the other
  systems mostly gas (the Galaxy's `Life`: `steamsml`). Each fires a particle every 0.15 s
  that flies at 8 units a second for three seconds (gas: two), growing by 1.1–1.4 units
  a second from the sprite node's size (2; gas 1.2).
- **`ST3D_ParticleEmitter_Spray`** (vtable 0x6bc26c): its `RenderInternal` (slot 11, the
  base class's 0x613030) is called for the emitter while its node is shown. It runs the
  emitter's `Simulate` (0x612a70), which makes a `SprayParticle` (0x6130a0 →
  0x612ee0) when the firing period has run down, and then hands the current particle
  list to the device. While it runs, the engine's current world matrix (0x7ad640, three
  rows and the translation) is the emitter node's own: the new particle's axes are
  that matrix turned by a random angle within the spread, and its place is the
  matrix's translation.
- **A particle** (`ST3D_Particle`, constructor 0x6120a0) keeps its emitter at +4, its
  axes at +8 (three rows; a spray particle flies along the third, +0x20), its place at
  +0x2c, where it was made at +0x38, and its age at +0x44. `SprayParticle::Simulate`
  (0x612f00) moves it along +0x20 at the emitter's speed (+0xe8) and ends it at the
  emitter's life (+0xec). The places are in the world: the particles stay where they were
  made as the ship flies on, so a moving ship leaves its puffs 0.15 s of flight apart
  (14 units at the Galaxy's speed on the bench, against a puff 2–6 wide).
- **Whose list.** `ST3D_Instance::RenderInternal` (0x62e780) puts the instance at
  +0x100 of the engine (0x7ad508) while it draws, simulates and draws its own particle
  list (+0x1c) through `ST3D_GraphicsEngine::SimulateParticleList` (0x62d380) and
  `RenderParticleList` (0x62d420), and then traverses its nodes, the emitters among
  them, which add to that list and hand it to the device again. So the device may be
  handed one particle more than once a frame.
- **`ST3D_DeviceDirectX8::RenderParticle`** (0x6262f0, slot 50 of the vtable at
  0x6bc6ac) draws one: the emitter's sprite node data (+0xd0; its name, a `char *`, at
  +8 as every database element's, its size at +0x18) gives the sprite, the size is that
  plus the emitter's growth (+0xdc) times the age, the colour animation runs on the
  particle's age, and `ST3D_Sprite` draws it at the particle's place.

On the bench (`SCENE=venting`): the Galaxy's plasma a stream of red-orange smoke sprites
from the starboard nacelle, yellow where it leaves and red at its end, 24 units long; the
gas a short blue-violet jet off the hull under the saucer; the Klingon's the same from a
wing and its neck. Neither lights anything.

## The hooks

Every site is checked before it is written (`RenderParticle`'s and `RenderInternal`'s
first bytes, the sprite node data's offset in `RenderParticle`, the speed's and life's
offsets in `SprayParticle::Simulate`, and a `call` at each call site); a different
`Armada2.exe` leaves the plugin inert and says so in `Venting.log`. **Every hook is
chained**: the plugin calls whatever was there when it patched it, so another plugin's
hook at the same place keeps working, and the log names a target that is not the
engine's.

- **`ST3D_DeviceDirectX8`'s vtable, slot 50** (`RenderParticle`): a particle whose
  emitter's sprite node data is named `ventplas` or `ventgas` (`PlasmaSprite`,
  `GasSprite`) is copied (its place, way, age; its emitter's life, growth and the
  sprite's size) and not drawn while the plugin draws; every other particle calls
  through. Each sprite node data is looked at by name once and remembered.
- **`ST3D_ParticleEmitter_Spray`'s vtable, slot 11** (`RenderInternal`): a vent emitter's
  place and axis are copied from the engine's world matrix, with the ship's bounding
  radius (+0x40 of the instance at the engine's +0x100), then it calls through. Only a
  vent spraying this frame is noted, which is what decides that it glows and lights.
- **The call to `ST3D_GraphicsEngine::RenderChunks` at 0x598446 in
  `Armada_RenderAllOurStuff`**: after the objects, with the hulls in the depth buffer.
  The plugin calls it, then draws everything noted since the last time and forgets it.
  `Shields.asi`, `Explosions.asi` and `Repair.asi` chain the same call.
- **The call to `Ordnance::PreRenderAll` at 0x598199**, right after
  `GameObject_PreRenderAll`: the plugin calls it, then registers the vents' lights.
  `Shields.asi` and `Repair.asi` chain the same call.
- **The d3d8 device's `Reset`** (slot 14), chained: the state block is let go.

Copying, not keeping pointers: a particle may be freed between the frame that drew it
and the next, so nothing noted is read again later.

## The draw

Every quad is built on the CPU, in the world, and drawn in one `DrawPrimitiveUP`:
additive, depth-tested against the hulls, no depth write. Each has a floor in pixels
(from the projection and the viewport), dimmed by how much that enlarged it, so the
vents still read from RTS distance.

- **The puffs.** One per particle, the stock sprite's size times `Size` and growing as
  stock's do, stretched along its flight as the camera sees it (`Stretch`). The pixel
  shader carves each with domain-warped value noise (four octaves, each turned off the
  last one's axes), seeded per particle and rolling with its age (`BillowScale`,
  `BillowFlow`): solid billows in the middle, wisps and holes towards the edge
  (`BillowContrast`, `EdgeSoftness`). With round, softly modulated puffs (the first
  version) twenty overlapping ones averaged their noise away into one smooth blob; carved,
  the plume keeps its billows. A puff fades in over its first tenth of a second and out
  over the last fifth of its life. Its age is the simulation's, so the gas stands still
  while the game is paused.
- **Their colour.** Plasma: from `PlasmaHot` through `PlasmaMid` to `PlasmaCool` by its
  heat (1 at the vent, 0 at the end of its life) to the power `PlasmaCooling`, hotter in
  its dense billows; gas: from `GasBright` to `GasCold`. The cooling had to be steep: at
  1.6, with a cream hot colour, the parked plume looked right (its three seconds packed
  into 24 units, with the old puffs piled together) but a moving ship's trail, whose
  visible stretch is its first 0.8 s, stayed cream all along. At 3, with a yellower hot
  colour, both read as flame; the white at the vent comes from the jet and the glow.
- **The trail.** A moving ship leaves its puffs apart. Each puff is stretched back to the
  next younger one of the same emitter, and the youngest to the vent (the emitter's place
  this frame), into a capsule, whenever that is longer than its own stretch and shorter
  than `MaxLink` times its width (a ship at warp keeps its row of puffs). Parked, the
  links are shorter than the stretch and change nothing.
- **The vent.** At each emitter spraying this frame: a glow (`Glow`, `GlowSize`, never
  under `GlowMinPixels`), pulled towards the camera by `Pull` times its size so the hull
  the vent sits on does not hide it, and a jet core from the vent along its axis
  (`Jet`, `JetLength` seconds of the particles' flight, `JetWidth`), narrowing, white at
  its centre, with knots of brightness running outward. Both flicker (`Flicker`): two
  slow waves and a quick one, out of phase per vent, on wall time. The vent comes from
  the emitter, not the youngest particle, which on a moving ship trails the nacelle by up
  to 0.15 s of its flight.

## The light

Each vent spraying lights the hull about it: an ordinary `ST3D_Point_Light` registered
with the engine after `GameObject_PreRenderAll` (the chained call to
`Ordnance::PreRenderAll`), `LightColour` x `Light` x the vent's flicker, `LightLift`
bounding radii out along the stream, full to `LightReach` radii and gone a fifth of that
further on, which `Lighting.asi`'s `pick_points` takes as a hard light at its place, as
it takes a weld's (`repair/README.md`). The vents are those the last frame drew; a pass
that draws none (the HUD's action camera may show no ship) keeps them, a second clears
them. Up to sixteen, one light object each, made on first use. On the bench the plasma
light (reach 17 on the Galaxy, radius 48) warms the nacelle pylon and the back of the
engineering hull below it; with `PlasmaLight=0` they are grey.

## Venting.ini

`Enable`, `Reload`, `Log`, `LogPuffs`; `Plasma`, `Gas`, `PlasmaSprite`, `GasSprite`; per
kind (`Plasma…`, `Gas…`): `Brightness`, `Size`, `Stretch`, `MinPixels`, `Glow`,
`GlowSize`, `GlowMinPixels`, `GlowColour`, `Jet`, `JetLength`, `JetWidth`, `Flicker`,
`Light`, `LightColour`, `LightReach`, `LightLift`, `Cooling`; `PlasmaHot`, `PlasmaMid`,
`PlasmaCool`, `GasBright`, `GasCold`; `BillowScale`, `BillowFlow`, `BillowContrast`,
`EdgeSoftness`; `MaxLink`, `Pull`. Each is described in the file.

## On the bench

`SCENE=venting` (`testbench/scene/README.md`): a healed Galaxy class and Klingon
battleship with every damage node showing (`Vent=all`), which the scene's `vent`
command switches per system. `goto galaxy 2000 0 3500` sets the Galaxy moving.
1920x1080, 2026-10-13: from `orbit galaxy 200 15 120` the plume leaves the starboard
nacelle white-hot, billows through yellow and orange and frays out red, as long as
stock's; the gas a blue jet with a white root under the saucer; on the moving ship two
unbroken trails, flame and blue, from the vents; from 700 away both still read.

`./a2test run venting --install .` checks the hooks and that vents were recognised,
drawn and lit, and keeps five shots; `no-assets` checks it installs and patches without
assets.

## Not covered yet

- The release package (`publish/`, the GUI installer) does not carry the layer.
- Only the vents: the crew-damage fire sprites (`crew1`..`crew5`, `Sprites/damage.spr`)
  and the other emitters (the latinum spray, the rift, special weapons' clouds) stay
  stock; `Venting.log` names each other sprite node it is handed, once.
- A faded hull (the camera very close, `NearFade=1`) is drawn at the device's `Flush`,
  after the vents, so it lies over them, as over the explosions.
- Not yet seen in game, nor in a battle with many damaged ships; a vent costs one quad per
  particle (about 20) and two more, with three noise lookups per pixel.
