# Changelog — render3d

`Render3D.asi`: the 3D view drawn through render3d, in Rust. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The reasoning is
in [`README.md`](README.md), the plan in [`PLAN.md`](PLAN.md).

## 0.16.4 — 2026-10-21

### Changed
- `Render3D.asi` carries a version resource (`VERSIONINFO`): product, description, the
  layer's version as the file version and the project's as the product version, which
  Windows shows in the file's Properties. Built by `platform/versioninfo/build-res.sh` (915cad8).

Confirmed in game 2026-10-22.

## 0.16.3 — 2026-10-21

### Fixed
- A black hole (`mblkhole`) under the fog of war was a near-black disc in the fog
  (issue #3): a fog-of-war ghost's `additive` meshes (the black hole's disc, a
  wormhole's swirl) were drawn opaque and lit like a hull. They are added now, unlit,
  at half strength after the opaque ones, as stock draws them; an `alphathreshold` mesh
  of a ghost is cut out at one half. `Render3D.log` says how many additive meshes the
  first ghost of each class has (7e7a184).

Confirmed in game 2026-10-22.

## 0.16.2 — 2026-10-21

### Fixed
- A freighter carrying latinum trails the golden latinum spray again (GitHub issue #2,
  7c49408): the emitter `FreighterClass` adds at the ODF's `latinum_spray_hardpoint`,
  which is in no SOD, now fires from the particles module at the hold's latinum fill
  (`particles::sim`'s `object_set`, `cargo_rate`).
- A freighter's cargo pods light in the colour of what it carries (dilithium pale blue,
  latinum yellow), up to the fill, as `Freighter::UpdateDilithiumMarkings` lights them;
  they were drawn as plain plating whatever the hold (`hulls::game::cargo_markings`).
- Meshes of SODs older than 1.7 whose texture material is alpha-tested in the engine
  (modes other than `2 2`) are drawn as cut-outs (`scene::mesh_blend`): the Klingon and
  Romulan freighters' pod lattices, which hid the markings, and other old models'
  alpha holes (`Kfreight`, `Rfreight`, `Bcruise1`, `Bbuildng`, `Fbee`, `Fresear`,
  `Mdmoon` textures).

Confirmed in game 2026-10-22.

## 0.16.1 — 2026-10-21

### Fixed
- On Windows, on a card that takes shaders of 4096 instruction slots (Direct3D 9's own
  runtime refuses a longer one; DXVK takes any), the procedural sky and the planets'
  surfaces were never baked: black space behind the main menu and every map, and the
  planets in their stock texture. The sky's bake (8761 slots) is now a chain of five
  passes and the height of a planet (17812) one of up to eight (`gfx/chain.rs`), none
  over 2940; they bake what the one shader did (0bd0a97).
- `Render3D.log` names each shader Direct3D refuses, with its slots and the card's
  (`gfx: Direct3D refused ...`), and the card's slots at start; the version it logs is
  the changelog's again (it said 0.6.1).

### Changed
- Build: `d3dbc-fix.py` refuses a shader past 4096 slots unless its entry is marked
  `:long` (the code stands down without it: the `Face=0` reference sky and the nebulae's
  march, which falls back to slices on such a card). `hlsl-rs.sh` compiles from the
  shader's directory, for `#include "chain.hlsli"`.

Confirmed in game 2026-10-21.

## 0.16.0 — 2026-10-17

### Added
- A torpedo or pulse lights its own ship where it leaves it (`lights.rs`, the muzzle
  light): the shot's light colour at the point it is first seen, held on the owner,
  reaching its bounding radius and gone over 0.5 s. `[Ordnance] MuzzleLights` (1),
  `MuzzleBrightness` (4), `MuzzleTime` (0.5), `MuzzleReach` (1).

Installed, not yet seen in game.

## 0.15.2 — 2026-10-17

### Changed
- The auto-assimilator's beam (`[Ordnance.borgbor]`) reads clearly against dark space
  at 720p: `Brightness` 2.0 (was 0.9) and `Fall` 0.4 (was 0.12), so a long, thin cone
  (a target far off, met edge-on) no longer fades out before it reaches it. Still a
  translucent cone with fans; the other beams are unchanged.

Installed, not yet seen in game.
## 0.15.1 — 2026-10-18

### Fixed
- On Windows, switching away from the game and back ended in "Fatal error in
  Direct3D": the hulls' posed views and the ghosts' queue kept models, whose buffers
  are in the default pool, through the device's `Reset`. Both are dropped at a `Reset`
  now, and the log names any default-pool object still held then.

Confirmed in game 2026-10-18.

## 0.15.0 — 2026-10-18

### Added
- `d3dbc-fix.py`: rewrites the vkd3d bytecode Windows' own Direct3D 9 refuses (two
  constant or two input registers in one instruction, `sincos` into its own source,
  `oDepth` with a partial mask, temporaries past r31, given out again by liveness);
  `--check` also lists a `texld` inside an `if`, which it cannot rewrite.
- `hlsl-rs.sh --fix`, which puts each shader through it; `build.sh` and `ui/build.sh`
  use it.

### Fixed
- On Windows, without DXVK, render3d stood down ("the post shaders could not be made:
  stock") and the engine drew the 3D view and the menu scene: 51 of the 86 shaders were
  refused. Every one is made now. No sampling inside a branch any more: the fire, the
  rocks, the surfaces' albedo and the nebulae read with gradients, at level 0, or
  flattened, each sampling what it did before (README.md, "Shaders on Windows").
- On Windows, starting a mission ended in "Fatal error in Direct3D. Please reboot.":
  what render3d held kept the shell's device alive, and the engine could not create the
  mission's. `DestroyRenderSurfaces` is chained now, and render3d lets go of every object
  on the device, its shaders and declarations included, before the engine releases it;
  the log counts the references left, and names by kind any object of ours still held
  (`d3d::live`). The hulls' posed views and the ghosts' queue held 49 models' buffers.

Confirmed in game 2026-10-18.

## 0.14.0 — 2026-10-17

### Added
- `[World] MoonCrackDepth` (0.02, in the moon's radii) and `MoonCrackFalloff` (3).
- `[World] Reload=1` reads the rocks' keys (`Rock*`, `Moon*`) again too.

### Changed
- A dilithium moon's light lies at the bottom of its cracks (`ps_rock`, c36.zw): each
  crack is a ravine the view is marched into, its walls dark rock and its light fading
  up them, the near wall hiding it at an angle. Fades to the flat look where a crack is
  only a pixel or two wide.

Confirmed in game 2026-10-17.

## 0.13.0 — 2026-10-17

### Added
- `[World] PlanetGhost` (0.2): a fog-of-war planet ghost's brightness against the
  planet seen, stock's measured dimming.

### Fixed
- A planet left under the fog of war is drawn by the world module on its own sphere
  with its own surface, dimmed, instead of falling back to the engine's low-polygon
  sphere and stock texture (`world::planet_ghost`, called from `overlays::ghosts`).

Confirmed in game 2026-10-17.

## 0.12.0 — 2026-10-17

### Added
- The menu scene's decloaking ships are heard: each plays its group's `CloakSound`
  (default `clkdisen.wav`, the Klingon cloaking device's `offSound`; empty for none)
  as its decloak begins, through the game's 2D sound (`engine/sound.rs`).

Confirmed in game 2026-10-17.

## 0.11.0 — 2026-10-17

### Added
- `[World] MoonCracks` (4), `MoonPulse` (0.5), `MoonCrackCore` (1), `MoonNimbus` (0).

### Changed
- A dilithium moon's cracks are lit from inside (`ps_rock`, c35/c36) instead of cut
  through to the dark shell: the nimbus's colour at its peak (purple on the infinite
  moons), half of its pulse, times the share of dilithium left, whiter at a crack's
  core and bright enough to bloom. The recipe's `Glow=` beside the cracks takes that
  light's hue and pulse. The `oreglow` nimbus, the round halo behind the moon, is no
  longer drawn (`MoonNimbus=1` brings it back).

Confirmed in game 2026-10-17.

## 0.10.1 — 2026-10-17

### Fixed
- A departure's warp flashes drew as hard-edged boxes: the `warp_sprite` art's core is a
  flat white diamond and its halo reaches its border, and at four times stock's light
  both showed. The flash now fades round from near its centre (`flash_ps`) at 1.5 times
  stock's light.

Confirmed in game 2026-10-17.

## 0.10.0 — 2026-10-17

### Added
- The departure: a second export, `Render3D_MenuWarp(name, x, y, w, h)`, draws a view of
  the menu map whose ships go to warp front first (the hull's warp stretch, forward
  only, then a flash of the stock `warp_sprite`), the camera swinging behind them and
  following, and a warp tunnel coming in once they have gone (`tunnel_ps`, a new pass
  `Warp` drawn by the sky module). Keys `[Warp]` (`Delay`, `Stagger`, `Ramp`,
  `Stretch`, `Speed`, `Turn`, `Follow`, `TunnelAt`, `TunnelIn`, `Dim`, `TunnelSpeed`,
  `Widen`, `Flash`, `FlashTime`), `[Group] WarpColour`, `[View] WarpEye`/`WarpTarget`,
  `[Ship] Warp`; the map's starbase stays (`Warp=0`) and the Borg's tunnel is green.

Confirmed in game 2026-10-17.

## 0.9.0 — 2026-10-16

### Added
- The menu scene is one map, `menus/map.ini` (`Menu\map.ini` in the game directory
  takes precedence), asked for by view: `main`, and for the campaign selection
  `tutorials` (a starbase over the planet), `federation` (the fleet), `klingon` (Klingon
  ships decloaking before a Metreon nebula), `borg` (a fusion cube casting its shadow
  through a Mutara nebula, the queen's Diamond close to the camera). The camera moves
  across the map between views (`Blend`, 1.6 s).
  Keys `[Group.*]` (with `CloakView`, `CloakDelay`, `CloakTime`, `CloakStagger`),
  `[View.*]`, `[Ship.*]`, `[Nebula.*]` (its `KeyShare`, `Scatter`, `Opacity`,
  `Brightness` over the recipe's, for the menu's own class).
- The world module draws the menu's gas (`Nebulae::draw_menu`, menu pass `Gas`), and
  the hulls module decloaks menu ships with the cloak's look.

### Removed
- The recipes `menus/main.ini` and `menus/single.ini` and their keys (`Base`,
  `[Camera]`, `[Orbit]`, `[Ship<n>]`): the map's views take their place.

### Fixed
- Menu hulls cast no shadow: the shadow map read a model with no instance as having
  no faces to draw.
- The shadow map leaves no cut-out mesh out: it casts what the hull pass draws of it,
  its texture's alpha cut at 0.5 (half of a Borg fusion cube cast nothing, in missions
  too).

Confirmed in game 2026-10-16.

## 0.8.0 — 2026-10-16

### Added
- The menu scene: a fleet in orbit over a planet, drawn behind the front end's main
  menu and campaign selection on request through `Render3D_MenuScene`, render3d's first
  export (UI.asi's `[Menus] Scene`). Recipes `menus/main.ini` and `menus/single.ini`,
  built in; `Menu\<name>.ini` in the game directory takes precedence and is read again
  when it changes. The sky, world and hulls modules draw their parts (`Module::menu`):
  the recipe sky with stars and a sun, a class planet with no game object behind it
  (`Planets::draw_menu`), lit hulls with shadows and running lights; then the frame's
  bloom and tone map (`frame::menu_frame`).

Confirmed in game 2026-10-16.

## 0.7.1 — 2026-10-16

### Fixed
- Planets, moons, asteroid fields and nebulae drawn by the world module are marked on
  screen (the byte `Entity::Render` writes), so the pointer picks them again: a planet
  could not be clicked, nor a colony ship right-clicked onto it, only drag-selected.
- The selection bubbles are not drawn while the engine's `RenderSelectionEffects` has
  been made to return at once (UI.asi's `[HUD] SelectionBubbles=0`).

Confirmed in game 2026-10-17.

## 0.7.0 — 2026-10-16

### Added
- A third view: a non-main view from a camera other than the action camera's (UI.asi's
  info-panel close-up) keeps per-view state of its own (`View::key` 2), and is logged as
  "other".

Confirmed in game 2026-10-17.

## 0.6.1 — 2026-10-16

### Fixed
- `build.sh` writes `src/shaders/mod.rs` and `build/Render3D.asi` through temp files of
  its own run (`mktemp`, then a rename), so installs of one checkout running at once
  (`a2film --jobs`, `a2test -j`) no longer collide on `src/shaders/mod.rs.new`.

Tooling only; nothing to see in game.

## 0.6.0 — 2026-10-16

### Changed
- Tractor, holding and assimilation beams (`tracbm`, `tracbmo`, `holdbm`,
  `techassimbeam`, `borgbor`) are drawn as a translucent cone through vapour, after
  `ART-DIRECTION.md`: fans of light with hard bright edges that fade in and out each on
  its own, drifting vapour, no core, widening to the
  target's width across the beam (its meshes' boxes, not its bounding sphere), with a
  soft light of the beam's colour on the target's hull.
- Shield hits: an uneven mist about the impact, a soft curved edge at the shell's
  silhouette, rays on Klingon shields only, and a point light on the struck hull.
- Shield colour defaults: Klingon green (was orange), Species 8472 orange (was purple).
- The auto-assimilator (`borgbor`) is the cube's assimilation beam in its blue, with a
  fan of green now and then (`Sprinkle`, `SprinkleShare`).
- The cloak's waves sway the hull across the screen only, never in depth, smaller and
  finer: `CloakWave` 0.03 (was 0.08), `CloakWaveBands` 3 (was 1.5).

### Added
- `Kind=cone` in ordnance recipes, with `Spread`, `Beyond`, `Pull`, `Fans`, `Sprinkle`,
  `SprinkleShare`,
  `EdgePixels`, `Vapour`, `Fall`, `WashBack`, `WashReach`.
- `[Shields]` keys `Mist`, `MistReach`, `MistShare`, `MistContrast`, `MistScale`,
  `MistDrift`, `Edge`, `Rays`, `RayLength`, `RaySharpness`, `RayFactions`, `HitLight`,
  `HitLightWhite`, `HitLightReach`, `HitLightPower`, `HitLights`.

Confirmed in game 2026-10-17.

## 0.5.0 — 2026-10-16

### Changed
- Cloaking in `ART-DIRECTION.md`'s look: while a ship cloaks or decloaks, waves sway
  its geometry so the silhouette bends, and the hull thins into the glass band by band
  in place of the noise threshold; the green front is a thin line along the bands.
- Fully cloaked, slow waves drift in the refraction.
- `CloakHull` is the floor for the player's own ships, now 0.1 (was 0 for every ship).

### Added
- Keys `CloakHullOthers` (0), `CloakWave`, `CloakWaveBands`, `CloakWaveSpeed`,
  `CloakWaveTilt`, `CloakShimmer` and `CloakShimmerSpeed` in `[Hulls]`.

Confirmed in game 2026-10-17.

## 0.4.0 — 2026-10-16

### Added
- Nebulae: `[Nebulae]` `Steps=` (8, 12, 16 or 24; default 8), `Denoise=` (the
  filter's sigma in pixels, default 1.5) and `Footprint=` (the noise reads' footprint,
  default 1) (d367dca).
- Nebulae: the marched gas is added up over frames (`temporal_ps`): the thresholds walk
  on from frame to frame, and each frame is blended into a history per view, reprojected
  by the gas's depth, clipped to the neighbourhood and dropped where the scene's depth
  changed (a ship moved over the gas); the clip's top reaches the brightest pixel two
  out, so thin gas keeps its history; and each pixel's ray moves within the pixel from
  frame to frame, so gas finer than a pixel adds up instead of leaving a still grain.
  `[Nebulae]` `Temporal=` (this frame's share, default 0.1; 0 off), `TemporalClip=`
  (default 1.25) and `Jitter=` (the ray's walk within its pixel, in pixels, default 1;
  0 keeps it at the centre) (9a54a5f, 65b182b, a9da7c5, 0e2f6ed).

### Changed
- Nebulae: the gas is ray-marched at full size, a ray per pixel through the baked
  volume up to the scene's depth (RESZ into INTZ), into a half-float target, and put
  onto the frame through a depth-aware 5x5 filter with `Opacity=`/`Obscure=`. 5.6 ms
  against 3.4 ms on the bench at 1080p, same grain (README.md, "Nebulae"). The slices
  remain only without INTZ and for `Bake=0`.

### Removed
- Nebulae: the half-size pass, its depth-weighted composite and the edge refinement;
  the `[Nebulae]` keys `Resolution=` and `EdgeRefine=`.

### Fixed
- Nebulae: the march's rays were mirrored top to bottom, so the gas was drawn
  reflected about the view and slid as the RTS camera panned (316df23).

Confirmed in game 2026-10-17.

## 0.3.0 — 2026-10-16

### Added
- Sky: recipe key `Octaves=` (1-6, default 6): how many of the gas's octaves count, a
  fraction fading the finest out.
- `perf/sky-shape.py`: measures a recipe's shape against the stock faces (log
  luminance in octave bands and standardised tones, at 512 a face with stock magnified
  bilinearly), and with `--fit` fits `Scale`, `Detail`, `Octaves`, `Coverage`,
  `Softness` (only wider) and `Warp` (only lower) on a running bench game.

### Changed
- Sky: every recipe in `skies/` has its shape fitted to its stock sky (distance from
  stock's structure 0.12-0.45 after grading, from 0.13-1.16), then its grade refitted:
  many mid-sized clumps where most skies had one or two large billows (a40a752).
- Sky: Aqua and `mbgrg` are graded to stock like the rest (0.92 and 0.90 of stock's
  luminance); `perf/sky-grade.py`'s POLICY is empty.

### Fixed
- Sky: a recipe star's halo was cut at the lattice cells' borders, a faint square
  about `mbgpur`'s big star; it now fades out within the cells searched (a40a752).

Confirmed in game 2026-10-17.

## 0.2.1 — 2026-10-16

### Changed
- Sky: the recipes draw the broad gas only. `Grain=` and `Dust=` are gone from every
  recipe in `skies/`, and `Fine=` (the draw-time detail) defaults to `0` (off); the keys
  remain. `GradeGain=`/`GradeCurve=` refitted to the stock faces without them, three
  passes of `perf/sky-grade.py` (0.89-0.92 of stock's luminance; `mbgrg` 0.55, Aqua
  ungraded, as before) (712a09a).

### Fixed
- `perf/sky-grade.py`: `mbgblue` is no longer fitted again on top of the `mbg02` grade
  it is given (it wrote gains of 18-19) (712a09a).

Confirmed in game 2026-10-17.

## 0.2.0 — 2026-10-15

### Added
- The body runs as render3d's own sequence of the engine's steps (`Steps=1`), each
  called through its call site, and modules take steps, objects and game features
  over (`[Modules] <name>=0|1` in `Render3D.ini`); a module stands down while the
  plugin it replaces is loaded.
- `a2fmt`: SOD, TGA (stock mip chains), `.spr` and ODF readers; the scene layer draws
  SOD instances lit by the engine's registered lights.
- Module `hulls` (on; stands down while `Lighting.asi` is loaded, which still draws
  the planets): ships, stations and other plain-SOD objects from their SODs in
  `Lighting.asi`'s look (scene lights, team colours, the Borg bump profile, point lights
  picked as its `pick_points`, the highlight shoulder), fades, cloak glass, warp
  stretch, sprite nodes, billboard effects and flipbooks, and a shadow map of this
  frame's hulls. The scene layer's culling and normals fixed (back faces showed).
- Module `effects` (on): explosions (`Explosions.asi`'s fireball and ring, the core
  blooming by energy in HDR), shield hits and the shields-down arcs (`Shields.asi`),
  repair welding (`Repair.asi`), and hull debris as render3d's own simulation; keys in
  `[Explosions]`, `[Shields]`, `[Repair]`, `[Debris]`.
- Module `ordnance` (on): every shot and the warp flash: phasers, pulses, torpedoes,
  plasma, polerons and artillery in stock's look (near-white texels lifted past white,
  `Hot`/`HotFrom`), the special weapons' beams, bolts, fields and waves and the warp
  flash from `Specials.asi`'s recipes (`ordnance.ini`, built in; Render3D.ini sections
  of the same name override), torpedo lights recoloured and phaser lights as
  `Lighting.asi` makes them. Shots drawn as models (mines, colony pods, the commando
  team, the myotron, black hole, flux wave, stasis, transwarp) through the scene layer,
  in their owner's team colour, with a pulsing glow and light by `[Model.<odf>]`, their
  emitters as render3d's particles (`[Ordnance] Models=`); the mining, orbital and
  repair beams as recipes flowing back to the ship, their impact where the beam enters
  the target (`ImpactAtSurface=`).
- Module `sky` (on): all of `Background_Render`: `Sky.asi`'s recipe skies (built in
  from `sky/skies/`; a `Sky\<name>.ini` in the game directory wins), baked on the GPU
  into a half-float cube; stock faces where there is no recipe; the starfield from the
  engine's octree, point stars lifted into HDR (`StarLift`); background planets and
  suns (`SunLift`); keys in `[Sky]`.
- Module `overlays` (on): the fog of war and shroud with `Fog.asi`'s smooth edges over
  the engine's own scan textures, the tactical grid, the selection bubbles and the
  fog-of-war ghosts of stations, and the height posts (`DrawHeightPost`) as
  antialiased lines (`HeightPosts`, `HeightPostWidth`, `HeightPostSoft`,
  `HeightPostBrightness`); keys in `[Overlays]`.
- Module `hulls` also draws the Evolver and its cocoon (whose burst is render3d's
  debris), holograms as the class they show in a shader of scanlines, flicker and an
  edge glow (`ps_holo`), and the build ghost as stock's pale layered hull
  (`[Hulls] BuildGhost=1`, `GhostDepth=`); `Reload=1` is read while the game runs. The
  log names, once per class, any object no module draws.
- Module `effects` also draws construction (the station rising face by face in the
  hull look, each edge welded with an arc, hot spot, sparks and a seam cooling from
  white to red, lighting the hull; `[Construction]`), the worker bees and drones as
  lit models with a crackling welding beam (`[Bees]`), every spark that is not a weld
  (systems blowing up, phaser flares, beam hits) with stock's sprite and a burst of
  streaking sparks and a flash (`[Sparks]`), and a cancelled construction's debris.
- `Timing=1` (`TimingEvery=`): CPU and GPU time per view and per step, CPU zones per
  module, and any view over 8 ms with its steps, in `Render3D.log`; with `Enable=0` it
  times the engine's own view, which `perf/compare.sh` uses to measure render3d against
  the plugins it replaces (`PERF.md`).
- Module `specials` (on, after `particles`): `Step::Specials`, the timed special-object
  effects (Computer Override, Energy Dissipator, Psionic Disruption): their cloud
  emitters as render3d's particles, plus a shader layer over the struck hull (Borg
  circuitry, a film of veins, violet ripples) and a light at each cloud; an effect it
  does not know is the engine's; keys in `[Specials]`.

### Changed
- The plugins the modules replace were removed from the repository (MSAA, Lighting,
  Sky, Nebulae, Surfaces, Explosions, Shields, Repair, Venting, Specials, Fog, and
  models' `Planets.asi`); a module turned off leaves its part to the engine. The
  recipes moved here: `skies/` (built in), `surfaces/` and `nebulae/` (installed into
  the game's `Surfaces\` and `Nebulae\`); `perf/sky-grade.py` was `sky/grade.py`.
  `install.sh` takes out any old plugin an earlier install left, and Sky.asi's recipe
  copies.
- The state saved around each draw is a recorded block of what our draws change, not
  the whole device state (`gfx::Guard`): render3d's CPU time in a battle 4.7 -> 1.3 ms
  a view, below the plugins' 1.8 ms (`PERF.md`).
- Ini files are kept parsed and read again only when they change: the sky's
  once-a-second re-read cost a ~20 ms hitch under Wine.
- `install.sh` removes each plugin a module that is on replaces (effects: Explosions,
  Shields, Repair; ordnance: Specials; sky: Sky; overlays: Fog), reading `[Modules]`
  from the game's `Render3D.ini`; `./install` runs it last. Its ini merge keeps each
  added key in its own section. The log names each module loaded at start.
- Module `particles` (on): render3d's own particle simulation (spray, cloud and the
  global list, after the engine's rules and quirks, on the sim clock) for every
  emitter node of every object; damage vents in `Venting.asi`'s look with their hull
  lights, other particles, nebula puffs (classes without a volume recipe) and
  billboard effects in stock's; keys in `[Particles]`. `install.sh` also takes out
  `Venting.asi`.
- Module `world` (on): planets on our own sphere in `Lighting.asi`'s planet look with
  `Surfaces.asi`'s grounds (city lights by `cfgPOP_HEAVY`), asteroid fields and
  dilithium moons with their recipe surfaces, nebulae as `Nebulae.asi`'s volumes with
  ships' shadows from the hulls module, and the planet and nebula lights
  `Lighting.asi` added; keys in `[World]`. `install.sh` takes out `Surfaces.asi` and
  `Nebulae.asi` (and copies their recipes into the game), and `Lighting.asi` while
  hulls and world are both on.

### Fixed
- An object's SOD is its ODF's `baseName` when it sets one (`assets::sod_of`), as the
  engine reads it: Martok's Negh'Var (`kbattle`), Caddebostan's ship, the Ferengi
  scavenger, the 8472 hero mothership, Martok's base and the prison were left to the
  engine (stock light and cloak), as were their emitters, debris, ghosts and holograms.

Confirmed in game 2026-10-17.

## 0.1.0 — 2026-10-15

### Added
- `Render3D.asi`: both calls of `Armada_RenderAllOurStuff` (0x59886e, the main view;
  0x4e60f6, the HUD's action camera) go through render3d. The engine's steps draw into
  a half-float target with 4x MSAA, which is resolved, bloomed by energy and
  tone-mapped into the back buffer at the view's rectangle; the engine's depth is
  cleared there for the HUD. `Render3D.ini`: `Enable`, `Log`, `MSAA`, `HDR`, `Bloom`,
  `BloomThreshold`, `BloomKnee`, `Exposure`, `Shoulder`, `Inset`.
- `install.sh` builds it (rustup, `i686-pc-windows-msvc`) and removes `MSAA.asi`, which
  it supersedes; `./install` runs it in MSAA's place, and `a2mod` switches it as the
  `render3d` layer.

Confirmed in game 2026-10-17.
