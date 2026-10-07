# Changelog — lighting

`Lighting.asi`: ships and stations on the engine's static vertex buffers, scene lights
in place of each map's own, and light sources. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.17.2 — 2026-10-07

### Fixed
- Whole hulls (a Steamrunner on the bench, an Akira's top in the first mission) went dark
  under `Shadows=1`. A draw with alpha blending or alpha testing on (an overlay shell, a
  decal) was drawn into the shadow map as an opaque hull; such draws now cast nothing,
  and still take the shadow.

Installed, not yet seen in game.

## 1.17.1 — 2026-10-07

### Fixed
- Ships and stations went dark, one at a time and from frame to frame, with `Shadows=1`.
  The engine renders the world twice a frame (the main view and the selection's 3D
  portrait), and the shadow map of each was made from the other's hulls. The hull
  lists, and the "previous frame" they stand for, are now kept per camera.

Installed, not yet seen in game.

## 1.17.0 — 2026-10-07

### Added
- `NearFade=1`, `NearFadeDepth=0.3`, `NearFadeMin=0.125`: a ship or station fades only
  once the near clipping plane cuts into its bounding box, not once it fills half the
  view, down to `NearFadeMin` when the plane has cut `NearFadeDepth` of the box's depth.

### Fixed
- A faded ship or station (any fade below 1, however faint) went to the CPU path and lost
  the shaders until the camera drew back. With `Shaders=1` it stays in the shaders and is
  drawn blended at the device's `Flush`, farthest first, front surface only (`misc.w`).

Confirmed in game 2026-10-07.

## 1.16.0 — 2026-10-07

### Added
- The Borg profile: with `BumpShaders=1`, bump-mapped hulls (the Borg) take
  `BorgAmbient=0.015 0.018 0.015`, `BorgSpecular=0.15`, `BorgSpecularPower=64`,
  `BorgRimLight=0.06 0.13 0.03` (a sickly green), `BorgSun=0.8` and
  `BorgSelfIllumination` (left out: `SelfIllumination`) in place of `Ambient`,
  `Specular`, `SpecularPower`, `RimLight`, `HullSun` and `SelfIllumination`. Darker
  plating, the night lights carrying the look.

Seen on the bench 2026-10-07 (`SCENE=firing`, weapons off; the cube beside `mnebula8`).
Confirmed in game 2026-10-07.

## 1.15.0 — 2026-10-07

### Added
- `Shadows=1`, `ShadowSize=2048`, `ShadowStrength=1.0`: with `Shaders=1`, ships and
  stations cast the Key's shadow on themselves and each other. Every hull draw in the
  shaders is drawn again next frame into an R32F depth map from the Key (`hull.hlsl`'s
  `depth_vs`/`depth_ps`), fitted round the hulls on screen; each hull looks itself up at
  the pose it had then, so its shadow on itself does not lag. 3x3 soft lookup, normal
  offset. The d3d8 device's `Reset` is wrapped to let the map go first.
- `PlanetShadows=1`: a planet between a hull and the Key shadows it, tested as a sphere
  in the hull shaders.

Seen on the bench 2026-10-07 (a shadow scene: Galaxy, a destroyer behind the planet,
a shipyard, a Borg cube, a destroyer under way; `SCENE=firing`). Confirmed in game 2026-10-07.

## 1.14.0 — 2026-10-07

### Added
- `Phasers=1`: a phaser lights the firing ship where its beam leaves the hull, while
  the beam is drawn: a hot spot in the beam's own colour (read from its sprite texture)
  times `PhaserBrightness=4.5`, full to `PhaserStart=0`, gone at `PhaserRange=24`,
  lifted `PhaserLift=3` units out along the beam. In the hull shaders its diffuse term
  wraps round by `PhaserWrap=0.6`, so the plating it grazes takes it, and its falloff
  is raised to `PhaserFalloff=2` (`hull.hlsl`, `pfall.z`/`.w`).
- `PhaserImpact=2.0`: the beam's end lights the target where it strikes, in the beam's
  colour times this (`0` none), full to `PhaserImpactStart=6`, gone at
  `PhaserImpactRange=70`, lifted `PhaserImpactLift=8` units back along the beam.

Seen on the bench 2026-10-07 (`SCENE=firing`, dorsal and ventral emitters). Confirmed in game 2026-10-07.

## 1.13.0 — 2026-10-07

### Changed
- `CityLights`: a developed planet's night side draws its towns as a web of light, in
  place of one flat colour over each town: streets and finer lanes beaded with lights,
  highways reaching out between towns, single lights past their edges, a glow and
  brighter downtowns, all generated in `planet.hlsl`'s `city_ps` from the development
  texture and the population map. Fades to an even glow where a line is under a pixel.

Seen on the bench 2026-10-07 (a class M planet settled by a colony ship). Confirmed in game 2026-10-07.

## 1.12.0 — 2026-10-07

### Added
- `OrdnanceColours=1`: a torpedo's or pulse's light takes the hue of its own sprite
  (the mean of its first frame in the installed texture) at the ODF's brightness, in
  place of the ODF's `lightColor` (cyan on every Federation photon, green on nearly every
  other weapon). Federation orange, Klingon red, Borg cyan, Cardassian yellow, Romulan
  green, Species 8472 yellow-green. `0` keeps the ODF colours.

Seen on the bench 2026-10-07 (`SCENE=factions`). Confirmed in game 2026-10-07.

## 1.11.0 — 2026-10-07

### Added
- `BumpShaders=1`: with `Shaders=1` under d3d8to9, bump-mapped hulls (the Borg, and any
  hull `models/hull-bump.py` patched) are drawn once in `hull.hlsl`'s new
  `bump_vs`/`bump_ps` instead of the engine's dot3 passes, taking `Ambient`, point lights
  at their positions, specular, rim, the night lights and the highlight shoulder. The
  normal is the SOD's vertex normal tilted by the engine's normal map. Slot 3 of the
  `ST3D_Dot3_MeshVB` vtable; `Lighting.log` reads `call sites patched 13`.

Seen on the bench 2026-10-06 (`SCENE=firing`, the Borg cube; `SCENE=planet`, a Galaxy with
`hull-bump.py`). Confirmed in game 2026-10-07.

## 1.10.0 — 2026-10-05

### Added
- `HullSun=1.1`: with `Shaders=1`, the Key on GPU-drawn hulls times this; `hull.hlsl` no
  longer clamps the summed light at 1, so explosions and torpedoes beside a hull can
  drive it past its texture.
- `HighlightKnee=0.8`: in the hull and planet shaders, colour above it rolls off towards
  white instead of clipping; `1` clips as before.
- `SelfIllumination` above 1 draws the night lights brighter than their texture.

### Changed
- `SelfIllumination=1.8` (was 1.0), `Specular=0.7` (was 0.35); the built-in defaults
  match.

Seen on the bench 2026-10-05 (`SCENE=planet`, `SCENE=firing`). Confirmed in game 2026-10-05.

## 1.9.0 — 2026-10-05

### Added
- `PlanetShaders=1`: under d3d8to9, planets and their cloud shells are drawn from the
  engine's own arrays through shaders (`planet.hlsl`, built into the committed
  `planet_shaders.h`) instead of the CPU path: lit per pixel with the sphere's own
  normal, a night side for the clouds too, a warm terminator (`PlanetWrap`,
  `PlanetDusk`), the atmosphere at the limb (`PlanetHaze`, `PlanetHazeColour`,
  `PlanetHazePower`), a glint off water (`PlanetGlint`, `PlanetGlintPower`), city lights
  at night (`CityLights`, `CityLightColour`), `PlanetFill` for the lights other than
  the Key and `PlanetSun` for the Key.
- Shipped `Lighting.ini`: `PlanetAmbient=0.018 0.018 0.027` (was 0.02 0.02 0.03). `Lighting.log` reads `call sites patched 12`.

Seen on the bench 2026-10-05 (`SCENE=planet`). Confirmed in game 2026-10-05.

## 1.8.0 — 2026-10-05

### Added
- `Specular=0.35`, `SpecularPower=24`: with `Shaders=1`, a highlight from every light,
  times the texture's brightness as a gloss mask.
- `RimLight=0.12 0.14 0.20`, `RimPower=3.0`: with `Shaders=1`, light on the faces turned
  edge-on to the camera, so a hull's dark side keeps its outline (871c985).

Seen on the bench 2026-10-05 (`SCENE=planet`, `SCENE=firing`). Confirmed in game 2026-10-05.

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
