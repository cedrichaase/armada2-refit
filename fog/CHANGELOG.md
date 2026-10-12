# Changelog — fog

`Fog.asi`: the fog of war and the shroud in the 3D view drawn with smooth edges.
Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first.
The reasoning is in [`README.md`](README.md).

## 1.0.0 — 2026-10-12

### Added
- `Fog.asi`: the calls to `ST3D_Node::Render` for the fog and the shroud in
  `Terrain_Geometry::Render` are chained, and the one `DrawIndexedPrimitive` inside each
  goes through a pixel shader that reconstructs the scan grid's mask with a cubic filter
  and draws its edge as an antialiased smoothstep; the shroud is blended instead of
  alpha-tested. The engine's quad, texture, tint and opacity are kept. Without a
  Direct3D 9 device behind d3d8, the stock layers.
- `Fog.ini`: `Enable`, `Log`, `Reload`, `Fog`, `Shroud`, `Filter` (`bspline`,
  `catmull-rom`), `FogThreshold`, `FogSoftness`, `ShroudThreshold`, `ShroudSoftness`,
  `Antialias`.
- `install.sh` (`--remove`); `./install` runs it, `a2mod` switches it as the `fog` layer.

Installed, not yet seen in game.
