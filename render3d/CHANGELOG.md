# Changelog — render3d

`Render3D.asi`: the 3D view drawn through render3d, in Rust. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The reasoning is
in [`README.md`](README.md), the plan in [`PLAN.md`](PLAN.md).

## 0.7.1 — 2026-10-16

### Fixed
- Planets, moons, asteroid fields and nebulae drawn by the world module are marked on
  screen (the byte `Entity::Render` writes), so the pointer picks them again: a planet
  could not be clicked, nor a colony ship right-clicked onto it, only drag-selected.
- The selection bubbles are not drawn while the engine's `RenderSelectionEffects` has
  been made to return at once (UI.asi's `[HUD] SelectionBubbles=0`).

Installed, not yet seen in game.

## 0.7.0 — 2026-10-16

### Added
- A third view: a non-main view from a camera other than the action camera's (UI.asi's
  info-panel close-up) keeps per-view state of its own (`View::key` 2), and is logged as
  "other".

Installed, not yet seen in game.

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

Installed, not yet seen in game.

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

Installed, not yet seen in game.

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

Installed, not yet seen in game.

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

Installed, not yet seen in game.

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

Installed, not yet seen in game.

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

Installed, not yet seen in game.

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

Installed, not yet seen in game.
