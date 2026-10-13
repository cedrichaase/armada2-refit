# Changelog — surfaces

`Surfaces.asi`: planets' and the dilithium moon's surfaces made from recipes, drawn by
`Lighting.asi`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The reasoning is in [`README.md`](README.md).

## 1.1.1 — 2026-10-13

### Changed
- The class planets' colours graded against stock on the bench, in the same light: each
  class within 8% of stock's on-screen channel means from two sides (D's red and L's
  blue within 13%), the detail unchanged. `Ramp*`/`Alt*` of `PB_CLSSD`, `PB_CLSSH`,
  `PB_CLSSJ` (pale mint and cream bands, softer peach; `StormColour`), `PB_CLSSK`
  (violet `Alt` regions, `Region` 0.55 → 0.65), `PB_CLSSL` (even orange, less contrast
  across height) and `PB_CLSSM` (turquoise land and sea, `Deep`, `Shallow`, `Shore`
  1.6 → 2.4).
- Cloud as stock's shell has it: `PB_CLSSK` gains `Clouds=`; `PB_CLSSH` and `PB_CLSSL`
  denser and whiter.

Installed, not yet seen in game.

## 1.1.0 — 2026-10-13

### Added
- Recipes `Master1` and `Master2`: the asteroid fields' rock (`Master.SOD`,
  `Master1..7.SOD`), matte grey stone with big and small crater bowls over lumpy ground,
  at `Face=1024`; their bake takes 126–173 ms each.
- Recipe keys `Face=` (the recipe's own cube edge, in place of `Surfaces.ini`'s),
  `Mapping=triplanar` (a model's lookup: each axis's two cube faces across its extents,
  blended by the surface's facing, instead of the direction from its centre) and
  `Gloss=` (a model's highlight times the hulls'; 1 as before).
- `A2Surface.model[4]` in `surfaces.h` (`Mapping=`, `Gloss=`); the struct's size changes,
  so a `Lighting.asi` older than 1.23.0 draws the stock textures.

Installed, not yet seen in game.

## 1.0.1 — 2026-10-12

### Fixed
- Rebuilt with platform 3.4.1's `d9_device()`, which tells a new Direct3D device by the
  d3d9 device rather than the d3d8 device's address, so the bake's shaders are made
  again on a device at a re-used address.

Installed, not yet seen in game.

## 1.0.0 — 2026-10-12

### Added
- `Surfaces.asi`: exports `a2_surface` (`surfaces.h`); bakes a recipe into albedo,
  relief and cloud cubes on its first draw (106–480 ms; the session's first 2–3 s).
- `Surfaces.ini`: `Enable`, `Face`, `CloudFace`, `Reload`, `Log`.
- Recipes `PB_CLSSD`, `PB_CLSSH`, `PB_CLSSJ`, `PB_CLSSK`, `PB_CLSSL`, `PB_CLSSM`, `Mdmoon`.
- `install.sh` (`--remove`); `./install` runs it, and `a2mod` switches `Surfaces.asi`,
  `Surfaces.ini` and `Surfaces/*.ini` as the `surfaces` layer.
- Recipe keys `FissureShape=` (warp, share of borders kept) and `Cut=`: the moon's rock
  split into plates by straight cracks, cut through as stock's are.

Installed, not yet seen in game.
