# Changelog — lighting

`Lighting.asi`: ships and stations on the engine's static vertex buffers, and two scene
lights in place of each map's own. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-04

### Added
- `Lighting.asi`, `Lighting.ini` (`GPU`, `Lights`, `KeyColour`, `KeyAxis`, `FillColour`,
  `FillAxis`, `Ambient`, `FixMirrored`, `Log`): every game object type's model drawn
  through the engine's static vertex buffers with a neutral material and normalized
  normals, mirrored meshes lit the right way round, and the map's lights replaced by a
  key and a fill (8f71677, b6179ff). `install.sh` (`--remove`); `./install` runs it and
  `a2mod` switches it. Not in the release package or the installers yet.

Confirmed in game 2026-10-04.
