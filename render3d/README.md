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
   - **With a recipe** (`sky/skies/<name>.ini`, built into the plugin; a
     `Sky\<name>.ini` in the game directory takes precedence and is re-read when it
     changes), the procedural sky of `Sky.asi` (sky/README.md, whose settled looks
     it keeps: Aqua dimmer and softer, the grades, no filaments): one quad over the
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
| `Dump` | 0 | 1: each bake writes its faces to `Sky\<name>.dump0..5.tga` for `sky/grade.py` |
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

**The fog and the shroud** are `Fog.asi`'s look (`fog/README.md`: the cubic mask, the
pixel-wide smoothstep edge, stock's rim, the taps held inside the grid; `Filter`,
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
`PlaceholderFeature::Render` 0x4738a0, `sRenderInternal` 0x473aa0) is the engine's by
default (`BuildGhost=0`): stock draws the station being placed (class `+0x34`, at
`+0x38`, colour `+0x68`, ambient from 0x736d40 unless `+0x74`) as a faded hull through
the engine's sorted translucent path, which comes out as a bright glow over the
station's lit plating, and `BuildGhost=1`'s additive approximation does not match it
yet; the faded-hull look belongs with the hulls module. `DrawHeightPost` (0x593840)
stays the engine's either way.
