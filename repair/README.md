# repair — `Repair.asi`

While a ship's damaged system is being repaired, stock flashes small white star sprites
on the hull, one at a time, at the hardpoints of that system: a welding effect.
`Repair.asi` draws that welding itself, in shaders: at each spark an arc burns,
flickering blue-white with thin rays that turn with every flicker and stuttering out for
a beat now and then; it leaves an orange hot spot on the hull that cools once it is out;
it throws off a spray of sparks along the hull's outward direction that streak away and
cool from white through orange to red; and it lights the hull about it in its own
blue-white light.

It changes what is drawn and nothing else: the sparks are the engine's, made, placed and
timed by its own repairs, and nothing the simulation reads is touched, so a player with
it can play a player without it.

```
repair/install.sh            build, then install Repair.asi and Repair.ini
repair/install.sh --remove   take them out again
```

`./install` runs it; `./a2mod` switches it as the `repair` layer. It needs nothing from
`A2_DATA`: the effect is procedural. Without a Direct3D 9 device behind d3d8 (DXVK's own
d3d8, `platform/D3D9.md`) or with `Enable=0`, the game draws its own sparks.

## How stock draws the welding

Addresses from `armada2.map`, GOG patch 1.1. Established by reading `Armada2.exe`
for the hooks below, and on the bench.

- **`Craft::RepairSystem(CraftSystem *, float)`** (0x4c8fe0) repairs one system a tick
  while it is below its maximum. It counts time up at the system's +0x24 and, each time
  that passes the system's interval (+0x20), picks one of the system's hardpoints (its
  `ST3D_Node` list at +0x2c) at random and calls
  **`NodeParticleEffect::AddParticle(name, colour, GameObject *, ST3D_Node *, size,
  life)`** (0x4733c0, static, cdecl; call at 0x4c90c0) with the config string
  `SYSTEM_REPAIR_SPARK_SPRITE` (`xspark`), `ST3D_Colour_White`, the size
  `SYSTEM_REPAIR_SPARK_SIZE_FACTOR` (0.14) times the craft's bounding radius, and a life
  of one second.
- **`NodeParticleEffect`** (the static `nodeParticleEffect`, a `GameFeature`) keeps its
  particles in a `std::list` whose head node is at [0x736cfc] and count at 0x736d00.
  `AddParticle` refuses a craft that has four already (its +0x38) or a list of over 200,
  and appends a node: the object's handle at +8, the hardpoint at +0xc (the other form,
  0x4734c0, takes a point in the object's frame at +0x10 instead), the sprite (an
  `ST3D_DatabaseElement` found by name) at +0x1c, the colour at +0x20, the size at +0x2c,
  its age at +0x30 and +0x34, its life at +0x38. `Simulate` ages them and erases each at
  the end of its life, and gives the craft its count back.
- **`NodeParticleEffect::Render`** (0x473670) skips a particle whose object the player
  cannot see (`CanUserSee`), places it with `Entity::GetWorldPosition(ST3D_Node *)`
  (0x4cff70) and draws its sprite with `ST3D_Sprite::SetColour` and
  **`ST3D_Sprite::DrawScaled3D`** (0x63aa60, the call at 0x473767), the node in `esi`.
- The same list carries other effects' particles: the sparks of a hit on a hull
  (`Craft::DamageAlloc`), a phaser's (`Phaser::Init`, `Beam::Simulate`), the dying
  sequence's. They may share the `xspark` art (a class's default spark sprite is
  `xspark`), so a weld cannot be told by its sprite: it is told by the call that made it.
- **`Sprites/weapon.spr`**: `xspark`, `xspark01.tga`, a 4x4 flipbook, additive: a white
  star with a few blue-violet spark trails. On the bench (`SCENE=shields`, the target's
  shields shot out, then left alone) that is a white starburst at a hardpoint, one at a
  time, about a second each, on the saucer's rim and neck and the nacelle pylons. The
  small orange stars that come and go on a damaged hull are something else
  (`Sprites/damage.spr`, the crew-damage sprites), left stock.

## The hooks

Every site is checked before it is written (`AddParticle`'s, `DrawScaled3D`'s and
`GetWorldPosition`'s first bytes, the list's head read in `Render`, and a `call` at each
call site); a different `Armada2.exe` leaves the plugin inert and says so in
`Repair.log`. **Every hook is a call site, chained**: the plugin calls whatever the call
went to when it patched it, so another plugin's hook at the same site keeps working, and
the log names a target that is not the engine's.

- **The call to `AddParticle` at 0x4c90c0 in `Craft::RepairSystem`**: the plugin calls
  it; if the count went up by one, the new tail of the list is a weld, noted by its node
  with the handle, hardpoint and sprite it holds, so that a node freed and reused by
  another effect is not taken for one.
- **The call to `DrawScaled3D` at 0x473767 in `NodeParticleEffect::Render`**, through a
  thunk that notes `esi` (the particle's node) and jumps to the wrapper with the call's
  own arguments: a weld is skipped while the plugin draws; every other particle calls
  through.
- **The call to `ST3D_GraphicsEngine::RenderChunks` at 0x598446 in
  `Armada_RenderAllOurStuff`**: after the objects and the game features, with the hulls
  in the depth buffer. The plugin calls it, then draws the welds. `Shields.asi` and
  `Explosions.asi` chain the same call.
- **The call to `Ordnance::PreRenderAll` at 0x598199**, right after
  `GameObject_PreRenderAll`: the plugin calls it, then registers the arcs' lights.
  `Shields.asi` chains the same call.
- **The d3d8 device's `Reset`** (slot 14), chained: the state block is let go.

## The draw

Per frame, the noted welds whose node has left the list are dropped. Each of the rest is
placed as `Render` places it, aged by its own +0x34 (simulation time, so the sparks stand
still while the game is paused; the flicker runs on wall time), and given the hull's
outward direction there: the gradient of the ship's shield ellipsoid (the same transform
`Shields.asi` draws on) at the weld, which follows the hull's shape better than the
direction from its centre (up on a saucer's top, not out to its rim). Every quad is
built on the CPU, in the world, facing the camera, and drawn in one
`DrawPrimitiveUP`: additive, depth-tested against the hulls, no depth write. Sizes are
in the ship's bounding radius, so a frigate and a station look alike; each has a floor
in pixels (from the projection and the viewport), so the welding still reads from RTS
distance.

- **The arc** burns for the first `ArcTime` of the spark's life: a new brightness
  `FlickerRate` times a second, and about `Stutter` of the time it drops out for a beat.
  Its flare (`Flare` times the stock sprite's size, at least `FlareMinPixels` across) is
  a white core, a glow in `ArcColour`, and four rays and four fainter diagonals at a new
  angle with every flicker.
- **The hot spot** (`HotSize`, `HotColour`) glows up under the arc in its first quarter
  second and cools over `HotTime` once it is out.
- **The sparks**: `Sparks` a weld, each thrown off at its own moment while the arc
  burns, out of the hull about its outward direction (`SparkSpread`), at up to
  `SparkSpeed` bounding radii a second, slowing (`SparkDrag`), living `SparkLife`
  seconds. Each is a capsule from where it was `StreakTime` ago to where it is, brighter
  at its head, white-hot at first and cooling through yellow and orange to dull red; at
  least `SparkMinPixels` thick, dimmed by how much that thickened it.
- **Pull.** A ship's hardpoints sit a little inside its hull: at the first bench run
  only the sparks showed, the flares and hot spots were hidden by the hull they sat on.
  Both are drawn `Pull` times their half size towards the camera; at 2 they show, and
  a weld on the far side of a thin saucer could poke through it, which was not seen.

## The light

Each burning arc lights the hull about it: an ordinary `ST3D_Point_Light` registered
with the engine after `GameObject_PreRenderAll` (the chained call to
`Ordnance::PreRenderAll`), `LightColour` x `Light` x the arc's flicker,
`LightLift` bounding radii off the hull along its outward direction, full to
`LightReach` radii and gone a fifth of that further on, which `Lighting.asi`'s
`pick_points` takes as a hard light at its place, as it takes a torpedo's impact. Up to
sixteen, one light object each, made on first use. On the bench a weld at the bridge lit
the dome and the deck about it blue, flickering with the arc.

## Repair.ini

`Enable`, `Reload`, `LogWelds`, `Log`; `Brightness`; `ArcTime`, `FlickerRate`,
`Stutter`; `Flare`, `FlareMinPixels`, `ArcColour`, `FlareCore`, `FlareGlow`,
`FlareRays`, `FlareRaySharpness`; `Sparks`, `SparkSpeed`, `SparkSpread`, `SparkLife`,
`SparkDrag`, `SparkWidth`, `SparkMinPixels`, `StreakTime`, `SparkBrightness`; `HotSpot`,
`HotSize`, `HotColour`, `HotTime`; `Light`, `LightColour`, `LightReach`, `LightLift`;
`Pull`. Each is described in the file.

## On the bench

`SCENE=shields` with `heal target off` until the target's shields are down, then
`weapons shooter off` and `weapons klingon off`: its damaged systems are repaired, a weld
about every second (`Repair.log`: `weld on fgalaxy.odf (xspark), size 6.70, life 1.00, at
a hardpoint`; the Galaxy's bounding radius is 48). 1920x1080, `orbit target 140 30 95`,
2026-10-12: blue-white flares with rays at the saucer's rim, the neck, the bridge and the
pylons, the hull about them lit blue; orange hot spots; thin spark streaks. The first
defaults (a flare the stock sprite's size, `Pull` 0.6) drew too small and were hidden in
the hull; the defaults now are those that read at that distance.

`./a2test run shields-down-repair --install .` checks the hooks and that welds were
made, drawn and lit, and keeps three shots; `no-assets` checks it installs and patches
without assets.

## Not covered yet

- The release package (`publish/`, the GUI installer) does not carry the layer.
- Only the repair sparks: a hull hit's sparks, a phaser's and the dying sequence's in
  the same list stay stock, as does the Federation repair team's sprite (`wfedrep`) and
  the construction welding (`BuildingEffect`, the worker bees), which are other effects.
- Not yet seen in game, nor in a battle with many ships under repair; a weld costs at
  most 22 quads, and there are at most four per ship.
