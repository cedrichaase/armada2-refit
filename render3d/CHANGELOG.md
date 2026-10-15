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
- Module `hulls` (off by default): ships and stations from their SODs.
- Module `effects` (on): explosions (`Explosions.asi`'s fireball and ring, the core
  blooming by energy in HDR), shield hits and the shields-down arcs (`Shields.asi`),
  repair welding (`Repair.asi`), and hull debris as render3d's own simulation; keys in
  `[Explosions]`, `[Shields]`, `[Repair]`, `[Debris]`.
- Module `ordnance` (on): every shot and the warp flash: phasers, pulses, torpedoes,
  plasma, polerons and artillery in stock's look (near-white texels lifted past white,
  `Hot`/`HotFrom`), the special weapons' beams, bolts, fields and waves and the warp
  flash from `Specials.asi`'s recipes (`ordnance.ini`, built in; Render3D.ini sections
  of the same name override), torpedo lights recoloured and phaser lights as
  `Lighting.asi` makes them. Mining beams, mines and model-drawn specials without a
  recipe stay the engine's.
- Module `sky` (on): all of `Background_Render`: `Sky.asi`'s recipe skies (built in
  from `sky/skies/`; a `Sky\<name>.ini` in the game directory wins), baked on the GPU
  into a half-float cube; stock faces where there is no recipe; the starfield from the
  engine's octree, point stars lifted into HDR (`StarLift`); background planets and
  suns (`SunLift`); keys in `[Sky]`.
- Module `overlays` (on): the fog of war and shroud with `Fog.asi`'s smooth edges over
  the engine's own scan textures, the tactical grid, the selection bubbles and the
  fog-of-war ghosts of stations; keys in `[Overlays]`. The build ghost and footprints
  stay the engine's.

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
