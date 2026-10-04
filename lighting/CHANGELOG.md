# Changelog — lighting

`Lighting.asi`: ships and stations on the engine's static vertex buffers, and two scene
lights in place of each map's own. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.2.0 — 2026-10-04

### Changed
- The key light comes in at about 60° off vertical instead of 37° (`KeyAxis=0.50 -0.50
  0.71`, the fill opposite it), so it grazes what the top-down camera sees.
- Darker base lighting, so that the light sources still to come stand out:
  `FillColour=0.06 0.08 0.18` (was `0.10 0.12 0.24`), `Ambient=0.05 0.05 0.07` (was
  `0.10 0.10 0.12`), and `PlanetAmbient=0.02 0.02 0.03` for more contrast between a
  planet's day and night sides.

Seen on the bench 2026-10-04 (`SCENE=planet`).

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
