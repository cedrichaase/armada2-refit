# render3d — the 3D view through our own renderer

`Render3D.asi` takes over the engine's 3D view: both calls of `Armada_RenderAllOurStuff`
(0x598110) come to it, so the scene is drawn into render3d's own targets and put into
the back buffer by render3d. It is written in `no_std` Rust (`src/`), and the plan, its
milestones and every change to it are in [`PLAN.md`](PLAN.md).

What it does today:

- **The frame is ours.** The view is drawn into a half-float (`A16B16G16R16F`) target
  with 4x MSAA and a depth of its own, then resolved, bloomed and tone-mapped into the
  engine's back buffer at the view's rectangle. Additive light (explosions, weapons,
  shield hits, glows) now sums past white instead of clipping, and what lies past
  white glows by how bright it is: bloom by energy, which an 8-bit back buffer cannot
  hold. Below the tone map's shoulder every colour is stock's.
- **The body is ours**: `Armada_RenderAllOurStuff`'s steps run in render3d's own
  sequence (`frame/steps.rs`), and seven modules (`## Modules` below) draw every
  category of the 3D view from the game's own assets, read by `a2fmt`: hulls, the sky
  and background, particles, ordnance, effects, the world (planets, moons, asteroids,
  nebulae) and the overlays (fog, grid, selection, ghosts). The cosmetic simulation
  (particles, debris, animation, LOD) is render3d's own, on the sim clock; the game's
  simulation is only read. A part no module draws yet is the engine's, called where
  stock calls it.
- **It replaces the per-part plugins**, which are no longer in the repository: `MSAA.asi`,
  `Lighting.asi`, `Sky.asi`, `Nebulae.asi`, `Surfaces.asi`, `Explosions.asi`,
  `Shields.asi`, `Repair.asi`, `Venting.asi`, `Specials.asi`, `Fog.asi` and
  `Planets.asi`. Their looks are ported here, with the findings the code needs written
  beside each hook below. A module stands down while a stale copy of its old plugin is
  loaded (`SUPERSEDED` in `src/lib.rs`), and `install.sh` takes any such copy out.
- **New, beyond the plugins:** bloom by energy (explosions' cores, weapon cores, star
  cores), MSAA on render3d's own targets, and shadows from this frame's hulls (the
  plugin's were the last frame's).

Still the engine's: nothing in a whole match by default. A module hands an object or
effect back to the engine only where it cannot draw it: a SOD that does not load or
is not a plain database (a hologram's class among them), an effect class it does not
know, or a module switched off (construction and the bees need the hulls module, the
emitters the particles module). Nanites have no draw of their own: `NaniteEffect` is
never made, and what shows is the ordnance and `Step::Specials`.

## Building and installing

    render3d/install.sh            # build, install Render3D.asi, Render3D.ini and the recipes; take out any old plugin
    render3d/install.sh --remove   # take it out again

`./install` runs it. Building needs rustup with the 32-bit Windows
target (`rustup target add i686-pc-windows-msvc`) and `lld-link` (LLVM); `build.sh`
uses `~/.cargo/bin/cargo` first, since a distribution's cargo has no such target. The
crate links with `/nodefaultlib`, `kernel32` its only import (an import library made by
`llvm-dlltool --kill-at` from `kernel32.def` in `build.rs`), and brings its own
allocator, memory functions and stack probe (`src/rt/`). Shaders are HLSL in
`shaders/`, compiled by `hlsl-rs.sh` with `vkd3d-compiler` into `src/shaders.rs`, which
is committed with a stamp so a machine without vkd3d builds as long as it is current.

## `Render3D.ini`

| Key | Default | |
|---|---|---|
| `Enable` | 1 | 0: the engine draws the 3D view, as without the plugin |
| `Log` | 1 | `Render3D.log` in the game directory |
| `MSAA` | 4 | samples per pixel on the scene target: 0, 2, 4 or 8 |
| `HDR` | 1 | 0: an 8-bit scene target (no light past white, no bloom), for comparison |
| `Bloom` | 0.12 | how much of the light past the threshold spreads into a glow |
| `BloomThreshold` | 1.0 | where bloom starts, in stock's white |
| `BloomKnee` | 0.5 | how soft that threshold is |
| `Exposure` | 1.0 | scales the scene before the tone map |
| `Shoulder` | 0.8 | where the tone map starts to roll off towards white |
| `Inset` | 1 | 0: the HUD's action-camera view is left to the engine |
| `Timing` | 0 | 1: CPU and GPU time per view and step in the log (below); with `Enable=0`, the engine's own view timed |
| `TimingEvery` | 5 | seconds between timing reports |

**`[Modules]`.** Each module has a key of its own, `<name>=1` (the names are in the
sections below: `effects`, `ordnance`, `specials`, `sky`, `overlays`, `particles`,
`hulls`, `world`). **`<name>=0` leaves that part to the engine: its stock look, drawn by
`Armada2.exe` at the step as without the module.** There is no plugin to fall back to
any more, so the old plugins' looks exist only as these modules.

## Timing

`Timing=1` (`src/timing.rs`) times each view as a run of marks: the seam, our targets,
each step of the body (`setup` is the lights, the distance cull, the modules' lights
and the scene layer's shadow map; then `background`, `terrain`, `objects`, `ordnance`,
`specials`, `features`, `warp`, `hud prep`, `chunks`, `particles`, `translucent`,
`selection, end`) and `post` (resolve, bloom, tone map). Each mark notes the CPU clock
and issues a Direct3D 9 timestamp query; a span is booked to the mark that ends it.
The queries are read back from a ring four frames deep without waiting, so timing
does not stall the GPU; a set not finished when its turn comes again is counted as
late. Every `TimingEvery` seconds the log has the frame's period (from one main view
to the next), each view's CPU and GPU time with their maxima, the time outside the 3D
views (the HUD, the simulation, `Present`), and the split by step.

With `Enable=0` and `Timing=1` nothing of render3d draws, but both call sites of
`Armada_RenderAllOurStuff` go through a body that only marks around the engine's
(`frame::install_measure`), with every plugin hooked into it at work. Installed beside
the plugins render3d replaces, that measures them the same way: the comparison in
`PERF.md`.

## How the frame works

**The seam.** The main view calls the body from `Armada_RenderFrame` (`call` at
0x59886e), and the HUD's action camera from `CinematicView::RenderView` (0x4e60f6). Both
calls are re-pointed at render3d's own function, each only if it still calls 0x598110,
so a different `Armada2.exe` or another plugin at either site makes render3d stand
down and the game draws as stock. The body is `cdecl` with three arguments (the
camera, the `Render_Stuff` flags, `isMain`), which render3d passes on unchanged.

**Into our target.** Before the body, render3d binds its scene target and depth in
place of the engine's, keeping the viewport the caller set, and clears colour and
depth: the engine never clears colour in a mission (it relies on the sky to cover every
pixel), and our target has no last frame to fall back on. The body then runs as it
always did. The two views share one set of targets the size of the back buffer, since
each view is a rectangle of it (`camera+0x1b0`, four floats in back-buffer pixels).

**Out again.** After the body, for that view's rectangle only:

1. resolve the samples (`StretchRect`);
2. bloom: a prefilter to half size with a soft threshold and a 1/(1+luma) weighting of
   its four taps, so one hot pixel cannot flicker as a blob; four more levels down a
   dual-filter chain, and back up adding each level onto the one above;
3. the final pass into the engine's back buffer: the scene a texel per pixel plus the
   bloom, through the tone map (unchanged below `Shoulder=`, rolling off towards white
   per channel above it);
4. the engine's depth cleared to 1 in the rectangle, for the HUD drawn over the view
   next.

Every tap is clamped to the view's rectangle, so neither view bleeds into the other's
glow. The device's state is captured into a state block before the passes and put back
after, and the engine's own target and depth are bound again, so the engine (and
d3d8to9, which keeps its own record of what is bound) sees what it left.

**The first row and column.** DXVK offsets a Direct3D 9 viewport by half a pixel, so
with MSAA half the samples of the view's first row and column are never drawn. The final pass reads those pixels from their neighbours instead, which does what
`MSAA.asi`'s edge fill did without the frame's delay.

**Reset.** Every target is in the default pool and must be gone before the device
resets. Slot 14 of the d3d8 device's vtable (`Reset`) is chained, as `Lighting.asi`
chains it: render3d drops its targets and passes on the call, and makes new ones at
the next frame, at the new size. A display mode chosen in Graphics Settings is not a
`Reset`: the engine releases its device and creates another. The first frame on a
device render3d has not seen ("frame: a new device") drops the frame's state, hooks
the new device's `Reset` and hands the device to every module, which makes its
resources again; scenario `render3d-mode-switch` checks it, switching to
800x600 in a running battle.

**Without Direct3D 9.** The d3d9 device is asked for behind the game's d3d8 one
(`platform/D3D9.md`). Under DXVK's own d3d8, or when the targets cannot be made, the
body runs as stock. An `A16B16G16R16F` target with MSAA falls back to one without, then
to 8 bits, and the log says which.

## Modules

### effects

`src/mods/effects/`, on by default. The game features that are effects, hull debris,
and their lights; it ports `Explosions.asi`, `Shields.asi` and `Repair.asi`, whose
looks were settled with the user (kept as they were; the settings keep the plugins'
keys, in `Render3D.ini` sections `[Explosions]`, `[Shields]`, `[Repair]` and
`[Debris]`, read once at start). Each part stands down while a stale copy of its old
plugin is loaded. Everything noted while the features run is drawn at `Step::Chunks`, after
the debris and the engine's own `RenderChunks` (called through its call site, so
construction's chunks and any plugin chained there still draw), when every opaque hull
is in the depth buffer.

- **Explosions** (`explosions.rs`). `ExplosionFeature::Render` (0x464ba0) walks
  `Explosion::explosionList` (head node at 0x7367c4) and submits each explosion whose
  slot 2 says it is visible (slot 15, `Submit(cam)`). The module does the same in its
  place, noting `FireballExplosion`s (vtable 0x6afbd4) and `ShockwaveExplosion`s
  (0x6afdac) and submitting any other class (`OmegaExplosion`) to the engine. The
  fireball is ray-marched, the shockwave a band of fire, as in the plugin. New: the
  scene is HDR, so the light above `HdrFrom=` (1.5) is kept in part (`Hdr=`, 0.5) and
  the white-hot core and the flash bloom by their energy; below it the plugin's look is
  unchanged. Each fireball also registers a point light (stock gives it none): up
  to full in its first 0.15 s, then down with the square of the time left, full out to
  the explosion's bounding radius and gone at `LightRange=10` radii (no less than 40
  units), in `LightColour` times `LightBrightness=4`. It is an ordinary point light,
  which the engine's own CPU lighting (planets, cloaking ships) takes too. Skipped
  while a stale `Lighting.asi` is loaded, which adds its own.
- **Shield hits** (`shields.rs`). `ShieldEffect::Render` (0x474910) calls
  `ShieldHit::DrawShield(cam, [0x736e2c])` (0x474070) for each hit of `m_shieldList`
  (0x736de8) in order; the module does the same for every hit that is not its own
  (types 0 and 1, and a special weapon's shell with a `[Shell.<name>]` recipe, named
  from the engine's table at 0x6b00e0). The hits' strength comes from the four weapon
  calls of `CreateShieldHit` (0x58bb95, 0x58cad8, 0x58d502, 0x58dd65), chained through
  thunks that note the ordnance in `ebx`. The shields-down discharges register a point
  light on the hull.
- **Repair welding** (`repair.rs`). `NodeParticleEffect` stays the engine's (its
  `Render` also prunes particles of dead objects): the call to `AddParticle` in
  `Craft::RepairSystem` (0x4c90c0) is chained to note each weld, and the call to
  `DrawScaled3D` in `NodeParticleEffect::Render` (0x473767) goes through a thunk that
  reads the particle's node from `esi` and skips a weld (and, while the sparks below
  are on, every other particle too). The welds are drawn as in the plugin, and each
  burning arc registers a point light.
- **Debris** (`chunks.rs`), `Step::Chunks`: our own chunks (`spec-chunks-anim-lod.md`
  §1). The call to `ST3D_Instance::Fragment` (0x62eb20) in `Craft::Explode` (0x4c6bfc)
  is chained: for a craft whose SOD we read, the module notes its ODF, transform,
  animation clock and team colour and does not call the engine's (so the engine makes
  no chunks of it); otherwise the engine's runs. At the next `Step::Chunks` each mesh
  of the LOD shown breaks into patches as `CreateChunks` does (a random seed face, every
  face with a corner within half the mesh's radius), flying and spinning on the sim
  clock with stock's `ART_CFG.h` numbers (4 ± 2 s, 90 units/s, 4 rad/s, at most 1500),
  drawn through the scene layer's hull shading, and gone without fading at the end of
  their life. The Evolver's cocoon bursting is taken the same way: its four calls of
`Fragment` (0x4b07b4, 0x4b0872, 0x4b0914, 0x4b09b4, in `Evolver::mDoRemoveConstructionEffect`
and the other construction-effect functions; each breaks the whole cocoon, faces -1),
each re-pointed only while it calls `Fragment` itself; the cocoon (a `CocoonInstance`,
vtable 0x6b2630, of the database at 0x73fccc) at its instance's transform and scale.
  A construction cancelled or destroyed breaks up the same way: the call to `Fragment` in `BuildingEffect::SafeDestroyConstruction` (0x46a0f1) is chained too,
  its instance the one embedded in the construction (class at its `Info`'s `+0xd0`,
  named by `GameObjectClass::GetOdfName` 0x4ce370), only the faces built so far (the
  call's face count) breaking, in the SOD's order.
- **Construction** (`construction.rs`, `weldfx.rs`, `shaders/effects_construction.hlsl`).
  `BuildingEffect` (vtable 0x6afe4c) taken through `feature()`, in place of
  `BuildingEffect::Render` (0x469660). Stock: `BuildingEffect::m_list` (a `std::map`,
  head node at 0x736bd4, `_Nil` at 0x736c1c, node `+0x14` the `Info`) holds a
  construction per station being built. The `Info`: `+0` the builder's handle, `+4`
  the teams that see it, `+8` its colour, `+0x20` the station's instance (a
  `CraftInstance`, its class at `+0xb8`), `+0xb4` the handle `ApplyScaleEffects` is
  asked for, `+0xc8` the station's faces, `+0xcc` those built so far, `+0xd0` the
  class, `+0xe0` `weldingSpeed` (an edge's progress a second, default 1), `+0xe8`
  `weldingRadius` (the spark's half size, default 1; `weldingSpriteName` is `xspark`),
  `+0x100` the welding edges' list head. `BuildingEffect::Simulate` (0x469490) moves
  the faces built with the build time and, for each new face
  (`AddNewWeldingEdges`, from the faces' corners in the world), adds its edges (the
  list node: start `+0x14`, end `+0x20`, progress `+0x2c` 0..1, erased at 1).
  `Render` draws, while the builder is seen (or its team is one the player sees,
  `Entity::GetTeamsVisibleToUser` 0x4d0070, or the editor is on, or `View_Record` is
  2), when faces are built and `GameObjectInstance::DistanceCull` (0x4d5bb0) passes:
  `ApplyScaleEffects` (0x599760), the frustum test (slot 3), the face limit
  (`SetRenderFaceLimit` 0x62ea80: `+0x10`) and the instance in the builder's team
  colour (`GameObject::GetTeamColor` 0x4d5000, into `Storm3D+0x34`); the construction
  object's on-screen byte (`ConstructionRig::GetConstructionObject` 0x4afe80, `+0x25`;
  `ConstructionObject::Render` 0x4a3790 draws nothing); and one `xspark` sprite per
  edge sliding from its start to its end. Ours makes the same calls (each writes only
  the effect's own instance) and draws the station with the hulls module's look
  (`hulls::draw_instance`), faces limited as stock limits them, and two-sided: the
  engine's limited draw shows the inside of the part-built hull, which culling hid
  (seen on the bench). Each edge is welded as the repair welding welds (Repair.asi's
  look): a flickering arc with turning rays at the weld (on a share of the edges at a
  time, `ArcShare`), a hot spot, sparks streaking off and cooling, and behind it the
  seam, white-hot at the weld and cooling along the edge through the black-body ramp
  to dull red, which goes on cooling once the edge is done (`CoolTime`); a construction gone before
  all its faces were built (cancelled, destroyed: its chunks fly off) takes its seams
  with it. A few arcs per
  construction light its hull (`Lights`). With the hulls module off, or its SOD
  unread, the engine draws it. `[Construction]`: `Enable`, `Brightness`, `Flare`,
  `FlareMinPixels`, `ArcShare`, `FlickerRate`, `Stutter`, `ArcColour`, `HotColour`,
  `HotSpot`, `Seam`, `SeamWidth`, `SeamMinPixels`, `CoolTime`, `Sparks`, `SparkSpeed`
  (times the station's radius), `SparkSpread`, `SparkLife`, `SparkDrag`, `SparkWidth`,
  `SparkMinPixels`, `SparkBrightness`, `StreakTime`, `Lights`, `Light`, `LightColour`,
  `LightReach` (times the station's radius); `ABTest=1` (bench): while
  `Render3D.construction-off` exists in the game directory, construction, the bees and
  the sparks are the engine's, for a comparison at the same moment.
- **Worker bees and drones** (`bees.rs`). `WorkerBee` (vtable 0x6b017c) and
  `DroneColony` (0x6afe98) taken through `feature()`, in place of `BeeColony::Render`
  (0x4688a0). Stock: the colony's bees (its `GetBeeList`, vtable `+0x40`: a multimap,
  `_Nil` at 0x736bc4, the bee at node `+0x10`) are `ST3D_Instance`s of the bee's SOD
  (`workerBeeName` of the construction ship's ODF, `fbee` by default, its database at
  `ConstructionRigClass+0x494`; the drones' from `gdrones`, `DronesClass+0x2f4`),
  drawn by `ST3D_Instance::Render` while `BeeColony::ShouldDrawBee` says so (the owner,
  `+0xbc`, is seen); a bee that welds (`+0x8c` = 4) draws `weldingBeamSprite` (`+0x84`)
  as a ribbon (`RenderAsPolygon`) from itself (`+0x68`) to its spot (`+0xac`),
  `weldingBeamWidth` (`+0x88`) wide. Ours: the bee through `hulls::draw_instance`, lit
  as a hull, its strobes drawn as running lights, its emitters (the drones' steam) fed
  to the particles module (`particles::feed`); the beam as a crackling arc of light
  with the welding's flare, hot spot and sparks at the spot, and a light on the hull.
  A bee is a plain `ST3D_Instance`: its vtable has no `IsCraftInstance` slot (slot 10
  is RTTI data, and calling it hung the game), so `draw_instance` is told. `[Bees]`:
  `Enable`, `Beam`, `BeamColour`, `BeamMinPixels`, `Flare`, `FlareMinPixels`,
  `HotSpot`, `HotColour`, `Sparks`, `SparkEvery`, `SparkLife`, `SparkSpeed`,
  `SparkSpread`, `SparkBrightness`, `SparkMinPixels`, `FlickerRate`, `Light`,
  `LightColour`, `LightReach`, `Brightness`.
- **Sparks** (`sparks.rs`): every particle of `NodeParticleEffect` that is not a weld.
  Stock adds them with `NodeParticleEffect::AddParticle` (0x4733c0 at a hardpoint,
  0x4734c0 at a point in the object's frame; at most 200, 4 per object): a system
  blowing up (`Craft::DamageAlloc`, `Craft::DramaticDeathSequence`: `systemexpl1..7`,
  half size the bounding radius times `SYSTEM_EXPLOSION_SIZE_FACTOR` 0.14, at least 4,
  a second), a phaser's flare at its emitter (`Phaser::Init`, its class's sprite at
  `+0x160` in its colour) and a beam's hit (`Beam::Simulate`, `beam explosion1`).
  `Render` draws each with `ST3D_Sprite::DrawScaled3D` (colour `+0x20`, half size
  `+0x2c`) at its hardpoint (`Entity::GetWorldPosition` 0x4cff70) or its point
  (`+0x10`) in the object's frame; age `+0x34`, life `+0x38`. Ours, while repair.rs's
  thunk is in place: stock's sprite and flipbook drawn as stock draws it (the ordnance
  module's `sprite::Sprites`, its hottest texels past white), and new, a burst of sparks
  streaking off it, cooling (a system's blast more than a weapon's), and a flash of
  light on the hull. `[Sparks]`: `Enable`, `Hot`, `HotFrom`, `BlastSparks`, `HitSparks`,
  `SparkSpeed` (times the sprite's size), `SparkSpread`, `SparkLife`,
  `SparkBrightness`, `SparkMinPixels`, `BurstTime`, `BlastLight`, `Light`,
  `LightReach`, `LightTime`.
- **Nanites**: nothing to take. `NaniteEffect` (vtable 0x6affdc, `Render` 0x473280: one
  sprite flying from an object towards the camera) is never made: nothing in
  `Armada2.exe` calls its constructor (0x472fe0). What a player sees of nanites is the
  `Nanites` ordnance (a `Bullet`, `Submit` through its instance) and the
  `TimedScrambleInterface` it leaves on its target: the ordnance module's and
  `Step::Specials`.
- **Lights.** `Module::lights()` runs once per view after `GameObject_PreRenderAll` and
  `Ordnance::PreRenderAll`, before `scene::begin_view` reads the light list: the module
  registers ordinary `ST3D_Point_Light`s (`operator new` 0x652710, the constructor
  0x62f240, `RegisterLight` 0x62d250), which the engine deletes the records of at the
  view's end, so both the engine's draws and the scene layer see them.
- **Still the engine's**: construction and the bees while the hulls module does not
  run. `Step::Specials` is the `specials` module's.

### ordnance

The `ordnance` module (on by default; `[Modules] ordnance=0` leaves it to the engine)
draws every shot and the warp flash, in place of `Ordnance::RenderAll` (0x58e580,
`Step::Ordnance`) and around `RenderWarpSpriteEffects` (0x59c560, `Step::Warp`). It
supersedes `Specials.asi`, and stands down while that is loaded.

**The loop.** `RenderAll` walks `Ordnance::ordnanceList` (the `std::list` at 0x771fa8,
its head node's pointer at 0x771fac) and submits a shot when its owner (handle +0x5c,
`Entity::Get`, a game object, not +0x113) is not culled for this view (+0x183) and the
player can see it (`CanUserSee`, vtable slot 2), else the same of its target (+0x60),
else when the shot itself is within `cfgOBJECT_CULLING_DISTANCE` (0x6fcac0) of the
camera and visible. Ours makes the same tests in the same order, **and the one write
stock makes there**: a shot whose target has +0x10f set, or whose target has +0x112
while its owner (`GameObject::GetObjEvenIfDestroyed`) has not, forgets its target
(+0x60 = 0). That write is the engine's own behaviour in its render pass; leaving it
out would change the game. A submitted shot is then drawn by, in turn:

1. a **recipe** by its ODF (the class +0x34 keeps its `cPrjID` at +0x150,
   `cPrjID::GetOdfName` 0x6593a0): `Specials.asi`'s beams, bolts and fields, ported
   unchanged (`fx.rs`, `fx_ps` in `shaders/ordnance.hlsl`), on the sim clock;
2. **stock's own look, drawn by us** (`sprite.rs`), for the classes whose `Submit`
   (vtable +0x5c) is `Phaser` 0x57edc0, `PulsePhaser` 0x581cb0, `PhotonTorpedo`
   0x57f100, `PlasmaCannon` 0x57f580, `PoleronTorpedo` 0x5800d0, `Artillery` 0x5730a0
   or `ShieldRemovingTorpedo` 0x5853b0. Those draw the class's sprite (+0x12c) as a
   ribbon (`ST3D_Sprite::RenderAsPolygon` 0x63a810: from the beam's start +0xbc to its
   end +0xc8, the class's width +0x148 wide, turned about its length by
   (a − eye) × (b − eye); a pulse from its head +0xc8 back towards the start, at most
   the class's length +0x144) or as a camera-facing square (`DrawScaled3D` 0x63aa60:
   at the shot's position, `Entity::GetTransform` +0x24, half size +0x148), coloured
   by the shot (phaser +0xf0, photon, poleron and shield-removing torpedo +0xd0,
   pulse and artillery +0xd4; the plasma cannon sets none and keeps the sprite's own
   +0x24). Both end in `ClipAndRender3D` (0x63b4b0); what we read of it: the sprite's
   rectangle +0x38 (u, v, w, h, fractions of the texture), its flags +0x50 (0x10 not
   animated, 0x20 tiled along the ribbon once per its width +0x40), its material +0x54
   (blend factors +0x1c source, +0x20 destination, Direct3D's numbers), its texture
   +0x58 (named at +0x8); the flipbook's frame is what `ST3D_Sprite::Animate`
   (0x63a580) leaves in `m_texture_coordinate_offset` (0x7ad6d0), called where
   `ClipAndRender3D` calls it, so every flipbook shows stock's frame; the colour is
   times Storm3D +0x4, the alpha the device's +0x48. Corners in the order stock gives
   their texture coordinates (u0 v0, u1 v0, u1 v1, u0 v1). The one change: texels near
   white are lifted past it (`Hot=`, `HotFrom=`), so beam cores and torpedo hearts
   bloom by their energy;
3. **the engine's own `Submit`**, called through the vtable, for everything else,
   with its draw of a model ours (below, "The models"): mines, colony pods, the
   commando team, the myotron, the black hole, flux wave, stasis and transwarp
   weapons. The chain pulsar's `Submit` (0x574f20) draws its class's sprite at its
   position (as a torpedo's, in the sprite's own colour) and renders its emitter node
   (+0xdc); while the particles module runs, which fires that emitter itself, the
   sprite is drawn by `sprite.rs` and the `Submit` not called.

The mining, orbital and repair beams (`MiningBeam`, ODF class `minebm`: stock stretches
`wmbeam.sod` and its kin from the ship to its target, `MiningBeam::Submit` 0x57d1e0) are
recipes (case 1): beams flowing back to the ship in stock's colours (dilithium blue,
latinum green, the Borg metal and orbital beams violet, the repair beam orange,
flowing into the hull). A beam ends inside the moon it mines (+0xc8), where a flare
and a light show nothing: `ImpactAtSurface=k` puts its impact flare and its light where
the beam enters the target's bounding sphere (+0x60's object, its instance's radius
times k), so the light falls on the moon's face. The world module's moons take it as
any point light.

**The models.** Every model-drawing class's `Submit` ends by calling `Entity::Render`
(0x4cfda0) through slot 11 of its vtable: the instance's frustum test (instance slot 3),
the on-screen byte (+0x25), the instance's draw (instance slot 5). Before that, some set
the instance's scale (`Entity::EnableScaling` 0x4cff50 writes instance +0x74,
`SetScaleFactor` 0x4cff20 instance +0x28): the mining beam along its length
(class +0x164 across, the length over class +0x160 along), the stasis field to its
target's size (class +0x148 over instance +0x40), the black hole and flux wave by their
growth (class +0x15c times +0xbc, drawn only from 0.01); the transwarp drive draws only
while class +0x154 is set. So the module does not redo any of that: for a class it hands
to the engine, slot 11 of the class's vtable is pointed at `models.rs`'s `ord_render`
(only if it holds `Entity::Render`; checked and logged), and the engine's `Submit` runs
as it always did. While the ordnance step has a shot in that `Submit`, `ord_render`
draws it: the frustum test called as stock calls it, +0x25 set as stock sets it, then the
SOD named by the ODF's `shotGeometry` (whose database must be the instance's plain
`ST3D_Database`) at `Entity::GetTransform` times the instance's scale, posed on the
instance's clock, through the scene layer (lit by the scene's lights; the `team*` groups, a
mine's whole hull, in the owner's team colour (+0x5c): stock tints them with whatever team
colour `GameObject::RenderInternal` last handed the engine (Storm3D +0x34), the last
object drawn as far as the bench shows (a red player's mine showed green); additive meshes
such as the rift's and the transwarp's queued for the flush). Its emitters (the commando
team's and the pods' rockets, the myotron's contrails, the rift's clouds, the flux
wave's plasma) go to the particles module (`particles::feed`, keyed by the shot and its
class) and are drawn with every other particle; without the particles module a model
with emitters is the engine's. A recipe bolt's SOD emitters (the micro-organisms'
contrails) are fed the same way. Anything else (no SOD, another database) falls through
to `Entity::Render`. On top, `[Model.<odf>]` in `ordnance.ini` gives a model a glow at
its heart, an HDR flare pulsing on the sim clock with its own light on the hulls about
it: the antimatter mine's white-gold, the gravity mine's violet, the myotron's green.

**The warp flash.** `RenderWarpSpriteEffects` still runs: `WarpSpriteEffect::Render`
moves each flash onto its craft's line of motion and removes it, which the game
relies on. Its one draw, the call to `DrawScaled3D` at 0x59cc6b, is re-pointed at us
(only if it still calls 0x63aa60): the effect (its handle +0, its point +4, its
distance +0x10) is noted, and the flash drawn after the step as `Specials.asi` drew
it (`[Warp]`), with its afterglow and ring once the engine has dropped the effect.

**The special weapons' waves** are `ShockwaveExplosion`s, which the effects module
draws: it asks `mods::ordnance::draw_wave(expl, view)` first. A wave whose class
(+0x30) names, through its `cPrjID` (+0x24), an ODF with a `[Wave.]` recipe is drawn
there (`ShockwaveExplosion::GetCurrentRadius` 0x467730, its end +0x50, the class's
`fadeFactor` +0xe0, in the plane normal to its transform's second axis) and the call
answers true; any other is left to the caller's ring.

**Light.** At the start of each view, after `Ordnance::PreRenderAll` has registered
every torpedo's and pulse's light: their records in the registry (Storm3D +0x60:
record +0 the light, +4 its colour) take the hue of their sprite's first frame,
scaled to the ODF colour's peak (the class's light +0x10, its colour +0xf4), which is
`Lighting.asi`'s recolouring; every visible phaser (vtable 0x6b8ee4, not expired
+0x27, visible +0x24) gets an emitter and an impact light in its texture's colour
times its tint, up over 0.06 s and down over its last 0.25 s (time left +0xac of the
class's `lifeSpan` +0x1c); and the special weapons' and the warp flashes' lights of the
last frame are registered. New lights are `ST3D_Point_Light`s made once with the
engine's constructor (0x62f240) and registered with `RegisterLight` (0x62d250), so a
hull the engine draws takes them too; the scene's list is then read again. With
`Lighting.asi` loaded, it does the recolouring and the phaser lights itself.

**Settings.** The recipes and `[Warp]` are `render3d/ordnance.ini`, built into the
plugin; a section or key of the same name in `Render3D.ini` overrides it. The
module's own keys, in `Render3D.ini` `[Ordnance]`:

| Key | Default | |
|---|---|---|
| `Stock` | 1 | 0: stock's sprites are drawn by the engine's `Submit` |
| `Specials` | 1 | 0: no recipes for shots; they fall to stock |
| `Waves` | 1 | 0: `draw_wave` draws none; the special weapons' waves are drawn as any shockwave |
| `Hot`, `HotFrom` | 0.6, 0.7 | how far past white a sprite's hottest texels go, from which level |
| `Recolour` | 1 | torpedo and pulse lights in their sprite's colour |
| `PhaserLights` | 1 | the phasers' emitter and impact lights (`PhaserBrightness`, `PhaserStart`, `PhaserRange`, `PhaserLift`, `PhaserImpact`, `PhaserImpactStart`, `PhaserImpactRange`, `PhaserImpactLift`: `Lighting.ini`'s) |
| `Models` | 1 | 0: the shots that draw a model are drawn by the engine (no vtable slot is taken) |
| `Survey` | 0 | log every ordnance ODF the first time it shows, with its vtable and `Submit` |

### specials

`src/mods/specials/`, `shaders/specials.hlsl`, on by default (`[Modules] specials=0`
leaves it to the engine). `Step::Specials`: the special-object effects, which stock
draws in `SpecialObjectEffect::RenderAll` (0x60be90, called at 0x598372).

**What stock draws there.** `RenderAll` walks `SpecialObjectEffect::mAllEffects` (its
head node pointer at 0x7a8c0c; each node's +8 is an effect) and calls each effect's
`Render(cam)` (vtable slot 0). The base `SpecialObjectEffect::Render` (0x49ea70) draws
nothing (`ConvertEffect`, `Disable`, `SlowEffect`, `VampireEffect`...).
`TimedSpecialObjectEffect::Render` (0x60c460, every `Timed*` class, `BoardingParty`,
`LineOfSightEffect`) draws the `ST3D_Instance` embedded at +0x2c only when the weapon
gave it a database (+0xac, the instance's +0x80) and a visual target (+0xb0): the
instance is put at the target's transform (`Entity::GetTransform`) and rendered.
`TimedChangeTeam::Render` (0x60e090) also wants a node (+0xc8) and a target that is a
craft (flags bit 8) and not expired (+0x27), and puts it at that node of the target
(`Entity::GetWorldTransform`, 0x4cff90). Three weapons give an effect a database, each
from its ordnance class: `ComputerOverride::Hit` (a `TimedChangeTeam`, class +0x190),
`MicroOrganism::Hit` (a `TimedDrain`, class +0x188) and `PsionicDisruption::Simulate`
(a `TimedCrewDisruption`, class +0x160): the ordnance ODF's `particleArt`, a SOD that
is a single cloud emitter (`wovremit`: `override`, `wmicremit`: `microorg`,
`psionicdisruption`: `psiondis`; Cardassian `microorgs` names `wmicremi2`, which no
SOD is). Everything else in the list draws nothing in this step: the boarding parties
(their transporter shimmer is the billboard effect, the particles module's), the
hologram (`TimedHoloEffect` keeps its own instance at +0xb4, drawn from
`GameObject::RenderInternal`) and the rest of the timed effects.

**What the module does.** Once a frame (in `lights()`, after the particles module has
simulated) it walks the same list. For each effect stock would draw it finds the SOD:
the effect's weapon class (+0x4) holds its ordnance class at +0x18
(`OrdnanceClass::Find` of the weapon's `ordName`, in `WeaponClass`'s constructor), whose
cPrjID (+0x150, `cPrjID::GetOdfName` 0x6593a0) names the ordnance ODF; and hands the
instance's emitters to the particles module at stock's placement (`particles::feed`),
so the cloud is our particles, drawn at `Step::Translucent` in stock's look. The step
then draws only what it cannot take: an effect of a class whose `Render` is not one of
the three above, or whose SOD does not load, is rendered by the engine (its own
`Render`), as stock. Without the particles module running the whole step is the
engine's.

**The looks** (`look.rs`, `specials.hlsl`), new, on top of stock's cloud, for as long as
the effect is listed (effect +0x28: seconds left; +0xc: finished): a layer of light
over the hull of the object the effect is on (+0x8, if it is still in the object list,
not expired and seen by the player; posed by the hulls module, `hulls::posed`; a
cloaked hull gets none), additive, depth-tested, pushed out a little along the normal,
in the object's own frame so it stays on the hull; and a point light at the cloud. Up
over `Rise` seconds, down over the kind's last `Fade` seconds; everything above 1 blooms.

- **Computer Override** (`TimedChangeTeam`): green Borg circuitry (traces laid out per
  face of the object's box, nodes, pulses running along them) creeping over the ship
  from the side the Borg ship is on, with a bright front. Stock's cloud of coloured stars
  sits on the Borg ship while the beam flies and on the victim after the hit.
- **Energy Dissipator** (`TimedDrain`): a pink film of veins and breathing blobs, in the
  spores' colour.
- **Psionic Disruption** (`TimedCrewDisruption`): violet interference ripples. The
  effect is renewed every half second while the beam holds, so it fades over its last
  quarter second only.

| Key (`[Specials]`) | Default | |
|---|---|---|
| `Enable` | 1 | 0: the engine draws the step (read again every 2 s with `Reload=1`, for comparing at the same moment) |
| `Overlay`, `Lights` | 1 | 0: no layer over the hull / no light |
| `Survey` | 1 | logs each effect class and name the first time it is seen |
| `Rise` | 0.4 | seconds the looks take to come up |
| `Push` | 0.004 | how far the layer sits off the hull, in radii |
| `RimPower`, `CoreWhite` | 2.5, 0.35 | the layer's rim; how much its hot cores whiten |
| `<Kind>` | 1 | `Override`, `Drain`, `Psionic`: 0 turns that kind's look off |
| `<Kind>Colour`, `<Kind>Gain`, `<Kind>Strength` | | the layer's colour (0..255), times the gain (light), and strength |
| `<Kind>Creep`, `<Kind>Scale`, `<Kind>Rim`, `<Kind>Fade` | | seconds to cover the hull, pattern cells per radius, rim, fade-out seconds |
| `<Kind>LightColour`, `<Kind>LightBrightness`, `<Kind>LightRange` | | the light at the cloud (range in the ship's radii) |
| `LogEvery` | 600 | frames between log lines |

Seen on the bench (2026-10-15, `specials` scene, 1280x720), against stock at the same
moment (`Enable=0` with `Reload=1`): the clouds sit where the engine puts them; the
layers come up on the Sovereign as described, main view and with the HUD's inset.
Scenario `render3d-specialfx`.

### sky

Takes over `Step::Background`, the whole of `Background_Render` (0x590d90), and
supersedes `Sky.asi` (it stands down while that plugin is loaded). In stock's order:

1. **The far plane.** `ST3D_Camera::SetFarPlane` (0x6198a0) to `STAR_FAR_CLIP_PLANE`
   (0x6fcac8, 80000) for the step and back to `FAR_CLIPPING_PLANE` (0x6fcab0) after, as
   stock does. That changes only the camera's CPU projection, so our draws use the
   device's projection with its far plane moved out to the same distance: under the
   scene's far plane the starfield (10000-20000 out) would be clipped.
2. **The sky**, when the view shows one (`Render_Stuff` +4, and the detail
   configuration `Storm3D+0x104` +0x2c not 2) and the map has a sky database
   (`g_background_geometry_db`, 0x7720fc). The map's background name is the lower-cased
   string `Starfield_Load_Background_Geometry` (0x590c30) keeps at 0x738538:
   `<name>.sod` or a bare prefix.
   - **With a recipe** (`render3d/skies/<name>.ini`, built into the plugin; a
     `Sky\<name>.ini` in the game directory takes precedence and is re-read when it
     changes), the procedural sky of `Sky.asi` (its settled looks kept: Aqua
     dimmer and softer than stock, the grades, soft billows and no filaments; "Sky recipes" below): one quad over the
     viewport carrying each corner's world ray (from the inverse of view x projection,
     far point minus near point, which is affine in screen position), sampling a cube
     baked once per recipe. The bake is on the GPU only: each face is drawn by
     `bake_ps` at twice its edge (Sky.asi's texel mapping, edge texels on the edges),
     then `down_ps` averages each 2x2 with one bilinear tap into the face of a
     half-float cube render target. The cube is in the default pool: dropped before a
     `Reset`, baked again after. The gas is saturated as before; the recipe's stars are
     added after that, so a star core may pass white (to 2) and bloom a little.
     `fine_z`'s noise volume is `Sky.asi`'s, made the same way (managed pool).
   - **Without one** (`Mbgstars`, or `Recipes=0`), the stock faces, unlit, at the
     camera, depth off. A cube SOD's meshes through `scene::model`, culled as the
     engine culls them (`mbg02.sod` holds two cubes, and only the one wound inwards
     shows). A prefix sky as `CreateBackgroundFace` (0x590f10) builds it: face `i`
     shows `<prefix><i>` on a quad at +-100 about the camera, texture coordinates
     (0,0) (1,0) (1,1) (0,1) at its corners, clamped (`ClampTexturesRecursive`). A face
     whose texture is missing draws white, as the engine's does.
   - Otherwise a black clear of the view: stock's only colour clear.
3. **The starfield** (`Render_Stuff` +8; off in the HUD's action camera), read every
   frame from the engine's own: `g_starfield` (0x773d04) +4 is the root
   `OctTree_Node`, whose objects (+0x30) and children (+0x3c) are VC6 `std::list`s of
   `auto_ptr`s (the pointer at list node +0xc). A star (`Starfield::mCreateStarSprite`
   builds them once, at random from `g_asyncRandom`, so every session differs) holds
   its `ST3D_Sprite*` at +0 and its quad's four corners in world space at +0xc, which
   `OctTree_Node::Render` hands to `ST3D_Sprite::DrawQuad` unchanged. The sprite's name
   (database element +0x8) finds its texture and rectangle in the sprite table; its
   colour is at +0x24. Added whole (the texture's alpha plays no part), as stock's
   additive material. The point stars (`mstars`) are lifted past white near their
   cores (`StarLift`); the blobs and clusters are not.
4. **The background objects**: every object with flag `0x10000` (the
   `BackgroundObject` constructor sets it; the object loop skips them), unless its
   class is `hidden` (+0x19c; `dlight`) outside edit mode (`UserProfile` 0x76b5ac,
   +0x84). Stock ones are `background_obj`s (`BackgroundObjectClass`: the suns
   `Msun1-3`, `mwreck`, `mnebula`, `mgate`), drawn by `GameObject::Render`. Ours: the
   SOD's meshes now, lit, all of them (nothing may wait for the flush: the depth is
   cleared before the scene), and its sprite nodes as billboards. A sun is one
   sprite node (`sun1..3` on `Mmoon`, half extent 200): `ST3D_SpriteNode::RenderInternal`
   (0x63c300) colours it by the node data's colour times `s_diffuseColour` (white
   outside the nebulae) and sizes it by `m_billboard_render_scaling_factor` (1). The
   brightest texels go past white (`SunLift`). An object whose class overrides
   `Render`, or whose SOD does not load (`mgate.sod` is version 1.2), is the engine's.
5. The device's `Flush` (anything the engine queued), the far plane back, and
   `ClearDepthBuffer` over the view for the scene.

`Render3D.ini`, `[Sky]` (read again every 60 views):

| Key | Default | |
|---|---|---|
| `Draw` | 1 | 0: the engine's `Background_Render` (for comparing on the bench) |
| `Recipes` | 1 | 0: the stock faces where a recipe exists |
| `Face` | 1536 | the baked cube's edge; 0 computes the sky at every pixel (the reference) |
| `Reload` | 1 | re-read `Sky\<name>.ini` in the game directory every 30 views when it changes |
| `Dump` | 0 | 1: each bake writes its faces to `Sky\<name>.dump0..5.tga` for `perf/sky-grade.py` |
| `StarLift`, `SunLift` | 0.5, 1 | how far past white the brightest texels of the point stars and of the suns go (0: stock's light) |
| `Load` | | bench only: a background name to load in place of the map's (`Starfield_Load_Background_Geometry`), once per new value |

Seen on the bench (2026-10-15, `a2_borg01`, 1920x1080), each against the engine's own
draw in the same session and frame (`Draw=0`, paused): the stock faces of `mbgaqu`,
`mbgkling.sod`, `mbgpur`, `mbgbaku.sod`, `mbgborg.sod` and `mbgstars` match (sky regions
equal to 0.1/255; `mbgborg` means equal to 0.05), the starfield matches with
`StarLift=0`, two spawned suns match to 0.2/255 with `SunLift=0`. `mbg02.sod` (and
`mbgblue.sod`, the same model) is 2-5% brighter than the engine's: its material is lit
(model 1), and the engine lights the sky with the scene lights; ours is unlit. The
recipe skies match `Sky.asi`'s away from the (random) starfield blobs. The HUD's action
camera draws the sky without stars, as stock.

**Sky recipes** (`render3d/skies/<name>.ini`, section `[Sky]`, one per stock sky name;
built into the plugin with `include_str!`, so the sky needs no file in the game
directory; a `Sky\<name>.ini` there takes precedence). Colours are 0-255; directions
are yaw (degrees about +y; 0 looks along +z, 90 along +x) and pitch (up from the
horizon). Each recipe's header says which maps use it and the stock channel means it
aims at.

| Key | Default | |
|---|---|---|
| `Seed` | 1 | moves the noise domain: a different sky with the same character |
| `Scale` | 2 | the largest structure's frequency over the sphere; 2-3 gives a few billows per cube face |
| `Warp` | 0.4 | domain warp; over ~0.5 it drags streaks |
| `Detail` | 0.5 | amplitude ratio between octaves: lower is softer |
| `Coverage` | 1.0 | where the gas begins, in units of the field's spread: higher is emptier |
| `Softness` | 2.0 | the width of the gas's edge in the same units; 2-3 is cloud (a wider one restores depth where dense skies saturate into plateaus) |
| `Patchiness` | 0.5 | a one-octave field at a third of `Scale` that gathers the gas in parts of the sky |
| `Ridge` | 0 | 0-1: towards soft bands |
| `Gamma` | 1 | on the density: above 1 thins the faint gas |
| `HueScale`, `HueMix` | 2, 0.5 | how sharply the two gas hues separate; 0: hue by region, 1: hue by density |
| `Brightness` | 1 | the whole sky |
| `Dither` | 1 | the per-pixel dither, in steps of the 8-bit back buffer |
| `Deep`, `GasA`, `GasB`, `Glow` | 0 | r, g, b |
| `Core1`, `Core2` | 0,0,10,0 | yaw, pitch, size (degrees), gather (field units); `Core1Colour`, `Core2Colour`: the core's own glow |
| `Band` | 0,90,20,0 | yaw, pitch of the band's pole (0,90: the horizon); half-width (degrees); gather (0: no band) |
| `Stretch` | 0,90,1 | yaw, pitch of the axis; how far structure is drawn out along it (1: none) |
| `Grain`, `Dust`, `Haze` | | detail: grain (amount, scale times `Scale`, gain between octaves), dust (amount, scale, coverage), haze (amount, coverage); amount 0 is off |
| `Stars` | 0,1,0.5,0 | how many over the sphere, brightness, size (degrees), in-gas weight; `StarA`, `StarB`: the two ends of the colours |
| `GradeGain`, `GradeCurve` | 1,1,1 | the grade per channel, `gain * in^curve`, after `Brightness` and before the stars |
| `Fine` | 0.1,0.3,0.55 | detail finer than the cube, at draw time: amount (0: off), the largest octave's size (degrees), gain between octaves. The cube is magnified about 3.4x at 4K, so finer detail comes from here, not from a bigger cube |

Edit a recipe in the game directory's `Sky\` with the game running and it re-bakes
within half a second (`Reload=1`). **The grade** is fitted, not hand-tuned:
`perf/sky-grade.py` fits `GradeGain`/`GradeCurve` per channel by least squares through
the log percentiles 50..99.5 of the bake against the stock faces (the stock faces'
40-texel corner notches masked), then scales every gain together so the sky lands at
0.92 of stock's luminance, a little under and never over. Aqua is deliberately left
ungraded (dimmer and softer than stock, as the user prefers) and `mbgrg` gets its
channel balance only, at 0.57 of stock's luminance. Refit after changing a palette or
`Brightness`; its docstring has the procedure (`Dump=1` writes the faces it reads).

### overlays

`src/mods/overlays/`, `shaders/overlays.hlsl`; on unless `[Modules] overlays=0`, and
standing down while `Fog.asi` is loaded (it supersedes it). It draws what the engine
draws over and about the map rather than as objects, and nothing it does is written
where the simulation reads. Settings in `Render3D.ini`, `[Overlays]`.

**`Step::Terrain`** (`Terrain_Geometry::Render`, 0x593200, on `g_terrain_geometry`
0x772140) is run as stock runs it. Until the engine's first call has made its meshes
(`Terrain_Geometry::m_initialized`, 0x772180) the step is the engine's, and so is the
whole step in the tactical view (`View_Record` 1) when the profile's shroud option
(`UserProfile+0x45`, 1 or 2) has the shroud drawn as a `TreeGrid` (`+0x28`), a path
not seen on the bench. In order:

1. the build footprints, the engine's own sprites
   (`PlaceholderFeature::RenderFootprints` 0x473930 on `placeholderFeature` 0x736d50,
   at the grid's height `+0x1c`);
2. the tactical grid, when `GridVisible()` (0x51e180, the current `View_Record`'s
   `GridRenderState`) says so: one line every `GRID_SIZE` (0x71a78c) across the map's
   rectangle (`+0x4` x0, `+0xc` z0, `+0x10` x1, `+0x18` z1, `+0x1c` y; `+0x20` + 1
   lines along z and `+0x24` + 1 along x, summed a step at a time as stock sums them),
   in `ST3D_Colour_Dark_Orange` (0x7a8ee8), depth-tested and written as stock's
   `ClipAndRenderLine` draws them. `Grid=0` calls the engine's `DrawGrid` (0x593480);
3. the fog of war, unless the map editor is on (`UserProfile+0x84`), only in the RTS
   view (`View_Record` 0) and when the game has fog (`GameSetup::isFog` 0x5462f0);
4. the shroud over it, under the same conditions with `GameSetup::isShroud`
   (0x546300).

**The fog and the shroud** are `Fog.asi`'s look (the cubic mask, the
pixel-wide smoothstep edge, stock's rim, the taps held inside the grid, which keeps the
shroud from thinning along the map's border; `Filter`,
`FogThreshold`, `ShroudThreshold`, `FogSoftness`, `ShroudSoftness`, `FogRim`,
`ShroudRim`, `RimWidth`, `Antialias` keep its keys and defaults), drawn on a quad of
our own instead of through the engine's draw. The quad is the map's rectangle at the
grid's height, with the UVs `Terrain_Geometry::SetVertexPositions` (0x592e10) gives the
engine's mesh: the map centred in the texture's unit square with square texels, v
running against z. The textures are the engine's: `fogTexture` (`+0x34`) and
`shroudTexture` (`+0x38`), `ST3D_Texture`s whose per-device texture sits at `+0x40` +
the device index (`Storm3D+0xc0`) times 4, and whose `+4` there is the
`IDirect3DTexture8`, which d3d8to9 answers for its `IDirect3DTexture9`. They are never
resized (one texel per scan cell; resizing them froze the game). The grid in them is
`Scanner::m_fog` / `m_shroud` (0x738a54 / 0x738a58: width `+0xc`, height `+0x10`),
centred. Stock's states: the depth test always passes and nothing is written; the fog
blends with an opacity of 0.7 (its texture material `fog` is made 0.3 translucent:
`ST3D_TextureMaterial+0x24` = 1 - 0.3, read into the device's material opacity), the
shroud is opaque and alpha-tested, here blended so its edge can be soft. On the bench
(`SCENE=fog`, 1920x1080, against `Fog.asi` at the same moment) both layers read the
same grey levels to the unit, inside and at the edge.

**`Step::Selection`** (`RenderSelectionEffects` 0x599900, main view only, not in a
cineractive): the selection bubbles, from `SelectionEffect::sm_EffectHash` (0x773c88;
16 buckets at 0x773c8c chained by `+0x9c`), read and never written. An entry is an
instance of `select.sod` with `+0x84` its reasons (1 the player's selection, 2 a
co-player's, 4 under the pointer), `+0x88` the object's handle (`Entity::Get`
0x4cfff0), `+0x8c` the damage pulse's phase (which `SimulateSelectionEffects` advances)
and `+0x90` its base scale (the shield ellipsoid, each semi-axis at least 15, times 1.2
over the sphere's radius), times `Entity::GetScaleFactor` when `IsScaled`. The colours
are `SelectionEffect::Render`'s (0x599e80): relation to the player's team on a craft,
(0.7, 0, 1) enemy, (0, 0.7, 1) neutral, (0.6, 0.6, 0.6) ally
(`GameObject::GetTeamRelation` 0x4d1080 against `Transport::getUserTeam` 0x5578b0),
times `0.25 + 0.2 (1 - health) sin(phase)`; on anything else the team colour times
0.25; a second shell at 1.1 times the scale in the team colour (full under the pointer,
else times 0.25), all times `UserProfile+0x48`; a co-player's selection blinks on wall
time. The material `selection` is constant: the colour is added as it is, depth-tested,
not written, front faces only (on the bench stock's bubble adds one layer of its
colour, not two). Seen on the bench: own ships grey with the team shell, an enemy
Klingon violet with a red shell, the hover shell on a cube in its team colour, each
matching stock. Stock stretches a warping craft's bubble with its hull
(`WarpInstance`); not done here yet.

**Ghosts** (the feature `Ghost`, vtable 0x6aff8c; `Ghost::Render` 0x472cc0):
`Ghost::ghostList` (its node pointer at 0x736c94) holds a spirit per object the player
has seen and no longer sees: `+4` its class (`GameObjectClass::GetOdfName` 0x4ce370
names the SOD, as for a hull), `+0xc` its team (`Team::GetTeam` 0x496340, colour at
`+8`), `+0x10` a copy of its instance (transform `+0x44`, position `+0x68`, scale
`+0x28`, visible database `+0x80`). Stock draws each (not in a cineractive or the
editor) with the lighting material `Ghost+0x24` in place of every mesh's own
(`light.odf`: ambient and diffuse 0.1 at the material's `+0x18` and `+0x24`) and the
team colour times `ghostTeamColorFactor` (`Ghost+0x28`) on team faces, culled by
`GameObjectInstance::DistanceCull` (0x4d5bb0, which writes only the spirit's own
instance), opaque: on the bench a ghost is near black and hides what is behind it.
Here: texture times (the material's ambient + its diffuse times the directional
lights), the team faces tinted, self-illuminated texels lit as on a hull (the windows
show on stock's ghosts too). A spirit that does not show a plain SOD (a planet's, an
asteroid field's, a nebula's) is left to its own `Render`, under stock's state
(`Storm3D+0xa8` the material, `+0x4` 0.5, `+0x34` the tint).

**The build ghost** (the feature `PlaceholderFeature`, vtable 0x6b005c;
`PlaceholderFeature::Render` 0x4738a0): taken here and handed to the hulls module
(`hulls::placeholder`, "The build ghost" under hulls), in the main view only as stock
draws it: the class `+0x34`, where it would stand `+0x38` (a `Matrix34`), its colour
`+0x68`, and `+0x74` whether the placement is clear (set: the footprints' `valid_build`
sprite; clear: `invalid_build`, and stock lights the ghost with the dark red ambient at
0x736d40 in place of the scene's). With the hulls module off or `[Hulls] BuildGhost=0`
the feature is the engine's.

**Height posts** (`posts.rs`, `vs_line`/`ps_line` in `shaders/overlays.hlsl`).
`DrawHeightPost(const Vector3&, ST3D_Camera*)` (0x593840, cdecl) draws a line from a
point straight to the grid's plane (`Terrain_Geometry+0x1c`, 0x77215c), in
`ST3D_Colour_Green` (0x7a8f68) when the point is above it and `ST3D_Colour_Red`
(0x7a8f78) when not, through `ST3D_Camera::ClipAndRenderLine` (0x618590): a pixel wide,
aliased. Its five calls are re-pointed (each only while it still calls 0x593840):
`PlaceholderFeature::sRenderInternal` (0x473bbc; the hulls module's build ghost notes
its post directly), and from `DisplayInterface::PreRenderAll`, inside the body, the
overview's `mNormalModeRender` (0x522080, the object under the pointer), `mRenderWhere`
(0x5222fd, the move target), `mDrawPosts` (0x524c02) and its one-point helper
(0x524c2d). While one of our views runs a post is noted and drawn at the flush; outside
one, the engine's runs. Ours: a quad about the segment in screen space, `HeightPostWidth`
pixels wide at 1080 lines (scaled with the view's height), covered by the distance from
its centre line with a soft edge `HeightPostSoft` pixels wide, so it is as crisp at any
resolution; the segment is cut where it passes behind the eye; depth-tested, not
written, blended. At most 4096 posts a view.

| Key (`[Overlays]`) | Default | |
|---|---|---|
| `HeightPosts` | 1 | 0: the engine's `DrawHeightPost` |
| `HeightPostWidth`, `HeightPostSoft` | 1.5, 1 | the line's width (pixels at 1080 lines) and its soft edge (pixels) |
| `HeightPostBrightness` | 1 | the colour times this (above 1 it blooms) |

### particles

#### particles

`src/mods/particles/`, `shaders/particles*.hlsl`, on by default; stands down while
`Venting.asi` is loaded. Our own particle simulation and drawing in place of the
engine's, after `spec-particles.md` in the notes; ports `Venting.asi`'s look for the
vents (the look settled with the user, and the settings keep its keys, in
`Render3D.ini [Particles]`).

- **Who owns which emitters.** Every game object in `GameObject::objectList` whose SOD
  (named after its ODF) has emitter nodes: the damage vents, the drones' steam, a gate's
  or a colony pod's emitters. Each frame the SOD is posed with `scene::pose` at the
  object's transform scaled by its instance scale and the ODF's `ScaleSOD`, and each
  emitter fires from its node's world matrix (spray: one particle per frame at most,
  the node's +Z tilted by a random pitch and yaw within the spread; cloud: the whole
  list topped up to its count, with stock's per-frame drift taken at `CloudFps=30`).
  The flight axis is a unit one: the engine puts `ScaleSOD` into the model's positions,
  not into its node matrices, so a plume is speed × life long whatever the ship.
  Simulated once per displayed frame on the sim clock, for every object whether drawn
  or not: stock's double ageing in the action camera and its freeze of an off-screen
  object's particles are not reproduced. A destroyed object's particles go with it.
- **Damage nodes.** A craft shows or hides its damage nodes per system from the bytes at
  `+0x9c..+0xa1` of its `CraftInstance` (vtable 0x6b3e2c), which the instance's `Update`
  (slot 14, 0x4cb390) writes from the craft's systems and `CraftInstance::RenderInternal`
  (0x4cb0e0) applies: a node is hidden while its byte is set, except `borg`'s, shown
  while it is. The node of a system is the first in the SOD whose name is the engine's
  (`CraftClass::InitializeDamageNodes`, 0x4bfc70: `shields`, `engines`, `target`,
  `sensors`, `life`, `borg`), so a node named otherwise is never hidden, as in stock. An
  object a module draws in the engine's place never gets its `Update`, so the module
  calls it through the vtable (it only reads the craft; the bench's `Scene.asi` wraps
  it for `Vent=`). While the instance shows another database than its class's
  (`+0x80` against the class's `+0x1dc`), none of its emitters fire.
- **The global list.** The chain pulsar (vtable 0x6b7c74, its `crpulsar` emitter at
  `+0xdc`) and the Manheim effect (0x6b89fc, `gemini` at `+0xd4`), found in
  `Ordnance::ordnanceList` (0x771fa8): their emitters fire at the shot's transform and
  their particles outlive the shot, as the engine's global list's do.
- **No double draw.** `ST3D_DeviceDirectX8::RenderParticle` (vtable 0x6bc6ac slot 50,
  0x6262f0) is chained: while one of our views runs, a particle the engine hands over
  while its current instance (`Storm3D+0x100`) is an object we own, or none (the global
  list), is dropped. With the hulls module on the engine never renders those instances;
  with it off, it renders them and simulates their lists as before, and only their draw
  is dropped. The special-object effects' clouds are fed by the `specials` module.
  Every other instance's particles (the emitters on weapons' SODs: contrails, rockets,
  rifts) are still the engine's, drawn into our target. A module that draws such an instance in the engine's place calls
  `particles::feed` with it every frame instead. (The freighter's `latinum_spray`,
  which `Freighter::RenderInternal`, 0x4b22c0, looks up by name, is in no stock SOD.)
- **The look.** Drawn at `Step::Translucent` as one batch per texture and material
  through a `DynVb`: quads parallel to the image plane, half extents = the sprite node's
  size plus growth × age (times the instance's scale), the node data's colour animation
  and the sprite's flipbook on the particle's age, additive. Each particle's colour is
  multiplied by 1 + the emitters its owner showed this frame, which is what stock's
  repeated hand-off of the whole list by every emitter node adds up to. The vents
  (`ventplas`, `ventgas`) are drawn as `Venting.asi` drew them (`vent.rs`,
  `particles_vent.hlsl`): billowing puffs that cool, linked into one trail behind a
  moving ship, a glow and a jet core at each vent, and a light on the hull.
- **Vent lights.** One `ST3D_Point_Light` per vent (up to 16; `operator new` 0x652710,
  the constructor 0x62f240), registered for each view with
  `ST3D_GraphicsEngine::RegisterLight` (0x62d250) so the hulls the engine draws take it,
  and added to the scene layer's list for ours.
- **Billboard effects** (the transporter shimmer and the scripts' `AddBillboardEffect`):
  the node data at `GameObject+0x3c`, drawn at the object's bounding sphere
  (`Entity::GetBoundingSphere`, 0x4cfd70) centre pulled towards the eye by its radius,
  that radius in half size. The engine's call to `RenderSpriteNodeEffect` (0x62ebd0) in
  `GameObject::RenderInternal` (0x4d544c) is re-pointed at a stub that skips it while
  our view runs.
- **Nebula puffs.** `object()` takes nebulae (vtable 0x6b148c) and latinum nebulae
  (0x6b132c, drawn by the plain `GameObject::Render`, which is why the module is
  registered before hulls) whose class has no volume recipe (`Nebulae\<odf>.ini` while
  the world module or `Nebulae.asi` runs): their SODs' sprite nodes as additive
  billboards (half extents) or, for `lightning#`, quads of the full size in the node's
  XY plane, coloured by the class's animation on absolute sim time. `ScaleSOD` spreads
  the nodes but does not grow the quads. `Nebula::sCullOccludedNebula` still runs in the
  step body: its hidden flag at `+0x1b0` skips a nebula, its intensity at `+0x1d0`
  dims it, times `GameObjectInstance::ComputeFadeOut` (0x4d5a20) when it reports a near
  fade, as `NebulaInstance::ComputeFadeOut` (0x4a5680) does.

| Key (`[Particles]`) | Default | |
|---|---|---|
| `Enable` | 1 | 0: the engine draws its particles, as without the module (read again every 2 s with `Reload=1`, for a comparison at the same moment) |
| `CloudFps` | 30 | the frame rate a cloud particle's per-frame drift is taken at |
| `EngineLights` | 1 | 0: the vents' lights only for the hulls we draw |
| `LogEvery` | 600 | frames between log lines |
| `Plasma`, `Gas` | 1 | 0: that kind of vent is the engine's sprite, stock's look |
| `Plasma<K>`, `Gas<K>` | | the vent look, one key per kind: `Brightness`, `Size`, `Stretch`, `MinPixels`, `Flicker`, `Cooling`, `Glow`, `GlowSize`, `GlowMinPixels`, `GlowColour`, `Jet`, `JetLength`, `JetWidth`, `Light`, `LightColour`, `LightReach`; also `PlasmaHot`/`Mid`/`Cool`, `GasBright`/`Cold`, `BillowScale`/`Flow`/`Contrast` and `EdgeSoftness`. Defaults in `Cfg::read` (`vent.rs`) |

### hulls

#### hulls

`src/mods/hulls/`, `shaders/hulls.hlsl`, on by default; supersedes `Lighting.asi`'s
hull path (and stands down while a stale `Lighting.asi` is loaded). Settings in `[Hulls]`
(Lighting.asi's keys and settled values: `Ambient`, `Specular`, `SpecularPower`,
`RimLight`, `RimPower`, `HullSun`, `HighlightKnee`, `SelfIllumination`, the `Borg*`
profile, `PointLights`, `NearFade*`, `Cloak*`, plus `Shadows`, `ShadowSize`,
`ShadowStrength`, `PlanetShadows`, `BumpStrength`, `SpriteNodes`, `AdditiveMeshes`,
`TeamColours`, `Scrap`) and the scene lights in `[Lights]` (`Replace`, `KeyColour`,
`KeyAxis`, `FillColour`, `FillAxis`, `SkyLight`).

**What it claims.** An object whose class's `Render` (vtable slot 11) is
`GameObject::Render` (0x4d54e0), `Shipyard::Render` (0x4bb900, Starbase too),
`MiningStation::Render` (0x4b44a0), `ConstructionRig::Render` (0x4af9b0) or
`Scrap::Render` (0x4a58e0), with a `RenderInternal` (slot 22) of `GameObject`
(0x4d52b0), `Craft` (0x4c8630) or `Freighter` (0x4b22c0), whose instance (+0x80) draws
a plain `ST3D_Database` (vtable 0x6bc56c) that is its class's visible database
(`GameObjectClass+0x1dc`); and the Evolver (`RenderInternal` `Evolver` 0x4b1170, "The
Evolver" below, `[Hulls] Evolver=1`). Anything else (planets, other databases) is
left to the engine. In `object` the module does what `GameObject::Render` does around
the draw: expired and unseen objects draw nothing; an Evolver's cocoon first ("The
Evolver"); a hologram (+0x174) in place of the hull ("Holograms"); +0x26 suppresses the
draw; a rig's next station as a build ghost for the player's team ("The build ghost");
the instance's
frustum test (instance slot 3, as `Entity::Render` makes it) is called, not redone,
because for a craft it also applies the scale effects into the instance's scale
(+0x28, valid while +0x74); a pass sets the on-screen byte (+0x25) the HUD reads. Those
are the only writes; the instance's `Update` (slot 14) is called as stock calls it
before it reads the damage bytes.

**Posing** (`begin_view`). Every object the object loop will reach is posed for the
view (the scene layer's `pose`, on the instance's clock: start +0x18, flags +0xc), with
the nodes the engine shows or hides per instance by writing into the shared database
applied to this pose instead: `hardpoints` never; `lights`/`light` at detail level 0
(`ST3D_GraphicsEngine_Configuration+0x40`) or while the instance's +0x90 says so; the
damaged systems' nodes (`CraftClass::InitializeDamageNodes`: class +0x40c/+0x418/+0x410/
+0x414/+0x41c against the instance bytes +0x9c..+0xa0) hide their children; the crew
fires (class +0x408) shown one more for each fifth of the crew lost (`+0xa4`, as
`CrewDamageTraverse`, children in the engine's reversed order); `borg` (class +0x420)
only while assimilated (+0xa1, which also swaps every mesh to its second texture as
`CraftClass::SetBorgMeshTextures` does); the shipyard's build node (class +0x4a4, shown
while +0x2b8 is not `NO_ENTITY`) and the mining station's dock node (class +0x4b0).
`hulls::posed(o)` gives other modules this view's pose of an object (every node's
world matrix and whether it is shown).

**The look** (`hulls.hlsl`). `Lighting.asi`'s: ambient, the scene's directional lights
with the Key times `HullSun` and taken away by shadows, Blinn-Phong specular weighted by
the texture's brightness, the rim light, self-illuminating groups folded in by the
texture's alpha, team colour on `team*` groups (the team's colour, `Team::GetTeam(+0xec)
+ 8`, or the player's own in a game of type 2, while `Storm3D+0x40` allows team colours),
and its highlight shoulder: the lit hull rolls off towards white above
`HighlightKnee` and stays within 1, as it did when `Lighting.asi` drew into the same
target, so a hot spot whitens plating without blooming over the hull. Bump-mapped
meshes (the Borg) take the Borg profile and a normal map built from the height map as
`ST3D_CreateNormalMap` builds it, in a tangent frame worked out per pixel. Normals:
the SOD's winding makes them inward, as the engine's are (`scene::NORMAL_SIGN` = -1
turns them out); a mirrored node swaps the cull and needs no flip of its own. A mesh's
texture flipbook (a SOD animation reference) and the engine's wave lights
(`GameObject::RenderInternal`'s four special-weapon getters, `Craft::RenderInternal`'s
white one while +0x240; colour +0xf4, wavelength +0x13c, speed +0x140, direction
+0x144) are reproduced; the construction face limit (+0x10) too.

**Point lights**, per mesh, as `Lighting.asi`'s `pick_points` chose them for its
shaders: a hard light (fading over under a quarter of its start: torpedoes, the
shields-down discharges, the welds) is picked within its reach plus 150 and scored by
its peak and nearness; a soft one (nebulae, explosions) within its reach, scored by its
falloff at the mesh's origin; of one colour only the strongest; at most `PointLights`
(12). The shader takes each where it is, falloff per pixel. A module that registers a
light may say how it is to be taken with `scene::lights::set_style` (hard, never merged
by colour, wrap, falloff power): the ordnance module's phaser emitter (hard, distinct,
wrap 0.6, power 2) and impact (hard, distinct) lights, as `Lighting.asi` took its own.
The engine's stock meshes have inward normals, which is why a point light given where
it is lit the far side of a hull in `Lighting.asi`'s fixed-function path (it handed a
soft light to Direct3D mirrored through the draw's origin); the shaders here take every
point light where it is and turn the normal round instead (`scene::NORMAL_SIGN`), so no
mirroring is needed.

**Scene lights.** `scene::Lights::gather` replaces the map's directional lights with
`Lighting.asi`'s Key, Fill and sky light (the sky's mean colour weighted by chroma,
from its brightest side; the background name is read where
`Starfield_Load_Background_Geometry` keeps it, 0x738538), unless `Lighting.asi` is
loaded or `[Lights] Replace=0`.

**Shadows** (`shadow.rs`). At the start of each view every posed hull that can throw a
shadow into it is drawn into an R32F depth map from the Key, fitted round them
(width in steps of 2^(1/4), centre snapped to texels): this frame's hulls, not the last
frame's as `Lighting.asi` had to. Planets (vtable 0x6b2b3c) are tested as spheres.
`hulls::shadow_map()` lends it to other modules as `LightingShadowMap` did.

**Fades, cloak, warp.** The far fade is the engine's (`GameObjectInstance::
ComputeFadeOut`, 0x4d5a20); crafts take `Lighting.asi`'s near fade (the near plane
cutting into the hull's box) while `Storm3D+0xe8` says near fades are on. Faded hulls
are drawn at `Step::Translucent`, far to near, depth first. A cloaking device (+0x108:
state +0x3c, alpha +0x44, phase +0x48) draws the hull as `Lighting.asi`'s glass over a
copy of the frame. The warp stretch is `Craft::RenderInternal`'s while +0x244 is above
0 (`spec-chunks-anim-lod.md` §5).

**Sprites** (`sprites.rs`, `anim.rs`). SOD sprite nodes (running lights, strobes, crew
fires) and the billboard effect pinned to an object (`GameObject::SetBillboardEffect`,
+0x3c) are drawn as `ST3D_SpriteNode::RenderInternal` and `ST3D_Instance::
RenderSpriteNodeEffect` draw them, their animations played as `ST3D_AnimationChannel::
Evaluate` plays them (quirks kept: unevenly judged keys played evenly, `linear`
wrapping), at `Step::Translucent` after the hulls. A scrap object's `glowdmoon` node
takes `Scrap::Render`'s colour.

**Holograms** (`TimedHoloEffect`, the object's `+0x174`). `GameObject::RenderInternal`
draws an object with a hologram as the class the hologram shows, in place of its hull:
`TimedHoloEffect::RenderHologram` (0x60f730) calls that class's `DisableSpecialNodes`
(class vtable slot 7) and renders the effect's `ST3D_Instance` (`+0xb4`), made by
`TimedHoloEffect::init` of the class's (`+0xc4`) visible database `+0x1dc` and put at the
object's transform every tick by `Simulate`; the call's answer becomes the on-screen
byte. Two weapons make one: the Romulan holo-emitter (`gholoemt` on `rspecial`; the
target shows as the firing ship's class, 0x57a8ff) and the holo-generator (`ghologen`,
`HoloGenWeapon::Simulate` 0x66b711: the ship disguises itself as its target's class).
Here (`Holograms=1`): the class's SOD (named by `GameObjectClass::GetOdfName` 0x4ce370,
at its `ScaleSOD`) at the instance's transform, clean (`hardpoints`, the craft class's
damage nodes' children, its crew fires and Borg node hidden), its running lights drawn,
the instance's frustum test (slot 3) for the on-screen byte, and drawn at the flush by
`ps_holo` in the object's team colour: the lit hull of the class it shows, drawn part
of the way towards that colour (`HoloTint`), at `HoloAlpha`, scanlines `HoloScanPitch`
pixels apart rolling down the screen (`HoloScan`, `HoloScanRoll`), a bright band
sweeping along the hull's up axis (`HoloSweep`, `HoloSweepRate`), a stepped flicker
(`HoloFlicker`) with rare dropouts (`HoloDropout`), and a glow at the silhouette past
white (`HoloEdge`, `HoloEdgePower`) that blooms; premultiplied, after a depth pass. The
disguise still reads as the ship it shows. The real hull is not posed for the shadow
map while it wears a hologram. A hologram whose class is not a plain SOD is the
engine's.

**The Evolver** (8472's `8472_passive` and `8472_active`, class label `evolver`).
`Evolver::RenderInternal` (0x4b1170): while it evolves it has a cocoon (`+0x2ac`, a
`CocoonInstance`, vtable 0x6b2630, of `8472_cocoon.sod`, whose database the engine
finds once and keeps at 0x73fccc), culled by distance (`GameObjectInstance::
DistanceCull`), its strategic-view scale effect made (0x4d5eb0) and its scale effects
applied (`ApplyScaleEffects` 0x599760, which `Evolver::mUpdateConstructionEffect`
0x4b0a10 sets every tick to grow it from the Evolver's box to the new class's), then
drawn; past a third of the evolution (`+0x2b0`) only the cocoon is drawn and its
answer is the on-screen byte, before that the ship is drawn too (`Craft::
RenderInternal`). Here the same, the cocoon posed at its instance's transform and scale
and drawn as a hull, solid: stock's fade for it (`CocoonInstance::ComputeFadeOut`
0x4b1320, the base far fade times a ramp `+0x98` over the first third) comes out 0 on
the bench throughout (the base reads the cocoon's `+0x98`, an instance's class
elsewhere, as a class), yet stock's draw shows the cocoon solid from its first frame.
The burst is the effects module's debris ("Debris"). `Evolver=0` leaves the Evolver
to the engine.

**The build ghost** (`hulls::placeholder`, for the overlays module's
`PlaceholderFeature` and for `ConstructionRig::Render`'s next item). Stock
(`PlaceholderFeature::sRenderInternal`, 0x473aa0) makes a `PlaceHolderInstance` (a
`CraftInstance`, vtable 0x6b0094) on the stack of the class's databases, culls it by
distance, hides its `lights` (`+0x90`), takes the strategic view's scale, sets the team
colour to the colour it is given and renders it; `PlaceHolderInstance::ComputeFadeOut`
(0x473c30) halves the far fade, or gives a half. So it goes down the engine's sorted
translucent path: every layer of the hull (the inside through the plating) blended at
half, which reads pale and washed towards white with hot patches where layers pile up.
Then `DrawHeightPost` from where it stands. Here: the class's SOD clean, `lights`
hidden, culled by distance as `DistanceCull` decides (the height counted a quarter,
against `cfgOBJECT_CULLING_DISTANCE`), at the flush with every layer (`GhostDepth=0`;
1: only the nearest), by `ps_ghost`: the lit hull lifted towards its texture at full
light (`GhostLift`), a broad highlight towards each directional light past white
(`GhostSheen`, `GhostSheenPower`) that blooms, the blocked placement's red added on the
texture (`GhostBlocked`, stock's ambient), at `GhostAlpha` (stock's half); then its
additive meshes; its height post through the overlays module. A construction rig's
next station (`ConstructionRig::Render`, 0x4af9b0, the player's own rig, through
`PlaceholderFeature::sRenderInternal` at 0x4afa11) is drawn the same way.

| Key (`[Hulls]`) | Default | |
|---|---|---|
| `Holograms` | 1 | 0: the engine draws holograms (the class plainly) |
| `HoloAlpha`, `HoloTint` | 0.8, 0.35 | how opaque, and how far towards the team colour |
| `HoloScan`, `HoloScanPitch`, `HoloScanRoll` | 0.35, 3, 0.5 | the scanlines' depth, pitch (pixels) and roll (pitches a second) |
| `HoloSweep`, `HoloSweepRate` | 1.2, 0.35 | the sweeping band's strength and sweeps a second |
| `HoloFlicker`, `HoloDropout` | 0.12, 0.03 | the flicker's depth and the chance of a dropout (6 a second) |
| `HoloEdge`, `HoloEdgePower` | 2.5, 2.5 | the silhouette's glow and its exponent |
| `Evolver` | 1 | 0: the Evolver and its cocoon are the engine's |
| `BuildGhost` | 1 | 0: the build ghost (and a rig's) is the engine's |
| `GhostAlpha`, `GhostLift` | 0.5, 0.5 | its opacity (stock's half) and how far the plating is lifted |
| `GhostSheen`, `GhostSheenPower` | 1.5, 6 | the broad highlight past white and its exponent |
| `GhostBlocked` | 0.5, 0, 0 | what a blocked placement adds |
| `GhostDepth` | 0 | 1: only the ghost's nearest layer |
| `Reload` | 0 | 1: `[Hulls]` read again every second (the bench; itself read every second) |

**The survey.** `mods::object` logs, once per class (vtable), every object no module
draws: `survey: left to the engine: vtable ... (Render ..., RenderInternal ...),
database vtable ..., odf '...'`.

**For other modules**: `posed(o)`, `shadow_map()`, `draws_plain_sod(o)`,
`instance_scale(inst)`, `placeholder(v, class, place, colour, blocked)`, and
`draw_instance(v, key, inst, craft, sod, world, team)`: an
`ST3D_Instance` that is no game object's (a station being built, a worker bee) drawn
now with the hull look, its additive meshes and sprite nodes at `Step::Translucent`
(the effects module's construction and bees). Under a face limit (`+0x10`) the faces
are counted in the engine's walk of the node tree (children in reverse file order),
groups and faces in file order (`fbase` on the bench: the engine's mesh groups and
first faces matched the file's), and the hull is drawn two-sided, as the engine's
limited draw shows the inside of a part-built station.

### world

`src/mods/world/`, `shaders/world_*.hlsl`, on by default, registered first (it takes
the nebulae and the moons before `particles` and `hulls` see them). The objects whose
class overrides `Render` and draws a world of its own: planets, the dilithium moons,
asteroid fields, nebulae. It ports `Surfaces.asi` (the recipes, `Surfaces\<texture>.ini`),
`Lighting.asi`'s planets and its nebula and planet lights, `Planets.asi` (moot: our own
sphere) and `Nebulae.asi` (the recipes, `Nebulae\<odf>.ini`); the looks settled with
the user are kept. Each part stands down while a stale copy of the plugin that drew it
is loaded: the planets beside `Lighting.asi`, the moons and asteroids beside
`Lighting.asi` or `Surfaces.asi`, the gas beside `Nebulae.asi`, the lights beside
`Lighting.asi`. The recipes live in this folder: `skies/` (built into the plugin),
`surfaces/` and `nebulae/` (copied by `install.sh` into the game's `Surfaces\` and
`Nebulae\`, where an edited copy is re-read while the game runs); see "Recipes" below. Settings in `[World]` (the parts: `Planets`, `Rocks`, `Nebulae`,
`Surfaces`; the planet look under `Lighting.ini`'s names: `PlanetAmbient`, `PlanetDusk`,
`PlanetWrap`, `PlanetFill`, `PlanetSun`, `PlanetHaze*`, `PlanetGlint*`, `CityLights`,
`CityLightColour`, `PlanetHalo*`; the rocks' `RockAmbient`, `RockSpecular*`,
`RockRimLight`, `RockRimPower`, `RockSun`, `MoonShell`; the lights' `PlanetGlows`,
`PlanetGlow`, `PlanetGlowRange`, `NebulaLights`, `NebulaBrightness`, `NebulaRange`,
`NebulaRecipeColour`, `NebulaCull`) and `[Nebulae]` (`Enable` 1, `Steps` 8, `Denoise` 1.5, `Footprint` 1, `Temporal` 0.1, `TemporalClip` 1.25, `Jitter` 1, `Slices` 40, `MinStep` 6, `NoiseSize` 128, `Obscure` 0.5, `Samples` 2, `KeyShare` 0.8, `Scatter` 0.2, `Bake` 1, `VolumeSize` 256, `VolumeHeight` 160, `VolumeTexel` 3, `Shadows` 1, `ShadowSoftness` 0.02), read again
every second with `[World] Reload=1`; `[World] Enable=0` hands everything back to the
engine at once, for comparing with stock at the same moment. `BesidePlugins=1` (bench
only) draws even with an old plugin loaded, so `Enable` flips between theirs and ours.

**Surfaces** (`surfaces.rs`, `world_surface.hlsl`): `Surfaces.asi`'s bake, unchanged,
into three cubes per recipe (albedo, relief, clouds) that are render targets in the
default pool with device-made levels; a `Reset` drops them and the next draw bakes
again. Asked for by texture name; a name without a recipe keeps its stock texture.

**Planets** (`planets.rs`, `world_planet.hlsl`), objects with the `Planet` vtable
(0x6b2b3c) and a `Planet_Database` (vtable 0x6bab74, the instance's +0x80). Stock draws
two `GroundMesh` hemispheres (database +0xa8, +0xac) and the atmosphere's cloud shell
(+0xb0), radius at each mesh's +0x150, re-tessellated against the camera every frame;
we draw one sphere of our own whose vertices carry the gore UVs `GroundMesh::mSetUVs`
gives (the class textures are a four-lobe unwrap), so stock's art lands where it did.
`PlanetInstance::RenderInternal` draws only while the planet's flags (+0x14) have bit 2
and its byte +0x113 is clear. The ground is the class's `groundTextureName`
(`PlanetClass+0x4bc`) with 1 and 2 appended, or its recipe; the shell the
`atmosphereTextureName` (+0x4ac), turned about the axis by minus the instance's +0xb4
times the database's +0xa4, its turbulence on the instance's +0xb8 while the class's
+0x4cc is positive, tinted (`mSetupAtmosphere`) from white towards the race's colour
(race +0x464) as the population (+0x2c8) nears the class's maximum (its level at
+0x490 indexes the four `cfgPOP_*` at 0x702130). The look is `Lighting.asi`'s
`planet.hlsl` (night side, dusk, the limb's haze, the air past the limb, the glint off
water, cities at night), less its highlight shoulder, which the tone map does. The
shell writes no depth. **Cities** (pass 1 of `ST3D_PlanetaryMaterial`): the race's
(`Planet+0x2c4`) development texture `PD_<cityTextureName><1|2>` (race +0x452) times
the planet's own map of where its people live, made again as `PlanetInstance::
mSetupHemisphere` makes it from the race's `CityAllocArray` for that hemisphere
(+0x514, +0x518: width, height, floats): each texel's alpha 0.4 x clamp((P - value) k),
k the race's +0x470 and P the population over `cfgPOP_HEAVY` (0x70213c, set from
`RTS_CFG.h` at load; the 25000 in the exe's data is not what runs) plus 1 / k. On the
bench a full colony's map was 0x66 everywhere, the engine's and ours.

**Rocks** (`rocks.rs`, `world_rock.hlsl`). An asteroid field (vtable 0x6b0cb4) holds
its rocks as `ST3D_Instance`s (instance +0x98, count +0x9c, 0x98 bytes each: transform
+0x44, database +0x80), the class's databases at +0x1e0 named by the table at 0x6b07e8
(`master`, `master1`..`master5`, `master4` again, `master7`); each is culled as
`GameObjectInstance::DistanceCull` culls it (height counted a quarter, against
`cfgOBJECT_CULLING_DISTANCE`, 0x6fcac0) and drawn at the field's `ScaleSOD`. A `Scrap`
(vtable 0x6b15b4) is ours only when a mesh of it has a recipe (the dilithium moons'
`Mdmoon`); any other is the hulls module's. A mesh with a recipe is drawn as
`Lighting.asi`'s `moon_ps` drew it (relief, the recipe's cut-out as holes, triplanar
for the asteroids), about its mesh's centre scaled to its extents. The moon's inner
shell (`glowdmoon` material, whose colour `Scrap::Render` sets to (0, 0.5, 1) times the
share of dilithium left, `ResourceInterface::GetTotalValue` over the class's
`GetTotalMaxValue`, resource at +0x17c) is drawn dark and opaque as under `Lighting.asi`
(`MoonShell=dark`): the settled look, black cracks with the glow showing only where a
crack opens past the rock's edge; `MoonShell=glow` adds it in its colour, as stock
did. The moon's `oreglow` sprite node (the nimbus) is drawn at `Step::Translucent`, its
`moonpulse` animation on the instance's clock.

**Nebulae** (`nebulae.rs`, `world_nebula.hlsl`): `Nebulae.asi`, its decisions kept
(the envelope, the bake, the slices on a ladder fixed in distance from the
eye, per view, the blue-noise thresholds that stay put, the spectral noise). A class
with a recipe draws as gas at `Step::Translucent`, its billboards skipped in the object
loop; any other is left to the particles module. Every nebula the loop reaches is
marked on screen (+0x25) as `Entity::Render` marks it when the instance's frustum test
(slot 3) passes: `Nebula::Simulate` registers the nebula's light only while it is.
The Key is the main view's (the action camera's view keeps the bake), the ships'
shadows the hulls module's map (`hulls::shadow_map()`, sampler 8). A ghosted nebula of
a gas class keeps its gas: slot 3 of the `GameObjectSpirit` vtable (0x6b1ca4) is
wrapped. Everything is in the default pool; a `Reset` bakes again.
The gas is **marched** at full size, not sliced. Each class draws its gas box once
(the faces facing the eye, or those facing away when the eye is in or near it), and each
pixel finds its own ray (c41..c43: the ray at view depth 1 is linear in the pixel), cuts
it to the box and to the scene's depth (a RESZ copy into INTZ, read as a texture; with
this projection `z - _33` is below 0), and takes `Steps=` steps (8, 12, 16 or 24, an
entry point each, since vkd3d unrolls the loop) spaced in proportion to the distance,
each jittered in its span by the pixel's blue-noise threshold walked by the golden
ratio. Its sums (light, optical depth) are added into a half-float target; `comp_ps`
filters them with a 5x5 Gaussian of `Denoise=` pixels (sigma) that leaves out pixels
whose scene depth differs by over 5%, so hull edges stay a pixel sharp, and applies
`Opacity=`/`Obscure=` onto the frame. The slices (`vol_ps`) remain only without INTZ
and for `Bake=0`. vkd3d has no `tex3Dlod`, and a read's level from the hardware's
derivatives is undefined inside a branch that differs per pixel, so the march reads the
noise with `tex3Dgrad` (the pixel's footprint at the step's distance, times
`Footprint=`) and can then skip rays that miss the gas, columns the occupancy record
says are empty at that height, and points the volume says are clear. The flow, a slow
field, is read every fourth step and drawn straight in between.

Measured on the bench (`nebula` scene, 1920x1080, RX 5700, the camera inside a cloud
about 1700 x 1000 units, so every ray crosses a long path of gas), with the gas split
out by `Timing=1`'s marks:

| | march | filter, composite | total | grain |
|---|---|---|---|---|
| `Nebulae.asi`'s half-size slices, 40 x 2 reads | | | 3.39 ms | 0.65 |
| full-size slices, 40 | | | 8.16 ms | 1.21 |
| march, 8 steps | 4.79 ms | 0.81 ms | 5.6 ms | 0.68 |
| march, 12 steps | 6.05 ms | 0.77 ms | 6.9 ms | 0.61 |
| march, 16 steps | 7.12 ms | 0.72 ms | 7.9 ms | 0.59 |

Grain is the mean |shot - its 1.5-pixel blur| over the middle of the frame, in 8-bit
steps. Unfiltered, the march's grain goes as one over the root of the steps (24 steps
3.1, 12 steps 4.6): a few samples of a long path, not a lattice. The half-size path's
tent (about 10 screen pixels wide) had hidden the same noise; the 1.5-pixel filter takes
it out while the gas's own structure stays, since the volume's texels (6.6 units here)
are several pixels wide. Per sample the march costs about what the slices did; what it
saves is the blending of every slice, and what it spends is four times the pixels.
About 2.5 ms of the march does not depend on the step count (8 to 16 steps adds 0.29 ms
a step). It is not the unrolled shader's size: Microsoft's compiler (`d3dcompiler_43`,
run under Wine for the measurement only, never part of the build) makes the 8-step march
a real `rep` loop of 2,200 dwords instead of vkd3d's 10,400, and on the bench (720p) the
march took 2.53 ms either way against vkd3d's 2.52: DXVK's SPIR-V and the driver end
up at the same code.

**`Temporal=`** (default 0.1) adds the frames up, since in game 8 steps' stipple still
showed through the filter. The thresholds then walk the golden ratio from frame to frame
(`k_ray2.w`, the frame), so each frame's samples fall between the last's, and
`temporal_ps` blends this frame's march into the view's history, a share `Temporal=` of
it. A pixel's gas is found in the history where it was the frame before: the march
writes a second target (MRT), the view depth of each step weighted by its optical depth,
and that point goes through the last frame's VIEW x PROJECTION. History the present
frame's 3x3 neighbourhood does not support (gas a hull now covers, gas that moved) is
clipped to its mean +- `TemporalClip=` (1.25) spreads; a value the half floats cannot
hold, or a NaN, is replaced by the mean so it cannot stay. The clip alone let a ship
that moved over the gas trail its outline in game (eight steps' noise makes the
neighbourhood's spread wide), so history is also dropped wherever the scene's view depth
differs by over 5% from what it was there (the march stopped at another hull): beside
the history a second target keeps that depth and how many frames the history holds,
and a pixel takes each frame at 1 / (frames + 1) until that reaches `Temporal=`, so a
dropped pixel is clean again within a few frames. In thin gas (the Big Cerulean
Nebula's specks) a frame's eight steps find gas in few pixels, and a 3x3 that found
none has a mean and a spread of zero, which wiped the history whatever
`TemporalClip=` was (1.25, 2.5 and 4 measured the same) and left the stipple there:
the clip's top also reaches the brightest pixel of the ring two pixels out, which
smoothed those wisps on the bench. Zoomed out, a grain stayed in game that zooming in
smoothed away: the volume's texels and the swirl's folds were then finer than a pixel,
and a ray through the pixel's centre alone gave each pixel a value of its own that every
frame repeated, so no number of frames averaged it out. So the ray also walks within its
pixel from frame to frame (`Jitter=`, its span in pixels, default 1; an R2 sequence on
the frame, `k_ray1.w`), as TAA's camera jitter does, and the frames add up the gas over
the whole pixel. It costs nothing in the march. On the bench (720p, high above the Big
Cerulean Nebula) the wisps came out finer and less blotchy than with `Jitter=0`, whose
grain number was the same (0.94 / 0.95; `Footprint=2` 0.94): the metric mostly measures
the gas's own structure there. The history is four targets
per view (the action camera keeps its own), dropped with the device and whenever no gas
was marched. `comp_ps` then filters and composites the history as it did the march.
The pass costs 0.22 ms at 720p. With `Temporal=0` the thresholds stay put, as before:
moved without adding the frames up, they made the gas shimmer. Shadows cost 0.6 ms of 7.6 at 24 steps; the noise reads (flow, fine
detail) were half of it before the flow was thinned. `Footprint=2` saved nothing.

**Lights** (`lights.rs`), without `Lighting.asi`. *Planet glows*: per planet and view
(`Module::lights`), a point light at its centre (the instance's transform +0x44 plus
its bounding sphere +0x34) in the Key's colour times its ground's mean colour times
`PlanetGlow`, full at the surface and gone at `PlanetGlowRange` radii, marked a day
side (`scene::lights::set_day_side`): a draw takes it times the share of the day side
facing it (`Lights::share`), and the planets take none. *Nebula lights*: the call to
`Simulate_Nebula_Lights` (0x4a5160) at 0x4a4a7a in `Nebula::Simulate` is re-pointed;
stock's colour (the class's glow, +0x22c, swung by a noise term) came out black, so
the light is the glow times `NebulaBrightness`, steady, in a gas class's recipe hue,
its falloff (class +0x244, +0x248) times `NebulaRange` (written on the nebula's light
object, +0x1ac). *Nebula cull*: slot 3 of the `NebulaInstance` vtable (0x6b1448,
`ST3D_Instance::FrustumTest` 0x62e990) tests the bounding sphere (the node's at +0x1c
when the instance has one at +0x80, else its own at +0x34; radius +0xc) with its radius
times `NebulaCull`, and puts it back.

**Recipes.** Three folders, one per kind, all plain `.ini` files you can edit.

- `skies/<name>.ini`: the sky recipes, built into the plugin ("Sky recipes" above).
- `surfaces/<texture>.ini`: planets (classes D, H, J, K, L, M), the dilithium moon
  `Mdmoon` and the asteroids `Master1`/`Master2`. Installed by `install.sh` into the
  game's `Surfaces\`; a name without a file keeps its stock texture. A recipe for a hull
  texture is found at that texture's first draw only, so adding one needs a restart.
- `nebulae/<odf>.ini`: the gas classes (`mnebula6`..`10`, `mnebula12`, `latinum`,
  `fluidicnebula`, `generatednebula`). Installed into the game's `Nebulae\`; a class
  without one keeps its billboards.

Edited copies in the game directory are re-read within half a second with `Reload=1`,
and a changed recipe bakes again. Each file's header gives what it aims at.

*Surface recipes* (`[Surface]`; colours 0-255, heights in field units, about -4 to 5;
directions are yaw about the planet's +y and pitch up from its equator):

| Key | Default | |
|---|---|---|
| `Kind` | `rock` | `gas` for a gas giant |
| `Seed`, `Scale`, `Warp`, `Detail` | 1, 2.5, 0.4, 0.5 | as for the sky; high `Warp` turns the ground wormy |
| `Continents`, `Ridges` | 0 | the slow field that lifts and sinks regions; mountain chains on land |
| `Sea`, `Shore` | none, 1 | the sea level (left out: a dry world); how deep the water must be to reach `Deep` |
| `Relief` | 0.02 | the height's scale against the radius per field unit: the normal's tilt |
| `Craters` | 0 12 1 0.3 | share of cells with one; cells across; depth; rim |
| `Fissures` | 0 0.05 1 1 | cells across (0: none); trench width; depth; glow. `FissureShape` (2.5 0.33): the warp, and the share of the net's borders kept; `Branches` (0.6): the finer net |
| `Cut` | 0 | the glow at which an alpha-tested model is cut through (the moon's cracks are holes) |
| `Ramp1`..`Ramp6`, `Alt1`..`Alt6` | | `height r g b` (gas: `0..1 r g b` across one band); `Alt` is a second ramp mixed in by region (`Region`: scale, share, colour noise, its scale) |
| `Deep`, `Shallow` | | the sea; a gas giant's poles are `Deep` |
| `Ice`, `IceColour` | | latitude it starts at (0..1), softness, ragged edge, amount; colour |
| `Cavity` | 0.25 0.15 | hollows darkened, crests lightened |
| `Glow`, `GlowDay` | 0, 0.3 | the fissures' light, and how much of it shows by day |
| `Clouds` | none | scale, coverage (higher is clearer), softness, warp; `CloudShape`, `CloudColour`, `CloudOpacity`, `CloudDrift` (degrees per second), `CloudShadow` (strength, reach in radians) |
| `Depth` | 1 | the relief's strength in the light, at draw time (no re-bake) |
| `Bands`, `Storm`, `StormColour` | | gas giants: bands, turbulence, warp share, stretch; the storm's yaw, pitch, size, strength |
| `Face` | `[Surfaces]`'s | this recipe's cube edge (64-4096) |
| `Mapping` | `radial` | a model's lookup: `radial`, the direction from its mesh's centre; `triplanar`, each axis's cube faces across its extents. The asteroids are `triplanar`: the direction from the centre smeared their flat faces into streaks |
| `Gloss` | 1 | a model's highlight, times the hulls' |

*Nebula recipes* (`[Nebula]`; `Cls::read` in `nebulae.rs` is the authority for defaults).
Shape: `Extent`, `Height`, `Lumps`, `EdgeWarp`, `Radius`. Noise: `Seed`, `Tile`, `Warp`,
`WarpScale`, `Detail`, `FineDetail`, `Coverage`, `Softness`, `Gamma`, `Ridge`. Colour:
`Brightness`, `GasA`, `GasB`, `Glow`, `HueScale`, `HueMix`, `HueCycle`, `HueSwing`.
Light: `Light`, `ShadeStep`, `Shading`, `Knots`, `KnotScale`, `Lanes`, `LaneScale`.
Motion: `Flow`, `FlowScale`, `FlowSpeed`, `Swirl`, `SwirlScale`, `SwirlPeriod`. Edges:
`Fringe`, `FringeReach`, `FringeScale`, `Fuzz`, `FuzzSpeed`. Filaments: `Filaments`,
`FilamentScale`, `FilamentSharpness`, `FilamentReach`. Viewing: `Opacity`, `EdgeOnDim`,
`KeyShare`, `Scatter`, `NearFade`. Lightning: `Lightning` (flashes a minute, a nebula),
`LightningSize`, `LightningBrightness`, `LightningColour`. The core: `Core`,
`CoreSharpness`, `Pulse`, `PulseSpeed`, `PulseWander`, and its ring of colours `Ring`,
`RingAt`, `RingWidth`, `RingScale`, `RingPatch`, `RingSoftness`, `RingSpread`,
`RingCycle`, `RingA`..`RingD`. A shadow takes only the Key's share of the gas's own
light (`KeyShare`), scattered forward (`Scatter`); dimming self-lit gas by a fraction
left the shafts invisible except in a difference image, and they show looking towards
the Key.
