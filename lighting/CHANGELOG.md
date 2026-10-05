# Changelog — lighting

`Lighting.asi`: ships and stations on the engine's static vertex buffers, scene lights
in place of each map's own, and light sources. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.8.0 — 2026-10-05

### Added
- `Specular=0.35`, `SpecularPower=24`: with `Shaders=1`, a highlight from every light,
  times the texture's brightness as a gloss mask.
- `RimLight=0.12 0.14 0.20`, `RimPower=3.0`: with `Shaders=1`, light on the faces turned
  edge-on to the camera, so a hull's dark side keeps its outline.

Seen on the bench 2026-10-05 (`SCENE=planet`, `SCENE=firing`). Installed, not yet seen in game.

## 1.7.0 — 2026-10-05

### Added
- `SelfIllumination=1.0`: with `Shaders=1`, a hull whose material is an
  `ST3D_SelfIlluminatingMaterial` shows its night lights (the texture's alpha: windows,
  nacelle grilles, bussards) as the CPU path's second pass does; the GPU path had dropped
  that pass. `0` leaves them dark (5ca6262).

Seen on the bench 2026-10-05 (`SCENE=planet`, against the CPU path). Confirmed in game 2026-10-05.

## 1.6.0 — 2026-10-05

### Changed
- With `Shaders=1`, point lights reach the shaders at their real positions, against the
  inward normal turned round, with the engine's falloff per pixel: torpedo and pulse
  lights now light the side of a hull facing them, and mirrored meshes get point lights.
  Direct3D's slots (any draw that falls back) are as before (18e2677).
- `PointLights=12` (was 6): up to 16 per draw in the shaders, still 6 in Direct3D's slots.
- `Lighting.log` names the stage setup of a draw left fixed-function, and the first draw
  that takes a hard point light or more than 6.

Seen on the bench 2026-10-05 (`SCENE=firing`, `SCENE=planet`). Confirmed in game 2026-10-05.

## 1.5.0 — 2026-10-05

### Added
- `Shaders=1`: under crosire's d3d8to9 (`platform/d3d8-chain.py --use d3d8to9`), the
  GPU-drawn hulls are lit per pixel by a `vs_3_0`/`ps_3_0` pair (`hull.hlsl`, built
  into the committed `hull_shaders.h`) that reproduces the fixed-function lighting with
  the same lights and material. With any other d3d8, or `0`, nothing changes;
  `Lighting.log` says which. Built with `platform/d3d9/` (98c75a1).

Seen on the bench 2026-10-05 (`SCENE=planet`, both chains). Confirmed in game 2026-10-05.

## 1.4.0 — 2026-10-04

### Added
- `NebulaCull=3.0`: a nebula counts as on screen while its bounding sphere times this is
  in view (stock: the sphere alone), so zooming in on a ship beside a nebula no longer
  makes the nebula and its light vanish. `Lighting.log` now reads `call sites patched
  11` (c1d3708).

Seen on the bench 2026-10-04 (`SCENE=nebula`, at 2). Confirmed in game 2026-10-04.

## 1.3.3 — 2026-10-04

### Changed
- `PlanetGlow=1.5` (was 1.2), `PlanetGlowRange=6.0` (was 5.0), `NebulaRange=8.0` (was
  5.0, as in 1.3.1). The built-in defaults match.

Confirmed in game 2026-10-04.

## 1.3.2 — 2026-10-04

### Fixed
- Soft point lights (nebulae, planets, explosions) lit the side of a hull turned away
  from them: the stock meshes' normals point inward. They now go to Direct3D mirrored
  through the draw's origin. Torpedo and pulse lights are unchanged and still inverted
  (8f0d60f).
- A planet's glow comes from its centre, scaled per object by how much of its day side
  faces it, instead of from the surface under the sun, where the Key light hid it.

### Changed
- `NebulaRange=5.0` (was 8.0 in 1.3.1): a nebula's full-strength reach outdid a nearby
  planet's.

Seen on the bench 2026-10-04 (`SCENE=planet`). Confirmed in game 2026-10-04.

## 1.3.1 — 2026-10-04

### Changed
- Stronger, wider light sources (1.3.0 was too subtle in game): `NebulaBrightness=2.0`
  (was 1.0), `NebulaRange=8.0` (was 5.0), `PlanetGlow=1.2` (was 0.6), `PlanetGlowRange=5.0`
  (was 3.0), `SkyLight=0.35` (was 0.15), `ExplosionBrightness=4.0` (was 2.0),
  `ExplosionRange=10.0` (was 5.0). The built-in defaults match.

Not seen on the bench (it could not start from this session). Confirmed in game 2026-10-04.

## 1.3.0 — 2026-10-04

### Added
- Point lights on the GPU path (`PointLights=`): torpedoes and pulses light the hulls
  they pass again, as stock did on the CPU path, and the light sources below reach
  ships. Hard-edged lights (torpedoes) are lit per vertex; one colour counts once
  (a1cb893).
- `Nebulae=`, `NebulaBrightness=`, `NebulaRange=`: nebulae light their surroundings in
  their glow colour.
- `PlanetGlows=`, `PlanetGlow=`, `PlanetGlowRange=`: a planet's day side lights what is
  near it in the mean colour of its ground texture.
- `SkyLight=`: a faint third directional light in the skybox's dominant colour, from the
  side that shows it.
- `Explosions=`, `ExplosionColour=`, `ExplosionBrightness=`, `ExplosionRange=`: ship and
  station explosions light their surroundings. `Lighting.log` now reads `call sites
  patched 10`.

Seen on the bench 2026-10-04 (`SCENE=nebula`, `SCENE=planet`, a scratch battle scene).
Confirmed in game 2026-10-04.

## 1.2.0 — 2026-10-04

### Changed
- The key light comes in at about 60° off vertical instead of 37° (`KeyAxis=0.50 -0.50
  0.71`, the fill opposite it), so it grazes what the top-down camera sees.
- Darker base lighting, so that the light sources still to come stand out:
  `FillColour=0.06 0.08 0.18` (was `0.10 0.12 0.24`), `Ambient=0.05 0.05 0.07` (was
  `0.10 0.10 0.12`), and `PlanetAmbient=0.02 0.02 0.03` for more contrast between a
  planet's day and night sides.

Seen on the bench 2026-10-04 (`SCENE=planet`). Confirmed in game 2026-10-04.

## 1.1.0 — 2026-10-04

### Added
- `Planets=`, `PlanetAmbient=`, `PlanetDiffuse=`: planets and their cloud shells lit by
  Key and Fill with a night side. The planet material's constant half-white term becomes
  `PlanetAmbient` (default `Ambient`) and its 0.75 diffuse `PlanetDiffuse` (default 1).
  `Lighting.log` now reads `call sites patched 7`.

Seen on the bench 2026-10-04 (`SCENE=planet`).

## 1.0.0 — 2026-10-04

### Added
- `Lighting.asi`, `Lighting.ini` (`GPU`, `Lights`, `KeyColour`, `KeyAxis`, `FillColour`,
  `FillAxis`, `Ambient`, `FixMirrored`, `Log`): every game object type's model drawn
  through the engine's static vertex buffers with a neutral material and normalized
  normals, mirrored meshes lit the right way round, and the map's lights replaced by a
  key and a fill (8f71677, b6179ff). `install.sh` (`--remove`); `./install` runs it and
  `a2mod` switches it. Not in the release package or the installers yet.

Confirmed in game 2026-10-04.
