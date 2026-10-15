# Changelog — render3d

`Render3D.asi`: the 3D view drawn through render3d, in Rust. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The reasoning is
in [`README.md`](README.md), the plan in [`PLAN.md`](PLAN.md).

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
- Module `specials` (on, after `particles`): `Step::Specials`, the timed special-object
  effects (Computer Override, Energy Dissipator, Psionic Disruption): their cloud
  emitters as render3d's particles, plus a shader layer over the struck hull (Borg
  circuitry, a film of veins, violet ripples) and a light at each cloud; an effect it
  does not know is the engine's; keys in `[Specials]`.

### Changed
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
