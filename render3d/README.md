# render3d — the 3D view through our own renderer

`Render3D.asi` takes over the engine's 3D view: both calls of `Armada_RenderAllOurStuff`
(0x598110) come to it, so the scene is drawn into render3d's own targets and put into
the back buffer by render3d. It is written in `no_std` Rust (`src/`), and the plan, its
milestones and every change to it are in [`PLAN.md`](PLAN.md).

What it does today (milestone 2, "wrap mode"):

- **The frame is ours.** The view is drawn into a half-float (`A16B16G16R16F`) target
  with 4x MSAA and a depth of its own, then resolved, bloomed and tone-mapped into the
  engine's back buffer at the view's rectangle. Additive light (explosions, weapons,
  shield hits, glows) now sums past white instead of clipping, and what lies past
  white glows by how bright it is: bloom by energy, which an 8-bit back buffer cannot
  hold. Below the tone map's shoulder every colour is stock's.
- **Every step inside the view is still the engine's**, so every plugin hooked into
  those steps (`Lighting.asi`, `Sky.asi`, `Nebulae.asi` and the rest) keeps drawing
  its part, into our target. Modules of our own replace the steps one category at a
  time (PLAN.md, milestone 4).
- **`MSAA.asi` is superseded**: the scene target has samples of its own, and the back
  buffer stays 1x. `install.sh` removes it.

## Building and installing

    render3d/install.sh            # build, install Render3D.asi + Render3D.ini, remove MSAA.asi
    render3d/install.sh --remove   # take it out again

`./install` runs it in MSAA's place. Building needs rustup with the 32-bit Windows
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
with MSAA half the samples of the view's first row and column are never drawn
(`msaa/README.md`, "The top row and left column"). The final pass reads those pixels
from their neighbours instead, which does what `MSAA.asi`'s edge fill did without the
frame's delay.

**Reset.** Every target is in the default pool and must be gone before the device
resets. Slot 14 of the d3d8 device's vtable (`Reset`) is chained, as `Lighting.asi`
chains it: render3d drops its targets and passes on the call, and makes new ones at
the next frame, at the new size.

**Without Direct3D 9.** The d3d9 device is asked for behind the game's d3d8 one
(`platform/D3D9.md`). Under DXVK's own d3d8, or when the targets cannot be made, the
body runs as stock. An `A16B16G16R16F` target with MSAA falls back to one without, then
to 8 bits, and the log says which.

## Modules

### effects

`src/mods/effects/`, on by default. The game features that are effects, hull debris,
and their lights; it ports `Explosions.asi`, `Shields.asi` and `Repair.asi`, whose
looks were settled with the user (their READMEs hold the derivations; the settings
keep the plugins' keys, in `Render3D.ini` sections `[Explosions]`, `[Shields]`,
`[Repair]` and `[Debris]`, read once at start). Each part stands down while its plugin
is loaded. Everything noted while the features run is drawn at `Step::Chunks`, after
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
  unchanged. Each fireball also registers a point light (`lighting/README.md`,
  "Explosions": `LightColour`, `LightBrightness=4`, `LightRange=10` radii, at least 40
  units) unless `Lighting.asi` is loaded, which adds its own.
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
  reads the particle's node from `esi` and skips a weld. The welds are drawn as in the
  plugin, and each burning arc registers a point light.
- **Debris** (`chunks.rs`), `Step::Chunks`: our own chunks (`spec-chunks-anim-lod.md`
  §1). The call to `ST3D_Instance::Fragment` (0x62eb20) in `Craft::Explode` (0x4c6bfc)
  is chained: for a craft whose SOD we read, the module notes its ODF, transform,
  animation clock and team colour and does not call the engine's (so the engine makes
  no chunks of it); otherwise the engine's runs. At the next `Step::Chunks` each mesh
  of the LOD shown breaks into patches as `CreateChunks` does (a random seed face, every
  face with a corner within half the mesh's radius), flying and spinning on the sim
  clock with stock's `ART_CFG.h` numbers (4 ± 2 s, 90 units/s, 4 rad/s, at most 1500),
  drawn through the scene layer's hull shading, and gone without fading at the end of
  their life. The other callers (a cancelled construction) stay the engine's.
- **Lights.** `Module::lights()` runs once per view after `GameObject_PreRenderAll` and
  `Ordnance::PreRenderAll`, before `scene::begin_view` reads the light list: the module
  registers ordinary `ST3D_Point_Light`s (`operator new` 0x652710, the constructor
  0x62f240, `RegisterLight` 0x62d250), which the engine deletes the records of at the
  view's end, so both the engine's draws and the scene layer see them.
- **Still the engine's**: construction (`BuildingEffect`, the bees and drones), nanites,
  `Step::Specials` (the special-object effects), and every spark that is not a weld.

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
3. **the engine's own `Submit`**, called through the vtable, for everything else:
   mines, pods, the commando team, the mining beam (`wmbeam.sod`, an instance scaled
   along the beam, `MiningBeam::Submit` 0x57d1e0) and the special weapons that draw a
   model and have no recipe.

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
| `Survey` | 0 | log every ordnance ODF the first time it shows, with its vtable and `Submit` |
