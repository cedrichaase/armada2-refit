# Changelog — surfaces

`Surfaces.asi`: planets' and the dilithium moon's surfaces made from recipes, drawn by
`Lighting.asi`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The reasoning is in [`README.md`](README.md).

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
